use crate::blocks::*;
use crate::chemistry::Property::*;
use crate::chemistry::*;
use lazy_static::lazy_static;
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
    selectors
        .into_iter()
        .reduce(|left, right| Bind(Box::new(left), Box::new(right)))
        .unwrap()
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

#[derive(Debug)]
pub(crate) struct SpellRule {
    pub(crate) name: &'static str,
    pub(crate) rate: f32,
    pub(crate) selector: SpellSelector,
    pub(crate) effects: Vec<SpellEffect>,
}

lazy_static! {
    pub(crate) static ref NATURAL_RULES: Vec<SpellRule> = vec![
        SpellRule {
            name: "Fire disappears over time",
            rate: 10.05,
            selector: bind([
                Is(BlockProperty(BlockProperties::BURNING)),
                Is(Material(*AIR))
            ]),
            effects: vec![Receive(BlockProperty(BlockProperties::BURNING))],
        },
        // SpellRule {
        //     name: "Fire makes coal start burning",
        //     rate: 0.2,
        //     selector: bind([
        //         Is(BlockProperty(BlockProperties::BURNING)),
        //         Is(Material(*AIR)),
        //         Adjacent,
        //         Is(Material(*COAL)),
        //         // not(Is(BlockProperty(BlockProperties::BURNING)))
        //     ]),
        //     effects: vec![Send(BlockProperty(BlockProperties::BURNING))],
        // },
        // SpellRule {
        //     name: "Coal burns out over time",
        //     rate: 0.01,
        //     spell: basic(
        //         [
        //             Is(Material(*COAL)),
        //             Is(BlockProperty(BlockProperties::BURNING))
        //         ],
        //         [Receive(BlockProperty(BlockProperties::BURNING))]
        //     )
        // },
        // SpellRule {
        //     name: "Burning coal lights the air around it on fire",
        //     rate: 0.2,
        //     spell: basic(
        //         [
        //             Is(Material(*COAL)),
        //             Is(BlockProperty(BlockProperties::BURNING)),
        //             Adjacent,
        //             Is(Material(*AIR)),
        //         ],
        //         [Send(BlockProperty(BlockProperties::BURNING))]
        //     )
        // },
        // SpellRule {
        //     name: "Burning coal transforms into smoke",
        //     rate: 0.005,
        //     spell: basic(
        //         [
        //             Is(Material(*COAL)),
        //             Is(BlockProperty(BlockProperties::BURNING)),
        //         ],
        //         [Send(Material(*SMOKE))]
        //     )
        // },
        // SpellRule {
        //     name: "Smoke disappears over time",
        //     rate: 0.002,
        //     spell: basic([Is(Material(*SMOKE))], [Send(Material(*AIR))])
        // },
        // SpellRule {
        //     name: "Fire and water combine to make air and steam",
        //     rate: 1.0,
        //     spell: Select(
        //         bind([
        //             Is(BlockProperty(BlockProperties::BURNING)),
        //             Is(Material(*AIR))
        //         ]),
        //         Box::new(merge(
        //             basic([], [Receive(BlockProperty(BlockProperties::BURNING))]),
        //             basic([Adjacent, Is(Material(*WATER))], [Send(Material(*STEAM))])
        //         ))
        //     )
        // },
        // SpellRule {
        //     name: "Steam transforms into water over time",
        //     rate: 0.002,
        //     spell: basic([Is(Material(*STEAM))], [Send(Material(*WATER))])
        // },
    ];
}
