use std::f32::consts::PI;

use bevy::{
    math::Vec2,
    prelude::{info_span, Component},
    tasks::ComputeTaskPool,
    utils::HashMap,
};

pub(crate) const N: usize = 10000;
const DT: f32 = 1.0 / 60.0;
const H: f32 = 2.0;
const RHO0: f32 = 1.0;
const ITERS: usize = 2;
const STEP_SIZE: f32 = 0.5;
const COLLISION_PADDING: f32 = 1.0;
const MAX_VELOCITY: f32 = 200.0;

const ASYNC_CHUNK_SIZE: usize = 50;

trait Kernel {
    fn value(pos: Vec2) -> f32;
    fn gradient(pos: Vec2) -> Vec2;
}

struct WSpiky;
impl Kernel for WSpiky {
    fn value(pos: Vec2) -> f32 {
        let r = pos.length();
        if r < H {
            10.0 / (PI * f32::powi(H, 5)) * f32::powi(H - r, 3)
        } else {
            0.0
        }
    }

    fn gradient(pos: Vec2) -> Vec2 {
        let r = pos.length();
        if r < H && r > 1e-6 {
            (-30.0 / (PI * f32::powi(H, 5)) * f32::powi(H - r, 2) / r) * pos
        } else {
            Vec2::ZERO
        }
    }
}

fn grid_key(pos: Vec2) -> (i32, i32) {
    ((pos.x / H).floor() as i32, (pos.y / H).floor() as i32)
}

fn compute_parallel<F, T>(task_pool: &ComputeTaskPool, vec: &mut Vec<T>, f: &F)
where
    F: Fn(usize, &mut T) + Sync,
    T: Send,
{
    task_pool.scope(|s| {
        let chunks = vec.chunks_mut(ASYNC_CHUNK_SIZE);
        for (block, chunk) in chunks.enumerate() {
            s.spawn(async move {
                // let span = info_span!("Compute thread").entered();
                for i in (block * ASYNC_CHUNK_SIZE)..((block + 1) * ASYNC_CHUNK_SIZE) {
                    f(i, &mut chunk[i % ASYNC_CHUNK_SIZE]);
                }
                // span.exit();
            });
        }
    });
}

// Computation utility functions

fn compute_density(positions: &Vec<Vec2>, neighbors: &Vec<Vec<usize>>, i: usize) -> f32 {
    let pos = positions[i];
    neighbors[i]
        .iter()
        .map(|&j| WSpiky::value(positions[j] - pos))
        .sum()
}

fn compute_nearby<F>(
    positions: &Vec<Vec2>,
    hashmap: &HashMap<(i32, i32), Vec<usize>>,
    pos: Vec2,
    dist: f32,
    f: &mut F,
) where
    F: FnMut(usize),
{
    let key_ll = grid_key(pos - Vec2::splat(dist));
    let key_ur = grid_key(pos + Vec2::splat(dist));
    for x in key_ll.0..key_ur.0 + 1 {
        for y in key_ll.1..key_ur.1 + 1 {
            match hashmap.get(&(x, y)) {
                Some(js) => {
                    for &j in js {
                        let j_pos = positions[j];
                        if (pos - j_pos).length_squared() < dist * dist {
                            f(j);
                        }
                    }
                }
                None => {}
            }
        }
    }
}

// Constraints

#[derive(Clone)]
pub(crate) enum Constraint {
    Collision(usize),
    StayInWorld(Vec2, Vec2),
}

impl Constraint {
    fn compute_dp(&self, positions: &Vec<Vec2>, densities: &Vec<f32>, i: usize) -> Vec2 {
        match self {
            Constraint::Collision(j) => {
                let diff = positions[i] - positions[*j];
                let density = 0.5 * densities[i] + 0.5 * densities[*j];
                let to_goal_density = f32::min(0.0, 1.0 - density / RHO0);
                // let to_goal_density = 1.0 - density / RHO0;
                WSpiky::gradient(diff) * to_goal_density
                // if diff.length_squared() < H * H {
                //     0.5 * (H / diff.length() - 1.0) * diff
                // } else {
                //     Vec2::ZERO
                // }
            }
            Constraint::StayInWorld(ll, ur) => {
                let pos = positions[i];
                let nearest_in_world = pos.clamp(*ll, *ur);
                nearest_in_world - pos
            }
        }
    }
}

#[derive(Component, Default)]
pub(crate) struct ParticleList {
    solid: Vec<bool>,
    positions: Vec<Vec2>,
    prev_positions: Vec<Vec2>,
    densities: Vec<f32>,
    dp: Vec<Vec2>,
    neighbors: Vec<Vec<usize>>,
    constraints: Vec<Vec<Constraint>>,
    hashmap: HashMap<(i32, i32), Vec<usize>>,
}

pub(crate) struct ParticleData {
    pub(crate) solid: bool,
    pub(crate) pos: Vec2,
    pub(crate) vel: Vec2,
}

impl ParticleList {
    pub(crate) fn add(&mut self, pos: Vec2, solid: bool) {
        self.solid.push(solid);
        self.positions.push(pos);
        self.prev_positions.push(pos);
        self.densities.push(0.0);
        self.dp.push(Vec2::ZERO);
        self.neighbors.push(vec![]);
        self.constraints.push(vec![]);
    }

    fn apply_force(&mut self, i: usize, force: Vec2) {
        self.prev_positions[i] = self.positions[i] - DT * (self.vel(i) + force * DT)
    }

    fn step(&mut self, i: usize, step_size: f32) {
        let step = step_size * (self.positions[i] - self.prev_positions[i]);
        self.positions[i] += step;
        self.prev_positions[i] += step;
    }

    fn vel(&self, i: usize) -> Vec2 {
        1.0 / DT * (self.positions[i] - self.prev_positions[i])
    }

    // Functions that iterate over all particles

    pub(crate) fn iter_all(&self) -> impl Iterator<Item = ParticleData> + '_ {
        (0..N).map(|i| ParticleData {
            solid: self.solid[i],
            pos: self.positions[i],
            vel: self.vel(i),
        })
    }

    pub(crate) fn apply_forces<F>(&mut self, f: &F)
    where
        F: Fn(usize, Vec2) -> Vec2 + Sync,
    {
        for i in 0..N {
            if !self.solid[i] {
                self.apply_force(i, f(i, self.positions[i]));
            }
        }
    }

    pub(crate) fn simulation_step(&mut self, bounds: (Vec2, Vec2), task_pool: &ComputeTaskPool) {
        // Clamp velocities
        let span = info_span!("Clamp velocities").entered();
        for i in 0..N {
            let vel = self.vel(i);
            if vel.length_squared() > MAX_VELOCITY * MAX_VELOCITY {
                let new_vel = (MAX_VELOCITY / vel.length()) * vel;
                self.prev_positions[i] = self.positions[i] - DT * new_vel;
            }
        }
        span.exit();

        // Rebuild grid hashmap
        let span = info_span!("Rebuild grid hashmap").entered();
        self.hashmap.clear();
        for i in 0..N {
            let pos = self.positions[i];
            self.hashmap.entry(grid_key(pos)).or_default().push(i);
        }
        span.exit();

        // Find neighbors
        let span = info_span!("Find neighbors").entered();
        compute_parallel(task_pool, &mut self.neighbors, &|i, c| {
            c.clear();
            compute_nearby(
                &self.positions,
                &self.hashmap,
                self.positions[i],
                COLLISION_PADDING * H,
                &mut |j| c.push(j),
            );
        });
        span.exit();

        // Find constraints
        let span = info_span!("Find constraints").entered();
        compute_parallel(task_pool, &mut self.constraints, &|i, c| {
            c.clear();
            c.push(Constraint::StayInWorld(bounds.0, bounds.1));
            if !self.solid[i] {
                for &j in self.neighbors[i].iter() {
                    if i != j {
                        c.push(Constraint::Collision(j));
                    }
                }
            }
        });
        span.exit();

        // Solve constraints
        let span = info_span!("Solve constraints").entered();
        for _iter in 0..ITERS {
            // Step by velocity
            let span2 = info_span!("Step by velocity").entered();
            for i in 0..N {
                self.step(i, 1.0 / ITERS as f32);
            }
            span2.exit();

            // Calculate densities
            let span2 = info_span!("Calc densities").entered();
            compute_parallel(task_pool, &mut self.densities, &|i, density| {
                *density = compute_density(&self.positions, &self.neighbors, i)
            });
            span2.exit();

            // Calculate delta pos
            let span2 = info_span!("Calc delta pos").entered();
            compute_parallel(task_pool, &mut self.dp, &|i, dp| {
                *dp = Vec2::ZERO;
                for c in self.constraints[i].iter() {
                    *dp += c.compute_dp(&self.positions, &self.densities, i);
                }
            });
            span2.exit();

            // Update position
            let span2 = info_span!("Update pos").entered();
            for i in 0..N {
                let step = STEP_SIZE * self.dp[i];
                self.positions[i] += step;
            }
            span2.exit();
        }
        span.exit();
    }
}
