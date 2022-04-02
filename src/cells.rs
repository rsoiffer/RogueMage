use crate::blocks::*;
use crate::chemistry::DynamicProperty::*;
use crate::chemistry::Property::*;
use crate::chemistry::*;
use bevy::prelude::Image;
use rand::seq::SliceRandom;

/// The size of the whole grid of blocks
pub(crate) const GRID_SIZE: usize = 256;

pub(crate) struct BlockGrid {
    /// The 2d array of blocks
    grid: Vec<Block>,
}

impl Default for BlockGrid {
    fn default() -> Self {
        Self {
            grid: vec![Block::default(); GRID_SIZE * GRID_SIZE],
        }
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

    pub(crate) fn set(&mut self, x: i32, y: i32, block: Block) {
        self.grid[y as usize * GRID_SIZE + x as usize] = block;
    }
}

pub(crate) fn neighbors_shuffle<I1, I2>(x: i32, y: i32, xs: I1, ys: I2) -> Vec<(i32, i32)>
where
    I1: IntoIterator<Item = i32>,
    I2: IntoIterator<Item = i32> + Clone,
{
    let mut r = neighbors(x, y, xs, ys).collect::<Vec<_>>();
    r.shuffle(&mut rand::thread_rng());
    r
}

pub(crate) fn neighbors<I1, I2>(x: i32, y: i32, xs: I1, ys: I2) -> impl Iterator<Item = (i32, i32)>
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

pub(crate) fn update_texture_pixel(info: &WorldInfo, texture: &mut Image, x: i32, y: i32) {
    let block = info.get_block(x, y).unwrap();
    let mut color = block.color();

    // Draw all burning blocks as fire
    if info.get(Target::Block(x, y), Dynamic(Burning)) > 0.0 {
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

pub(crate) fn set_block_range<I1, I2>(info: &mut WorldInfo, xs: I1, ys: I2, id: u16)
where
    I1: IntoIterator<Item = i32>,
    I2: IntoIterator<Item = i32> + Clone,
{
    set_block_range_func(info, xs, ys, |block| *block = Block::new(id));
}

pub(crate) fn set_block_range_func<I1, I2, F>(info: &mut WorldInfo, xs: I1, ys: I2, f: F)
where
    I1: IntoIterator<Item = i32>,
    I2: IntoIterator<Item = i32> + Clone,
    F: Fn(&mut Block),
{
    for x in xs {
        for y in ys.clone() {
            let mut block = info.get_block(x, y).unwrap();
            f(&mut block);
            info.set_block(x, y, block);
        }
    }
}
