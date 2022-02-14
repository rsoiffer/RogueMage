use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use rand::Rng;
use rand_distr::Distribution;
use std::collections::HashMap;

use crate::chemistry::*;

pub const STONE: usize = 1;
pub const DIRT: usize = 2;
pub const GRASS: usize = 3;
pub const PLANKS: usize = 4;
pub const CLAY: usize = 104;
pub const BURNT_WOOD: usize = 151;
pub const LAVA: usize = 443;

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
    let texture_atlas = TextureAtlas::from_grid(texture_handle, Vec2::new(16.0, 16.0), 24, 44);
    let texture_atlas_handle = texture_atlases.add(texture_atlas);
    *block_texture_atlas_resource = BlockTextureAtlasResource {
        texture_atlas_handle,
    };
}

#[derive(Component, Clone, Copy)]
pub(crate) struct BlockInfo {
    pub x: i32,
    pub y: i32,
    pub id: usize,
}

pub(crate) struct BlockSpawner<'a, 'w1, 's, 'w2> {
    pub(crate) commands: &'a mut Commands<'w1, 's>,
    pub(crate) block_texture_atlas_resource: &'a Res<'w2, BlockTextureAtlasResource>,
}

impl<'a, 'w1, 's, 'w2> BlockSpawner<'a, 'w1, 's, 'w2> {
    pub(crate) fn spawn(&mut self, x: i32, y: i32, id: usize) {
        self.spawn_block_info_fire(BlockInfo { x, y, id }, 0.0);
    }

    pub(crate) fn spawn_fire(&mut self, x: i32, y: i32, id: usize, fire: f32) {
        self.spawn_block_info_fire(BlockInfo { x, y, id }, fire);
    }

    pub(crate) fn spawn_block_info(&mut self, block: BlockInfo) {
        self.spawn_block_info_fire(block, 0.0);
    }

    pub(crate) fn spawn_block_info_fire(&mut self, block: BlockInfo, fire: f32) {
        self.commands
            .spawn_bundle(SpriteSheetBundle {
                texture_atlas: self
                    .block_texture_atlas_resource
                    .texture_atlas_handle
                    .clone(),
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
            .insert(new_chemistry(block, fire))
            .insert(ChemistryGraphics::new());
    }
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
        } else if chemistry.get(Property::Clay) > 0.5 {
            sprite.index = CLAY;
        } else if chemistry.get(Property::Wooden) > 0.5 {
            sprite.index = PLANKS;
        } else if chemistry.get(Property::BurntWooden) > 0.5 {
            sprite.index = BURNT_WOOD;
        }
    }
}

#[derive(Component)]
pub(crate) struct ChemistryGraphics {
    to_next_fire_particle: f32,
}

impl ChemistryGraphics {
    fn new() -> ChemistryGraphics {
        return ChemistryGraphics {
            to_next_fire_particle: 1.0,
        };
    }
}

pub(crate) fn update_chemistry_graphics(
    mut commands: Commands,
    block_texture_atlas_resource: Res<BlockTextureAtlasResource>,
    time: Res<Time>,
    mut query: Query<(&Transform, &mut ChemistryGraphics, &Chemistry)>,
) {
    for (t, mut chem_graph, chem) in query.iter_mut() {
        chem_graph.to_next_fire_particle -=
            100.0 * time.delta_seconds() * chem.get(Property::Burning);
        if chem_graph.to_next_fire_particle < 0.0 {
            chem_graph.to_next_fire_particle += 1.0 * rand::thread_rng().gen::<f32>();
            commands
                .spawn_bundle(SpriteSheetBundle {
                    texture_atlas: block_texture_atlas_resource.texture_atlas_handle.clone(),
                    transform: Transform::from_translation(
                        t.translation
                            + Vec3::new(
                                rand::thread_rng().gen_range(-8.0..8.0),
                                rand::thread_rng().gen_range(-8.0..8.0),
                                0.0,
                            ),
                    )
                    .with_scale(Vec3::splat(0.5)),
                    sprite: TextureAtlasSprite {
                        index: LAVA,
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .insert(FireParticle { lifetime: 0.3 });
        }
    }
}

#[derive(Component)]
pub(crate) struct FireParticle {
    lifetime: f32,
}

pub(crate) fn update_fire_particles(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut FireParticle, &mut Transform)>,
) {
    for (entity, mut fire, mut t) in query.iter_mut() {
        fire.lifetime -= time.delta_seconds();
        t.translation += time.delta_seconds()
            * 50.0
            * Vec3::new(
                rand_distr::Normal::new(0.0, 1.0)
                    .unwrap()
                    .sample(&mut rand::thread_rng()),
                1.0,
                0.0,
            );
        if fire.lifetime < 0.0 {
            commands.entity(entity).despawn();
        }
    }
}
