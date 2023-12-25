use std::collections::HashSet;

use crate::{
    chunk::{Block, Chunk},
    math::Motor,
    texture::Texture,
};
use anyhow::bail;
use cgmath::InnerSpace;
use encase::{DynamicStorageBuffer, ShaderSize, ShaderType, UniformBuffer};
use wgpu::util::DeviceExt as _;
use winit::{keyboard::KeyCode, window::Window};

#[derive(ShaderType)]
struct Camera {
    transform: Motor,
    aspect: f32,
    near_clip: f32,
    far_clip: f32,
}

#[derive(ShaderType)]
struct Face {
    position: cgmath::Vector3<f32>,
    normal: cgmath::Vector3<f32>,
    color: cgmath::Vector3<f32>,
}

#[derive(ShaderType)]
struct Faces<'a> {
    vertices: [cgmath::Vector3<f32>; 6],
    #[size(runtime)]
    faces: &'a [Face],
}

struct FaceInfo {
    start_offset: u32,
    count: u32,
}

pub struct Game {
    face_infos: Vec<FaceInfo>,
    vertices_faces_bind_group: wgpu::BindGroup,

    camera: Camera,
    camera_uniform_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,

    render_pipeline: wgpu::RenderPipeline,
    depth_buffer: Texture,

    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,

    pub(crate) pressed_keys: HashSet<winit::keyboard::KeyCode>,

    // the window must be dropped last because its referenced by the surface
    window: Window,
}

impl Game {
    pub async fn new(window: Window) -> anyhow::Result<Self> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let surface = unsafe { instance.create_surface(&window) }?;

        let Some(adapter) = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
        else {
            bail!("could not find a suitable wgpu adapter");
        };

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::default(),
                    label: None,
                },
                None,
            )
            .await?;

        let size = window.inner_size();
        let surface_caps = surface.get_capabilities(&adapter);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_caps
                .formats
                .iter()
                .copied()
                .find(|f| f.is_srgb())
                .unwrap_or(surface_caps.formats[0]),
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::AutoNoVsync,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        let depth_buffer = Texture::new(
            Some("Depth Buffer"),
            Some("Depth Buffer Sampler"),
            &device,
            wgpu::Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            wgpu::TextureFormat::Depth32Float,
            wgpu::AddressMode::ClampToEdge,
            wgpu::FilterMode::Linear,
            wgpu::FilterMode::Linear,
            Some(wgpu::CompareFunction::LessEqual),
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        );

        let shader = device.create_shader_module(wgpu::include_wgsl!("./shader.wgsl"));

        let camera_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Camera Uniform Buffer"),
            size: Camera::SHADER_SIZE.get(),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Camera Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(Camera::SHADER_SIZE),
                    },
                    count: None,
                }],
            });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera Bind Group"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_uniform_buffer.as_entire_binding(),
            }],
        });

        let mut chunk = Chunk {
            blocks: Box::new(std::array::from_fn(|_| {
                std::array::from_fn(|_| std::array::from_fn(|_| Block::Air))
            })),
        };
        chunk.blocks[0][0][0] = Block::Stone;
        chunk.blocks[0][0][1] = Block::Stone;
        chunk.blocks[0][2][0] = Block::Stone;
        chunk.blocks[1][2][0] = Block::Stone;
        chunk.blocks[1][2][1] = Block::Stone;
        chunk.blocks[1][3][1] = Block::Stone;

        let mut face_infos = vec![];
        let faces_storage_buffer = {
            let mut buffer = Vec::with_capacity(Faces::min_size().get() as _);
            let faces = chunk.generate_faces();

            macro_rules! face {
                ($face:ident, $normal:expr, $vertices:expr $(,)?) => {{
                    let start_offset = buffer.len().try_into()?;

                    let mut storage_buffer = DynamicStorageBuffer::new(buffer);
                    storage_buffer.set_offset(start_offset as _);
                    let faces = faces
                        .$face
                        .into_iter()
                        .map(|(position, block)| Face {
                            position: position.cast().unwrap(),
                            normal: $normal,
                            color: match block {
                                Block::Air => unreachable!(),
                                Block::Stone => cgmath::vec3(0.2, 0.2, 0.2),
                            },
                        })
                        .collect::<Vec<_>>();
                    let face_data = Faces {
                        vertices: $vertices,
                        faces: &faces,
                    };
                    storage_buffer.write(&face_data)?;
                    face_infos.push(FaceInfo {
                        start_offset,
                        count: face_data.faces.len().try_into()?,
                    });

                    buffer = storage_buffer.into_inner();
                    buffer.resize(((buffer.len() + (256 - 1)) & !(256 - 1)), 0);
                }};
            }

            face!(
                back,
                cgmath::vec3(-1.0, 0.0, 0.0),
                [
                    cgmath::vec3(-0.5, -0.5, -0.5),
                    cgmath::vec3(-0.5, 0.5, -0.5),
                    cgmath::vec3(-0.5, -0.5, 0.5),
                    cgmath::vec3(-0.5, -0.5, 0.5),
                    cgmath::vec3(-0.5, 0.5, -0.5),
                    cgmath::vec3(-0.5, 0.5, 0.5),
                ],
            );
            face!(
                front,
                cgmath::vec3(1.0, 0.0, 0.0),
                [
                    cgmath::vec3(0.5, 0.5, -0.5),
                    cgmath::vec3(0.5, -0.5, -0.5),
                    cgmath::vec3(0.5, -0.5, 0.5),
                    cgmath::vec3(0.5, 0.5, -0.5),
                    cgmath::vec3(0.5, -0.5, 0.5),
                    cgmath::vec3(0.5, 0.5, 0.5),
                ],
            );

            face!(
                top,
                cgmath::vec3(0.0, 1.0, 0.0),
                [
                    cgmath::vec3(-0.5, 0.5, 0.5),
                    cgmath::vec3(-0.5, 0.5, -0.5),
                    cgmath::vec3(0.5, 0.5, -0.5),
                    cgmath::vec3(-0.5, 0.5, 0.5),
                    cgmath::vec3(0.5, 0.5, -0.5),
                    cgmath::vec3(0.5, 0.5, 0.5),
                ],
            );
            face!(
                bottom,
                cgmath::vec3(0.0, -1.0, 0.0),
                [
                    cgmath::vec3(-0.5, -0.5, -0.5),
                    cgmath::vec3(-0.5, -0.5, 0.5),
                    cgmath::vec3(0.5, -0.5, -0.5),
                    cgmath::vec3(0.5, -0.5, -0.5),
                    cgmath::vec3(-0.5, -0.5, 0.5),
                    cgmath::vec3(0.5, -0.5, 0.5),
                ],
            );

            face!(
                left,
                cgmath::vec3(0.0, 0.0, -1.0),
                [
                    cgmath::vec3(-0.5, 0.5, -0.5),
                    cgmath::vec3(-0.5, -0.5, -0.5),
                    cgmath::vec3(0.5, -0.5, -0.5),
                    cgmath::vec3(-0.5, 0.5, -0.5),
                    cgmath::vec3(0.5, -0.5, -0.5),
                    cgmath::vec3(0.5, 0.5, -0.5),
                ],
            );
            face!(
                right,
                cgmath::vec3(0.0, 0.0, 1.0),
                [
                    cgmath::vec3(-0.5, -0.5, 0.5),
                    cgmath::vec3(-0.5, 0.5, 0.5),
                    cgmath::vec3(0.5, -0.5, 0.5),
                    cgmath::vec3(0.5, -0.5, 0.5),
                    cgmath::vec3(-0.5, 0.5, 0.5),
                    cgmath::vec3(0.5, 0.5, 0.5),
                ],
            );

            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Faces Storage Buffer"),
                usage: wgpu::BufferUsages::STORAGE,
                contents: &buffer,
            })
        };

        let vertices_faces_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Vertices Faces Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: true,
                        min_binding_size: Some(Faces::min_size()),
                    },
                    count: None,
                }],
            });

        let vertices_faces_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Vertices Faces Bind Group"),
            layout: &vertices_faces_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &faces_storage_buffer,
                    offset: 0,
                    size: wgpu::BufferSize::new(
                        faces_storage_buffer.size() / face_infos.len() as wgpu::BufferAddress,
                    ),
                }),
            }],
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&camera_bind_group_layout, &vertices_faces_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vertex",
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Cw,
                cull_mode: Some(wgpu::Face::Back),
                // cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "pixel",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
        });

        Ok(Game {
            face_infos,
            vertices_faces_bind_group,

            camera: Camera {
                transform: Motor::translation(cgmath::vec3(-2.0, 0.0, 0.0)),
                aspect: size.width as f32 / size.height as f32,
                near_clip: 0.01,
                far_clip: 100.0,
            },
            camera_uniform_buffer,
            camera_bind_group,

            render_pipeline,
            depth_buffer,

            surface,
            device,
            queue,
            config,

            pressed_keys: HashSet::new(),

            window,
        })
    }

    pub fn update(&mut self, dt: std::time::Duration) -> anyhow::Result<()> {
        let ts = dt.as_secs_f32();

        let mut movement = cgmath::vec3(0.0, 0.0, 0.0);

        if self.pressed_keys.contains(&KeyCode::KeyW) {
            movement.x += 1.0;
        }
        if self.pressed_keys.contains(&KeyCode::KeyS) {
            movement.x -= 1.0;
        }
        if self.pressed_keys.contains(&KeyCode::KeyA) {
            movement.z -= 1.0;
        }
        if self.pressed_keys.contains(&KeyCode::KeyD) {
            movement.z += 1.0;
        }
        if self.pressed_keys.contains(&KeyCode::KeyQ) {
            movement.y -= 1.0;
        }
        if self.pressed_keys.contains(&KeyCode::KeyE) {
            movement.y += 1.0;
        }

        const CAMERA_SPEED: f32 = 3.0;
        if movement.magnitude2() > 0.001 {
            self.camera.transform = self
                .camera
                .transform
                .apply(Motor::translation(movement.normalize() * CAMERA_SPEED * ts));
        }

        Ok(())
    }

    pub fn lost_focus(&mut self) {
        self.pressed_keys.clear();
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        let [width, height] = [width, height].map(|dim| dim.max(1));

        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);

        self.depth_buffer.resize(
            &self.device,
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        self.camera.aspect = width as f32 / height as f32;
    }

    pub fn render(&mut self) -> anyhow::Result<()> {
        let output = loop {
            match self.surface.get_current_texture() {
                Ok(output) => break output,
                Err(error @ wgpu::SurfaceError::Timeout) => {
                    eprintln!("{error}");
                    return Ok(()); // just give up on rendering until the next draw request
                }
                Err(wgpu::SurfaceError::Outdated) => {
                    let size = self.window.inner_size();
                    self.resize(size.width, size.height);
                }
                Err(wgpu::SurfaceError::Lost) => {
                    self.resize(self.config.width, self.config.height);
                }
                Err(error) => return Err(error.into()),
            }
        };

        let output_view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Upload camera data
        {
            let mut buffer = UniformBuffer::new([0; Camera::SHADER_SIZE.get() as _]);
            buffer.write(&self.camera)?;
            let buffer = buffer.into_inner();

            self.queue
                .write_buffer(&self.camera_uniform_buffer, 0, &buffer);
        }

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &output_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.2,
                            g: 0.3,
                            b: 0.8,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: self.depth_buffer.view(),
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            for face_info in &self.face_infos {
                render_pass.set_bind_group(
                    1,
                    &self.vertices_faces_bind_group,
                    &[face_info.start_offset],
                );
                render_pass.draw(0..6 * face_info.count, 0..1);
            }
        }
        self.queue.submit([encoder.finish()]);

        self.window.pre_present_notify();
        output.present();

        Ok(())
    }

    pub fn window(&self) -> &Window {
        &self.window
    }
}
