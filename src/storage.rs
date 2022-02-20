use crate::chemistry::*;
use crate::spells::SpellSelector;
use crate::spells::SpellSelector::*;
use bevy::{
    prelude::Entity,
    utils::{HashMap, HashSet},
};
use lazy_static::lazy_static;
use std::{collections::hash_map::Entry, hash::Hash, ops::Mul};

pub(crate) struct Digraph<N, E> {
    storage: HashMap<N, HashMap<N, E>>,
}

impl<N, E> Default for Digraph<N, E> {
    fn default() -> Self {
        Self {
            storage: Default::default(),
        }
    }
}

impl<N, E> Digraph<N, E>
where
    N: Eq + Hash,
{
    pub(crate) fn clear(&mut self) {
        for inner_map in self.storage.values_mut() {
            inner_map.clear();
        }
    }

    pub(crate) fn entries(&self) -> impl Iterator<Item = (&N, &N, &E)> {
        self.storage.iter().flat_map(|(source, storage2)| {
            storage2
                .iter()
                .map(move |(target, value)| (source, target, value))
        })
    }

    pub(crate) fn get(&self, source: &N, target: &N) -> Option<&E> {
        self.storage
            .get(source)
            .and_then(|storage2| storage2.get(target))
    }

    pub(crate) fn get_all(&self, source: &N) -> impl Iterator<Item = (&N, &E)> {
        self.storage.get(source).into_iter().flat_map(|x| x.iter())
    }

    pub(crate) fn set(&mut self, source: N, target: N, value: Option<E>) -> Option<E> {
        match value {
            Some(t) => {
                let storage2 = self.storage.entry(source).or_default();
                storage2.insert(target, t)
            }
            None => self
                .storage
                .get_mut(&source)
                .and_then(|storage2| storage2.remove(&target)),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Object {
    Block(i32, i32),
    Rigidbody(Entity),
    Zone(Entity),
}

pub(crate) struct ReactiveStorage {
    selector: SpellSelector,
    digraph: Digraph<Object, f32>,
    changes: Digraph<Object, f32>,
}

impl ReactiveStorage {
    fn add(&mut self, source: Object, target: Object, diff: f32) {
        if f32::abs(diff) > 1e-12 {
            self.set(source, target, self.get(&source, &target) + diff);
        }
    }

    fn compute(&self, source: &Object, target: &Object) -> Option<f32> {
        match self.selector {
            Adjacent => {
                todo!()
            }
            _ => None,
        }
    }

    fn compute_all(&self, source: &Object) -> Option<Vec<(&Object, &f32)>> {
        match self.selector {
            Adjacent => {
                todo!()
            }
            _ => None,
        }
    }

    pub(crate) fn get(&self, source: &Object, target: &Object) -> f32 {
        self.compute(source, target).unwrap_or_else(|| {
            self.digraph
                .get(source, target)
                .cloned()
                .unwrap_or_default()
        })
    }

    pub(crate) fn get_all(&self, source: &Object) -> Vec<(&Object, &f32)> {
        self.compute_all(source)
            .unwrap_or_else(|| self.digraph.get_all(source).collect::<Vec<_>>())
    }

    pub(crate) fn set(&mut self, source: Object, target: Object, new_value: f32) {
        let value = if new_value == 0.0 {
            None
        } else {
            Some(new_value)
        };
        let old_value = self.digraph.set(source, target, value).unwrap_or_default();
        if f32::abs(new_value - old_value) > 1e-12 {
            let old_change = self
                .changes
                .get(&source, &target)
                .cloned()
                .unwrap_or_default();
            self.changes
                .set(source, target, Some(new_value - old_value + old_change));
        }
    }

    fn mul<'a>(&'a self, rhs: &'a Self) -> impl Iterator<Item = (&Object, &Object, f32)> {
        self.entries().flat_map(|(source, middle, &value1)| {
            rhs.get_all(middle)
                .map(move |(target, &value2)| (source, target, value1 * value2))
        })
    }

    /// Who are my parents?
    fn parents(&self) -> Vec<SpellSelector> {
        match self.selector {
            Bind(left, right) => vec![*left, *right],
            _ => vec![],
        }
    }

    /// Called once/frame, updates my digraph from parents' changes
    fn recompute_cache(&mut self, storage_manager: &StorageManager) {
        match self.selector.clone() {
            Bind(left, right) => {
                let left = storage_manager.storages.get(&left).unwrap();
                let right = storage_manager.storages.get(&right).unwrap();
                for (source, target, diff) in left.changes.mul(&right.digraph) {
                    self.add(*source, *target, diff);
                }
                for (source, target, diff) in left.digraph.mul(&right.changes) {
                    self.add(*source, *target, diff);
                }
                for (source, target, diff) in left.changes.mul(&right.changes) {
                    self.add(*source, *target, -diff);
                }
            }
            _ => {}
        }
    }
}

pub(crate) struct StorageManager {
    storages: HashMap<SpellSelector, ReactiveStorage>,
}

impl StorageManager {
    fn recompute_all_caches(&mut self) {
        // TODO - topological sort
        for (selector, storage) in self.storages.iter_mut() {
            storage.recompute_cache(self);
        }
        for (selector, storage) in self.storages.iter_mut() {
            storage.changes.clear();
        }
    }
}

/*

old_value = Left * Right
new_value = Left2 * Right2

new_value - old_value
= Left2 * Right2 - Left * Right
= Left2 * Right2 - (Left2 - dLeft) * (Right2 - dRight)
= dLeft * Right2 + Left2 * dRight - dLeft * dRight




CachedSelector =
| Zone
| Base(Property)
| Bind(CachedSelector, CachedSelector)


Selector = Object -> Vec<(Object, f32)> = Map<Object, Map<Object, f32>>


Is(BlockProperty(Burning)) - caches a bool


Is(Material(Coal)) - caches a Vec<SpellTarget>
Is(BlockProperty(Burning)) - caches a Vec<SpellTarget>

Fn(Pair<Vec<SpellTarget>>, &mut Self) -> ()

fn update(Pair<bool>, Selector, &mut Storage) -> ()
fn update(Pair<&Vec<SpellTarget>>, Selector, &mut Storage) -> ()


Mana - Storage<Storage<Connection>>
Mana Fire - Storage<Storage<Connection>>
Mana Fire Adjacent - Storage<Storage<Connection>>
Mana Fire Adjacent Water - Storage<Storage<Connection>>

Zone - Storage<Storage<Connection>>
Zone Fire - Storage<Storage<Connection>>
Zone Fire Air - Storage<Storage<Connection>>
Zone Fire Air Adjacent Water - Storage<Storage<Connection>>

Vec<SpellTarget> = Vec<(Target, Connection)> = Map<Target, Connection> = Storage<Connection>

Storage<Storage<Connection>> = Map<(Source, Target), Connection>

HashMap:
  World -> Vec<SpellTarget> of everything in the world

HashMap:
  block1 -> singleton Vec<SpellTarget>
  block2 -> singleton Vec<SpellTarget>

HashMap:
  block1 -> ()
  block2 -> ()

HashSet:
  block1
  block2



"Heal over time"
    Player -> float
    Projectile -> float
    Sheep -> float

"Damage over time"
    Player -> float
    Projectile 1 -> float
    Projectile 2 -> float
    Sheep A -> float
    Sheep B -> float

*/
