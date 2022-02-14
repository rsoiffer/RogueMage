#[macro_use]
extern crate lazy_static;

mod blocks;
mod chemistry;
mod math_utils;
mod parser;

use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use blocks::*;
use chemistry::*;

fn main() {
    App::new()
        .insert_resource(WindowDescriptor {
            width: 960.0,
            height: 540.0,
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
        .init_resource::<BlockTextureAtlasResource>()
        .insert_resource(ClearColor(Color::rgb(0.5, 0.7, 1.0)))
        .insert_resource(RapierConfiguration {
            gravity: Vector::y() * -1000.0,
            ..Default::default()
        })
        .add_startup_system(setup_block_atlas.label("setup block atlas"))
        .add_startup_system(setup.after("setup block atlas"))
        .add_system(chemistry_system)
        .add_system(update_block_sprites)
        .add_system(update_chemistry_graphics)
        .add_system(update_fire_particles)
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    block_texture_atlas_resource: Res<BlockTextureAtlasResource>,
) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());

    let mut block_spawner = BlockSpawner {
        commands: &mut commands,
        block_texture_atlas_resource: &block_texture_atlas_resource,
    };
    for x in -16..16 {
        if x % 5 < 2 {
            block_spawner.spawn(x, 1, PLANKS);
        }
        block_spawner.spawn_fire(x, 0, GRASS, if x == 0 { 0.1 } else { 0.0 });
        for y in -3..0 {
            block_spawner.spawn(x, y, DIRT);
        }
        for y in -6..-3 {
            block_spawner.spawn(x, y, STONE);
        }
    }

    commands
        .spawn_bundle(SpriteBundle {
            texture: asset_server.load("sprites/cat_alive.png"),
            transform: Transform::from_xyz(100.0, 100.0, 2.0).with_scale(Vec3::splat(2.0)),
            ..Default::default()
        })
        .insert_bundle(RigidBodyBundle {
            position: Vec2::new(100.0, 100.0).into(),
            ..Default::default()
        })
        .insert_bundle(ColliderBundle {
            shape: ColliderShape::cuboid(16.0, 16.0).into(),
            ..Default::default()
        })
        .insert(RigidBodyPositionSync::Discrete);
}
