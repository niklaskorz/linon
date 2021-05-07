use crate::arcball::ArcballCamera;
use crate::texture::Texture;
use anyhow::{Context, Result};
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
}

impl CameraUniform {
    fn from(camera: &ArcballCamera<f32>) -> CameraUniform {
        let eye_pos = camera.eye_pos();
        let eye_dir = camera.eye_dir();
        let up_dir = camera.up_dir();
        CameraUniform {
            origin: [eye_pos.x, eye_pos.y, eye_pos.z, 0.0],
            view_direction: [eye_dir.x, eye_dir.y, eye_dir.z, 0.0],
            up: [up_dir.x, up_dir.y, up_dir.z, 0.0],
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

    uniform_buffer: wgpu::Buffer,
    camera: ArcballCamera<f32>,
    camera_op: CameraOperation,
    prev_cursor_pos: Option<PhysicalPosition<f64>>,
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
            present_mode: wgpu::PresentMode::Mailbox,
        };
        let swap_chain = device.create_swap_chain(&surface, &sc_desc);

        #[cfg(not(target_arch = "wasm32"))]
        let flags = wgpu::ShaderFlags::all();
        #[cfg(target_arch = "wasm32")]
        let flags = wgpu::ShaderFlags::VALIDATION;

        let compute_shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some("compute_shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("compute.wgsl"))),
            flags,
        });

        let display_shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some("display_shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("display.wgsl"))),
            flags,
        });

        let texture = Texture::new(&device, (size.width, size.height), Some("texture"));
        let mut camera = ArcballCamera::new(
            Vector3::new(278.0, 273.0, 279.6),
            1.0,
            [size.width as f32, size.height as f32],
        );
        camera.zoom(1079.6, 1.0);

        let uniform = CameraUniform::from(&camera);
        /*let uniform = CameraUniform {
            origin: [278.0, 273.0, -800.0, 0.0],
            view_direction: [0.0, 0.0, 1.0, 0.0],
            up: [0.0, 1.0, 0.0, 0.0],
        };*/
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("uniform_buffer"),
            contents: bytemuck::cast_slice(&[uniform]),
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
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
                    resource: uniform_buffer.as_entire_binding(),
                },
            ],
            label: Some("compute_bind_group"),
        });
        let compute_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("compute_pipeline_layout"),
                bind_group_layouts: &[&compute_bind_group_layout],
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
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
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

            uniform_buffer,
            camera,
            camera_op: CameraOperation::None,
            prev_cursor_pos: None,
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
                    resource: self.uniform_buffer.as_entire_binding(),
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
    }

    pub fn reload_compute_shader(&mut self, source: &str) -> Result<(), wgpu::Error> {
        let (tx, rx) = channel::<wgpu::Error>();
        self.device.on_uncaptured_error(move |e: wgpu::Error| {
            tx.send(e).expect("sending error failed");
        });

        let compute_shader = self
            .device
            .create_shader_module(&wgpu::ShaderModuleDescriptor {
                label: Some("compute_shader"),
                source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(source)),
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

        self.device.on_uncaptured_error(|e| panic!(e));

        if let Ok(err) = rx.try_recv() {
            return Err(err);
        }

        self._compute_shader = compute_shader;
        self.compute_pipeline = compute_pipeline;

        Ok(())
    }

    fn update_camera(&mut self) {
        let uniform = CameraUniform::from(&self.camera);
        println!("Updated camera: {:?}", uniform);
        self.queue
            .write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniform]))
    }

    pub fn on_mouse_wheel(&mut self, delta: MouseScrollDelta) {
        let y = match delta {
            MouseScrollDelta::LineDelta(_, y) => y,
            MouseScrollDelta::PixelDelta(p) => p.y as f32,
        };
        self.camera.zoom(y, 10.0);
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
            CameraOperation::Rotate => self.camera.rotate(
                Vector2::new(prev.x as f32, prev.y as f32),
                Vector2::new(pos.x as f32, pos.y as f32),
            ),
            CameraOperation::Pan => self.camera.pan(Vector2::new(
                (pos.x - prev.x) as f32,
                (pos.y - prev.y) as f32,
            )),
            CameraOperation::None => {}
        }
        self.prev_cursor_pos = Some(pos);
        self.update_camera();
    }

    pub fn render(&mut self) {
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

        encoder.push_debug_group("compute");
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("cpass"),
            });
            cpass.set_pipeline(&self.compute_pipeline);
            cpass.set_bind_group(0, &self.compute_bind_group, &[]);
            cpass.dispatch(
                (self.sc_desc.width + 7) / 8,
                (self.sc_desc.height + 7) / 8,
                1,
            );
        }
        encoder.pop_debug_group();

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
    }
}
