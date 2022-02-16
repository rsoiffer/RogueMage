use bevy::{
    math::Vec3,
    prelude::{AssetServer, Color, Commands, Component, Query, Res, ResMut, Transform},
    sprite::{Sprite, SpriteBundle},
};
use bitflags::bitflags;
use lazy_static::lazy_static;
use rand::seq::SliceRandom;

bitflags! {
    #[derive(Default)]
    struct BlockProperties: u32 {
        /// Has this block already moved this step
        const MOVED_THIS_STEP = 1 << 0;
        /// Has this block settled into a stable state - can only be true for powders
        const POWDER_STABLE = 1 << 1;
        /// Is this block currently on fire
        const BURNING = 1 << 2;
    }
}

#[derive(Clone, Copy, Debug)]
struct Block {
    /// The index into the block definitions array
    id: u16,
    /// Used to vary colors among blocks of the same type
    color_seed: u8,
    /// Reserved for future use
    damage: u8,
    /// The stored properties, all boolean-valued
    stored_properties: BlockProperties,
}

impl Default for Block {
    fn default() -> Self {
        Self {
            id: Default::default(),
            color_seed: rand::random(),
            damage: Default::default(),
            stored_properties: Default::default(),
        }
    }
}

impl Block {
    fn data(&self) -> &'static BlockData {
        ALL_BLOCK_DATA.get(self.id as usize).unwrap()
    }

    fn get(&self, property: BlockProperties) -> bool {
        self.stored_properties.contains(property)
    }

    fn get_any(&self, property: BlockProperties) -> bool {
        self.stored_properties.intersects(property)
    }

    fn set(&mut self, property: BlockProperties, value: bool) {
        self.stored_properties.set(property, value)
    }
}

#[derive(Debug, PartialEq)]
enum BlockPhysics {
    /// Doesn't move, can be pushed around
    None,
    /// Doesn't move, can't be pushed around
    Solid,
    /// Forms into piles, has friction
    Powder,
    /// Frictionless liquids and gasses
    Liquid,
}

#[derive(Debug)]
pub(crate) struct BlockData {
    /// Internal block name
    name: &'static str,
    /// First color extreme
    color1: Color,
    /// Second color extreme
    color2: Color,
    /// Mass of a single block
    density: f32,
    /// Physics of this block
    physics: BlockPhysics,
    /// Stability of this powder - only makes sense for powders
    powder_stability: f32,
}

lazy_static! {
    pub(crate) static ref ALL_BLOCK_DATA: Vec<BlockData> = vec![
        BlockData {
            name: "Air",
            color1: Color::rgba(0.0, 0.0, 0.0, 0.0),
            color2: Color::rgba(0.0, 0.0, 0.0, 0.0),
            density: 0.1,
            physics: BlockPhysics::None,
            powder_stability: 0.0,
        },
        BlockData {
            name: "Stone",
            color1: Color::rgb(0.5, 0.5, 0.5),
            color2: Color::rgb(0.3, 0.3, 0.3),
            density: 1.0,
            physics: BlockPhysics::Solid,
            powder_stability: 0.0,
        },
        BlockData {
            name: "Water",
            color1: Color::rgba(0.2, 0.4, 1.0, 0.7),
            color2: Color::rgba(0.2, 0.4, 1.0, 0.7),
            density: 0.5,
            physics: BlockPhysics::Liquid,
            powder_stability: 0.0,
        },
        BlockData {
            name: "Sand",
            color1: Color::rgb(1.0, 0.8, 0.3),
            color2: Color::rgb(0.8, 0.6, 0.2),
            density: 0.8,
            physics: BlockPhysics::Powder,
            powder_stability: 0.3,
        },
        BlockData {
            name: "Wood",
            color1: Color::rgb(0.8, 0.4, 0.3),
            color2: Color::rgb(0.6, 0.2, 0.2),
            density: 1.0,
            physics: BlockPhysics::Solid,
            powder_stability: 0.0,
        },
        BlockData {
            name: "Coal",
            color1: Color::rgb(0.2, 0.2, 0.2),
            color2: Color::rgb(0.1, 0.1, 0.1),
            density: 1.0,
            physics: BlockPhysics::Powder,
            powder_stability: 0.7,
        },
        BlockData {
            name: "Fire",
            color1: Color::rgb(1.0, 1.0, 0.4),
            color2: Color::rgb(1.0, 0.3, 0.0),
            density: 0.1,
            physics: BlockPhysics::None,
            powder_stability: 0.0,
        },
        BlockData {
            name: "Smoke",
            color1: Color::rgba(0.1, 0.1, 0.1, 0.5),
            color2: Color::rgba(0.2, 0.2, 0.2, 0.2),
            density: 0.05,
            physics: BlockPhysics::Liquid,
            powder_stability: 0.0,
        },
        BlockData {
            name: "Steam",
            color1: Color::rgba(1.0, 1.0, 1.0, 0.3),
            color2: Color::rgba(1.0, 1.0, 1.0, 0.1),
            density: 0.05,
            physics: BlockPhysics::Liquid,
            powder_stability: 0.0,
        },
    ];
}

/// The size of the whole grid of blocks
const GRID_SIZE: usize = 128;

pub(crate) struct BlockGrid {
    /// The 2d array of blocks
    grid: [[Block; GRID_SIZE]; GRID_SIZE],
    /// If true, simulate right-to-left
    flip_sim_dir: bool,
}

impl Default for BlockGrid {
    fn default() -> Self {
        Self {
            grid: [[Block::default(); GRID_SIZE]; GRID_SIZE],
            flip_sim_dir: Default::default(),
        }
    }
}

impl BlockGrid {
    fn get(&self, x: usize, y: usize) -> Block {
        return self.grid[x][y];
    }

    fn set(&mut self, x: usize, y: usize, block: Block) {
        self.grid[x][y] = block;
    }

    fn set_range<I1, I2>(&mut self, xs: I1, ys: I2, id: u16)
    where
        I1: IntoIterator<Item = usize>,
        I2: IntoIterator<Item = usize> + Clone,
    {
        for x in xs {
            for y in ys.clone() {
                self.set(
                    x,
                    y,
                    Block {
                        id,
                        ..Default::default()
                    },
                );
            }
        }
    }

    fn step(&mut self, update_rules: &UpdateRules) {
        let xs = if self.flip_sim_dir {
            num::range_step(GRID_SIZE as i32 - 1, 0, -1)
        } else {
            num::range_step(0, GRID_SIZE as i32, 1)
        };
        let ys = 0..GRID_SIZE;

        for rule in &update_rules.update_rules {
            for y in ys.clone() {
                for x in xs.clone() {
                    let x = x as usize;
                    rule.update(self, x, y);
                }
            }
        }
        self.flip_sim_dir = !self.flip_sim_dir;
    }

    fn neighbors<I1, I2>(&self, x: usize, y: usize, xs: I1, ys: I2) -> Vec<(usize, usize)>
    where
        I1: IntoIterator<Item = i32>,
        I2: IntoIterator<Item = i32> + Clone,
    {
        let mut r = vec![];
        for x_offset in xs {
            for y_offset in ys.clone() {
                let x2 = x as i32 + x_offset;
                let y2 = y as i32 + y_offset;
                if x2 >= 0 && x2 < GRID_SIZE as i32 && y2 >= 0 && y2 < GRID_SIZE as i32 {
                    r.push((x2 as usize, y2 as usize));
                }
            }
        }
        r.shuffle(&mut rand::thread_rng());
        r
    }
}

pub(crate) trait UpdateRule: Send + Sync {
    fn update(&self, grid: &mut BlockGrid, x: usize, y: usize);
}

pub(crate) struct UpdateRules {
    update_rules: Vec<Box<dyn UpdateRule>>,
}

struct ResetUpdateRule {}
impl UpdateRule for ResetUpdateRule {
    fn update(&self, grid: &mut BlockGrid, x: usize, y: usize) {
        let mut block = grid.get(x, y);
        block.set(BlockProperties::MOVED_THIS_STEP, false);
        grid.set(x, y, block);
    }
}

struct PowderUpdateRule {}
impl UpdateRule for PowderUpdateRule {
    fn update(&self, grid: &mut BlockGrid, x: usize, y: usize) {
        let mut block = grid.get(x, y);
        let block_data = block.data();
        if block.get(BlockProperties::MOVED_THIS_STEP) || block_data.physics != BlockPhysics::Powder
        {
            return;
        }

        if rand::random::<f32>() < block_data.powder_stability {
            block.set(BlockProperties::POWDER_STABLE, true);
            grid.set(x, y, block);
        }
        let mut to_check = grid.neighbors(x, y, [0], [-1]);
        if !block.get(BlockProperties::POWDER_STABLE) {
            to_check.extend(grid.neighbors(x, y, [-1, 1], [-1]));
            to_check.extend(grid.neighbors(x, y, [-1, 1], [0]));
        }
        for (x2, y2) in to_check {
            let mut block2 = grid.get(x2, y2);
            let block2_data = block2.data();
            if block2.get(BlockProperties::MOVED_THIS_STEP)
                || block2_data.density >= block_data.density
            {
                continue;
            }

            block.set(BlockProperties::MOVED_THIS_STEP, true);
            if block2.id != 0 {
                block2.set(BlockProperties::MOVED_THIS_STEP, true);
            }
            grid.set(x, y, block2);
            grid.set(x2, y2, block);

            for (x3, y3) in grid.neighbors(x, y, [-1, 0, 1], [-1, 0, 1]) {
                let mut block3 = grid.get(x3, y3);
                if block3.data().physics == BlockPhysics::Powder {
                    block3.set(BlockProperties::POWDER_STABLE, false);
                    grid.set(x3, y3, block3);
                }
            }
            return;
        }
    }
}

struct LiquidUpdateRule {}
impl UpdateRule for LiquidUpdateRule {
    fn update(&self, grid: &mut BlockGrid, x: usize, y: usize) {
        let mut block = grid.get(x, y);
        let block_data = block.data();
        if block.get(BlockProperties::MOVED_THIS_STEP) || block_data.physics != BlockPhysics::Liquid
        {
            return;
        }

        let down = if block_data.density >= 0.1 { -1 } else { 1 };
        let mut to_check = grid.neighbors(x, y, -1..2, [down]);
        to_check.extend(grid.neighbors(x, y, [-1, 1], [0]));
        for (x2, y2) in to_check {
            let mut block2 = grid.get(x2, y2);
            let block2_data = block2.data();
            if block2.get(BlockProperties::MOVED_THIS_STEP)
                || down as f32 * (block2_data.density - block_data.density) <= 0.0
            {
                continue;
            }

            block.set(BlockProperties::MOVED_THIS_STEP, true);
            if block2.id != 0 {
                block2.set(BlockProperties::MOVED_THIS_STEP, true);
            }
            grid.set(x, y, block2);
            grid.set(x2, y2, block);
            return;
        }
    }
}

struct FireUpdateRule {}
impl UpdateRule for FireUpdateRule {
    fn update(&self, grid: &mut BlockGrid, x: usize, y: usize) {
        let mut block = grid.get(x, y);
        if block.id != 6 {
            return;
        }
        if rand::random::<f32>() < 0.01 {
            block.id = 7;
            grid.set(x, y, block);
        }
        for (x2, y2) in grid.neighbors(x, y, -2..3, -2..3) {
            let mut block2 = grid.get(x2, y2);
            if block2.id != 5 {
                continue;
            }
            if rand::random::<f32>() < 0.01 {
                block2.set(BlockProperties::BURNING, true);
                grid.set(x2, y2, block2);
            }
        }
    }
}

struct BurnUpdateRule {}
impl UpdateRule for BurnUpdateRule {
    fn update(&self, grid: &mut BlockGrid, x: usize, y: usize) {
        let mut block = grid.get(x, y);
        if !block.get(BlockProperties::BURNING) {
            return;
        }
        if rand::random::<f32>() < 0.01 {
            block.set(BlockProperties::BURNING, false);
            grid.set(x, y, block);
        } else if rand::random::<f32>() < 0.005 {
            block.id = 7;
            block.set(BlockProperties::BURNING, false);
            grid.set(x, y, block);
        }
        for (x2, y2) in grid.neighbors(x, y, -1..2, -1..2) {
            let mut block2 = grid.get(x2, y2);
            if block2.id != 0 {
                continue;
            }
            if rand::random::<f32>() < 0.01 {
                block2.id = 6;
                block2.color_seed = rand::random();
                grid.set(x2, y2, block2);
            }
        }
    }
}

#[derive(Default)]
struct Reagent {
    dist: i32,
    id1: Option<u16>,
    props1: Option<BlockProperties>,
    neg_props1: Option<BlockProperties>,
    id2: Option<u16>,
    props2: Option<BlockProperties>,
    neg_props2: Option<BlockProperties>,
}

impl Reagent {
    fn new(dist: i32) -> Reagent {
        Reagent {
            dist,
            ..Default::default()
        }
    }

    fn select_block(mut self, id: u16) -> Reagent {
        self.id1 = Some(id);
        self
    }

    fn select_prop(mut self, property: BlockProperties, value: bool) -> Reagent {
        if value {
            self.props1 = Some(property);
        } else {
            self.neg_props1 = Some(property)
        }
        self
    }

    fn transform(mut self, id: u16) -> Reagent {
        self.id2 = Some(id);
        self
    }

    fn set_prop(mut self, property: BlockProperties, value: bool) -> Reagent {
        if value {
            self.props2 = Some(property);
        } else {
            self.neg_props2 = Some(property)
        }
        self
    }

    fn matches(&self, block: Block) -> bool {
        match self.id1 {
            Some(id1) => {
                if id1 != block.id {
                    return false;
                }
            }
            _ => {}
        }
        match self.props1 {
            Some(props1) => {
                if !block.get(props1) {
                    return false;
                }
            }
            _ => {}
        }
        match self.neg_props1 {
            Some(neg_props1) => {
                if block.get_any(neg_props1) {
                    return false;
                }
            }
            _ => {}
        }
        return true;
    }

    fn update(&self, block: &mut Block) {
        match self.id2 {
            Some(id2) => {
                block.id = id2;
                block.color_seed = rand::random();
            }
            _ => {}
        }
        match self.props2 {
            Some(props2) => {
                block.set(props2, true);
            }
            _ => {}
        }
        match self.neg_props2 {
            Some(neg_props2) => {
                block.set(neg_props2, false);
            }
            _ => {}
        }
    }
}

struct ChemicalUpdateRule {
    probability: f32,
    reagents: Vec<Reagent>,
}

impl UpdateRule for &ChemicalUpdateRule {
    fn update(&self, grid: &mut BlockGrid, x: usize, y: usize) {
        if rand::random::<f32>() < self.probability {
            let mut selections = vec![];
            'a: for reagent in &self.reagents {
                let range = -reagent.dist..reagent.dist + 1;
                for (x2, y2) in grid.neighbors(x, y, range.clone(), range) {
                    let block2 = grid.get(x2, y2);
                    if reagent.matches(block2) && !selections.contains(&(x2, y2)) {
                        selections.push((x2, y2));
                        continue 'a;
                    }
                }
                return;
            }
            for ((x, y), reagent) in Iterator::zip(selections.into_iter(), self.reagents.iter()) {
                let mut block = grid.get(x, y);
                reagent.update(&mut block);
                grid.set(x, y, block);
            }
        }
    }
}

lazy_static! {
    static ref FIRE_RULES: Vec<ChemicalUpdateRule> = vec![
        ChemicalUpdateRule {
            probability: 0.05,
            reagents: vec![Reagent::new(0).select_block(6).transform(0)],
        },
        ChemicalUpdateRule {
            probability: 0.1,
            reagents: vec![
                Reagent::new(0).select_block(6),
                Reagent::new(2)
                    .select_block(5)
                    .set_prop(BlockProperties::BURNING, true)
            ],
        },
        ChemicalUpdateRule {
            probability: 0.01,
            reagents: vec![Reagent::new(0)
                .select_prop(BlockProperties::BURNING, true)
                .set_prop(BlockProperties::BURNING, false)],
        },
        ChemicalUpdateRule {
            probability: 0.2,
            reagents: vec![
                Reagent::new(0).select_prop(BlockProperties::BURNING, true),
                Reagent::new(1).select_block(0).transform(6)
            ],
        },
        ChemicalUpdateRule {
            probability: 0.005,
            reagents: vec![Reagent::new(0)
                .select_prop(BlockProperties::BURNING, true)
                .transform(7)
                .set_prop(BlockProperties::BURNING, false)],
        },
        ChemicalUpdateRule {
            probability: 0.002,
            reagents: vec![Reagent::new(0).select_block(7).transform(0)],
        },
        ChemicalUpdateRule {
            probability: 1.0,
            reagents: vec![
                Reagent::new(0).select_block(6).transform(0),
                Reagent::new(1).select_block(2).transform(8),
            ],
        },
        ChemicalUpdateRule {
            probability: 0.002,
            reagents: vec![Reagent::new(0).select_block(8).transform(2)],
        },
    ];
}

#[derive(Component)]
pub(crate) struct BlockSprite {
    x: usize,
    y: usize,
}

/// Initialize the simulation and its graphics
pub(crate) fn system_setup_block_grid(mut commands: Commands, asset_server: Res<AssetServer>) {
    let mut block_grid = BlockGrid::default();
    block_grid.set_range(115..120, 5..125, 3);
    block_grid.set_range(15..20, 5..125, 2);
    block_grid.set_range(55..60, 5..125, 5);
    block_grid.set_range(65..70, 0..5, 6);
    commands.insert_resource(block_grid);

    let mut update_rules: Vec<Box<dyn UpdateRule>> = vec![
        Box::new(ResetUpdateRule {}),
        Box::new(PowderUpdateRule {}),
        Box::new(LiquidUpdateRule {}),
        // Box::new(FireUpdateRule {}),
        // Box::new(BurnUpdateRule {}),
    ];
    for r in FIRE_RULES.iter() {
        update_rules.push(Box::new(r));
    }
    commands.insert_resource(UpdateRules { update_rules });

    for x in 0..GRID_SIZE {
        for y in 0..GRID_SIZE {
            let scale = 3.0;
            commands
                .spawn_bundle(SpriteBundle {
                    transform: Transform::from_xyz(
                        -192.0 + scale * x as f32,
                        -192.0 + scale * y as f32,
                        2.0,
                    )
                    .with_scale(Vec3::splat(scale)),
                    ..Default::default()
                })
                .insert(BlockSprite { x, y });
        }
    }
}

/// Step the simulation, update the graphics
pub(crate) fn system_update_block_grid(
    mut block_grid: ResMut<BlockGrid>,
    update_rules: Res<UpdateRules>,
    mut query: Query<(&mut Sprite, &BlockSprite)>,
) {
    block_grid.step(&update_rules);
    for (mut sprite, bs) in query.iter_mut() {
        let block = block_grid.get(bs.x, bs.y);
        let x = block.color_seed as f32 / 255.0;
        sprite.color = block.data().color1 * x + block.data().color2 * (1.0 - x);

        // Draw all burning blocks as fire
        if block.get(BlockProperties::BURNING) {
            let x = rand::random::<f32>();
            let fire_data = ALL_BLOCK_DATA.get(6).unwrap();
            sprite.color = fire_data.color1 * x + fire_data.color2 * (1.0 - x);
        }
    }
}
