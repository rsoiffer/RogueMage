use crate::blocks::*;
use crate::cells::GRID_SIZE;
use crate::chemistry::DynamicProperty::*;
use crate::chemistry::Property::*;
use crate::chemistry::StaticProperty::*;
use crate::chemistry::*;
use lazy_static::lazy_static;
use Spell::*;
use SpellEffect::*;
use SpellSelector::*;
use Target::*;

impl Target {
    fn for_each_adjacent<F: FnMut(Target)>(&self, info: &WorldInfo, mut f: F) {
        match self {
            Block(x, y) => {
                for x2 in -1..2 {
                    for y2 in -1..2 {
                        if x2 != 0 || y2 != 0 {
                            if x + x2 >= 0
                                && x + x2 < GRID_SIZE as i32
                                && y + y2 >= 0
                                && y + y2 < GRID_SIZE as i32
                            {
                                f(Block(x + x2, y + y2));
                            }
                        }
                    }
                }
                for (&entity, collider) in info.entity_colliders.iter() {
                    if collider.intersects(&AABBCollider::from_block(*x, *y)) {
                        f(Entity(entity));
                    }
                }
            }
            Entity(entity) => {
                let collider = info.entity_colliders.get(&entity).unwrap();
                for x in (collider.ll.x.floor() as i32)..=(collider.ur.x.ceil() as i32) {
                    for y in (collider.ll.y.floor() as i32)..=(collider.ur.y.ceil() as i32) {
                        if x >= 0 && x < GRID_SIZE as i32 && y >= 0 && y < GRID_SIZE as i32 {
                            f(Block(x, y));
                        }
                    }
                }
                for (&entity2, collider2) in info.entity_colliders.iter() {
                    if *entity != entity2 {
                        if collider.intersects(collider2) {
                            f(Entity(entity2));
                        }
                    }
                }
            }
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub(crate) struct SpellTarget {
    pub(crate) target: Target,
    pub(crate) connection: f32,
}

impl SpellTarget {
    pub(crate) fn new(target: Target) -> SpellTarget {
        SpellTarget {
            target,
            connection: 1.0,
        }
    }
}

#[derive(Debug)]
pub(crate) enum SpellSelector {
    Adjacent,
    Is(Property),
    Not(Box<SpellSelector>),
    Bind(Box<SpellSelector>, Box<SpellSelector>),
}

impl SpellSelector {
    fn select(&self, info: &WorldInfo, target: Target, f: &mut dyn FnMut(SpellTarget)) {
        match self {
            Adjacent => target.for_each_adjacent(info, |a| f(SpellTarget::new(a))),
            Is(property) => {
                if info.get(target, *property) != 0.0 {
                    f(SpellTarget::new(target))
                }
            }
            Not(selector) => {
                let mut has_other_targets = false;
                selector.select(info, target, &mut |_| has_other_targets = true);
                if !has_other_targets {
                    f(SpellTarget::new(target))
                }
            }
            Bind(left, right) => SpellSelector::bind(info, target, left, right, f),
        }
    }

    fn bind(
        info: &WorldInfo,
        target: Target,
        left: &Box<SpellSelector>,
        right: &Box<SpellSelector>,
        f: &mut dyn FnMut(SpellTarget),
    ) {
        left.select_spell(info, SpellTarget::new(target), &mut |new_target| {
            right.select_spell(info, new_target, f)
        });
    }

    fn select_spell(
        &self,
        info: &WorldInfo,
        spell_target: SpellTarget,
        f: &mut dyn FnMut(SpellTarget),
    ) {
        self.select(info, spell_target.target, &mut |result| {
            f(SpellTarget {
                target: result.target,
                connection: spell_target.connection * result.connection,
            })
        })
    }
}

fn bind<I>(selectors: I) -> SpellSelector
where
    I: IntoIterator<Item = SpellSelector>,
{
    selectors
        .into_iter()
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .reduce(|acc, item| Bind(Box::new(item), Box::new(acc)))
        .unwrap()
}

fn not(spell: SpellSelector) -> SpellSelector {
    Not(Box::new(spell))
}

#[derive(Debug)]
pub(crate) enum SpellEffect {
    Summon,
    Add(Property),
    Remove(Property),
    Send(Property),
    Receive(Property),
}

// impl SpellEffect {
//     fn is_valid(&self, info: &WorldInfo, source: Target, target: Target) -> bool {
//         match self {
//             Summon => todo!(),
//             Send(property) => target.get(info, *property) < 1.0,
//             Receive(property) => source.get(info, *property) < 1.0,
//         }
//     }
// }

#[derive(Debug)]
pub(crate) struct SpellResult<'a> {
    pub(crate) target: SpellTarget,
    pub(crate) effects: &'a Vec<SpellEffect>,
}

#[derive(Debug)]
pub(crate) enum Spell {
    Effects(Vec<SpellEffect>),
    Select(SpellSelector, Box<Spell>),
}

impl Spell {
    pub(crate) fn cast<'a>(
        &'a self,
        info: &WorldInfo,
        target: SpellTarget,
        f: &mut impl FnMut(SpellResult<'a>),
    ) {
        match self {
            Effects(effects) => f(SpellResult { target, effects }),
            Select(selector, spell) => Spell::cast_select(info, target, selector, spell, f),
        }
    }

    fn cast_select<'a>(
        info: &WorldInfo,
        target: SpellTarget,
        selector: &SpellSelector,
        spell: &'a Box<Spell>,
        f: &mut impl FnMut(SpellResult<'a>),
    ) {
        selector.select_spell(info, target, &mut |target| spell.cast(info, target, f))
    }
}

fn basic<I1, I2>(selectors: I1, effects: I2) -> Spell
where
    I1: IntoIterator<Item = SpellSelector>,
    I2: IntoIterator<Item = SpellEffect>,
{
    let mut selectors = selectors.into_iter().peekable();
    let effects = Effects(effects.into_iter().collect());

    if selectors.peek().is_some() {
        Select(bind(selectors), Box::new(effects))
    } else {
        effects
    }
}

#[derive(Debug)]
pub(crate) struct SpellRule {
    pub(crate) name: &'static str,
    pub(crate) rate: f32,
    pub(crate) drain: Option<ManaId>,
    pub(crate) spell: Spell,
}

lazy_static! {
    pub(crate) static ref NATURAL_RULES: Vec<SpellRule> = vec![
        SpellRule {
            name: "Fire disappears over time",
            rate: 0.03,
            drain: None,
            spell: basic(
                [
                    Is(Dynamic(Burning)),
                    not(Is(Material(*COAL))),
                    // Is(Material(*AIR))
                ],
                [Receive(Dynamic(Burning))]
            )
        },
        SpellRule {
            name: "Fire makes coal start burning",
            rate: 0.2,
            drain: None,
            spell: basic(
                [
                    Is(Dynamic(Burning)),
                    Is(Material(*AIR)),
                    Adjacent,
                    Is(Material(*COAL)),
                    not(Is(Dynamic(Burning)))
                ],
                [Send(Dynamic(Burning))]
            ),
        },
        SpellRule {
            name: "Coal burns out over time",
            rate: 0.01,
            drain: None,
            spell: basic(
                [
                    Is(Material(*COAL)),
                    Is(Dynamic(Burning))
                ],
                [Receive(Dynamic(Burning))]
            )
        },
        SpellRule {
            name: "Burning coal lights the air around it on fire",
            rate: 0.2,
            drain: None,
            spell: basic(
                [
                    Is(Material(*COAL)),
                    Is(Dynamic(Burning)),
                    Adjacent,
                    Is(Material(*AIR)),
                ],
                [Send(Dynamic(Burning))]
            )
        },
        SpellRule {
            name: "Burning coal transforms into smoke",
            rate: 0.005,
            drain: None,
            spell: basic(
                [
                    Is(Material(*COAL)),
                    Is(Dynamic(Burning)),
                ],
                [Send(Material(*SMOKE))]
            )
        },
        SpellRule {
            name: "Smoke disappears over time",
            rate: 0.001,
            drain: None,
            spell: basic([Is(Material(*SMOKE))], [Send(Material(*AIR))])
        },
        SpellRule {
            name: "Fire turns water into steam",
            rate: 0.02,
            drain: None,
            spell: basic(
                [
                    Is(Dynamic(Burning)),
                    Is(Material(*AIR)),
                    Adjacent,
                    Is(Material(*WATER)),
                ],
                [Send(Material(*STEAM))]
            ),
        },
        SpellRule {
            name: "Steam transforms into water over time",
            rate: 0.001,
            drain: None,
            spell: basic([Is(Material(*STEAM))], [Send(Material(*WATER))])
        },
        SpellRule {
            name: "Burning materials light adjacent entities on fire",
            rate: 0.1,
            drain: None,
            spell: basic(
                [
                    Is(Dynamic(Burning)),
                    Adjacent,
                    Is(Static(IsEntity)),
                ],
                [Send(Dynamic(Burning))]
            ),
        },
        SpellRule {
            name: "Burning entities light adjacent coal on fire",
            rate: 0.1,
            drain: None,
            spell: basic(
                [
                    Is(Dynamic(Burning)),
                    Is(Static(IsEntity)),
                    Adjacent,
                    Is(Material(*COAL)),
                ],
                [Send(Dynamic(Burning))]
            ),
        },
    ];

    pub(crate) static ref PLAYER_RULES: Vec<SpellRule> = vec![
        SpellRule {
            name: "Create water",
            rate: f32::INFINITY,
            drain: Some(ManaId(0)),
            spell: basic(
                [
                    Adjacent,
                    Is(Material(*AIR)),
                ],
                [Add(Material(*WATER))],
            )
        },
        SpellRule {
            name: "Launch fireball",
            rate: f32::INFINITY,
            drain: Some(ManaId(1)),
            spell: basic(
                [],
                [
                    Summon,
                    Add(Dynamic(Forwards)),
                    Add(Dynamic(Mana(ManaId(2)))),
                ],
            )
        },
        SpellRule {
            name: "Fireball",
            rate: f32::INFINITY,
            drain: Some(ManaId(2)),
            spell: basic(
                [
                    Adjacent,
                    not(Is(Material(*AIR))),
                    Adjacent, // TODO: Area(5)
                ],
                [Add(Material(*FIRE))],
            )
        },
    ];
}
