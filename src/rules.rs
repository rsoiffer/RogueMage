use crate::blocks::*;
use crate::cells::*;
use crate::chemistry::DependentProperty::*;
use crate::chemistry::Property::*;
use crate::chemistry::StoredProperty::*;
use crate::chemistry::*;
use crate::spells::SpellSelector::*;
use crate::spells::*;
use bevy::prelude::Assets;
use bevy::prelude::Handle;
use bevy::prelude::Image;
use bevy::render::render_resource::Extent3d;
use bevy::render::render_resource::TextureDimension;
use bevy::render::render_resource::TextureFormat;
use bevy::{
    math::Vec3,
    prelude::{info_span, Commands, Query, Res, ResMut, Transform},
    sprite::SpriteBundle,
};

#[derive(Debug)]
pub(crate) enum UpdateRule {
    GravityUpdateRule,
    LiquidUpdateRule,
    SpellUpdateRule(&'static SpellRule),
}
impl UpdateRule {
    fn only_run_on(&self) -> Property {
        match self {
            UpdateRule::GravityUpdateRule => Dependent(Liquid),
            UpdateRule::LiquidUpdateRule => Dependent(Liquid),
            UpdateRule::SpellUpdateRule(sr) => match &sr.spell {
                Spell::Select(selector, _) => match selector {
                    Is(property) => *property,
                    Bind(selector, _) => match **selector {
                        Is(property) => property,
                        _ => panic!("Spell started with non-Is selector: {:?}", sr),
                    },
                    _ => panic!("Spell started with non-Is selector: {:?}", sr),
                },
                _ => panic!("Spell doesn't have any selectors: {:?}", sr),
            },
        }
    }

    fn update(&self, info: &mut WorldInfo, x: i32, y: i32) {
        match self {
            UpdateRule::GravityUpdateRule => gravity_update(&mut info.grid, x, y),
            UpdateRule::LiquidUpdateRule => liquid_update(&mut info.grid, x, y),
            UpdateRule::SpellUpdateRule(c) => spell_update(c, info, x, y),
        }
    }
}

fn gravity_update(grid: &mut BlockGrid, x: i32, mut y: i32) {
    let mut block = grid.get(x, y).unwrap();
    let block_data = block.data();
    if block.get(BlockProperties::MOVED_THIS_STEP) || block_data.physics != BlockPhysics::Liquid {
        return;
    }

    let down = if block_data.density >= 0.0 { -1 } else { 1 };
    for i in 0..5 {
        let y2 = y + down;
        let block2 = grid.get(x, y2);
        if block2.is_none() {
            break;
        }
        let mut block2 = block2.unwrap();
        let block2_data = block2.data();

        let fall_desire = down as f32 * (block2_data.density - block_data.density);
        if fall_desire <= 0.0 || i as f32 + rand::random::<f32>() > 2.0 * fall_desire {
            break;
        }

        block.set(BlockProperties::MOVED_THIS_STEP, true);
        if block2_data.physics != BlockPhysics::None {
            block2.set(BlockProperties::MOVED_THIS_STEP, true);
        }
        grid.set(x, y, block2);
        grid.set(x, y2, block);
        mark_unstable(grid, x, y, block.id);
        y += down;
    }
}

fn liquid_update(grid: &mut BlockGrid, x: i32, y: i32) {
    let mut block = grid.get(x, y).unwrap();
    let block_data = block.data();
    // if block.get(BlockProperties::MOVED_THIS_STEP) || block_data.physics != BlockPhysics::Liquid
    // {
    //     return;
    // }
    if block_data.physics != BlockPhysics::Liquid {
        return;
    }

    if rand::random::<f32>() < block_data.powder_stability {
        block.set(BlockProperties::POWDER_STABLE, true);
        grid.set(x, y, block);
    }

    if block.get(BlockProperties::POWDER_STABLE) {
        return;
    }

    let to_check = grid.neighbors_shuffle(x, y, [-1, 1], [0]);
    for (x2, y2) in to_check {
        let block2 = grid.get(x2, y2);
        if block2.is_none() {
            continue;
        }
        let mut block2 = block2.unwrap();
        let block2_data = block2.data();

        let density_advantage = block_data.density - block2_data.density;

        if block2.get(BlockProperties::MOVED_THIS_STEP)
            || (density_advantage <= 0.0 && block2_data.physics != BlockPhysics::None)
            || f32::abs(density_advantage) <= 1.0 * rand::random::<f32>()
        {
            continue;
        }

        block.set(BlockProperties::MOVED_THIS_STEP, true);
        if block2_data.physics != BlockPhysics::None {
            block2.set(BlockProperties::MOVED_THIS_STEP, true);
        }
        grid.set(x, y, block2);
        grid.set(x2, y2, block);

        mark_unstable(grid, x, y, block.id);
        return;
    }
}

fn mark_unstable(grid: &mut BlockGrid, x: i32, y: i32, id: u16) {
    for (x3, y3) in grid.neighbors(x, y, -1..2, -1..2) {
        let block3 = grid.get(x3, y3);
        if block3.is_none() {
            continue;
        }
        let mut block3 = block3.unwrap();
        let block3_data = block3.data();

        if block3_data.physics == BlockPhysics::Liquid && block3.id == id {
            block3.set(BlockProperties::POWDER_STABLE, false);
            grid.set(x3, y3, block3);
        }
    }
}

fn spell_update(spell_rule: &SpellRule, info: &mut WorldInfo, x: i32, y: i32) {
    // let mut block = grid.get(x, y).unwrap();
    // let block_data = block.data();

    let mut results: Vec<SpellResult> = vec![];
    spell_rule
        .spell
        .cast(info, SpellTarget::new(Target::Block(x, y)), &mut |result| {
            results.push(result)
        });

    for result in results {
        if rand::random::<f32>() > spell_rule.rate {
            continue;
        }
        let (x2, y2, mut block2) = match result.target.target {
            Target::Block(x2, y2) => match info.grid.get(x2, y2) {
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
        info.grid.set(x2, y2, block2);
    }
}

pub(crate) struct UpdateRules {
    update_rules: Vec<UpdateRule>,
}

/// Initialize the simulation and its graphics
pub(crate) fn system_setup_block_grid(mut commands: Commands, mut textures: ResMut<Assets<Image>>) {
    let mut info = WorldInfo::default();
    info.grid.set_range(115..120, 5..125, *SAND);
    info.grid.set_range(15..20, 5..125, *WATER);
    info.grid.set_range(55..60, 5..125, *COAL);
    // info.grid.set_range(65..70, 0..5, *FIRE);
    // info.grid.set_range_func(65..70, 0..5, |block| block.set(Stored(Burning), true));
    info.grid.set_range(135..230, 15..225, *WATER);

    let mut texture = Image::new_fill(
        Extent3d {
            width: GRID_SIZE as u32,
            height: GRID_SIZE as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Rgba8UnormSrgb,
    );
    for x in 0..GRID_SIZE as i32 {
        for y in 0..GRID_SIZE as i32 {
            update_texture_pixel(&info.grid, &mut texture, x, y);
        }
    }
    let texture_handle = textures.add(texture);

    let mut update_rules: Vec<UpdateRule> =
        vec![UpdateRule::GravityUpdateRule, UpdateRule::LiquidUpdateRule];
    for r in NATURAL_RULES.iter() {
        update_rules.push(UpdateRule::SpellUpdateRule(r));
    }
    commands.insert_resource(UpdateRules { update_rules });

    let scale = 1.0;
    commands
        .spawn_bundle(SpriteBundle {
            transform: Transform::from_xyz(0.0, 0.0, 2.0).with_scale(Vec3::splat(scale)),
            texture: texture_handle,
            ..Default::default()
        })
        .insert(info);
}

/// Step the simulation, update the graphics
pub(crate) fn system_update_block_grid(
    // mut block_grid: ResMut<BlockGrid>,
    update_rules: Res<UpdateRules>,
    mut textures: ResMut<Assets<Image>>,
    mut query: Query<(&mut WorldInfo, &Handle<Image>)>,
) {
    for (mut info, texture_handle) in query.iter_mut() {
        let span = info_span!("Stepping blocks").entered();
        step(&mut info, &update_rules);
        span.exit();

        let span = info_span!("Updating sprites").entered();
        let texture = textures.get_mut(texture_handle).unwrap();

        for (x, y) in info
            .grid
            .all_matching(BlockProperty(BlockProperties::CHANGED_THIS_STEP))
        {
            update_texture_pixel(&info.grid, texture, x, y);
        }
        span.exit();
    }
}

fn step(info: &mut WorldInfo, update_rules: &UpdateRules) {
    let span = info_span!("Reset flags").entered();
    info.grid
        .clear_block_property(BlockProperties::MOVED_THIS_STEP);
    info.grid
        .clear_block_property(BlockProperties::CHANGED_THIS_STEP);
    span.exit();

    for rule in &update_rules.update_rules {
        let span = info_span!("Rule", rule = &bevy::utils::tracing::field::debug(rule)).entered();

        for (x, y) in info
            .grid
            .all_matching(rule.only_run_on())
            .collect::<Vec<_>>()
        {
            rule.update(info, x, y);
        }

        span.exit();
    }
}
