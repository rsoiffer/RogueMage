use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use std::collections::HashMap;

use crate::chemistry::*;

pub const STONE: usize = 1;
pub const DIRT: usize = 2;
pub const GRASS: usize = 3;
pub const PLANKS: usize = 4;

#[derive(Default)]
pub struct BlockTextureAtlasResource {
    pub texture_atlas_handle: Handle<TextureAtlas>,
}

pub fn setup_block_atlas(
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut block_texture_atlas_resource: ResMut<BlockTextureAtlasResource>,
) {
    let texture_handle = asset_server.load("tileset.png");
    let texture_atlas = TextureAtlas::from_grid(texture_handle, Vec2::new(16.0, 16.0), 16, 16);
    let texture_atlas_handle = texture_atlases.add(texture_atlas);
    *block_texture_atlas_resource = BlockTextureAtlasResource {
        texture_atlas_handle,
    };
}

#[derive(Component, Clone, Copy)]
pub struct BlockInfo {
    pub x: i32,
    pub y: i32,
    pub id: usize,
}

pub fn spawn_block(
    commands: &mut Commands,
    block_texture_atlas_resource: &Res<BlockTextureAtlasResource>,
    block: BlockInfo,
    fire: f32,
) {
    commands
        .spawn_bundle(SpriteSheetBundle {
            texture_atlas: block_texture_atlas_resource.texture_atlas_handle.clone(),
            transform: Transform::from_xyz(block.x as f32 * 32.0, block.y as f32 * 32.0, 1.0)
                .with_scale(Vec3::splat(2.0)),
            sprite: TextureAtlasSprite {
                index: block.id,
                ..Default::default()
            },
            ..Default::default()
        })
        .insert_bundle(RigidBodyBundle {
            body_type: RigidBodyType::Static.into(),
            position: Vec2::new(block.x as f32 * 32.0, block.y as f32 * 32.0).into(),
            ..Default::default()
        })
        .insert_bundle(ColliderBundle {
            shape: ColliderShape::cuboid(16.0, 16.0).into(),
            ..Default::default()
        })
        .insert(new_chemistry(block, fire));
}

fn new_chemistry(block: BlockInfo, fire: f32) -> Chemistry {
    Chemistry {
        significance: 1.0,
        properties: match block.id {
            STONE => HashMap::from([(Property::Stone, 1.0), (Property::Burning, fire)]),
            DIRT => HashMap::from([(Property::Dirt, 1.0), (Property::Burning, fire)]),
            GRASS => HashMap::from([
                (Property::Dirt, 1.0),
                (Property::Grassy, 1.0),
                (Property::Burning, fire),
            ]),
            PLANKS => HashMap::from([(Property::Wooden, 1.0), (Property::Burning, fire)]),
            _ => HashMap::from([(Property::Metal, 1.0), (Property::Burning, fire)]),
        },
    }
}

pub(crate) fn update_block_sprites(mut query: Query<(&mut TextureAtlasSprite, &Chemistry)>) {
    for (mut sprite, chemistry) in query.iter_mut() {
        if chemistry.get(Property::Grassy) > 0.5 {
            sprite.index = GRASS;
        } else if chemistry.get(Property::Dirt) > 0.5 {
            sprite.index = DIRT;
        }

        sprite.color = Color::rgba(
            1.0,
            1.0 - 5.0 * chemistry.get(Property::Burning),
            1.0 - 5.0 * chemistry.get(Property::Burning),
            1.0,
        );
    }
}
