use crate::chemistry::{ChemEntity, DynamicProperty, ManaId, Target, WorldInfo};
use bevy::{
    math::{Vec3Swizzles, XY},
    prelude::*,
};
use bevy_rapier2d::prelude::*;

#[derive(Component)]
pub(crate) struct Player;

const ACCELERATION: Real = 1000.0;
const DRAG: Real = 10.0;
const CAMERA_RATE: Real = 4.0;

const SPELL_KEYS: &[(KeyCode, ManaId)] = &[(KeyCode::Key1, ManaId(0)), (KeyCode::Key2, ManaId(1))];

pub(crate) fn move_player_system(
    input: Res<Input<KeyCode>>,
    mut query: Query<
        (
            &RigidBodyMassPropsComponent,
            &RigidBodyVelocityComponent,
            &mut RigidBodyForcesComponent,
        ),
        With<Player>,
    >,
) {
    for (mass, velocity, mut forces) in query.iter_mut() {
        let thrust_unnormalized = vector![
            thrust_component(&input, KeyCode::D, KeyCode::A),
            thrust_component(&input, KeyCode::W, KeyCode::S)
        ];

        let thrust = if thrust_unnormalized.norm() < 1e-6 {
            Vector::zeros()
        } else {
            ACCELERATION * thrust_unnormalized.normalize()
        };

        let drag = -DRAG * velocity.linvel;
        forces.force = mass.mass() * (thrust + drag);
    }
}

pub(crate) fn spawn_player(commands: &mut Commands, texture: Handle<Image>) -> Entity {
    commands
        .spawn_bundle(SpriteBundle {
            texture,
            ..Default::default()
        })
        .insert_bundle(RigidBodyBundle {
            forces: RigidBodyForcesComponent(RigidBodyForces {
                gravity_scale: 0.0,
                ..Default::default()
            }),
            ..Default::default()
        })
        .insert_bundle(ColliderBundle::default())
        .insert(RigidBodyPositionSync::Discrete)
        .insert(Player)
        .insert(ChemEntity)
        .id()
}

pub(crate) fn move_camera_system(
    time: Res<Time>,
    mut query: QuerySet<(
        QueryState<&Transform, With<Player>>,
        QueryState<&mut Transform, With<Camera>>,
    )>,
) {
    let player_translation = query.q0().single().translation.xy();

    let mut camera_query = query.q1();
    let mut camera_transform = camera_query.single_mut();
    let XY { x, y } = *player_translation.lerp(
        camera_transform.translation.xy(),
        f32::exp(-CAMERA_RATE * time.delta_seconds()),
    );

    camera_transform.translation.x = x;
    camera_transform.translation.y = y;
}

pub(crate) fn cast_spell_system(
    input: Res<Input<KeyCode>>,
    mut world_query: Query<&mut WorldInfo>,
    player_query: Query<Entity, With<Player>>,
) {
    let mut world = world_query.single_mut();

    for player in player_query.iter() {
        for (key, mana_id) in SPELL_KEYS {
            if input.just_pressed(*key) {
                world.set(Target::Entity(player), DynamicProperty::Mana(*mana_id), 1.0);
            }
        }
    }
}

fn thrust_component(input: &Input<KeyCode>, positive: KeyCode, negative: KeyCode) -> Real {
    if input.pressed(positive) {
        1.0
    } else if input.pressed(negative) {
        -1.0
    } else {
        0.0
    }
}
