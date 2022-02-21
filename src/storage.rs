use crate::{
    blocks::BlockProperties,
    sparse_matrices::*,
    spells::SpellSelector::{self, *},
};
use bevy::{prelude::Entity, utils::HashMap};
use std::{
    hash::Hash,
    sync::{Mutex, MutexGuard},
};
use topological_sort::TopologicalSort;

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
    fn clear(&mut self) {
        for inner_map in self.map.values_mut() {
            inner_map.clear();
        }
    }

    pub(crate) fn get_option(&self, source: &N, target: &N) -> Option<&E> {
        self.map
            .get(source)
            .and_then(|storage2| storage2.get(target))
    }

    fn set_option(&mut self, source: N, target: N, value: Option<E>) -> Option<E> {
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

impl SparseMatrix for Digraph<Object, f32> {
    type Key = Object;

    fn entries(&self) -> Entries<Object> {
        Box::new(self.map.iter().flat_map(|(&source, storage2)| {
            storage2
                .iter()
                .map(move |(&target, &value)| (source, target, value))
        }))
    }

    fn row(&self, source: Object) -> Row<Object> {
        Box::new(
            self.map
                .get(&source)
                .into_iter()
                .flat_map(|x| x.iter())
                .map(|(&target, &val)| (target, val)),
        )
    }

    fn get(&self, source: Object, target: Object) -> f32 {
        self.get_option(&source, &target)
            .cloned()
            .unwrap_or_default()
    }
}

#[derive(Debug)]
pub(crate) struct TrackingDigraph<N, E> {
    pub(crate) current: Digraph<N, E>,
    previous: Digraph<N, E>,
}

impl<N, E> Default for TrackingDigraph<N, E> {
    fn default() -> Self {
        Self {
            current: Default::default(),
            previous: Default::default(),
        }
    }
}

impl<N, E> TrackingDigraph<N, E> {
    fn clear_previous(&mut self)
    where
        N: Eq + Hash,
    {
        self.previous.clear()
    }

    pub(crate) fn update<F>(&mut self, source: N, target: N, f: F)
    where
        N: Copy + Eq + Hash,
        E: Copy + Default + PartialEq,
        F: Fn(E) -> E,
    {
        let current_val = self
            .current
            .get_option(&source, &target)
            .cloned()
            .unwrap_or_default();
        if self.previous.get_option(&source, &target).is_none() {
            self.previous.set_option(source, target, Some(current_val));
        }
        let new_val = f(current_val);
        self.current.set_option(
            source,
            target,
            if new_val == E::default() {
                None
            } else {
                Some(new_val)
            },
        );
    }
}

impl TrackingSparseMatrix<Digraph<Object, f32>, Digraph<Object, f32>>
    for TrackingDigraph<Object, f32>
{
    type Key = Object;

    fn current(&self) -> &Digraph<Object, f32> {
        &self.current
    }

    fn previous(&self) -> &Digraph<Object, f32> {
        &self.previous
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Object {
    Block(i32, i32),
    Rigidbody(Entity),
    Zone,
}

pub(crate) struct ReactiveStorage {
    selector: SpellSelector,
    storage: TrackingDigraph<Object, f32>,
}

impl ReactiveStorage {
    // fn row(&self, source: Object) -> Row {
    //     self.compute_row(source)
    //         .unwrap_or_else(|| Box::new(self.storage.row(source)))
    // }

    // fn row_changes(&self, source: Object) -> Row {
    //     self.compute_row_changes(source)
    //         .unwrap_or_else(|| Box::new(self.storage.row_changes(source)))
    // }

    // fn current(&self) -> M {
    //     todo!()
    // }

    // fn previous(&self) -> M {
    //     todo!()
    // }

    // fn compute_row(&self, source: Object) -> Option<Row> {
    //     match self.selector {
    //         Bind(_, _) => None,
    //         Is(_) => None,
    //         Adjacent => match source {
    //             Object::Block(x, y) => Some(Box::new((-1..2).flat_map(move |x2| {
    //                 (-1..2).map(move |y2| (Object::Block(x + x2, y + y2), 1.0))
    //             }))),
    //             _ => todo!(),
    //         },
    //         _ => todo!(),
    //     }
    // }

    fn compute_row_changes(
        &self,
        source: Object,
    ) -> Option<Box<dyn Iterator<Item = (Object, f32)>>> {
        match self.selector {
            Bind(_, _) => None,
            Is(_) => None,
            Adjacent => None,
            _ => todo!(),
        }
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
                    mat_mul(diff(&left.storage), &right.storage.current).entries()
                {
                    self.storage.update(source, target, |x| x + diff);
                }
                for (source, target, diff) in
                    mat_mul(&left.storage.current, diff(&right.storage)).entries()
                {
                    self.storage.update(source, target, |x| x + diff);
                }
                for (source, target, diff) in
                    mat_mul(diff(&left.storage), diff(&right.storage)).entries()
                {
                    self.storage.update(source, target, |x| x - diff);
                }
            }
            Is(_) => {}
            Adjacent => {}
            _ => todo!(),
        }
    }
}

pub(crate) struct StorageManager {
    pub(crate) material: TrackingDigraph<Object, u16>,
    block_properties: HashMap<BlockProperties, Mutex<TrackingDigraph<Object, bool>>>,
    selectors: HashMap<SpellSelector, Mutex<ReactiveStorage>>,
}

// TODO - fix the unsafe blocks below by adding RefCells
impl StorageManager {
    pub(crate) fn new() -> StorageManager {
        let mut storage_manager = StorageManager {
            material: Default::default(),
            block_properties: Default::default(),
            selectors: Default::default(),
        };

        for prop in BlockProperties::iter_all() {
            storage_manager
                .block_properties
                .insert(prop, Mutex::new(Default::default()));
        }

        storage_manager
    }

    fn get(&self, selector: &SpellSelector) -> MutexGuard<ReactiveStorage> {
        self.selectors.get(selector).unwrap().lock().unwrap()
    }

    pub(crate) fn for_each_entry(
        &self,
        selector: &SpellSelector,
        f: impl Fn((Object, Object, f32)) -> (),
    ) {
        let reactive_storage = self.get(selector);
        for entry in reactive_storage.storage.current.entries() {
            f(entry);
        }
    }

    pub(crate) fn get_prop(
        &self,
        property: BlockProperties,
    ) -> MutexGuard<TrackingDigraph<Object, bool>> {
        self.block_properties
            .get(&property)
            .unwrap()
            .lock()
            .unwrap()
    }

    pub(crate) fn require(&mut self, selector: &SpellSelector) {
        match self.selectors.get(selector) {
            Some(_) => {}
            None => {
                let new_storage = ReactiveStorage {
                    selector: selector.clone(),
                    storage: Default::default(),
                };
                for parent in new_storage.parents() {
                    self.require(&parent);
                }
                self.selectors
                    .insert(selector.clone(), Mutex::new(new_storage));
            }
        }
    }

    pub(crate) fn recompute_all_caches(&mut self) {
        let mut toposort = TopologicalSort::<SpellSelector>::new();
        for (selector, storage) in self.selectors.iter() {
            toposort.insert(selector.clone());
            for parent in storage.lock().unwrap().parents() {
                toposort.add_dependency(parent, selector.clone());
            }
        }
        let toposort = toposort.collect::<Vec<_>>();

        // println!("Recomputing all caches...");
        // for selector in toposort.iter() {
        //     println!("Recomputing cache for selector {:?}", selector);
        // }

        for selector in toposort.iter() {
            self.get(selector).recompute_cache(self);
        }
        for selector in toposort.iter() {
            self.get(selector).storage.clear_previous();
        }
    }
}
