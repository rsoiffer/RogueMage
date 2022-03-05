use std::f32::consts::PI;

use bevy::{
    math::Vec2,
    prelude::{
        info_span, AssetServer, Camera, Color, Commands, Component, GlobalTransform, Query, Res,
        Transform, With,
    },
    sprite::{Sprite, SpriteBundle},
    tasks::{ComputeTaskPool, TaskPoolBuilder},
    utils::HashMap,
    window::Windows,
};
use noise::NoiseFn;

const N: usize = 10000;
const DT: f32 = 1.0 / 60.0;
const H: f32 = 2.0;
const RHO0: f32 = 1.0;
const ITERS: usize = 2;
const STEP_SIZE: f32 = 0.5;
const COLLISION_PADDING: f32 = 1.0;
const MAX_VELOCITY: f32 = 200.0;

const ASYNC_CHUNK_SIZE: usize = 50;

const SIM_SPACE: f32 = 200.0;

fn grid_key(pos: Vec2) -> (i32, i32) {
    ((pos.x / H).floor() as i32, (pos.y / H).floor() as i32)
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

#[derive(Clone)]
enum Constraint {
    Collision(usize),
    StayInWorld(),
}

// Solid: constrained to not overlap at all
// Liquid: constrained to satisfy density = 1.0

impl Constraint {
    fn suggest_dp(&self, i: usize, particles: &ParticleList) -> Vec2 {
        match self {
            Constraint::Collision(j) => {
                let diff = particles.positions[i] - particles.positions[*j];
                let density = 0.5 * particles.densities[i] + 0.5 * particles.densities[*j];
                let to_goal_density = f32::min(0.0, 1.0 - density / RHO0);
                // let to_goal_density = 1.0 - density / RHO0;
                dWspiky(diff) * to_goal_density
                // if diff.length_squared() < H * H {
                //     0.5 * (H / diff.length() - 1.0) * diff
                // } else {
                //     Vec2::ZERO
                // }
            }
            Constraint::StayInWorld() => {
                let pos = particles.positions[i];
                let nearest_in_world = pos.clamp(Vec2::splat(0.0), Vec2::splat(SIM_SPACE));
                nearest_in_world - pos
            }
        }
    }
}

#[derive(Component)]
pub(crate) struct ParticleSprite;

#[derive(Component)]
pub(crate) struct ParticleList {
    solid: Vec<bool>,
    positions: Vec<Vec2>,
    prev_positions: Vec<Vec2>,
    densities: Vec<f32>,
    task_pool: ComputeTaskPool,
    hashmap: HashMap<(i32, i32), Vec<usize>>,
}

impl ParticleList {
    fn new() -> ParticleList {
        let task_pool_builder = TaskPoolBuilder::new();
        let task_pool = ComputeTaskPool(task_pool_builder.build());
        ParticleList {
            solid: Default::default(),
            positions: Default::default(),
            prev_positions: Default::default(),
            densities: Default::default(),
            task_pool,
            hashmap: HashMap::<(i32, i32), Vec<usize>>::default(),
        }
    }

    fn add(&mut self, pos: Vec2) {
        self.solid.push(false);
        self.positions.push(pos);
        self.prev_positions.push(pos);
        self.densities.push(0.0);
    }

    fn apply_force(&mut self, i: usize, force: Vec2) {
        self.prev_positions[i] = self.positions[i] - DT * (self.vel(i) + force * DT)
    }

    fn nearby<F>(&self, pos: Vec2, dist: f32, f: &mut F)
    where
        F: FnMut(usize, Vec2),
    {
        let key_ll = grid_key(pos - Vec2::splat(dist));
        let key_ur = grid_key(pos + Vec2::splat(dist));
        for x in key_ll.0..key_ur.0 + 1 {
            for y in key_ll.1..key_ur.1 + 1 {
                match self.hashmap.get(&(x, y)) {
                    Some(js) => {
                        for &j in js {
                            let j_pos = self.positions[j];
                            if (pos - j_pos).length_squared() < dist * dist {
                                f(j, j_pos);
                            }
                        }
                    }
                    None => {}
                }
            }
        }
    }

    fn density(&self, pos: Vec2) -> f32 {
        let mut density = 0.0;
        self.nearby(pos, H, &mut |j, j_pos| {
            density += Wspiky(j_pos - pos);
        });
        density
    }

    fn step(&mut self, i: usize, step_size: f32) {
        let step = step_size * (self.positions[i] - self.prev_positions[i]);
        self.positions[i] += step;
        self.prev_positions[i] += step;
    }

    fn vel(&self, i: usize) -> Vec2 {
        1.0 / DT * (self.positions[i] - self.prev_positions[i])
    }
}

pub(crate) fn particle_start(mut commands: Commands, asset_server: Res<AssetServer>) {
    let mut particles = ParticleList::new();
    for i in 0..5000 {
        particles.add(Vec2::new(
            SIM_SPACE * rand::random::<f32>(),
            SIM_SPACE * (0.5 + 0.5 * rand::random::<f32>()),
        ));
    }
    let simplex = noise::SuperSimplex::new();
    for i in 5000..N {
        loop {
            let pos = Vec2::new(
                SIM_SPACE * rand::random::<f32>(),
                SIM_SPACE * (0.5 * rand::random::<f32>()),
            );
            let pos2 = pos / 10.0;
            let noise = simplex.get([pos2.x as f64, pos2.y as f64]);
            if noise > 0.3 {
                particles.add(pos);
                particles.solid[i] = true;
                break;
            }
        }
    }
    commands.spawn().insert(particles);

    for i in 0..N {
        commands
            .spawn_bundle(SpriteBundle {
                sprite: Sprite {
                    color: Color::rgb(0.2, 0.4, 1.0),
                    custom_size: Some(Vec2::splat(1.0)),
                    ..Default::default()
                },
                // texture: asset_server.load("sprites/white_pixel.png"),
                ..Default::default()
            })
            .insert(ParticleSprite);
    }
}

fn get_cursor(
    // need to get window dimensions
    wnds: Res<Windows>,
    // query to get camera transform
    q_camera: Query<(&Camera, &GlobalTransform)>,
) -> Vec2 {
    // get the camera info and transform
    // assuming there is exactly one main camera entity, so query::single() is OK
    let (camera, camera_transform) = q_camera.single();

    // get the window that the camera is displaying to
    let wnd = wnds.get(camera.window).unwrap();

    // check if the cursor is inside the window and get its position
    if let Some(screen_pos) = wnd.cursor_position() {
        // get the size of the window
        let window_size = Vec2::new(wnd.width() as f32, wnd.height() as f32);

        // convert screen position [0..resolution] to ndc [-1..1] (gpu coordinates)
        let ndc = (screen_pos / window_size) * 2.0 - Vec2::ONE;

        // matrix for undoing the projection and camera transform
        let ndc_to_world = camera_transform.compute_matrix() * camera.projection_matrix.inverse();

        // use it to convert ndc to world-space coordinates
        let world_pos = ndc_to_world.project_point3(ndc.extend(-1.0));

        // reduce it to a 2D value
        world_pos.truncate()
    } else {
        Vec2::ZERO
    }
}

pub(crate) fn particle_update(
    mut query: Query<&mut ParticleList>,
    mut query2: Query<(&mut Transform, &mut Sprite), With<ParticleSprite>>,
    windows: Res<Windows>,
    q_camera: Query<(&Camera, &GlobalTransform)>,
    // mut query: QuerySet<(
    //     QueryState<&Transform, With<Player>>,
    //     QueryState<&mut Transform, With<Camera>>,
    // )>,
) {
    let mut particles = query.single_mut();
    // let mut player = query3.single_mut();

    let mut nearby_particles = vec![];
    particles.nearby(get_cursor(windows, q_camera), 8.0, &mut |j, j_pos| {
        nearby_particles.push(j);
    });
    for i in nearby_particles {
        if !particles.solid[i] {
            particles.apply_force(i, Vec2::new(100.0, 0.0));
        }
    }

    // Apply gravity
    let span = info_span!("Apply Gravity").entered();
    for i in 0..N {
        if !particles.solid[i] {
            particles.apply_force(i, Vec2::new(0.0, -40.0));
            let vel = particles.vel(i);
            if vel.length_squared() > MAX_VELOCITY * MAX_VELOCITY {
                particles.prev_positions[i] =
                    particles.positions[i] - vel * DT * MAX_VELOCITY / vel.length();
            }
        }
    }
    span.exit();

    // Build grid hashmap
    let span = info_span!("Build grid hashmap").entered();
    particles.hashmap.clear();
    for i in 0..N {
        let pos = particles.positions[i];
        particles.hashmap.entry(grid_key(pos)).or_default().push(i);
    }
    span.exit();

    // Find constraints
    let span = info_span!("Find constraints").entered();
    let mut constraints = vec![vec![Constraint::StayInWorld()]; N];
    compute_parallel(&particles.task_pool, &mut constraints, &|i, c| {
        if !particles.solid[i] {
            let i_pos = particles.positions[i];
            particles.nearby(i_pos, COLLISION_PADDING * H, &mut |j, j_pos| {
                if i != j {
                    c.push(Constraint::Collision(j));
                }
            });
        }
    });
    span.exit();

    // Solve constraints
    let span = info_span!("Solve constraints").entered();
    for iter in 0..ITERS {
        // Step by velocity
        let span2 = info_span!("Step by velocity").entered();
        for i in 0..N {
            particles.step(i, 1.0 / ITERS as f32);
        }
        span2.exit();

        // Calculate densities
        let span2 = info_span!("Calc densities").entered();
        let mut densities = vec![0.0; N];
        compute_parallel(&particles.task_pool, &mut densities, &|i, density| {
            *density = particles.density(particles.positions[i]);
        });
        particles.densities = densities;
        span2.exit();

        // Calculate delta pos
        let span2 = info_span!("Calc delta pos").entered();
        let mut dp = vec![Vec2::ZERO; N];
        compute_parallel(&particles.task_pool, &mut dp, &|i, dp| {
            for c in constraints[i].iter() {
                *dp += c.suggest_dp(i, &particles);
            }
        });
        span2.exit();

        // Update position
        let span2 = info_span!("Update pos").entered();
        for i in 0..N {
            let step = STEP_SIZE * dp[i];
            particles.positions[i] += step;
        }
        span2.exit();
    }
    span.exit();

    // Update entities
    let span = info_span!("Update entities").entered();
    for (i, (mut t, mut sprite)) in query2.iter_mut().enumerate() {
        t.translation.x = particles.positions[i].x;
        t.translation.y = particles.positions[i].y;

        sprite.color = if particles.solid[i] {
            Color::rgb(0.5, 0.5, 0.5)
        } else {
            Color::rgb(0.2, 0.4, 1.0)
        }
        // if i % 100 == 0 {
        //     println!(
        //         "{} -> {}",
        //         particles.prev_positions[i], particles.positions[i]
        //     );
        // }
    }
    span.exit();
}
