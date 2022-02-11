use bevy::prelude::*;

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
) {
    commands.spawn().insert(block);
    commands.spawn_bundle(SpriteSheetBundle {
        texture_atlas: block_texture_atlas_resource.texture_atlas_handle.clone(),
        transform: Transform::from_xyz(block.x as f32 * 32.0, block.y as f32 * 32.0, 1.0)
            .with_scale(Vec3::splat(2.0)),
        sprite: TextureAtlasSprite {
            index: block.id,
            ..Default::default()
        },
        ..Default::default()
    });
}
