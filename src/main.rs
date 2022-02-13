#[macro_use]
extern crate lazy_static;

mod blocks;
mod chemistry;

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
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    block_texture_atlas_resource: Res<BlockTextureAtlasResource>,
) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());

    let stone = 1;
    let dirt = 2;
    let grass = 3;
    for x in -16..16 {
        spawn_block(
            &mut commands,
            &block_texture_atlas_resource,
            BlockInfo { x, y: 0, id: grass },
        );
        for y in -3..0 {
            spawn_block(
                &mut commands,
                &block_texture_atlas_resource,
                BlockInfo { x, y, id: dirt },
            );
        }
        for y in -10..-3 {
            spawn_block(
                &mut commands,
                &block_texture_atlas_resource,
                BlockInfo { x, y, id: stone },
            );
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
