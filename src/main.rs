mod blocks;
mod cells;
mod chemistry;
mod math_utils;
mod parser;
mod rules_asset;
mod spells;

use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
};
use bevy_rapier2d::prelude::*;
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
        .add_plugin(LogDiagnosticsPlugin::default())
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
        .init_resource::<NaturalRules>()
        .insert_resource(ClearColor(Color::rgb(0.5, 0.5, 0.5)))
        .insert_resource(RapierConfiguration {
            gravity: Vector::y() * -1000.0,
            ..Default::default()
        })
        .add_asset::<RulesAsset>()
        .init_asset_loader::<RulesAssetLoader>()
        .add_startup_system(setup)
        .add_startup_system(system_setup_block_grid)
        .add_system(system_update_block_grid)
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut natural_rules: ResMut<NaturalRules>,
) {
    asset_server.watch_for_changes().unwrap();

    natural_rules.0 = asset_server.load("natural.rules");

    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
}
