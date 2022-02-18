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
    fn adjacent_map<A, F>(&self, f: F) -> Vec<A>
    where
        F: Fn(Target) -> A,
    {
        let mut targets = vec![];
        match self {
            Block(x, y) => {
                for x2 in -1..2 {
                    for y2 in -1..2 {
                        if x2 != 0 || y2 != 0 {
                            targets.push(f(Block(x + x2, y + y2)));
                        }
                    }
                }
            }
            _ => {}
        }
        targets
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
    fn select(&self, info: &WorldInfo, target: Target) -> Vec<SpellTarget> {
        match self {
            Adjacent => target.adjacent_map(SpellTarget::new),
            Is(property) => {
                if target.get(info, *property) == 0.0 {
                    vec![]
                } else {
                    vec![SpellTarget::new(target)]
                }
            }
            Not(selector) => {
                let other_targets = selector.select(info, target);
                if other_targets.len() == 0 {
                    vec![SpellTarget::new(target)]
                } else {
                    vec![]
                }
            }
            Bind(selectors) => {
                let mut new_targets = vec![SpellTarget::new(target)];
                for selector in selectors {
                    new_targets = new_targets
                        .into_iter()
                        .flat_map(|spell_target| selector.select_spell(info, spell_target))
                        .collect()
                }
                new_targets
            }
        }
    }

    fn select_spell(
        &self,
        info: &WorldInfo,
        spell_target: SpellTarget,
    ) -> impl Iterator<Item = SpellTarget> {
        self.select(info, spell_target.target)
            .into_iter()
            .map(move |result| SpellTarget {
                target: result.target,
                connection: spell_target.connection * result.connection,
            })
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
        info: &WorldInfo,
        target: SpellTarget,
    ) -> Vec<SpellResult<'a>> {
        match self {
            Effects(effects) => vec![SpellResult { target, effects }],
            Select(selector, spell) => selector
                .select_spell(info, target)
                .flat_map(|target| spell.cast(info, target))
                .collect(),
            Merge(spell1, spell2) => {
                let mut results1 = spell1.cast(info, target);
                let mut results2 = spell2.cast(info, target);
                let connection1 = results1.iter().map(|r| r.target.connection).sum::<f32>();
                let connection2 = results2.iter().map(|r| r.target.connection).sum::<f32>();
                let min_connection = f32::min(connection1, connection2);
                if min_connection < 1e-6 {
                    vec![]
                } else {
                    for r in results1.iter_mut() {
                        r.target.connection *= min_connection / connection1;
                    }
                    for r in results2.iter_mut() {
                        r.target.connection *= min_connection / connection2;
                    }
                    results1.extend(results2);
                    results1
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
                    Is(Material(*AIR)),
                    Is(BlockProperty(BlockProperties::BURNING))
                ],
                [Receive(BlockProperty(BlockProperties::BURNING))]
            )
        },
        SpellRule {
            name: "Fire makes coal start burning",
            rate: 0.2,
            spell: basic(
                [
                    Is(Material(*AIR)),
                    Is(BlockProperty(BlockProperties::BURNING)),
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
