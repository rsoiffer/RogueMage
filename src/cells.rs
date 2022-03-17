use crate::blocks::*;
use crate::chemistry::Property::*;
use crate::chemistry::StoredProperty::*;
use crate::chemistry::*;
use bevy::prelude::Image;
use bevy::utils::HashMap;
use bevy::utils::HashSet;
use rand::seq::SliceRandom;

/// The size of the whole grid of blocks
pub(crate) const GRID_SIZE: usize = 256;

pub(crate) struct BlockGrid {
    /// The 2d array of blocks
    grid: Vec<Block>,
    /// Stores the value of every property on every block
    properties: HashMap<(i32, i32, StoredProperty), f32>,
    /// Caches the set of active blocks that satisfy each property
    active: HashMap<Property, HashSet<(i32, i32)>>,
}

impl Default for BlockGrid {
    fn default() -> Self {
        let mut grid = Self {
            grid: vec![Block::default(); GRID_SIZE * GRID_SIZE],
            properties: HashMap::default(),
            active: HashMap::default(),
        };
        grid.set_range(0..GRID_SIZE as i32, 0..GRID_SIZE as i32, 0);
        grid
    }
}

impl BlockGrid {
    pub(crate) fn get(&self, x: i32, y: i32) -> Option<Block> {
        if x >= 0 && x < GRID_SIZE as i32 && y >= 0 && y < GRID_SIZE as i32 {
            Some(self.grid[y as usize * GRID_SIZE + x as usize])
        } else {
            None
        }
    }

    pub(crate) fn set(&mut self, x: i32, y: i32, mut block: Block) {
        block.set(BlockProperties::CHANGED_THIS_STEP, true);
        self.set_no_change(x, y, block);
    }

    pub(crate) fn set_no_change(&mut self, x: i32, y: i32, block: Block) {
        if x >= 0 && x < GRID_SIZE as i32 && y >= 0 && y < GRID_SIZE as i32 {
            let old_block = self.grid[y as usize * GRID_SIZE + x as usize];
            if block != old_block {
                for p in old_block.iter_properties() {
                    self.active.entry(p).or_default().remove(&(x, y));
                }
                for p in block.iter_properties() {
                    self.active.entry(p).or_default().insert((x, y));
                }
                self.grid[y as usize * GRID_SIZE + x as usize] = block;
            }
        }
    }

    pub(crate) fn get_property(&self, x: i32, y: i32, property: Property) -> f32 {
        match property {
            Material(id) => {
                if self.get(x, y).unwrap().id == id {
                    1.0
                } else {
                    0.0
                }
            }
            BlockProperty(property) => {
                if self.get(x, y).unwrap().get(property) {
                    1.0
                } else {
                    0.0
                }
            }
            Stored(prop) => self
                .properties
                .get(&(x, y, prop))
                .cloned()
                .unwrap_or_default(),
            Dependent(_) => todo!(),
        }
    }

    pub(crate) fn clear_block_property(&mut self, property: BlockProperties) {
        for (x, y) in self
            .all_matching(BlockProperty(property))
            .collect::<Vec<_>>()
        {
            let mut block = self.get(x, y).unwrap();
            block.set(property, false);
            self.set_no_change(x, y, block);
        }
    }

    pub(crate) fn set_range<I1, I2>(&mut self, xs: I1, ys: I2, id: u16)
    where
        I1: IntoIterator<Item = i32>,
        I2: IntoIterator<Item = i32> + Clone,
    {
        self.set_range_func(xs, ys, |block| *block = Block::new(id));
    }

    pub(crate) fn set_range_func<I1, I2, F>(&mut self, xs: I1, ys: I2, f: F)
    where
        I1: IntoIterator<Item = i32>,
        I2: IntoIterator<Item = i32> + Clone,
        F: Fn(&mut Block),
    {
        for x in xs {
            for y in ys.clone() {
                let mut block = self.get(x, y).unwrap();
                f(&mut block);
                self.set(x, y, block);
            }
        }
    }

    pub(crate) fn all_matching<'a>(
        &'a self,
        property: Property,
    ) -> impl Iterator<Item = (i32, i32)> + 'a {
        self.active
            .get(&property)
            .into_iter()
            .flat_map(|x| x.iter())
            .map(|&(x, y)| (x, y))
    }

    pub(crate) fn neighbors_shuffle<I1, I2>(
        &self,
        x: i32,
        y: i32,
        xs: I1,
        ys: I2,
    ) -> Vec<(i32, i32)>
    where
        I1: IntoIterator<Item = i32>,
        I2: IntoIterator<Item = i32> + Clone,
    {
        let mut r = self.neighbors(x, y, xs, ys).collect::<Vec<_>>();
        r.shuffle(&mut rand::thread_rng());
        r
    }

    pub(crate) fn neighbors<I1, I2>(
        &self,
        x: i32,
        y: i32,
        xs: I1,
        ys: I2,
    ) -> impl Iterator<Item = (i32, i32)>
    where
        I1: IntoIterator<Item = i32>,
        I2: IntoIterator<Item = i32> + Clone,
    {
        xs.into_iter().flat_map(move |x_offset| {
            ys.clone().into_iter().filter_map(move |y_offset| {
                let x2 = x + x_offset;
                let y2 = y + y_offset;
                if x2 >= 0 && x2 < GRID_SIZE as i32 && y2 >= 0 && y2 < GRID_SIZE as i32 {
                    Some((x2, y2))
                } else {
                    None
                }
            })
        })
    }
}

pub(crate) fn update_texture_pixel(grid: &BlockGrid, texture: &mut Image, x: i32, y: i32) {
    let block = grid.get(x, y).unwrap();
    let mut color = block.color();

    // Draw all burning blocks as fire
    if grid.get_property(x, y, Stored(Burning)) > 0.0 {
        let x = rand::random::<f32>();
        let fire_data = ALL_BLOCK_DATA.get(*FIRE as usize).unwrap();
        color = fire_data.color1 * x + fire_data.color2 * (1.0 - x);
    }

    let i = 4 * (x as usize + (GRID_SIZE - y as usize - 1) * GRID_SIZE);
    texture.data.splice(
        i..i + 4,
        color.as_rgba_f32().iter().map(|&v| (v * 255.0) as u8),
    );
}
