mod blocks;
mod cells;
mod chemistry;
mod math_utils;
mod parser;
mod rules_asset;

use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use blocks::*;
use cells::*;
use chemistry::*;
use rules_asset::{RulesAsset, RulesAssetLoader};

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
        .init_resource::<NaturalRules>()
        .insert_resource(ClearColor(Color::rgb(0.5, 0.7, 1.0)))
        .insert_resource(RapierConfiguration {
            gravity: Vector::y() * -1000.0,
            ..Default::default()
        })
        .add_asset::<RulesAsset>()
        .init_asset_loader::<RulesAssetLoader>()
        .add_startup_system(setup_block_atlas.label("setup block atlas"))
        .add_startup_system(setup.after("setup block atlas"))
        .add_system(chemistry_system)
        .add_system(update_block_sprites)
        .add_system(update_chemistry_graphics)
        .add_system(update_fire_particles)
        .add_startup_system(system_setup_block_grid)
        .add_system(system_update_block_grid)
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    block_texture_atlas_resource: Res<BlockTextureAtlasResource>,
    mut natural_rules: ResMut<NaturalRules>,
) {
    asset_server.watch_for_changes().unwrap();

    natural_rules.0 = asset_server.load("natural.rules");

    commands.spawn_bundle(OrthographicCameraBundle::new_2d());

    // let mut block_spawner = BlockSpawner {
    //     commands: &mut commands,
    //     block_texture_atlas_resource: &block_texture_atlas_resource,
    // };
    // for x in -16..16 {
    //     if x % 5 < 2 {
    //         block_spawner.spawn(x, 1, PLANKS);
    //     }
    //     block_spawner.spawn_fire(x, 0, GRASS, if x == 0 { 0.1 } else { 0.0 });
    //     for y in -3..0 {
    //         block_spawner.spawn(x, y, DIRT);
    //     }
    //     for y in -6..-3 {
    //         block_spawner.spawn(x, y, STONE);
    //     }
    // }

    // commands
    //     .spawn_bundle(SpriteBundle {
    //         texture: asset_server.load("sprites/cat_alive.png"),
    //         transform: Transform::from_xyz(100.0, 100.0, 2.0).with_scale(Vec3::splat(2.0)),
    //         ..Default::default()
    //     })
    //     .insert_bundle(RigidBodyBundle {
    //         position: Vec2::new(100.0, 100.0).into(),
    //         ..Default::default()
    //     })
    //     .insert_bundle(ColliderBundle {
    //         shape: ColliderShape::cuboid(16.0, 16.0).into(),
    //         ..Default::default()
    //     })
    //     .insert(RigidBodyPositionSync::Discrete);
}
