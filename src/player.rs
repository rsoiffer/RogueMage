use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

#[derive(Component)]
pub(crate) struct Player;

const ACCELERATION: Real = 1000.0;
const DRAG: Real = 10.0;

pub(crate) fn move_player_system(
    input: Res<Input<KeyCode>>,
    mut query: Query<(
        &Player,
        &RigidBodyMassPropsComponent,
        &RigidBodyVelocityComponent,
        &mut RigidBodyForcesComponent,
    )>,
) {
    for (_, mass, velocity, mut forces) in query.iter_mut() {
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
        .id()
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
