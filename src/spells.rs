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

type BoxIter<'a, A> = Box<dyn Iterator<Item = A> + 'a>;

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
    fn adjacent_map<'a, A, F>(self, f: &'a F) -> BoxIter<'a, A>
    where
        F: Fn(Target) -> A,
        A: 'a,
    {
        match self {
            Block(x, y) => Box::new((-1..2).flat_map(move |x2| {
                (-1..2).filter_map(move |y2| {
                    if x2 != 0 || y2 != 0 {
                        Some(f(Block(x + x2, y + y2)))
                    } else {
                        None
                    }
                })
            })),
            _ => Box::new(vec![].into_iter()),
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
    Adjacent,
    Is(Property),
    Not(Box<SpellSelector>),
    Bind(Vec<SpellSelector>),
}

impl SpellSelector {
    fn select<'a>(&'a self, info: &'a WorldInfo, target: Target) -> BoxIter<'a, SpellTarget> {
        match self {
            Adjacent => target.adjacent_map(&SpellTarget::new),
            Is(property) => {
                if target.get(info, *property) == 0.0 {
                    Box::new(vec![].into_iter())
                } else {
                    Box::new(vec![SpellTarget::new(target)].into_iter())
                }
            }
            Not(selector) => {
                let other_targets = selector.select(info, target).collect::<Vec<_>>();
                if other_targets.len() == 0 {
                    Box::new(vec![SpellTarget::new(target)].into_iter())
                } else {
                    Box::new(vec![].into_iter())
                }
            }
            Bind(selectors) => {
                let mut new_targets: BoxIter<'a, SpellTarget> =
                    Box::new(vec![SpellTarget::new(target)].into_iter());
                for selector in selectors {
                    // if new_targets.len() == 0 {
                    //     break;
                    // }
                    new_targets = Box::new(
                        new_targets
                            .flat_map(|spell_target| selector.select_spell(info, spell_target)),
                    )
                }
                new_targets
            }
        }
    }

    fn select_spell<'a>(
        &'a self,
        info: &'a WorldInfo,
        spell_target: SpellTarget,
    ) -> BoxIter<'a, SpellTarget> {
        Box::new(
            self.select(info, spell_target.target)
                .map(move |result| SpellTarget {
                    target: result.target,
                    connection: spell_target.connection * result.connection,
                }),
        )
    }
}

fn bind<I>(selectors: I) -> SpellSelector
where
    I: IntoIterator<Item = SpellSelector>,
{
    Bind(selectors.into_iter().collect::<Vec<_>>())
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
        info: &'a WorldInfo<'a>,
        target: SpellTarget,
    ) -> BoxIter<SpellResult<'a>> {
        match self {
            Effects(effects) => Box::new(vec![SpellResult { target, effects }].into_iter()),
            Select(selector, spell) => Box::new(
                selector
                    .select_spell(info, target)
                    .flat_map(|target| spell.cast(info, target)),
            ),
            Merge(spell1, spell2) => {
                let mut results1 = spell1.cast(info, target).collect::<Vec<_>>();
                let mut results2 = spell2.cast(info, target).collect::<Vec<_>>();
                let connection1 = results1.iter().map(|r| r.target.connection).sum::<f32>();
                let connection2 = results2.iter().map(|r| r.target.connection).sum::<f32>();
                let min_connection = f32::min(connection1, connection2);
                if min_connection < 1e-6 {
                    Box::new(vec![].into_iter())
                } else {
                    for r in results1.iter_mut() {
                        r.target.connection *= min_connection / connection1;
                    }
                    for r in results2.iter_mut() {
                        r.target.connection *= min_connection / connection2;
                    }
                    results1.extend(results2);
                    Box::new(results1.into_iter())
                }
            }
        }
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
            rate: 0.05,
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
                    not(Is(BlockProperty(BlockProperties::BURNING)))
                ],
                [Send(BlockProperty(BlockProperties::BURNING))]
            ),
        },
        SpellRule {
            name: "Coal burns out over time",
            rate: 0.01,
            spell: basic(
                [
                    Is(Material(*COAL)),
                    Is(BlockProperty(BlockProperties::BURNING))
                ],
                [Receive(BlockProperty(BlockProperties::BURNING))]
            )
        },
        SpellRule {
            name: "Burning coal lights the air around it on fire",
            rate: 0.2,
            spell: basic(
                [
                    Is(Material(*COAL)),
                    Is(BlockProperty(BlockProperties::BURNING)),
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
                    Is(Material(*COAL)),
                    Is(BlockProperty(BlockProperties::BURNING)),
                ],
                [Send(Material(*SMOKE))]
            )
        },
        SpellRule {
            name: "Smoke disappears over time",
            rate: 0.002,
            spell: basic([Is(Material(*SMOKE))], [Send(Material(*AIR))])
        },
        SpellRule {
            name: "Fire and water combine to make air and steam",
            rate: 1.0,
            spell: Select(
                bind([
                    Is(BlockProperty(BlockProperties::BURNING)),
                    Is(Material(*AIR))
                ]),
                Box::new(merge(
                    basic([], [Receive(BlockProperty(BlockProperties::BURNING))]),
                    basic([Adjacent, Is(Material(*WATER))], [Send(Material(*STEAM))])
                ))
            )
        },
        SpellRule {
            name: "Steam transforms into water over time",
            rate: 0.002,
            spell: basic([Is(Material(*STEAM))], [Send(Material(*WATER))])
        },
    ];
}
