use crate::particle_model::*;
use bevy::{
    ecs::system::{lifetimeless::SRes, SystemParamItem},
    math::{Vec2, Vec4},
    pbr::{Material, MaterialMeshBundle, MaterialPipeline},
    prelude::{
        info_span, shape, AssetServer, Assets, Camera, Color, Commands, Component, GlobalTransform,
        Handle, Mesh, Query, Res, ResMut, Shader, Transform, With,
    },
    reflect::TypeUuid,
    render::{
        render_asset::{PrepareAssetError, RenderAsset},
        render_resource::{
            std140::{AsStd140, Std140},
            BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
            BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, Buffer,
            BufferBindingType, BufferInitDescriptor, BufferSize, BufferUsages, ShaderStages,
        },
        renderer::RenderDevice,
    },
    sprite::{Material2d, Material2dPipeline, MaterialMesh2dBundle, Mesh2dHandle},
    tasks::ComputeTaskPool,
    window::Windows,
};
use noise::NoiseFn;

const SIM_SPACE: f32 = 200.0;

#[derive(Component)]
pub(crate) struct ParticleSprite;

pub(crate) fn particle_start(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<CustomMaterial>>,
) {
    let mut particles = ParticleList::default();
    for i in 0..5000 {
        particles.add(
            Vec2::new(
                SIM_SPACE * rand::random::<f32>(),
                SIM_SPACE * (0.5 + 0.5 * rand::random::<f32>()),
            ),
            false,
        );
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
                particles.add(pos, true);
                break;
            }
        }
    }
    commands.spawn().insert(particles);

    let quad = Mesh2dHandle(meshes.add(Mesh::from(shape::Quad::new(Vec2::splat(1.0)))));
    for i in 0..N {
        commands
            .spawn()
            .insert_bundle(MaterialMesh2dBundle {
                mesh: quad.clone(),
                material: materials.add(CustomMaterial { color: Color::RED }),
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
    mut query2: Query<(&mut Transform, &Handle<CustomMaterial>), With<ParticleSprite>>,
    mut materials: ResMut<Assets<CustomMaterial>>,
    windows: Res<Windows>,
    q_camera: Query<(&Camera, &GlobalTransform)>,
    task_pool: Res<ComputeTaskPool>,
) {
    let mut particles = query.single_mut();

    // Apply forces
    let span = info_span!("Apply forces").entered();
    let cursor = get_cursor(windows, q_camera);
    particles.apply_forces(&|i, i_pos| {
        let f = Vec2::new(0.0, -40.0);
        if (i_pos - cursor).length_squared() < 8.0 * 8.0 {
            f + Vec2::new(100.0, 0.0)
        } else {
            f
        }
    });
    span.exit();

    // Simulation step
    let span = info_span!("Simulation step").entered();
    particles.simulation_step((Vec2::splat(0.0), Vec2::splat(SIM_SPACE)), &task_pool);
    span.exit();

    // Update entities
    let span = info_span!("Update entities").entered();

    for ((mut t, material_handle), data) in query2.iter_mut().zip(particles.iter_all()) {
        t.translation.x = data.pos.x;
        t.translation.y = data.pos.y;

        let new_color = if data.solid {
            Color::rgb(0.5, 0.5, 0.5)
        } else {
            Color::rgb(0.2, 0.4, 1.0)
        };

        if materials.get(material_handle).unwrap().color == new_color {
            continue;
        }
        let material = materials.get_mut(material_handle).unwrap();
        material.color = new_color;

        // if i % 100 == 0 {
        //     println!(
        //         "{} -> {}",
        //         particles.prev_positions[i], particles.positions[i]
        //     );
        // }
    }
    span.exit();
}

// This is the struct that will be passed to your shader
#[derive(Debug, Clone, TypeUuid)]
#[uuid = "f690fdae-d598-45ab-8225-97e2a3f056e0"]
pub struct CustomMaterial {
    color: Color,
}

#[derive(Clone)]
pub struct GpuCustomMaterial {
    _buffer: Buffer,
    bind_group: BindGroup,
}

// The implementation of [`Material`] needs this impl to work properly.
impl RenderAsset for CustomMaterial {
    type ExtractedAsset = CustomMaterial;
    type PreparedAsset = GpuCustomMaterial;
    type Param = (SRes<RenderDevice>, SRes<Material2dPipeline<Self>>);
    fn extract_asset(&self) -> Self::ExtractedAsset {
        self.clone()
    }

    fn prepare_asset(
        extracted_asset: Self::ExtractedAsset,
        (render_device, material_pipeline): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self::ExtractedAsset>> {
        let color = Vec4::from_slice(&extracted_asset.color.as_linear_rgba_f32());
        let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            contents: color.as_std140().as_bytes(),
            label: None,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });
        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: None,
            layout: &material_pipeline.material2d_layout,
        });

        Ok(GpuCustomMaterial {
            _buffer: buffer,
            bind_group,
        })
    }
}

impl Material2d for CustomMaterial {
    fn fragment_shader(asset_server: &AssetServer) -> Option<Handle<Shader>> {
        Some(asset_server.load("particle_material.wgsl"))
    }

    fn bind_group(render_asset: &<Self as RenderAsset>::PreparedAsset) -> &BindGroup {
        &render_asset.bind_group
    }

    fn bind_group_layout(render_device: &RenderDevice) -> BindGroupLayout {
        render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: BufferSize::new(Vec4::std140_size_static() as u64),
                },
                count: None,
            }],
            label: None,
        })
    }
}
