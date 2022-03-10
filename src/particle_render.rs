use crate::particle_model::*;
use bevy::{
    core::{FloatOrd, Pod, Zeroable},
    core_pipeline::Transparent2d,
    ecs::system::{
        lifetimeless::{Read, SQuery, SRes},
        SystemParamItem,
    },
    math::{Vec2, Vec3},
    prelude::{
        info_span, shape, App, AssetServer, Assets, Camera, Color, Commands, Component,
        ComputedVisibility, Entity, FromWorld, GlobalTransform, Handle, Mesh, Msaa, Plugin, Query,
        Res, ResMut, Shader, Transform, Visibility, With, World,
    },
    render::{
        mesh::GpuBufferInfo,
        render_asset::RenderAssets,
        render_component::{ExtractComponent, ExtractComponentPlugin},
        render_phase::{
            AddRenderCommand, DrawFunctions, EntityRenderCommand, RenderCommandResult, RenderPhase,
            SetItemPipeline, TrackedRenderPass,
        },
        render_resource::{
            Buffer, BufferInitDescriptor, BufferUsages, PrimitiveTopology, RenderPipelineCache,
            RenderPipelineDescriptor, SpecializedPipeline, SpecializedPipelines, VertexAttribute,
            VertexBufferLayout, VertexFormat, VertexStepMode,
        },
        renderer::RenderDevice,
        view::ExtractedView,
        RenderApp, RenderStage,
    },
    sprite::{
        Mesh2dHandle, Mesh2dPipeline, Mesh2dPipelineKey, Mesh2dUniform, SetMesh2dBindGroup,
        SetMesh2dViewBindGroup,
    },
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
    // mut materials: ResMut<Assets<CustomMaterial>>,
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

    commands.spawn().insert_bundle((
        Mesh2dHandle(meshes.add(Mesh::from(shape::Quad::new(Vec2::splat(1.0))))),
        Transform::from_xyz(0.0, 0.0, 0.0),
        GlobalTransform::default(),
        InstanceMaterialData(
            (0..N)
                .map(|i| InstanceData {
                    position: Vec3::splat(0.0),
                    scale: 1.0,
                    color: Color::WHITE.as_rgba_f32(),
                })
                .collect(),
        ),
        Visibility::default(),
        ComputedVisibility::default(),
    ));
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
    mut query2: Query<&mut InstanceMaterialData>,
    // mut query2: Query<(&mut Transform, &Handle<CustomMaterial>), With<ParticleSprite>>,
    // mut materials: ResMut<Assets<CustomMaterial>>,
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
    let instance_data = &mut query2.single_mut().0;

    for (i, data) in instance_data.iter_mut().zip(particles.iter_all()) {
        i.position.x = data.pos.x;
        i.position.y = data.pos.y;

        let color = if data.solid {
            Color::rgb(0.5, 0.5, 0.5)
        } else {
            Color::rgb(0.2, 0.4, 1.0)
        };
        i.color = color.as_linear_rgba_f32();

        // if i % 100 == 0 {
        //     println!(
        //         "{} -> {}",
        //         particles.prev_positions[i], particles.positions[i]
        //     );
        // }
    }
    span.exit();
}

// I have no idea what's going on from here on down

#[derive(Component)]
pub(crate) struct InstanceMaterialData(Vec<InstanceData>);
impl ExtractComponent for InstanceMaterialData {
    type Query = &'static InstanceMaterialData;
    type Filter = ();

    fn extract_component(item: bevy::ecs::query::QueryItem<Self::Query>) -> Self {
        InstanceMaterialData(item.0.clone())
    }
}

pub struct CustomMaterialPlugin;

impl Plugin for CustomMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(ExtractComponentPlugin::<InstanceMaterialData>::default());
        app.sub_app_mut(RenderApp)
            .add_render_command::<Transparent2d, DrawCustom>()
            .init_resource::<CustomPipeline>()
            .init_resource::<SpecializedPipelines<CustomPipeline>>()
            .add_system_to_stage(RenderStage::Queue, queue_custom)
            .add_system_to_stage(RenderStage::Prepare, prepare_instance_buffers);
    }
}

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
struct InstanceData {
    position: Vec3,
    scale: f32,
    color: [f32; 4],
}

fn queue_custom(
    transparent_2d_draw_functions: Res<DrawFunctions<Transparent2d>>,
    custom_pipeline: Res<CustomPipeline>,
    msaa: Res<Msaa>,
    mut pipelines: ResMut<SpecializedPipelines<CustomPipeline>>,
    mut pipeline_cache: ResMut<RenderPipelineCache>,
    material_meshes: Query<
        Entity,
        // (Entity, &Mesh2dUniform),
        (With<Mesh2dHandle>, With<InstanceMaterialData>),
    >,
    mut views: Query<(&ExtractedView, &mut RenderPhase<Transparent2d>)>,
) {
    let draw_custom = transparent_2d_draw_functions
        .read()
        .get_id::<DrawCustom>()
        .unwrap();

    let key = Mesh2dPipelineKey::from_msaa_samples(msaa.samples)
        | Mesh2dPipelineKey::from_primitive_topology(PrimitiveTopology::TriangleList);
    let pipeline = pipelines.specialize(&mut pipeline_cache, &custom_pipeline, key);

    for (view, mut transparent_phase) in views.iter_mut() {
        let view_matrix = view.transform.compute_matrix();
        let view_row_2 = view_matrix.row(2);
        for entity in material_meshes.iter() {
            transparent_phase.add(Transparent2d {
                // sort_key: FloatOrd(view_row_2.dot(mesh_uniform.transform.col(3))),
                sort_key: FloatOrd(0.0),
                entity,
                pipeline,
                draw_function: draw_custom,
                batch_range: None,
            });
        }
    }
}

#[derive(Component)]
pub struct InstanceBuffer {
    buffer: Buffer,
    length: usize,
}

fn prepare_instance_buffers(
    mut commands: Commands,
    query: Query<(Entity, &InstanceMaterialData)>,
    render_device: Res<RenderDevice>,
) {
    for (entity, instance_data) in query.iter() {
        let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("instance data buffer"),
            contents: bytemuck::cast_slice(instance_data.0.as_slice()),
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        });
        commands.entity(entity).insert(InstanceBuffer {
            buffer,
            length: instance_data.0.len(),
        });
    }
}

pub struct CustomPipeline {
    shader: Handle<Shader>,
    mesh_pipeline: Mesh2dPipeline,
}

impl FromWorld for CustomPipeline {
    fn from_world(world: &mut World) -> Self {
        let world = world.cell();
        let asset_server = world.get_resource::<AssetServer>().unwrap();
        asset_server.watch_for_changes().unwrap();
        let shader = asset_server.load("particle_material.wgsl");

        let mesh_pipeline = world.get_resource::<Mesh2dPipeline>().unwrap();

        CustomPipeline {
            shader,
            mesh_pipeline: mesh_pipeline.clone(),
        }
    }
}

impl SpecializedPipeline for CustomPipeline {
    type Key = Mesh2dPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let mut descriptor = self.mesh_pipeline.specialize(key);
        descriptor.vertex.shader = self.shader.clone();
        descriptor.vertex.buffers.push(VertexBufferLayout {
            array_stride: std::mem::size_of::<InstanceData>() as u64,
            step_mode: VertexStepMode::Instance,
            attributes: vec![
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: 0,
                    shader_location: 3, // shader locations 0-2 are taken up by Position, Normal and UV attributes
                },
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: VertexFormat::Float32x4.size(),
                    shader_location: 4,
                },
            ],
        });
        descriptor.fragment.as_mut().unwrap().shader = self.shader.clone();
        descriptor.layout = Some(vec![
            self.mesh_pipeline.view_layout.clone(),
            self.mesh_pipeline.mesh_layout.clone(),
        ]);

        descriptor
    }
}

type DrawCustom = (
    SetItemPipeline,
    SetMesh2dViewBindGroup<0>,
    SetMesh2dBindGroup<1>,
    DrawMeshInstanced,
);

pub struct DrawMeshInstanced;
impl EntityRenderCommand for DrawMeshInstanced {
    type Param = (
        SRes<RenderAssets<Mesh>>,
        SQuery<Read<Mesh2dHandle>>,
        SQuery<Read<InstanceBuffer>>,
    );
    #[inline]
    fn render<'w>(
        _view: Entity,
        item: Entity,
        (meshes, mesh_query, instance_buffer_query): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let mesh_handle = mesh_query.get(item).unwrap();
        let instance_buffer = instance_buffer_query.get(item).unwrap();

        let gpu_mesh = match meshes.into_inner().get(&mesh_handle.0) {
            Some(gpu_mesh) => gpu_mesh,
            None => return RenderCommandResult::Failure,
        };

        pass.set_vertex_buffer(0, gpu_mesh.vertex_buffer.slice(..));
        pass.set_vertex_buffer(1, instance_buffer.buffer.slice(..));

        pass.set_vertex_buffer(0, gpu_mesh.vertex_buffer.slice(..));
        match &gpu_mesh.buffer_info {
            GpuBufferInfo::Indexed {
                buffer,
                index_format,
                count,
            } => {
                pass.set_index_buffer(buffer.slice(..), 0, *index_format);
                pass.draw_indexed(0..*count, 0, 0..instance_buffer.length as u32);
            }
            GpuBufferInfo::NonIndexed { vertex_count } => {
                pass.draw_indexed(0..*vertex_count, 0, 0..instance_buffer.length as u32);
            }
        }
        RenderCommandResult::Success
    }
}
