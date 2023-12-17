use std::collections::HashSet;

use crate::texture::Texture;
use anyhow::bail;
use encase::{ShaderSize, ShaderType, StorageBuffer, UniformBuffer};
use wgpu::util::DeviceExt as _;
use winit::{keyboard::KeyCode, window::Window};

#[derive(ShaderType)]
struct Camera {
    position: cgmath::Vector3<f32>,
    aspect: f32,
}

#[derive(ShaderType)]
struct Vertices {
    vertices: [cgmath::Vector3<f32>; 6],
}

#[derive(ShaderType)]
struct Face {
    position: cgmath::Vector3<f32>,
}

#[derive(ShaderType)]
struct Faces<'a> {
    #[size(runtime)]
    faces: &'a [Face],
}

pub struct Game {
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

    pressed_keys: HashSet<winit::keyboard::KeyCode>,

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

        let vertices_uniform_buffer = {
            let mut buffer = UniformBuffer::new([0; Vertices::SHADER_SIZE.get() as _]);
            buffer.write(&Vertices {
                vertices: [
                    cgmath::vec3(-0.5, -0.5, -0.5),
                    cgmath::vec3(-0.5, 0.5, -0.5),
                    cgmath::vec3(-0.5, -0.5, 0.5),
                    cgmath::vec3(-0.5, -0.5, 0.5),
                    cgmath::vec3(-0.5, 0.5, -0.5),
                    cgmath::vec3(-0.5, 0.5, 0.5),
                ],
            })?;
            let buffer = buffer.into_inner();

            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Vertices Uniform Buffer"),
                usage: wgpu::BufferUsages::UNIFORM,
                contents: &buffer,
            })
        };

        let faces_storage_buffer = {
            let mut buffer = StorageBuffer::new(Vec::with_capacity(Faces::min_size().get() as _));
            buffer.write(&Faces {
                faces: &[Face {
                    position: cgmath::vec3(0.0, 0.0, 0.0),
                }],
            })?;
            let buffer = buffer.into_inner();

            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Faces Storage Buffer"),
                usage: wgpu::BufferUsages::STORAGE,
                contents: &buffer,
            })
        };

        let vertices_faces_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Vertices Faces Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: Some(Vertices::SHADER_SIZE),
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: true,
                            min_binding_size: Some(Faces::min_size()),
                        },
                        count: None,
                    },
                ],
            });

        let vertices_faces_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Vertices Faces Bind Group"),
            layout: &vertices_faces_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: vertices_uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: faces_storage_buffer.as_entire_binding(),
                },
            ],
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
            vertices_faces_bind_group,

            camera: Camera {
                position: cgmath::vec3(-2.0, 0.0, 0.0),
                aspect: size.width as f32 / size.height as f32,
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

        if self.pressed_keys.contains(&KeyCode::KeyW) {
            self.camera.position.x += ts;
        }
        if self.pressed_keys.contains(&KeyCode::KeyS) {
            self.camera.position.x -= ts;
        }
        if self.pressed_keys.contains(&KeyCode::KeyA) {
            self.camera.position.z -= ts;
        }
        if self.pressed_keys.contains(&KeyCode::KeyD) {
            self.camera.position.z += ts;
        }
        if self.pressed_keys.contains(&KeyCode::KeyQ) {
            self.camera.position.y -= ts;
        }
        if self.pressed_keys.contains(&KeyCode::KeyE) {
            self.camera.position.y += ts;
        }

        Ok(())
    }

    pub fn key_event(&mut self, event: winit::event::KeyEvent) {
        match event.state {
            winit::event::ElementState::Pressed => {
                if let winit::keyboard::PhysicalKey::Code(code) = event.physical_key {
                    self.pressed_keys.insert(code);
                }
            }

            winit::event::ElementState::Released => {
                if let winit::keyboard::PhysicalKey::Code(code) = event.physical_key {
                    self.pressed_keys.remove(&code);
                }
            }
        }
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
            render_pass.set_bind_group(1, &self.vertices_faces_bind_group, &[0]);
            render_pass.draw(0..6, 0..1);
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
