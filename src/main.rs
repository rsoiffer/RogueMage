mod blocks;
mod cells;
mod chemistry;
mod player;
mod rules;
mod spells;

use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
};
use bevy_rapier2d::prelude::*;
use player::{cast_spell_system, move_camera_system, move_player_system, spawn_player};
use rules::*;

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
        .insert_resource(ClearColor(Color::rgb(0.2, 0.2, 0.2)))
        .insert_resource(RapierConfiguration {
            gravity: Vector::y() * -1000.0,
            ..Default::default()
        })
        .add_startup_system(setup)
        .add_startup_system(system_setup_block_grid)
        .add_system(system_update_block_grid)
        .add_system(move_player_system)
        .add_system(move_camera_system)
        .add_system(cast_spell_system)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    asset_server.watch_for_changes().unwrap();
    spawn_player(&mut commands, asset_server.load("sprites/cat_alive.png"));

    let mut camera = OrthographicCameraBundle::new_2d();
    camera.orthographic_projection.scale = 1.0 / 3.0;
    commands.spawn_bundle(camera);
}
