use crate::blocks::*;
use crate::cells::*;
use crate::chemistry::Property::*;
use crate::chemistry::*;
use bevy::prelude::Entity;
use lazy_static::lazy_static;
use Spell::*;
use SpellEffect::*;
use SpellSelector::*;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum SpellSelector {
    Adjacent,
    Is(Property),
    Not(Box<SpellSelector>),
    Bind(Box<SpellSelector>, Box<SpellSelector>),
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
