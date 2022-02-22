use crate::blocks::*;
use crate::chemistry::Property::*;
use crate::chemistry::*;
use crate::spells::*;
use bevy::prelude::Assets;
use bevy::prelude::Handle;
use bevy::prelude::Image;
use bevy::render::render_resource::Extent3d;
use bevy::render::render_resource::TextureDimension;
use bevy::render::render_resource::TextureFormat;
use bevy::utils::HashMap;
use bevy::utils::HashSet;
use bevy::{
    math::Vec3,
    prelude::{info_span, Commands, Component, Query, Res, ResMut, Transform},
    sprite::SpriteBundle,
};
use rand::seq::SliceRandom;

/// The size of the whole grid of blocks
const GRID_SIZE: usize = 256;

pub(crate) struct BlockGrid {
    /// The 2d array of blocks
    grid: Vec<Block>,
    /// If true, simulate right-to-left
    flip_sim_dir: bool,
    properties: HashMap<Property, HashSet<(i32, i32)>>,
}

impl Default for BlockGrid {
    fn default() -> Self {
        let mut grid = Self {
            grid: vec![Block::default(); GRID_SIZE * GRID_SIZE],
            flip_sim_dir: Default::default(),
            properties: HashMap::default(),
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

    fn set(&mut self, x: i32, y: i32, mut block: Block) {
        block.set(BlockProperties::CHANGED_THIS_STEP, true);
        self.set_no_change(x, y, block);
    }

    fn set_no_change(&mut self, x: i32, y: i32, block: Block) {
        if x >= 0 && x < GRID_SIZE as i32 && y >= 0 && y < GRID_SIZE as i32 {
            let old_block = self.grid[y as usize * GRID_SIZE + x as usize];
            if block != old_block {
                for p in old_block.iter_properties() {
                    self.properties.entry(p).or_default().remove(&(x, y));
                }
                for p in block.iter_properties() {
                    self.properties.entry(p).or_default().insert((x, y));
                }
                self.grid[y as usize * GRID_SIZE + x as usize] = block;
            }
        }
    }

    fn clear_property(&mut self, property: BlockProperties) {
        for (x, y) in self
            .all_matching(&BlockProperty(property))
            .collect::<Vec<_>>()
        {
            let mut block = self.get(x, y).unwrap();
            block.set(property, false);
            self.set_no_change(x, y, block);
        }
    }

    fn set_range<I1, I2>(&mut self, xs: I1, ys: I2, id: u16)
    where
        I1: IntoIterator<Item = i32>,
        I2: IntoIterator<Item = i32> + Clone,
    {
        self.set_range_func(xs, ys, |block| *block = Block::new(id));
    }

    fn set_range_func<I1, I2, F>(&mut self, xs: I1, ys: I2, f: F)
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

    fn all_matching<'a>(&'a self, property: &Property) -> impl Iterator<Item = (i32, i32)> + 'a {
        self.properties
            .get(property)
            .into_iter()
            .flat_map(|x| x.iter())
            .map(|&(x, y)| (x, y))
    }

    fn step(&mut self, update_rules: &UpdateRules) {
        let xs = if self.flip_sim_dir {
            num::range_step(GRID_SIZE as i32 - 1, 0, -1)
        } else {
            num::range_step(0, GRID_SIZE as i32, 1)
        };
        let ys = 0..GRID_SIZE as i32;

        let span = info_span!("Reset flags").entered();
        self.clear_property(BlockProperties::MOVED_THIS_STEP);
        self.clear_property(BlockProperties::CHANGED_THIS_STEP);
        span.exit();

        for rule in &update_rules.update_rules {
            let span =
                info_span!("Rule", rule = &bevy::utils::tracing::field::debug(rule)).entered();

            match rule {
                UpdateRule::SpellUpdateRule(
                    sr @ SpellRule {
                        spell: Spell::Select(SpellSelector::Bind(sc, _), _),
                        ..
                    },
                ) => match **sc {
                    SpellSelector::Is(p) => {
                        for (x, y) in self.all_matching(&p).collect::<Vec<_>>() {
                            rule.spell_update(sr, self, x, y);
                        }
                    }
                    _ => panic!("Spell started with non-Is selector: {:?}", sr),
                },
                _ => {
                    for y in ys.clone() {
                        for x in xs.clone() {
                            rule.update(self, x, y);
                        }
                    }
                }
            }

            span.exit();
        }
        self.flip_sim_dir = !self.flip_sim_dir;
    }

    fn neighbors_shuffle<I1, I2>(&self, x: i32, y: i32, xs: I1, ys: I2) -> Vec<(i32, i32)>
    where
        I1: IntoIterator<Item = i32>,
        I2: IntoIterator<Item = i32> + Clone,
    {
        let mut r = self.neighbors(x, y, xs, ys).collect::<Vec<_>>();
        r.shuffle(&mut rand::thread_rng());
        r
    }

    fn neighbors<I1, I2>(&self, x: i32, y: i32, xs: I1, ys: I2) -> impl Iterator<Item = (i32, i32)>
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

#[derive(Debug)]
pub(crate) enum UpdateRule {
    PowderUpdateRule,
    LiquidUpdateRule,
    SpellUpdateRule(&'static SpellRule),
}
impl UpdateRule {
    fn update(&self, grid: &mut BlockGrid, x: i32, y: i32) {
        match self {
            UpdateRule::PowderUpdateRule => self.powder_update(grid, x, y),
            UpdateRule::LiquidUpdateRule => self.liquid_update(grid, x, y),
            UpdateRule::SpellUpdateRule(c) => self.spell_update(c, grid, x, y),
        }
    }

    fn powder_update(&self, grid: &mut BlockGrid, x: i32, y: i32) {
        let mut block = grid.get(x, y).unwrap();
        let block_data = block.data();
        if block.get(BlockProperties::MOVED_THIS_STEP) || block_data.physics != BlockPhysics::Powder
        {
            return;
        }

        if rand::random::<f32>() < block_data.powder_stability {
            block.set(BlockProperties::POWDER_STABLE, true);
            grid.set(x, y, block);
        }

        if block.get(BlockProperties::POWDER_STABLE) {
            let to_check = grid.neighbors(x, y, [0], [-1]);
            self.powder_try_moves(grid, x, y, to_check);
        } else {
            let to_check = grid
                .neighbors(x, y, [0], [-1])
                .chain(grid.neighbors_shuffle(x, y, [-1, 1], [-1]))
                .chain(grid.neighbors_shuffle(x, y, [-1, 1], [0]));
            self.powder_try_moves(grid, x, y, to_check);
        }
    }

    fn powder_try_moves<I>(&self, grid: &mut BlockGrid, x: i32, y: i32, to_check: I)
    where
        I: IntoIterator<Item = (i32, i32)>,
    {
        let mut block = grid.get(x, y).unwrap();
        let block_data = block.data();
        for (x2, y2) in to_check {
            let block2 = grid.get(x2, y2);
            if block2.is_none() {
                continue;
            }
            let mut block2 = block2.unwrap();
            let block2_data = block2.data();
            if block2.get(BlockProperties::MOVED_THIS_STEP)
                || block2_data.density >= block_data.density
            {
                continue;
            }

            block.set(BlockProperties::MOVED_THIS_STEP, true);
            if block2_data.physics != BlockPhysics::None {
                block2.set(BlockProperties::MOVED_THIS_STEP, true);
            }
            grid.set(x, y, block2);
            grid.set(x2, y2, block);

            for (x3, y3) in grid.neighbors(x, y, -1..2, -1..2) {
                let block3 = grid.get(x3, y3);
                if block3.is_none() {
                    continue;
                }
                let mut block3 = block3.unwrap();
                if block3.data().physics == BlockPhysics::Powder {
                    block3.set(BlockProperties::POWDER_STABLE, false);
                    grid.set(x3, y3, block3);
                }
            }
            return;
        }
    }

    fn liquid_update(&self, grid: &mut BlockGrid, x: i32, y: i32) {
        let mut block = grid.get(x, y).unwrap();
        let block_data = block.data();
        if block.get(BlockProperties::MOVED_THIS_STEP) || block_data.physics != BlockPhysics::Liquid
        {
            return;
        }

        let down = if block_data.density >= 0.1 { -1 } else { 1 };
        let to_check = grid
            .neighbors_shuffle(x, y, -1..2, [down])
            .into_iter()
            .chain(grid.neighbors_shuffle(x, y, [-1, 1], [0]));
        for (x2, y2) in to_check {
            let block2 = grid.get(x2, y2);
            if block2.is_none() {
                continue;
            }
            let mut block2 = block2.unwrap();
            let block2_data = block2.data();
            if block2.get(BlockProperties::MOVED_THIS_STEP)
                || down as f32 * (block2_data.density - block_data.density) <= 0.0
            {
                continue;
            }

            block.set(BlockProperties::MOVED_THIS_STEP, true);
            if block2_data.physics != BlockPhysics::None {
                block2.set(BlockProperties::MOVED_THIS_STEP, true);
            }
            grid.set(x, y, block2);
            grid.set(x2, y2, block);
            return;
        }
    }

    fn spell_update(&self, spell_rule: &SpellRule, grid: &mut BlockGrid, x: i32, y: i32) {
        // let mut block = grid.get(x, y).unwrap();
        // let block_data = block.data();

        let mut results: Vec<SpellResult> = vec![];
        spell_rule.spell.cast(
            &WorldInfo { grid },
            SpellTarget::new(Target::Block(x, y)),
            &mut |result| results.push(result),
        );

        for result in results {
            if rand::random::<f32>() > spell_rule.rate {
                continue;
            }
            let (x2, y2, mut block2) = match result.target.target {
                Target::Block(x2, y2) => match grid.get(x2, y2) {
                    Some(block) => (x2, y2, block),
                    _ => continue,
                },
                _ => todo!(),
            };
            for effect in result.effects {
                match effect {
                    SpellEffect::Send(Material(id)) => {
                        block2 = Block::new(*id);
                    }
                    SpellEffect::Send(BlockProperty(property)) => {
                        block2.set(*property, true);
                    }
                    SpellEffect::Receive(BlockProperty(property)) => {
                        block2.set(*property, false);
                    }
                    _ => todo!(),
                }
            }
            grid.set(x2, y2, block2);
        }
    }
}

pub(crate) struct UpdateRules {
    update_rules: Vec<UpdateRule>,
}

#[derive(Component)]
pub(crate) struct BlockGridComponent {
    grid: BlockGrid,
}

/// Initialize the simulation and its graphics
pub(crate) fn system_setup_block_grid(mut commands: Commands, mut textures: ResMut<Assets<Image>>) {
    let mut block_grid = BlockGrid::default();
    block_grid.set_range(115..120, 5..125, *SAND);
    block_grid.set_range(15..20, 5..125, *WATER);
    block_grid.set_range(55..60, 5..125, *COAL);
    // block_grid.set_range(65..70, 0..5, *FIRE);
    block_grid.set_range_func(65..70, 0..5, |block| {
        block.set(BlockProperties::BURNING, true)
    });

    let texture = Image::new_fill(
        Extent3d {
            width: GRID_SIZE as u32,
            height: GRID_SIZE as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Rgba8UnormSrgb,
    );
    let texture_handle = textures.add(texture);

    let mut update_rules: Vec<UpdateRule> =
        vec![UpdateRule::PowderUpdateRule, UpdateRule::LiquidUpdateRule];
    for r in NATURAL_RULES.iter() {
        update_rules.push(UpdateRule::SpellUpdateRule(r));
    }
    commands.insert_resource(UpdateRules { update_rules });

    let scale = 2.0;
    commands
        .spawn_bundle(SpriteBundle {
            transform: Transform::from_xyz(0.0, 0.0, 2.0).with_scale(Vec3::splat(scale)),
            texture: texture_handle,
            ..Default::default()
        })
        .insert(BlockGridComponent { grid: block_grid });
}

/// Step the simulation, update the graphics
pub(crate) fn system_update_block_grid(
    // mut block_grid: ResMut<BlockGrid>,
    update_rules: Res<UpdateRules>,
    mut textures: ResMut<Assets<Image>>,
    mut query: Query<(&mut BlockGridComponent, &Handle<Image>)>,
) {
    for (mut block_grid, texture_handle) in query.iter_mut() {
        let span = info_span!("Stepping blocks").entered();
        block_grid.grid.step(&update_rules);
        span.exit();

        let span = info_span!("Updating sprites").entered();
        let texture = textures.get_mut(texture_handle).unwrap();

        for (x, y) in block_grid
            .grid
            .all_matching(&BlockProperty(BlockProperties::CHANGED_THIS_STEP))
        {
            let block = block_grid.grid.get(x, y).unwrap();
            let mut color = block.color();

            // Draw all burning blocks as fire
            if block.get(BlockProperties::BURNING) {
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
        span.exit();
    }
}
