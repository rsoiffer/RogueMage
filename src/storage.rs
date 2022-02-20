use crate::spells::SpellSelector;
use crate::spells::SpellSelector::*;
use bevy::{prelude::Entity, utils::HashMap};
use std::{
    hash::Hash,
    sync::{Mutex, MutexGuard},
};

#[derive(Debug)]
pub(crate) struct Digraph<N, E> {
    map: HashMap<N, HashMap<N, E>>,
}

impl<N, E> Default for Digraph<N, E> {
    fn default() -> Self {
        Self {
            map: Default::default(),
        }
    }
}

impl<N, E> Digraph<N, E>
where
    N: Eq + Hash,
{
    pub(crate) fn clear(&mut self) {
        for inner_map in self.map.values_mut() {
            inner_map.clear();
        }
    }

    pub(crate) fn entries(&self) -> impl Iterator<Item = (&N, &N, &E)> {
        self.map.iter().flat_map(|(source, storage2)| {
            storage2
                .iter()
                .map(move |(target, value)| (source, target, value))
        })
    }

    pub(crate) fn get(&self, source: &N, target: &N) -> E
    where
        E: Copy + Default + PartialEq,
    {
        self.get_option(source, target).cloned().unwrap_or_default()
    }

    pub(crate) fn get_option(&self, source: &N, target: &N) -> Option<&E> {
        self.map
            .get(source)
            .and_then(|storage2| storage2.get(target))
    }

    pub(crate) fn get_all(&self, source: &N) -> impl Iterator<Item = (&N, &E)> {
        self.map.get(source).into_iter().flat_map(|x| x.iter())
    }

    pub(crate) fn set(&mut self, source: N, target: N, value: E)
    where
        E: Copy + Default + PartialEq,
    {
        let value = if value == E::default() {
            None
        } else {
            Some(value)
        };
        self.set_option(source, target, value);
    }

    pub(crate) fn set_option(&mut self, source: N, target: N, value: Option<E>) -> Option<E> {
        match value {
            Some(t) => {
                let storage2 = self.map.entry(source).or_default();
                storage2.insert(target, t)
            }
            None => self
                .map
                .get_mut(&source)
                .and_then(|storage2| storage2.remove(&target)),
        }
    }
}

#[derive(Default)]
pub(crate) struct TrackingDigraph {
    pub(crate) digraph: Digraph<Object, f32>,
    changes: Digraph<Object, f32>,
}

impl TrackingDigraph {
    pub(crate) fn add(&mut self, source: Object, target: Object, diff: f32) {
        if f32::abs(diff) > 1e-12 {
            self.digraph
                .set(source, target, self.digraph.get(&source, &target) + diff);
            self.changes
                .set(source, target, self.changes.get(&source, &target) + diff);
        }
    }

    fn clear_changes(&mut self) {
        self.changes.clear()
    }
}

fn mul<'a, F>(
    lhs: &'a Digraph<Object, f32>,
    rhs: F,
) -> impl Iterator<Item = (Object, Object, f32)> + 'a
where
    F: Fn(Object) -> Box<dyn Iterator<Item = (Object, f32)> + 'a> + 'a,
{
    lhs.entries().flat_map(move |(&source, &middle, &value1)| {
        let iter = rhs(middle);
        iter.map(move |(target, value2)| (source, target, value1 * value2))
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Object {
    Block(i32, i32),
    Rigidbody(Entity),
    Zone,
}

pub(crate) struct ReactiveStorage {
    pub(crate) selector: SpellSelector,
    pub(crate) storage: TrackingDigraph,
}

impl ReactiveStorage {
    fn compute_all(
        &self,
        source: Object,
        return_changes: bool,
    ) -> Option<Box<dyn Iterator<Item = (Object, f32)>>> {
        match self.selector {
            Bind(_, _) => None,
            Is(_) => None,
            _ => todo!(),
        }
    }

    pub(crate) fn get_all<'a>(
        &'a self,
        source: Object,
        return_changes: bool,
    ) -> Box<dyn Iterator<Item = (Object, f32)> + 'a> {
        self.compute_all(source, return_changes).unwrap_or_else(|| {
            let digraph = if return_changes {
                &self.storage.changes
            } else {
                &self.storage.digraph
            };
            Box::new(digraph.get_all(&source).map(|(&x, &y)| (x, y)))
        })
    }

    /// Who are my parents?
    fn parents(&self) -> Vec<SpellSelector> {
        match &self.selector {
            Bind(left, right) => vec![*left.clone(), *right.clone()],
            _ => vec![],
        }
    }

    /// Called once/frame, updates my digraph from parents' changes
    fn recompute_cache(&mut self, storage_manager: &StorageManager) {
        match self.selector.clone() {
            Bind(left, right) => {
                let left = storage_manager.get(&left);
                let right = storage_manager.get(&right);
                for (source, target, diff) in
                    mul(&left.storage.changes, |middle| right.get_all(middle, false))
                {
                    self.storage.add(source, target, diff);
                }
                for (source, target, diff) in
                    mul(&left.storage.digraph, |middle| right.get_all(middle, true))
                {
                    self.storage.add(source, target, diff);
                }
                for (source, target, diff) in
                    mul(&left.storage.changes, |middle| right.get_all(middle, true))
                {
                    self.storage.add(source, target, -diff);
                }
            }
            Is(_) => {}
            Adjacent => {}
            _ => todo!(),
        }
    }
}

#[derive(Default)]
pub(crate) struct StorageManager {
    storages: HashMap<SpellSelector, Mutex<ReactiveStorage>>,
}

// TODO - fix the unsafe blocks below by adding RefCells
impl StorageManager {
    pub(crate) fn get(&self, selector: &SpellSelector) -> MutexGuard<ReactiveStorage> {
        self.storages.get(selector).unwrap().lock().unwrap()
    }

    pub(crate) fn require(&mut self, selector: &SpellSelector) {
        match self.storages.get(selector) {
            Some(_) => {}
            None => {
                let new_storage = ReactiveStorage {
                    selector: selector.clone(),
                    storage: Default::default(),
                };
                for parent in new_storage.parents() {
                    self.require(&parent);
                }
                self.storages
                    .insert(selector.clone(), Mutex::new(new_storage));
            }
        }
    }

    pub(crate) fn recompute_all_caches(&mut self) {
        // TODO - topological sort
        for (selector, storage) in self.storages.iter() {
            println!("Recomputing cache for {:?}", selector);
            storage.lock().unwrap().recompute_cache(self);
        }
        for (_, storage) in self.storages.iter() {
            storage.lock().unwrap().storage.clear_changes();
        }
    }
}
