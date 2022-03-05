use bevy::{
    math::Vec2,
    prelude::{info_span, AssetServer, Color, Commands, Component, Query, Res, Transform, With},
    sprite::{Sprite, SpriteBundle},
    tasks::{ComputeTaskPool, TaskPoolBuilder},
    utils::HashMap,
};

const N: usize = 5000;
const DT: f32 = 1.0 / 60.0;
const H: f32 = 1.2;
const ITERS: usize = 10;
const STEP_SIZE: f32 = 0.25;
const BETA_MUL: f32 = 1.0;
const BETA_POW: f32 = 1.5;

const ASYNC_CHUNK_SIZE: usize = 50;

const SIM_SPACE: f32 = 100.0;

fn grid_key(pos: Vec2) -> (i32, i32) {
    ((pos.x / H).floor() as i32, (pos.y / H).floor() as i32)
}

fn create_vec_parallel<F, T>(task_pool: &ComputeTaskPool, f: &F) -> Vec<T>
where
    F: Fn(usize) -> T + Sync,
    T: Clone + Default + Send,
{
    let mut vec = vec![T::default(); N];
    task_pool.scope(|s| {
        let chunks = vec.chunks_mut(ASYNC_CHUNK_SIZE);
        for (block, chunk) in chunks.enumerate() {
            s.spawn(async move {
                // let span = info_span!("Compute thread").entered();
                for i in (block * ASYNC_CHUNK_SIZE)..((block + 1) * ASYNC_CHUNK_SIZE) {
                    chunk[i % ASYNC_CHUNK_SIZE] = f(i);
                }
                // span.exit();
            });
        }
    });
    vec
}

#[derive(Clone)]
enum Constraint {
    Collision(usize),
    StayInWorld(),
}

impl Constraint {
    fn suggest_dp(&self, i: usize, positions: &Vec<Vec2>) -> Vec2 {
        match self {
            Constraint::Collision(j) => {
                let diff = positions[i] - positions[*j];
                if diff.length_squared() < H * H {
                    0.5 * (H / diff.length() - 1.0) * diff
                } else {
                    Vec2::ZERO
                }
            }
            Constraint::StayInWorld() => {
                let pos = positions[i];
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
    positions: Vec<Vec2>,
    prev_positions: Vec<Vec2>,
    delta: Vec<Vec2>,
    task_pool: ComputeTaskPool,
}

impl ParticleList {
    fn new() -> ParticleList {
        let task_pool_builder = TaskPoolBuilder::new();
        let task_pool = ComputeTaskPool(task_pool_builder.build());
        ParticleList {
            positions: Default::default(),
            prev_positions: Default::default(),
            delta: Default::default(),
            task_pool,
        }
    }

    fn add(&mut self, pos: Vec2) {
        self.positions.push(pos);
        self.prev_positions.push(pos);
        self.delta.push(Vec2::ZERO);
    }

    fn apply_force(&mut self, i: usize, force: Vec2) {
        self.prev_positions[i] = self.positions[i] - DT * (self.vel(i) + force * DT)
    }

    fn step(&mut self, i: usize) {
        let step = self.positions[i] - self.prev_positions[i];
        self.positions[i] += step;
        self.prev_positions[i] += step;
    }

    fn vel(&self, i: usize) -> Vec2 {
        1.0 / DT * (self.positions[i] - self.prev_positions[i])
    }
}

pub(crate) fn particle_start(mut commands: Commands, asset_server: Res<AssetServer>) {
    let mut particles = ParticleList::new();
    for i in 0..N {
        particles.add(Vec2::new(
            SIM_SPACE * rand::random::<f32>(),
            SIM_SPACE * rand::random::<f32>(),
        ));
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

pub(crate) fn particle_update(
    mut query: Query<&mut ParticleList>,
    mut query2: Query<&mut Transform, With<ParticleSprite>>,
) {
    let mut particles = query.single_mut();

    // Step by velocity
    let span = info_span!("Step by velocity").entered();
    for i in 0..N {
        particles.apply_force(i, Vec2::new(0.0, -40.0));
        particles.step(i);
    }
    span.exit();

    // Build grid hashmap
    let span = info_span!("Build grid hashmap").entered();
    let mut hashmap = HashMap::<(i32, i32), Vec<usize>>::default();
    for i in 0..N {
        let pos = particles.positions[i];
        hashmap.entry(grid_key(pos)).or_default().push(i);
    }
    span.exit();

    // Find constraints
    let span = info_span!("Find constraints").entered();
    let constraints = create_vec_parallel(&particles.task_pool, &|i| {
        let mut c = vec![Constraint::StayInWorld()];
        let i_pos = particles.positions[i];
        let key = grid_key(i_pos);
        for x in key.0 - 1..key.0 + 2 {
            for y in key.1 - 1..key.1 + 2 {
                match hashmap.get(&(x, y)) {
                    Some(js) => {
                        for &j in js {
                            let j_pos = particles.positions[j];
                            if i != j && (i_pos - j_pos).length_squared() < 2.0 * H * H {
                                c.push(Constraint::Collision(j));
                            }
                        }
                    }
                    None => {}
                }
            }
        }
        c
    });
    span.exit();

    // Solve constraints
    let span = info_span!("Solve constraints").entered();
    for iter in 0..ITERS {
        let beta = BETA_MUL * f32::powf(iter as f32 / ITERS as f32, BETA_POW);

        // Calculate delta pos
        let span2 = info_span!("Calc delta pos").entered();
        let dp = create_vec_parallel(&particles.task_pool, &|i| {
            let mut new_dp = Vec2::ZERO;
            for c in constraints[i].iter() {
                new_dp += c.suggest_dp(i, &particles.positions);
            }
            new_dp
        });
        span2.exit();

        // Update position
        let span2 = info_span!("Update pos").entered();
        for i in 0..N {
            let step = STEP_SIZE * dp[i] + beta * particles.delta[i];
            particles.delta[i] = step;
            particles.positions[i] += step;
        }
        span2.exit();
    }
    span.exit();

    // Update entities
    let span = info_span!("Update entities").entered();
    for (i, mut t) in query2.iter_mut().enumerate() {
        t.translation.x = particles.positions[i].x;
        t.translation.y = particles.positions[i].y;
        // if i % 100 == 0 {
        //     println!(
        //         "{} -> {}",
        //         particles.prev_positions[i], particles.positions[i]
        //     );
        // }
    }
    span.exit();
}
