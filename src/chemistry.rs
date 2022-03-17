use crate::{blocks::BlockProperties, cells::BlockGrid, spells::Target};
use bevy::{
    prelude::{Component, Entity},
    utils::HashMap,
};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum Property {
    Material(u16),
    BlockProperty(BlockProperties),
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
    /// The grid of all blocks in the world
    pub(crate) grid: BlockGrid,
    /// Stores the value of every property on every entity
    pub(crate) entity_properties: HashMap<(Entity, StoredProperty), f32>,
}

impl WorldInfo {
    pub(crate) fn get(&self, target: Target, property: Property) -> f32 {
        match target {
            Target::Block(x, y) => self.grid.get_property(x, y, property),
            Target::Entity(e) => match property {
                Property::Material(_) => todo!(),
                Property::BlockProperty(_) => todo!(),
                Property::Stored(property) => self
                    .entity_properties
                    .get(&(e, property))
                    .cloned()
                    .unwrap_or_default(),
                Property::Dependent(_) => todo!(),
            },
        }
    }
}
