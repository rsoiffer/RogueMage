use bevy::{
    math::Vec2,
    prelude::{info_span, AssetServer, Commands, Component, Query, Res, Transform, With},
    sprite::{Sprite, SpriteBundle},
    utils::HashMap,
};
use std::f32::consts::PI;

const N: usize = 5000;
const DT: f32 = 1.0 / 60.0;
const RHO0: f32 = 0.15;
const MASS: f32 = 1.0;
const H: f32 = 4.0;
const EPS: f32 = 1e-1;
const DIFFUSION: f32 = 0.5;
const ITERS: usize = 2;

const SIM_SPACE: f32 = 200.0;

fn Wpoly6(x: Vec2) -> f32 {
    let r2 = x.length_squared();
    if r2 < H * H {
        4.0 / (PI * f32::powi(H, 8)) * f32::powi(H * H - r2, 3)
    } else {
        0.0
    }
}

fn Wspiky(x: Vec2) -> f32 {
    let r = x.length();
    if r < H {
        10.0 / (PI * f32::powi(H, 5)) * f32::powi(H - r, 3)
    } else {
        0.0
    }
}

fn dWspiky(x: Vec2) -> Vec2 {
    let r = x.length();
    if r < H && r > 1e-6 {
        -30.0 / (PI * f32::powi(H, 5)) * f32::powi(H - r, 2) * x / r
    } else {
        Vec2::ZERO
    }
}

#[derive(Default)]
struct Particle {
    pos: Vec2,
    vel: Vec2,
    pred_pos: Vec2,
    delta_pos: Vec2,
    lambda: f32,
    neighbors: Vec<usize>,
}

#[derive(Component)]
pub(crate) struct ParticleSprite;

#[derive(Component)]
pub(crate) struct ParticleList {
    particles: Vec<Particle>,
}

impl ParticleList {
    fn Ci(&self, i: usize) -> f32 {
        self.density(i) / RHO0 - 1.0
    }

    fn dCi2(&self, i: usize, k: usize) -> f32 {
        if i == k {
            let mut v = Vec2::ZERO;
            for &j in self.particles[i].neighbors.iter() {
                v += dWspiky(self.particles[i].pred_pos - self.particles[j].pred_pos);
            }
            (v / RHO0).length_squared()
        } else {
            (dWspiky(self.particles[i].pred_pos - self.particles[k].pred_pos) / RHO0)
                .length_squared()
        }
    }

    fn density(&self, i: usize) -> f32 {
        let p1 = &self.particles[i];
        p1.neighbors
            .iter()
            .map(|&j| &self.particles[j])
            .map(|p2| MASS * Wpoly6(p2.pred_pos - p1.pred_pos))
            .sum()
    }
}

pub(crate) fn particle_start(mut commands: Commands, asset_server: Res<AssetServer>) {
    let mut particles = vec![];
    for i in 0..N {
        let mut p = Particle::default();
        p.pos = Vec2::new(
            SIM_SPACE * rand::random::<f32>(),
            SIM_SPACE * rand::random::<f32>(),
        );
        particles.push(p);
    }
    commands.spawn().insert(ParticleList { particles });

    for i in 0..N {
        commands
            .spawn_bundle(SpriteBundle {
                sprite: Sprite {
                    custom_size: Some(Vec2::splat(1.0)),
                    ..Default::default()
                },
                texture: asset_server.load("sprites/cat_alive.png"),
                ..Default::default()
            })
            .insert(ParticleSprite);
    }
}

pub(crate) fn particle_update(
    mut query: Query<&mut ParticleList>,
    mut query2: Query<&mut Transform, With<ParticleSprite>>,
) {
    let mut particles = query.single_mut();
    let span = info_span!("Stage 1").entered();
    for p in particles.particles.iter_mut() {
        p.vel += Vec2::new(0.0, -40.0 * DT);
        p.pred_pos = p.pos + DT * p.vel;
    }
    span.exit();

    // Find neighbors
    let span = info_span!("Find neighbors").entered();
    let mut hashmap = HashMap::<(i32, i32), Vec<usize>>::default();
    for i in 0..N {
        let p = &particles.particles[i];
        let key = (
            (p.pred_pos.x / H).floor() as i32,
            (p.pred_pos.y / H).floor() as i32,
        );
        let e = hashmap.entry(key);
        e.or_default().push(i);
    }
    for i in 0..N {
        unsafe {
            let p1 = &mut particles.particles[i] as *mut Particle;
            let p = &mut *p1;
            let key = (
                (p.pred_pos.x / H).floor() as i32,
                (p.pred_pos.y / H).floor() as i32,
            );
            p.neighbors.clear();
            for x in key.0 - 1..key.0 + 2 {
                for y in key.1 - 1..key.1 + 2 {
                    match hashmap.get(&(x, y)) {
                        Some(l) => p.neighbors.extend(l),
                        None => {}
                    }
                }
            }
        }
    }
    span.exit();

    let span = info_span!("Solve pressure").entered();
    // Solve pressure
    for iter in 0..ITERS {
        let span = info_span!("Iteration").entered();
        let span2 = info_span!("Calc lambdas").entered();
        for i in 0..N {
            let denom = particles.particles[i]
                .neighbors
                .iter()
                .map(|&j| particles.dCi2(i, j))
                .sum::<f32>();
            particles.particles[i].lambda = -particles.Ci(i) / (denom + EPS);
        }
        span2.exit();

        let span2 = info_span!("Calc delta pos").entered();
        for i in 0..N {
            let mut delta_pos = Vec2::ZERO;
            for &j in particles.particles[i].neighbors.iter() {
                let dpos = particles.particles[i].pred_pos - particles.particles[j].pred_pos;
                let scorr = -0.1 * f32::powi(Wpoly6(dpos) / Wpoly6(Vec2::new(0.0, 0.2 * H)), 4);
                delta_pos +=
                    (particles.particles[i].lambda + particles.particles[j].lambda + scorr)
                        * dWspiky(dpos)
                        / RHO0;
            }
            particles.particles[i].delta_pos = delta_pos;
        }
        span2.exit();

        let span2 = info_span!("Update pred pos").entered();
        for p in particles.particles.iter_mut() {
            p.pred_pos += p.delta_pos;
            p.pred_pos =
                0.1 * p.pred_pos + 0.9 * p.pred_pos.clamp(Vec2::splat(0.0), Vec2::splat(SIM_SPACE))
        }
        span2.exit();
        span.exit();
    }
    span.exit();

    let span = info_span!("Update velocity").entered();
    for i in 0..N {
        let mut vel = 1.0 / DT * (particles.particles[i].pred_pos - particles.particles[i].pos);

        let density_i = particles.density(i);
        // Vorticity and viscosity updates
        for &j in particles.particles[i].neighbors.iter() {
            let dv = 1.0 / DT * (particles.particles[j].pred_pos - particles.particles[j].pos)
                - 1.0 / DT * (particles.particles[i].pred_pos - particles.particles[i].pos);
            vel += DIFFUSION
                * dv
                * Wpoly6(particles.particles[j].pred_pos - particles.particles[i].pred_pos)
                / density_i;
        }

        particles.particles[i].vel = vel;
    }
    span.exit();

    let span = info_span!("End").entered();
    for p in particles.particles.iter_mut() {
        p.pos = p.pred_pos;
    }

    for (i, mut t) in query2.iter_mut().enumerate() {
        t.translation.x = particles.particles[i].pos.x;
        t.translation.y = particles.particles[i].pos.y;
        if i % 100 == 0 {
            // println!("{}", particles.density(i));
            // println!("{}", particles.particles[i].neighbors.len());
        }
    }
    span.exit();
}
