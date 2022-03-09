mod blocks;
mod cells;
mod chemistry;
mod math_utils;
mod parser;
mod particle_model;
mod particle_render;
mod player;
mod rules_asset;
mod spells;

use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    sprite::Material2dPlugin,
};
use bevy_rapier2d::prelude::*;
use cells::*;
use chemistry::*;
use particle_render::*;
use player::{move_camera_system, move_player_system, spawn_player};
use rules_asset::{RulesAsset, RulesAssetLoader};

fn main() {
    App::new()
        .insert_resource(WindowDescriptor {
            width: 960.0,
            height: 540.0,
            vsync: false,
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .add_plugin(LogDiagnosticsPlugin::default())
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
        .add_plugin(Material2dPlugin::<CustomMaterial>::default())
        .init_resource::<NaturalRules>()
        .insert_resource(ClearColor(Color::rgb(0.2, 0.2, 0.2)))
        .insert_resource(RapierConfiguration {
            gravity: Vector::y() * -1000.0,
            ..Default::default()
        })
        .add_asset::<RulesAsset>()
        .init_asset_loader::<RulesAssetLoader>()
        .add_startup_system(setup)
        // .add_startup_system(system_setup_block_grid)
        // .add_system(system_update_block_grid)
        .add_system(move_player_system)
        .add_system(move_camera_system)
        .add_startup_system(particle_start)
        .add_system(particle_update)
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut natural_rules: ResMut<NaturalRules>,
) {
    asset_server.watch_for_changes().unwrap();
    natural_rules.0 = asset_server.load("natural.rules");
    spawn_player(&mut commands, asset_server.load("sprites/cat_alive.png"));

    let mut camera = OrthographicCameraBundle::new_2d();
    camera.orthographic_projection.scale = 1.0 / 3.0;
    commands.spawn_bundle(camera);
}
