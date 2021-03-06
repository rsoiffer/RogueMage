use crate::chemistry::Property::*;
use crate::chemistry::StaticProperty::*;
use crate::chemistry::*;
use bevy::prelude::Color;
use bitflags::bitflags;
use lazy_static::lazy_static;

bitflags! {
    #[derive(Default)]
    pub(crate) struct PhysicsFlags: u32 {
        /// Has this block already moved this step
        const MOVED_THIS_STEP = 1 << 0;
        /// Has this block settled into a stable state - can only be true for powders
        const POWDER_STABLE = 1 << 1;
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Block {
    /// The index into the block definitions array
    pub(crate) id: u16,
    /// Used to vary colors among blocks of the same type
    color_seed: u8,
    /// Reserved for future use
    damage: u8,
    /// The physics flags.
    physics_flags: PhysicsFlags,
}

impl Default for Block {
    fn default() -> Block {
        Block {
            id: Default::default(),
            color_seed: rand::random(),
            damage: Default::default(),
            physics_flags: Default::default(),
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

    pub(crate) fn get(&self, flags: PhysicsFlags) -> bool {
        self.physics_flags.contains(flags)
    }

    pub(crate) fn set(&mut self, flags: PhysicsFlags, value: bool) {
        self.physics_flags.set(flags, value)
    }

    pub(crate) fn iter_properties<'a>(&'a self) -> impl Iterator<Item = Property> + 'a {
        [Property::Material(self.id)].into_iter().chain(
            if self.data().physics == BlockPhysics::Liquid {
                Some(Static(Liquid))
            } else {
                None
            },
        )
    }
}

#[derive(Debug, PartialEq)]
pub(crate) enum BlockPhysics {
    /// Doesn't move, can be pushed around
    None,
    /// Doesn't move, can't be pushed around
    Solid,
    /// Moving powders, liquids, and gasses
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
            density: 0.0,
            physics: BlockPhysics::None,
            powder_stability: 0.0,
        },
        BlockData {
            name: "Stone",
            color1: Color::rgb(0.5, 0.5, 0.5),
            color2: Color::rgb(0.3, 0.3, 0.3),
            density: 3.3,
            physics: BlockPhysics::Solid,
            powder_stability: 0.0,
        },
        BlockData {
            name: "Water",
            color1: Color::rgba(0.2, 0.4, 1.0, 0.7),
            color2: Color::rgba(0.2, 0.4, 1.0, 0.7),
            density: 2.9,
            physics: BlockPhysics::Liquid,
            powder_stability: 0.0,
        },
        BlockData {
            name: "Sand",
            color1: Color::rgb(1.0, 0.8, 0.3),
            color2: Color::rgb(0.8, 0.6, 0.2),
            density: 3.3,
            physics: BlockPhysics::Liquid,
            powder_stability: 0.3,
        },
        BlockData {
            name: "Wood",
            color1: Color::rgb(0.8, 0.4, 0.3),
            color2: Color::rgb(0.6, 0.2, 0.2),
            density: 2.7,
            physics: BlockPhysics::Solid,
            powder_stability: 0.0,
        },
        BlockData {
            name: "Coal",
            color1: Color::rgb(0.2, 0.2, 0.2),
            color2: Color::rgb(0.1, 0.1, 0.1),
            density: 3.0,
            physics: BlockPhysics::Liquid,
            powder_stability: 0.7,
        },
        BlockData {
            name: "Fire",
            color1: Color::rgb(1.0, 1.0, 0.4),
            color2: Color::rgb(1.0, 0.3, 0.0),
            density: 0.0,
            physics: BlockPhysics::None,
            powder_stability: 0.0,
        },
        BlockData {
            name: "Smoke",
            color1: Color::rgba(0.1, 0.1, 0.1, 0.5),
            color2: Color::rgba(0.2, 0.2, 0.2, 0.2),
            density: -0.6,
            physics: BlockPhysics::Liquid,
            powder_stability: 0.0,
        },
        BlockData {
            name: "Steam",
            color1: Color::rgba(1.0, 1.0, 1.0, 0.3),
            color2: Color::rgba(1.0, 1.0, 1.0, 0.1),
            density: -0.3,
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
