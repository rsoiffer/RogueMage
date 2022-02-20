use crate::blocks::*;
use crate::chemistry::Property::*;
use crate::chemistry::*;
use crate::spells::*;
use crate::storage::*;
use bevy::{
    math::Vec3,
    prelude::{info_span, Commands, Component, Query, ResMut, Transform},
    sprite::{Sprite, SpriteBundle},
};

fn set_range<I1, I2>(storage: &mut StorageManager, xs: I1, ys: I2, property: Property)
where
    I1: IntoIterator<Item = i32>,
    I2: IntoIterator<Item = i32> + Clone,
{
    storage.require(&SpellSelector::Is(property));
    let mut storage = storage.get(&SpellSelector::Is(property));
    for x in xs {
        for y in ys.clone() {
            storage
                .storage
                .add(Object::Block(x, y), Object::Block(x, y), 1.0);
        }
    }
}

fn run_natural_rule(storage: &mut StorageManager, spell: &SpellRule) {
    let storage1 = storage.get(&spell.selector);
    let targets = storage1.storage.digraph.entries();
    println!("Running spell {}", spell.name);
    println!("Spell selector: {:?}", spell.selector);
    println!("Spell selector digraph: {:?}", storage1.storage.digraph);
    println!(
        "Burning digraph: {:?}",
        storage
            .get(&SpellSelector::Is(BlockProperty(BlockProperties::BURNING)))
            .storage
            .digraph
    );
    for (&source, &target, &connection) in targets {
        println!("Applying spell {} to target {:?}", spell.name, target);
        let rate = spell.rate * connection;
        for effect in &spell.effects {
            match effect {
                SpellEffect::Summon => todo!(),
                SpellEffect::Send(property) => {
                    storage
                        .get(&SpellSelector::Is(*property))
                        .storage
                        .add(target, target, rate);
                }
                SpellEffect::Receive(property) => {
                    storage
                        .get(&SpellSelector::Is(*property))
                        .storage
                        .add(target, target, -rate);
                }
            }
        }
    }
}

#[derive(Component)]
pub(crate) struct BlockSprite {
    x: i32,
    y: i32,
}

/// Initialize the simulation and its graphics
pub(crate) fn system_setup_block_grid(mut commands: Commands) {
    let mut storage = StorageManager::default();
    set_range(&mut storage, 0..128, 0..128, Material(*AIR));
    set_range(&mut storage, 115..120, 5..125, Material(*SAND));
    set_range(&mut storage, 115..120, 5..125, Material(*SAND));
    set_range(&mut storage, 15..20, 5..125, Material(*WATER));
    set_range(&mut storage, 55..60, 5..125, Material(*COAL));
    // block_grid.set_range(65..70, 0..5, *FIRE);
    set_range(
        &mut storage,
        65..70,
        0..5,
        BlockProperty(BlockProperties::BURNING),
    );
    for spell_rule in NATURAL_RULES.iter() {
        storage.require(&spell_rule.selector);
    }
    commands.insert_resource(storage);

    // let mut update_rules: Vec<UpdateRule> = vec![
    //     UpdateRule::ResetUpdateRule,
    //     UpdateRule::PowderUpdateRule,
    //     UpdateRule::LiquidUpdateRule,
    // ];
    // for r in NATURAL_RULES.iter() {
    //     update_rules.push(UpdateRule::SpellUpdateRule(r));
    // }
    // commands.insert_resource(UpdateRules { update_rules });

    for x in 0..128 as i32 {
        for y in 0..128 as i32 {
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
    mut storage: ResMut<StorageManager>,
    mut query: Query<(&mut Sprite, &BlockSprite)>,
) {
    // let span = info_span!("Stepping blocks").entered();
    // block_grid.step(&update_rules);
    // span.exit();
    storage.recompute_all_caches();
    for spell_rule in NATURAL_RULES.iter() {
        run_natural_rule(storage.as_mut(), spell_rule);
    }

    let span = info_span!("Updating sprites").entered();
    for (mut sprite, bs) in query.iter_mut() {
        // let block = block_grid.get(bs.x, bs.y).unwrap();
        // sprite.color = block.color();

        // Draw all burning blocks as fire
        if storage
            .get(&SpellSelector::Is(BlockProperty(BlockProperties::BURNING)))
            .storage
            .digraph
            .get(&Object::Block(bs.x, bs.y), &Object::Block(bs.x, bs.y))
            > 0.5
        {
            let x = rand::random::<f32>();
            let fire_data = ALL_BLOCK_DATA.get(*FIRE as usize).unwrap();
            sprite.color = fire_data.color1 * x + fire_data.color2 * (1.0 - x);
        }
    }
    span.exit();
}
