use crate::arcball::ArcballCamera;
use crate::cornell_box as cbox;
use crate::gui::Gui;
use crate::texture::Texture;
use anyhow::{Context, Result};
use cgmath::Matrix4;
use cgmath::SquareMatrix;
use cgmath::{Vector2, Vector3};
use std::{borrow::Cow, sync::mpsc::channel};
use wgpu::util::DeviceExt;
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, MouseButton, MouseScrollDelta},
    window::Window,
};

#[derive(Debug, Copy, Clone)]
enum CameraOperation {
    None,
    Rotate,
    Pan,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    origin: [f32; 4],
    view_direction: [f32; 4],
    up: [f32; 4],
    view_matrix: [[f32; 4]; 4],
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Settings {
    field_weight: f32,
}

#[repr(C)]
#[derive(Debug, Clone)]
struct Vertices {
    length: u32,
    data: Vec<u32>,
}

#[repr(C)]
#[derive(Debug, Clone)]
struct Faces {
    length: u32,
    data: Vec<u32>,
}

impl CameraUniform {
    fn moving(camera: &ArcballCamera<f32>) -> CameraUniform {
        let eye_pos = camera.eye_pos();
        let eye_dir = camera.eye_dir();
        let up_dir = camera.up_dir();
        CameraUniform {
            origin: [eye_pos.x, eye_pos.y, eye_pos.z, 0.0],
            view_direction: [eye_dir.x, eye_dir.y, eye_dir.z, 0.0],
            up: [up_dir.x, up_dir.y, up_dir.z, 0.0],
            view_matrix: Matrix4::identity().into(),
        }
    }

    fn stationary(camera: &ArcballCamera<f32>) -> CameraUniform {
        CameraUniform {
            origin: [0.0, 0.0, 0.0, 0.0],
            view_direction: [0.0, 0.0, -1.0, 0.0],
            up: [0.0, 1.0, 0.0, 0.0],
            view_matrix: camera.get_mat4().into(),
        }
    }
}

pub struct Application {
    _instance: wgpu::Instance,
    surface: wgpu::Surface,
    _adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    sc_desc: wgpu::SwapChainDescriptor,
    swap_chain: wgpu::SwapChain,
    compute_shader_src: String,
    _compute_shader: wgpu::ShaderModule,
    _display_shader: wgpu::ShaderModule,
    texture: Texture,
    compute_bind_group_layout: wgpu::BindGroupLayout,
    compute_bind_group: wgpu::BindGroup,
    compute_pipeline_layout: wgpu::PipelineLayout,
    compute_pipeline: wgpu::ComputePipeline,
    render_bind_group_layout: wgpu::BindGroupLayout,
    render_bind_group: wgpu::BindGroup,
    _render_pipeline_layout: wgpu::PipelineLayout,
    render_pipeline: wgpu::RenderPipeline,

    camera_buffer: wgpu::Buffer,
    camera: ArcballCamera<f32>,
    camera_op: CameraOperation,
    settings_buffer: wgpu::Buffer,
    prev_cursor_pos: Option<PhysicalPosition<f64>>,
    needs_redraw: bool,

    mesh_bind_group_layout: wgpu::BindGroupLayout,
    mesh_bind_group: wgpu::BindGroup,

    pub gui: Gui,
}

impl Application {
    pub async fn new(window: &Window) -> Result<Self> {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
        let surface = unsafe { instance.create_surface(window) };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
            })
            .await
            .context("no compatible adapter found")?;
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::default(),
                },
                None,
            )
            .await?;

        let swapchain_format = adapter
            .get_swap_chain_preferred_format(&surface)
            .context("no compatible swap chain format found")?;
        let sc_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
            format: swapchain_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };
        let swap_chain = device.create_swap_chain(&surface, &sc_desc);

        let gui = Gui::new(size, window.scale_factor(), &device, swapchain_format);

        #[cfg(not(target_arch = "wasm32"))]
        let flags = wgpu::ShaderFlags::all();
        #[cfg(target_arch = "wasm32")]
        let flags = wgpu::ShaderFlags::VALIDATION;

        let mut vertices = cbox::VERTICES;
        for i in 0..(vertices.len() / 3) {
            // Invert x and z axis
            vertices[3 * i] = -vertices[3 * i];
            vertices[3 * i + 2] = -vertices[3 * i + 2];
        }
        normalize_vertices(&mut vertices);

        let compute_shader_src = include_str!("compute.wgsl");
        let compute_shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some("compute_shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(&with_field_function(
                compute_shader_src,
                &gui.field_function,
            ))),
            flags,
        });

        let display_shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some("display_shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("display.wgsl"))),
            flags,
        });

        let texture = Texture::new(&device, (size.width, size.height), Some("texture"));

        let center = get_center(&vertices);
        let mut camera = ArcballCamera::new(center, 1.0, [size.width as f32, size.height as f32]);
        camera.zoom(-1.0, 1.0);

        let camera_uniform = CameraUniform::moving(&camera);
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("camera_buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        });

        let settings = Settings {
            field_weight: gui.field_weight,
        };
        let settings_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("settings_buffer"),
            contents: bytemuck::cast_slice(&[settings]),
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        });

        let vertices_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("vertices_buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsage::STORAGE,
        });

        let faces_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("faces_buffer"),
            contents: bytemuck::cast_slice(&cbox::INDICES),
            usage: wgpu::BufferUsage::STORAGE,
        });

        let compute_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStage::COMPUTE,
                        ty: wgpu::BindingType::StorageTexture {
                            access: wgpu::StorageTextureAccess::WriteOnly,
                            format: texture.format,
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStage::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStage::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
                label: Some("compute_bind_group_layout"),
            });
        let compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &compute_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: settings_buffer.as_entire_binding(),
                },
            ],
            label: Some("compute_bind_group"),
        });
        let mesh_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStage::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStage::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
                label: Some("mesh_bind_group_layout"),
            });
        let mesh_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &mesh_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: vertices_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: faces_buffer.as_entire_binding(),
                },
            ],
            label: Some("mesh_bind_group"),
        });
        let compute_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("compute_pipeline_layout"),
                bind_group_layouts: &[&compute_bind_group_layout, &mesh_bind_group_layout],
                push_constant_ranges: &[],
            });
        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("compute_pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &compute_shader,
            entry_point: "main",
        });

        let render_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("render_bind_group_layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::ReadOnly,
                        format: texture.format,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                }],
            });
        let render_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("render_bind_group"),
            layout: &render_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&texture.view),
            }],
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("render_pipeline_layout"),
                bind_group_layouts: &[&render_bind_group_layout],
                push_constant_ranges: &[],
            });
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("render_pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &display_shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &display_shader,
                entry_point: "fs_main",
                targets: &[swapchain_format.into()],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
        });

        Ok(Self {
            _instance: instance,
            surface,
            _adapter: adapter,
            device,
            queue,
            sc_desc,
            swap_chain,
            compute_shader_src: compute_shader_src.to_string(),
            _compute_shader: compute_shader,
            _display_shader: display_shader,
            texture,
            compute_bind_group_layout,
            compute_bind_group,
            compute_pipeline_layout,
            compute_pipeline,
            render_bind_group_layout,
            render_bind_group,
            _render_pipeline_layout: render_pipeline_layout,
            render_pipeline,

            camera_buffer,
            camera,
            camera_op: CameraOperation::None,
            settings_buffer,
            prev_cursor_pos: None,
            needs_redraw: true,

            mesh_bind_group_layout,
            mesh_bind_group,

            gui,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.texture = Texture::new(&self.device, (width, height), Some("texture"));
        self.compute_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.compute_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&self.texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.settings_buffer.as_entire_binding(),
                },
            ],
            label: Some("compute_bind_group"),
        });
        self.render_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("texture_bind_group"),
            layout: &self.render_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&self.texture.view),
            }],
        });
        self.sc_desc.width = width;
        self.sc_desc.height = height;
        self.swap_chain = self.device.create_swap_chain(&self.surface, &self.sc_desc);
        self.camera.update_screen(width as f32, height as f32);
        self.update_camera();
        self.needs_redraw = true;
    }

    pub fn reload_compute_shader(&mut self, new_src: Option<&str>) -> Result<(), wgpu::Error> {
        let src = with_field_function(
            new_src.unwrap_or(&self.compute_shader_src),
            &self.gui.field_function,
        );

        let (tx, rx) = channel::<wgpu::Error>();
        self.device.on_uncaptured_error(move |e: wgpu::Error| {
            tx.send(e).expect("sending error failed");
        });

        let compute_shader = self
            .device
            .create_shader_module(&wgpu::ShaderModuleDescriptor {
                label: Some("compute_shader"),
                source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(&src)),
                flags: wgpu::ShaderFlags::all(),
            });
        let compute_pipeline =
            self.device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("compute_pipeline"),
                    layout: Some(&self.compute_pipeline_layout),
                    module: &compute_shader,
                    entry_point: "main",
                });

        self.device.on_uncaptured_error(|e| panic!("{}", e));

        if let Ok(err) = rx.try_recv() {
            return Err(err);
        }

        if let Some(new_src) = new_src {
            self.compute_shader_src = new_src.to_string();
        }
        self._compute_shader = compute_shader;
        self.compute_pipeline = compute_pipeline;
        self.needs_redraw = true;

        Ok(())
    }

    pub fn load_default_model(&mut self) {
        let mut vertices = cbox::VERTICES;
        self.load_model(&mut vertices, &cbox::INDICES);
    }

    pub fn load_model(&mut self, vertices: &mut [f32], indices: &[u32]) {
        normalize_vertices(vertices);

        let vertices_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("vertices_buffer"),
                contents: bytemuck::cast_slice(vertices),
                usage: wgpu::BufferUsage::STORAGE,
            });
        let faces_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("faces_buffer"),
                contents: bytemuck::cast_slice(indices),
                usage: wgpu::BufferUsage::STORAGE,
            });

        self.mesh_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.mesh_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: vertices_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: faces_buffer.as_entire_binding(),
                },
            ],
            label: Some("mesh_bind_group"),
        });

        let center = get_center(vertices);
        self.camera = ArcballCamera::new(
            center,
            1.0,
            [self.sc_desc.width as f32, self.sc_desc.height as f32],
        );
        self.camera.zoom(-1.0, 1.0);
        self.update_camera();
        self.needs_redraw = true;
    }

    fn update_camera(&mut self) {
        let uniform = if self.gui.rotate_scene {
            CameraUniform::stationary(&self.camera)
        } else {
            CameraUniform::moving(&self.camera)
        };
        self.queue
            .write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[uniform]));
        self.needs_redraw = true;
    }

    fn update_settings(&mut self) {
        let settings = Settings {
            field_weight: self.gui.field_weight,
        };
        self.queue
            .write_buffer(&self.settings_buffer, 0, bytemuck::cast_slice(&[settings]));
        self.needs_redraw = true;
    }

    pub fn reset_camera(&mut self) {
        let center = self.camera.center;
        self.camera = ArcballCamera::new(
            center,
            1.0,
            [self.sc_desc.width as f32, self.sc_desc.height as f32],
        );
        self.camera.zoom(-1.0, 1.0);
        self.update_camera();
    }

    pub fn on_mouse_wheel(&mut self, delta: MouseScrollDelta) {
        let y = match delta {
            MouseScrollDelta::LineDelta(_, y) => y,
            MouseScrollDelta::PixelDelta(p) => p.y as f32,
        };
        self.camera.zoom(y, 1.0 / 60.0);
        self.update_camera();
    }

    pub fn on_mouse_input(&mut self, state: ElementState, button: MouseButton) {
        self.camera_op = match state {
            ElementState::Pressed => match button {
                MouseButton::Left => CameraOperation::Rotate,
                MouseButton::Right => CameraOperation::Pan,
                _ => self.camera_op,
            },
            ElementState::Released => match (button, self.camera_op) {
                (MouseButton::Left, CameraOperation::Rotate) => CameraOperation::None,
                (MouseButton::Right, CameraOperation::Pan) => CameraOperation::None,
                _ => self.camera_op,
            },
        };
    }

    pub fn on_cursor_moved(&mut self, pos: PhysicalPosition<f64>) {
        if self.prev_cursor_pos.is_none() {
            self.prev_cursor_pos = Some(pos);
            return;
        }
        let prev = self.prev_cursor_pos.unwrap();
        match self.camera_op {
            CameraOperation::Rotate => {
                self.camera.rotate(
                    Vector2::new(prev.x as f32, prev.y as f32),
                    Vector2::new(pos.x as f32, pos.y as f32),
                );
                self.update_camera();
            }
            CameraOperation::Pan => {
                self.camera.pan(Vector2::new(
                    (pos.x - prev.x) as f32,
                    (pos.y - prev.y) as f32,
                ));
                self.update_camera();
            }
            CameraOperation::None => {}
        }
        self.prev_cursor_pos = Some(pos);
    }

    pub fn render(&mut self, scale_factor: f32) {
        let frame = self
            .swap_chain
            .get_current_frame()
            .expect("failed to acquire next swap chain texture")
            .output;
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("encoder"),
            });

        if self.needs_redraw {
            self.needs_redraw = false;
            encoder.push_debug_group("compute");
            {
                let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("cpass"),
                });
                cpass.set_pipeline(&self.compute_pipeline);
                cpass.set_bind_group(0, &self.compute_bind_group, &[]);
                cpass.set_bind_group(1, &self.mesh_bind_group, &[]);
                cpass.dispatch(
                    (self.sc_desc.width + 7) / 8,
                    (self.sc_desc.height + 7) / 8,
                    1,
                );
            }
            encoder.pop_debug_group();
        }

        encoder.push_debug_group("display");
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("rpass"),
                color_attachments: &[wgpu::RenderPassColorAttachment {
                    view: &frame.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
            });
            rpass.set_pipeline(&self.render_pipeline);
            rpass.set_bind_group(0, &self.render_bind_group, &[]);
            rpass.draw(0..6, 0..1)
        }
        encoder.pop_debug_group();

        self.queue.submit(Some(encoder.finish()));

        self.gui.draw(
            &frame.view,
            self.sc_desc.width,
            self.sc_desc.height,
            scale_factor,
            &self.device,
            &self.queue,
        );

        if self.gui.rotate_scene_changed {
            self.gui.rotate_scene_changed = false;
            self.update_camera();
        }
        if self.gui.field_weight_changed {
            self.gui.field_weight_changed = false;
            self.update_settings();
        }
        if self.gui.field_function_changed {
            self.gui.field_function_changed = false;
            if let Err(e) = self.reload_compute_shader(None) {
                self.gui.shader_error = Some(e.to_string());
            } else {
                self.gui.shader_error = None;
            }
        }
    }
}

fn normalize_vertices(vertices: &mut [f32]) {
    let mut max: f32 = 1.0;
    let mut min: f32 = -1.0;
    for (i, x) in vertices.iter().enumerate() {
        if i == 0 || *x > max {
            max = *x;
        }
        if i == 0 || *x < min {
            min = *x;
        }
    }
    for x in vertices.iter_mut() {
        *x = (*x - min) / (max - min) * 2.0 - 1.0;
    }
}

fn get_center(vertices: &[f32]) -> Vector3<f32> {
    let mut min_x = vertices[0];
    let mut min_y = vertices[1];
    let mut min_z = vertices[2];
    let mut max_x = vertices[0];
    let mut max_y = vertices[1];
    let mut max_z = vertices[2];

    let num_vertices = vertices.len() / 3;
    for i in 1..num_vertices {
        let x = vertices[3 * i];
        if x < min_x {
            min_x = x;
        }
        if x > max_x {
            max_x = x;
        }
        let y = vertices[3 * i + 1];
        if y < min_y {
            min_y = y;
        }
        if y > max_y {
            max_y = y;
        }
        let z = vertices[3 * i + 2];
        if z < min_z {
            min_z = z;
        }
        if z > max_z {
            max_z = z;
        }
    }

    Vector3::new(
        (min_x + max_x) / 2.0,
        (min_y + max_y) / 2.0,
        (min_z + max_z) / 2.0,
    )
}

fn with_field_function(shader_src: &str, field_function_body: &str) -> String {
    let field_function = format!(
        "fn field_function(p: vec3<f32>, v: vec3<f32>) -> vec3<f32> {{\n{}\n}}",
        field_function_body,
    );
    shader_src.replace(
        "fn field_function(p: vec3<f32>, v: vec3<f32>) -> vec3<f32> { return v; }",
        &field_function,
    )
}
