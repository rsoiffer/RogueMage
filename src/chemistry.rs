use crate::{
    blocks::{Block, BlockProperties},
    cells::BlockGrid,
};
use bevy::{
    math::Vec2,
    prelude::{Component, Entity, Query},
    utils::{HashMap, HashSet},
};
use bevy_rapier2d::prelude::AABB;
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
    IsEntity,
    Liquid,
}

#[derive(Default)]
pub(crate) struct AABBCollider {
    pub(crate) ll: Vec2,
    pub(crate) ur: Vec2,
}

impl AABBCollider {
    pub(crate) fn intersects(&self, other: &AABBCollider) -> bool {
        self.ll.x <= other.ur.x
            && self.ur.x >= other.ll.x
            && self.ll.y <= other.ur.y
            && self.ur.y >= other.ll.y
    }

    pub(crate) fn from_block(x: i32, y: i32) -> AABBCollider {
        AABBCollider {
            ll: Vec2::new(x as f32, y as f32),
            ur: Vec2::new((x + 1) as f32, (y + 1) as f32),
        }
    }
}

#[derive(Component)]
pub(crate) struct ChemEntity;

#[derive(Component, Default)]
pub(crate) struct WorldInfo {
    /// The grid of blocks in the world
    blocks: BlockGrid,
    /// The colliders of all entities in the world
    pub(crate) entity_colliders: HashMap<Entity, AABBCollider>,
    /// Stores the value of every property on every target
    properties: HashMap<Target, HashMap<StoredProperty, f32>>,
    /// Stores a set of active targets with each property
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
            (Entity(e), Material(id)) => 0.0,
            (target, Stored(property)) => self
                .properties
                .get(&target)
                .and_then(|m| m.get(&property))
                .cloned()
                .unwrap_or_default(),
            (target, Dependent(property)) => match (target, property) {
                (Block(x, y), DependentProperty::IsEntity) => 0.0,
                (Entity(e), DependentProperty::IsEntity) => 1.0,
                _ => todo!(),
            },
        }
    }

    pub(crate) fn set(&mut self, target: Target, property: StoredProperty, value: f32) {
        let properties = self.properties.entry(target).or_default();
        let old_value = properties.get(&property).cloned().unwrap_or_default();
        if old_value != value {
            if value == 0.0 {
                properties.remove(&property);
                self.active
                    .entry(Stored(property))
                    .or_default()
                    .remove(&target);
            } else {
                properties.insert(property, value);
                self.active
                    .entry(Stored(property))
                    .or_default()
                    .insert(target);
            }
            self.changed.insert(target);
        }
    }

    pub(crate) fn swap_properties(&mut self, target1: Target, target2: Target) {
        let properties1 = self.properties.remove(&target1);
        let properties2 = self.properties.remove(&target2);

        for &property in properties1.iter().flat_map(|m| m.keys()) {
            self.active
                .entry(Stored(property))
                .or_default()
                .remove(&target1);
        }
        for &property in properties2.iter().flat_map(|m| m.keys()) {
            self.active
                .entry(Stored(property))
                .or_default()
                .remove(&target2);
        }

        for &property in properties1.iter().flat_map(|m| m.keys()) {
            self.active
                .entry(Stored(property))
                .or_default()
                .insert(target2);
        }
        for &property in properties2.iter().flat_map(|m| m.keys()) {
            self.active
                .entry(Stored(property))
                .or_default()
                .insert(target1);
        }

        for properties1 in properties1 {
            self.properties.insert(target2, properties1);
        }
        for properties2 in properties2 {
            self.properties.insert(target1, properties2);
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
                Entity(entity) => {}
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
