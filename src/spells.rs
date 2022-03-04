use crate::blocks::*;
use crate::cells::*;
use crate::chemistry::Property::*;
use crate::chemistry::*;
use bevy::prelude::Entity;
use lazy_static::lazy_static;
use Spell::*;
use SpellEffect::*;
use SpellSelector::*;
use Target::*;

pub(crate) struct WorldInfo<'a> {
    pub(crate) grid: &'a BlockGrid,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum Target {
    Block(i32, i32),
    Entity(Entity),
    NewSummon,
}

impl Target {
    fn for_each_adjacent<F: FnMut(Target)>(&self, mut f: F) {
        match self {
            Block(x, y) => {
                for x2 in -1..2 {
                    for y2 in -1..2 {
                        if x2 != 0 || y2 != 0 {
                            f(Block(x + x2, y + y2))
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn get(self, info: &WorldInfo, property: Property) -> f32 {
        match self {
            Block(x, y) => match info.grid.get(x, y) {
                Some(block) => block.get_prop(property),
                None => 0.0,
            },
            _ => todo!(),
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
    Identity,
    Adjacent,
    Is(Property),
    Not(Box<SpellSelector>),
    Bind(Box<SpellSelector>, Box<SpellSelector>),
}

impl SpellSelector {
    fn select(&self, info: &WorldInfo, target: Target, f: &mut dyn FnMut(SpellTarget)) {
        match self {
            Identity => f(SpellTarget::new(target)),
            Adjacent => target.for_each_adjacent(|a| f(SpellTarget::new(a))),
            Is(property) => {
                if target.get(info, *property) != 0.0 {
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
        .unwrap_or(Identity)
}

fn not(spell: SpellSelector) -> SpellSelector {
    Not(Box::new(spell))
}

#[derive(Debug)]
pub(crate) enum SpellEffect {
    Summon,
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
    Merge(Box<Spell>, Box<Spell>),
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
            Merge(spell1, spell2) => {
                let (mut results1, mut results2) = Spell::merge(info, target, spell1, spell2);
                for r in results1.drain(..) {
                    f(r);
                }
                for r in results2.drain(..) {
                    f(r);
                }
            }
        }
    }

    fn merge<'a>(
        info: &WorldInfo,
        target: SpellTarget,
        spell1: &'a Box<Spell>,
        spell2: &'a Box<Spell>,
    ) -> (Vec<SpellResult<'a>>, Vec<SpellResult<'a>>) {
        let mut results1 = vec![];
        let mut results2 = vec![];
        spell1.cast(info, target, &mut |t| results1.push(t));
        spell2.cast(info, target, &mut |t| results2.push(t));

        let connection1 = results1.iter().map(|r| r.target.connection).sum::<f32>();
        let connection2 = results2.iter().map(|r| r.target.connection).sum::<f32>();
        let min_connection = f32::min(connection1, connection2);

        if min_connection < 1e-6 {
            (vec![], vec![])
        } else {
            for r in results1.iter_mut() {
                r.target.connection *= min_connection / connection1;
            }

            for r in results2.iter_mut() {
                r.target.connection *= min_connection / connection2;
            }

            (results1, results2)
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
    Select(
        bind(selectors),
        Box::new(Effects(effects.into_iter().collect::<Vec<_>>())),
    )
}

fn merge(spell1: Spell, spell2: Spell) -> Spell {
    Merge(Box::new(spell1), Box::new(spell2))
}

#[derive(Debug)]
pub(crate) struct SpellRule {
    pub(crate) name: &'static str,
    pub(crate) rate: f32,
    pub(crate) spell: Spell,
}

lazy_static! {
    pub(crate) static ref NATURAL_RULES: Vec<SpellRule> = vec![
        SpellRule {
            name: "Fire disappears over time",
            rate: 0.03,
            spell: basic(
                [
                    Is(BlockProperty(BlockProperties::BURNING)),
                    Is(Material(*AIR))
                ],
                [Receive(BlockProperty(BlockProperties::BURNING))]
            )
        },
        SpellRule {
            name: "Fire makes coal start burning",
            rate: 0.2,
            spell: basic(
                [
                    Is(BlockProperty(BlockProperties::BURNING)),
                    Is(Material(*AIR)),
                    Adjacent,
                    Is(Material(*COAL)),
                    // not(Is(BlockProperty(BlockProperties::BURNING)))
                ],
                [Send(BlockProperty(BlockProperties::BURNING))]
            ),
        },
        SpellRule {
            name: "Coal burns out over time",
            rate: 0.01,
            spell: basic(
                [
                    Is(BlockProperty(BlockProperties::BURNING)),
                    Is(Material(*COAL)),
                ],
                [Receive(BlockProperty(BlockProperties::BURNING))]
            )
        },
        SpellRule {
            name: "Burning coal lights the air around it on fire",
            rate: 0.2,
            spell: basic(
                [
                    Is(BlockProperty(BlockProperties::BURNING)),
                    Is(Material(*COAL)),
                    Adjacent,
                    Is(Material(*AIR)),
                ],
                [Send(BlockProperty(BlockProperties::BURNING))]
            )
        },
        SpellRule {
            name: "Burning coal transforms into smoke",
            rate: 0.005,
            spell: basic(
                [
                    Is(BlockProperty(BlockProperties::BURNING)),
                    Is(Material(*COAL)),
                ],
                [Send(Material(*SMOKE)), Receive(BlockProperty(BlockProperties::BURNING))]
            )
        },
        SpellRule {
            name: "Smoke disappears over time",
            rate: 0.002,
            spell: basic([Is(Material(*SMOKE))], [Send(Material(*AIR))])
        },
        SpellRule {
            name: "Fire and water combine to make air and steam",
            rate: 0.1,
            spell: basic(
                [
                    Is(BlockProperty(BlockProperties::BURNING)),
                    Is(Material(*AIR)),
                    Adjacent,
                    Is(Material(*WATER)),
                ],
                [Send(Material(*STEAM))]
            ),
            // spell: Select(
            //     bind([
            //         Is(BlockProperty(BlockProperties::BURNING)),
            //         Is(Material(*AIR))
            //     ]),
            //     Box::new(merge(
            //         basic([], [Receive(BlockProperty(BlockProperties::BURNING))]),
            //         basic([Adjacent, Is(Material(*WATER))], [Send(Material(*STEAM))])
            //     ))
            // )
        },
        SpellRule {
            name: "Steam transforms into water over time",
            rate: 0.002,
            spell: basic([Is(Material(*STEAM))], [Send(Material(*WATER))])
        },
    ];
}
