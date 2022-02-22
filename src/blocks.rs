use crate::chemistry::*;
use bevy::prelude::Color;
use bitflags::bitflags;
use lazy_static::lazy_static;

bitflags! {
    #[derive(Default)]
    pub(crate) struct BlockProperties: u32 {
        /// Has this block already moved this step
        const MOVED_THIS_STEP = 1 << 0;
        /// Has this block settled into a stable state - can only be true for powders
        const POWDER_STABLE = 1 << 1;
        /// Is this block currently on fire
        const BURNING = 1 << 2;
        /// Has this block changed at all this step
        const CHANGED_THIS_STEP = 1 << 3;
    }
}

impl BlockProperties {
    pub(crate) fn iter_all() -> impl Iterator<Item = BlockProperties> {
        [
            BlockProperties::MOVED_THIS_STEP,
            BlockProperties::POWDER_STABLE,
            BlockProperties::BURNING,
            BlockProperties::CHANGED_THIS_STEP,
        ]
        .into_iter()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Block {
    /// The index into the block definitions array
    id: u16,
    /// Used to vary colors among blocks of the same type
    color_seed: u8,
    /// Reserved for future use
    damage: u8,
    /// The stored properties, all boolean-valued
    stored_properties: BlockProperties,
}

impl Default for Block {
    fn default() -> Block {
        Block {
            id: Default::default(),
            color_seed: rand::random(),
            damage: Default::default(),
            stored_properties: Default::default(),
        }
    }
}

impl Block {
    pub(crate) fn new(id: u16) -> Block {
        Block {
            id,
            ..Default::default()
        }
    }

    pub(crate) fn color(&self) -> Color {
        let x = self.color_seed as f32 / 255.0;
        let data = self.data();
        data.color1 * x + data.color2 * (1.0 - x)
    }

    pub(crate) fn data(&self) -> &'static BlockData {
        ALL_BLOCK_DATA.get(self.id as usize).unwrap()
    }

    pub(crate) fn get_prop(&self, property: Property) -> f32 {
        match property {
            Property::Material(id) => {
                if self.id == id {
                    1.0
                } else {
                    0.0
                }
            }
            Property::BlockProperty(property) => {
                if self.get(property) {
                    1.0
                } else {
                    0.0
                }
            }
            _ => todo!(),
        }
    }

    pub(crate) fn get(&self, property: BlockProperties) -> bool {
        self.stored_properties.contains(property)
    }

    pub(crate) fn set(&mut self, property: BlockProperties, value: bool) {
        self.stored_properties.set(property, value)
    }

    pub(crate) fn iter_properties<'a>(&'a self) -> impl Iterator<Item = Property> + 'a {
        [Property::Material(self.id)]
            .into_iter()
            .chain(BlockProperties::iter_all().filter_map(|p| {
                if self.get(p) {
                    Some(Property::BlockProperty(p))
                } else {
                    None
                }
            }))
    }
}

#[derive(Debug, PartialEq)]
pub(crate) enum BlockPhysics {
    /// Doesn't move, can be pushed around
    None,
    /// Doesn't move, can't be pushed around
    Solid,
    /// Forms into piles, has friction
    Powder,
    /// Frictionless liquids and gasses
    Liquid,
}

#[derive(Debug)]
pub(crate) struct BlockData {
    /// Internal block name
    pub(crate) name: &'static str,
    /// First color extreme
    pub(crate) color1: Color,
    /// Second color extreme
    pub(crate) color2: Color,
    /// Mass of a single block
    pub(crate) density: f32,
    /// Physics of this block
    pub(crate) physics: BlockPhysics,
    /// Stability of this powder - only makes sense for powders
    pub(crate) powder_stability: f32,
}

fn get_id(name: &str) -> u16 {
    ALL_BLOCK_DATA.iter().position(|x| x.name == name).unwrap() as u16
}

lazy_static! {
    pub(crate) static ref ALL_BLOCK_DATA: Vec<BlockData> = vec![
        BlockData {
            name: "Air",
            color1: Color::rgba(0.0, 0.0, 0.0, 0.0),
            color2: Color::rgba(0.0, 0.0, 0.0, 0.0),
            density: 0.1,
            physics: BlockPhysics::None,
            powder_stability: 0.0,
        },
        BlockData {
            name: "Stone",
            color1: Color::rgb(0.5, 0.5, 0.5),
            color2: Color::rgb(0.3, 0.3, 0.3),
            density: 1.0,
            physics: BlockPhysics::Solid,
            powder_stability: 0.0,
        },
        BlockData {
            name: "Water",
            color1: Color::rgba(0.2, 0.4, 1.0, 0.7),
            color2: Color::rgba(0.2, 0.4, 1.0, 0.7),
            density: 0.5,
            physics: BlockPhysics::Liquid,
            powder_stability: 0.0,
        },
        BlockData {
            name: "Sand",
            color1: Color::rgb(1.0, 0.8, 0.3),
            color2: Color::rgb(0.8, 0.6, 0.2),
            density: 0.8,
            physics: BlockPhysics::Powder,
            powder_stability: 0.3,
        },
        BlockData {
            name: "Wood",
            color1: Color::rgb(0.8, 0.4, 0.3),
            color2: Color::rgb(0.6, 0.2, 0.2),
            density: 1.0,
            physics: BlockPhysics::Solid,
            powder_stability: 0.0,
        },
        BlockData {
            name: "Coal",
            color1: Color::rgb(0.2, 0.2, 0.2),
            color2: Color::rgb(0.1, 0.1, 0.1),
            density: 1.0,
            physics: BlockPhysics::Powder,
            powder_stability: 0.7,
        },
        BlockData {
            name: "Fire",
            color1: Color::rgb(1.0, 1.0, 0.4),
            color2: Color::rgb(1.0, 0.3, 0.0),
            density: 0.1,
            physics: BlockPhysics::None,
            powder_stability: 0.0,
        },
        BlockData {
            name: "Smoke",
            color1: Color::rgba(0.1, 0.1, 0.1, 0.5),
            color2: Color::rgba(0.2, 0.2, 0.2, 0.2),
            density: 0.05,
            physics: BlockPhysics::Liquid,
            powder_stability: 0.0,
        },
        BlockData {
            name: "Steam",
            color1: Color::rgba(1.0, 1.0, 1.0, 0.3),
            color2: Color::rgba(1.0, 1.0, 1.0, 0.1),
            density: 0.05,
            physics: BlockPhysics::Liquid,
            powder_stability: 0.0,
        },
    ];
    pub(crate) static ref AIR: u16 = get_id("Air");
    pub(crate) static ref STONE: u16 = get_id("Stone");
    pub(crate) static ref WATER: u16 = get_id("Water");
    pub(crate) static ref SAND: u16 = get_id("Sand");
    pub(crate) static ref WOOD: u16 = get_id("Wood");
    pub(crate) static ref COAL: u16 = get_id("Coal");
    pub(crate) static ref FIRE: u16 = get_id("Fire");
    pub(crate) static ref SMOKE: u16 = get_id("Smoke");
    pub(crate) static ref STEAM: u16 = get_id("Steam");
}
