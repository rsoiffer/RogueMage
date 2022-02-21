use crate::{blocks::*, chemistry::Property::*, spells::*, storage::*};
use bevy::{
    math::Vec3,
    prelude::{info_span, Commands, Component, Query, ResMut, Transform},
    sprite::{Sprite, SpriteBundle},
};
use rand::random;

fn set_range<I1, I2>(storage: &mut StorageManager, xs: I1, ys: I2, material: u16)
where
    I1: IntoIterator<Item = i32>,
    I2: IntoIterator<Item = i32> + Clone,
{
    for x in xs {
        for y in ys.clone() {
            storage
                .material
                .update(Object::Block(x, y), Object::Block(x, y), |x| material);
        }
    }
}

fn set_range_prop<I1, I2>(storage: &mut StorageManager, xs: I1, ys: I2, property: BlockProperties)
where
    I1: IntoIterator<Item = i32>,
    I2: IntoIterator<Item = i32> + Clone,
{
    let mut storage = storage.get_prop(property);
    for x in xs {
        for y in ys.clone() {
            storage.update(Object::Block(x, y), Object::Block(x, y), |_| true);
        }
    }
}

fn run_natural_rule(storage: &mut StorageManager, spell: &SpellRule) {
    storage.for_each_entry(&spell.selector, |(source, target, connection)| {
        let rate = spell.rate * connection;
        for effect in &spell.effects {
            match effect {
                SpellEffect::Send(BlockProperty(property)) => {
                    if random::<f32>() < rate {
                        storage.get_prop(*property).update(target, target, |x| true);
                    }
                }
                SpellEffect::Receive(BlockProperty(property)) => {
                    if random::<f32>() < rate {
                        storage
                            .get_prop(*property)
                            .update(target, target, |x| false);
                    }
                }
                _ => todo!(),
            }
        }
    });
}

#[derive(Component)]
pub(crate) struct BlockSprite {
    x: i32,
    y: i32,
}

/// Initialize the simulation and its graphics
pub(crate) fn system_setup_block_grid(mut commands: Commands) {
    let mut storage = StorageManager::new();
    set_range(&mut storage, 0..128, 0..128, *AIR);
    set_range(&mut storage, 115..120, 5..125, *SAND);
    set_range(&mut storage, 115..120, 5..125, *SAND);
    set_range(&mut storage, 15..20, 5..125, *WATER);
    set_range(&mut storage, 55..60, 5..125, *COAL);
    // block_grid.set_range(65..70, 0..5, *FIRE);
    set_range_prop(&mut storage, 35..70, 0..5, BlockProperties::BURNING);
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
        let material = storage
            .material
            .current
            .get_option(&Object::Block(bs.x, bs.y), &Object::Block(bs.x, bs.y))
            .cloned()
            .unwrap_or_default();
        let block_data = ALL_BLOCK_DATA.get(material as usize).unwrap();
        let x = random::<f32>();
        sprite.color = block_data.color1 * x + block_data.color2 * (1.0 - x);

        // Draw all burning blocks as fire
        if storage
            .get_prop(BlockProperties::BURNING)
            .current
            .get_option(&Object::Block(bs.x, bs.y), &Object::Block(bs.x, bs.y))
            .cloned()
            .unwrap_or_default()
        {
            let x = rand::random::<f32>();
            let fire_data = ALL_BLOCK_DATA.get(*FIRE as usize).unwrap();
            sprite.color = fire_data.color1 * x + fire_data.color2 * (1.0 - x);
        }
    }
    span.exit();
}
