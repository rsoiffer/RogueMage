use crate::{
    blocks::{Block, BlockProperties},
    cells::BlockGrid,
};
use bevy::{
    prelude::{Component, Entity},
    utils::{HashMap, HashSet},
};
use Property::*;
use Target::*;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum Target {
    Block(i32, i32),
    Entity(Entity),
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum Property {
    Material(u16),
    Stored(StoredProperty),
    Dependent(DependentProperty),
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum StoredProperty {
    Burning,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum DependentProperty {
    Liquid,
}

#[derive(Component, Default)]
pub(crate) struct WorldInfo {
    /// The grid of blocks in the world
    blocks: BlockGrid,
    /// Stores the value of every property on every target
    properties: HashMap<(Target, StoredProperty), f32>,
    /// Stores a set of active target with each property
    active: HashMap<Property, HashSet<Target>>,
    /// The set of targets that have changed so far this step
    changed: HashSet<Target>,
}

impl WorldInfo {
    pub(crate) fn get(&self, target: Target, property: Property) -> f32 {
        match (target, property) {
            (Block(x, y), Material(id)) => {
                if self.get_block(x, y).unwrap().id == id {
                    1.0
                } else {
                    0.0
                }
            }
            (Entity(e), Material(id)) => todo!(),
            (target, Stored(property)) => self
                .properties
                .get(&(target, property))
                .cloned()
                .unwrap_or_default(),
            (target, Dependent(property)) => todo!(),
        }
    }

    pub(crate) fn set(&mut self, target: Target, property: StoredProperty, value: f32) {
        let old_value = self
            .properties
            .get(&(target, property))
            .cloned()
            .unwrap_or_default();
        if old_value != value {
            if value == 0.0 {
                self.properties.remove(&(target, property));
                self.active
                    .entry(Stored(property))
                    .or_default()
                    .remove(&target);
            } else {
                self.properties.insert((target, property), value);
                self.active
                    .entry(Stored(property))
                    .or_default()
                    .insert(target);
            }
            self.changed.insert(target);
        }
    }

    pub(crate) fn get_block(&self, x: i32, y: i32) -> Option<Block> {
        self.blocks.get(x, y)
    }

    pub(crate) fn set_block(&mut self, x: i32, y: i32, block: Block) {
        let old_block = self.blocks.get(x, y).unwrap();
        if block != old_block {
            for p in old_block.iter_properties() {
                self.active.entry(p).or_default().remove(&Block(x, y));
            }
            for p in block.iter_properties() {
                self.active.entry(p).or_default().insert(Block(x, y));
            }
            self.blocks.set(x, y, block);
            self.changed.insert(Block(x, y));
        }
    }

    pub(crate) fn all_changed<'a>(&'a self) -> impl Iterator<Item = Target> + 'a {
        self.changed.iter().cloned()
    }

    pub(crate) fn reset_changes(&mut self) {
        for &target in self.changed.iter() {
            match target {
                Block(x, y) => {
                    let mut block = self.blocks.get(x, y).unwrap();
                    block.set(BlockProperties::MOVED_THIS_STEP, false);
                    self.blocks.set(x, y, block);
                }
                _ => todo!(),
            }
        }
        self.changed.clear()
    }

    pub(crate) fn active_matching<'a>(
        &'a self,
        property: Property,
    ) -> impl Iterator<Item = Target> + 'a {
        self.active
            .get(&property)
            .into_iter()
            .flat_map(|x| x.iter())
            .cloned()
    }
}
