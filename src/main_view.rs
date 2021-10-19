use crate::{
    arcball::{ArcballCamera, CameraOperation},
    functions::PredefinedFunction,
    texture::Texture,
};
use cgmath::{Matrix4, SquareMatrix, Vector2, Vector3};
use egui_wgpu_backend::epi::TextureAllocator;
use std::{borrow::Cow, sync::mpsc::channel};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    origin: [f32; 4],
    view_direction: [f32; 4],
    up: [f32; 4],
    view_matrix: [[f32; 4]; 4],
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

    fn _stationary(camera: &ArcballCamera<f32>) -> CameraUniform {
        CameraUniform {
            origin: [0.0, 0.0, 0.0, 0.0],
            view_direction: [0.0, 0.0, -1.0, 0.0],
            up: [0.0, 1.0, 0.0, 0.0],
            view_matrix: camera.get_mat4().into(),
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Settings {
    pub field_weight: f32,
    pub mouse_pos: [f32; 2],
    pub overlay_mode: i32,
    pub central_difference_delta: i32,
    pub lyapunov_scaling: f32,
}

pub struct MainView {
    texture: Texture,
    texture_id: egui::TextureId,
    ray_casting_texture: Texture,
    mapping_texture: Texture,
    shader_src: String,
    _shader: wgpu::ShaderModule,
    compute_bind_group_layout: wgpu::BindGroupLayout,
    compute_bind_group: wgpu::BindGroup,
    compute_pipeline_layout: wgpu::PipelineLayout,
    compute_pipeline: wgpu::ComputePipeline,
    mesh_bind_group_layout: wgpu::BindGroupLayout,
    mesh_bind_group: wgpu::BindGroup,
    ray_samples_bind_group: wgpu::BindGroup,
    overlay_bind_group_layout: wgpu::BindGroupLayout,
    overlay_bind_group: wgpu::BindGroup,
    overlay_pipeline: wgpu::ComputePipeline,
    settings_buffer: wgpu::Buffer,
    exponents_buffer: wgpu::Buffer,
    camera_buffer: wgpu::Buffer,
    camera: ArcballCamera<f32>,
    prev_pointer_pos: Option<(f32, f32)>,
    pub needs_redraw: bool,
}

impl MainView {
    pub fn new(
        rpass: &mut egui_wgpu_backend::RenderPass,
        device: &wgpu::Device,
        vertices_buffer_binding: wgpu::BindingResource,
        faces_buffer_binding: wgpu::BindingResource,
        center: Vector3<f32>,
        ray_samples_buffer_binding: wgpu::BindingResource,
        width: u32,
        height: u32,
    ) -> Self {
        let shader_src = include_str!("main_view.wgsl");
        let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some("compute_shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(&with_field_function(
                shader_src,
                &PredefinedFunction::MirageSphericalSigmoid.to_code(),
            ))),
        });

        let texture = Texture::new(
            device,
            (width, height),
            Some("main_texture"),
            wgpu::TextureFormat::Rgba8Unorm,
            true,
        );
        let texture_id = rpass.egui_texture_from_wgpu_texture(
            device,
            &texture.texture,
            wgpu::FilterMode::Linear,
        );

        let ray_casting_texture = Texture::new(
            device,
            (width, height),
            Some("ray_casting_texture"),
            wgpu::TextureFormat::Rgba8Unorm,
            true,
        );

        let mapping_texture = Texture::new(
            device,
            (width, height),
            Some("mapping_texture"),
            wgpu::TextureFormat::Rgba32Float,
            true,
        );

        let settings = Settings {
            field_weight: 1.0,
            mouse_pos: [0.5, 0.6],
            overlay_mode: 0,
            central_difference_delta: 1,
            lyapunov_scaling: 50.0,
        };
        let settings_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("settings_buffer"),
            contents: bytemuck::cast_slice(&[settings]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let exponents_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("exponents_buffer"),
            size: 4 * (width as u64) * (height as u64),
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        let mut camera = ArcballCamera::new(center, 1.0, [width as f32, height as f32]);
        camera.zoom(-1.0, 1.0);
        let camera_uniform = CameraUniform::moving(&camera);
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("camera_buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let compute_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::StorageTexture {
                            access: wgpu::StorageTextureAccess::WriteOnly,
                            format: texture.format,
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::StorageTexture {
                            access: wgpu::StorageTextureAccess::WriteOnly,
                            format: mapping_texture.format,
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
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
                    resource: wgpu::BindingResource::TextureView(&ray_casting_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&mapping_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: settings_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: exponents_buffer.as_entire_binding(),
                },
            ],
            label: Some("compute_bind_group"),
        });
        let mesh_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
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
                    resource: vertices_buffer_binding,
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: faces_buffer_binding,
                },
            ],
            label: Some("mesh_bind_group"),
        });
        let ray_samples_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("ray_samples_bind_group_layout"),
            });
        let ray_samples_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &ray_samples_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: ray_samples_buffer_binding,
            }],
            label: Some("ray_samples_bind_group"),
        });
        let compute_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("compute_pipeline_layout"),
                bind_group_layouts: &[
                    &compute_bind_group_layout,
                    &mesh_bind_group_layout,
                    &ray_samples_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });
        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("compute_pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &shader,
            entry_point: "main_view",
        });

        let overlay_shader_src = include_str!("overlay.wgsl");
        let overlay_shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some("overlay_shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(overlay_shader_src)),
        });
        let overlay_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::StorageTexture {
                            access: wgpu::StorageTextureAccess::WriteOnly,
                            format: texture.format,
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
                label: Some("overlay_bind_group_layout"),
            });
        let overlay_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &overlay_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&ray_casting_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&mapping_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: settings_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: exponents_buffer.as_entire_binding(),
                },
            ],
            label: Some("overlay_bind_group"),
        });
        let overlay_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("overlay_pipeline_layout"),
                bind_group_layouts: &[&overlay_bind_group_layout],
                push_constant_ranges: &[],
            });
        let overlay_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("overlay_pipeline"),
            layout: Some(&overlay_pipeline_layout),
            module: &overlay_shader,
            #[cfg(not(target_arch = "wasm32"))]
            entry_point: "overlay_desktop",
            #[cfg(target_arch = "wasm32")]
            entry_point: "overlay_web",
        });

        Self {
            texture,
            texture_id,
            ray_casting_texture,
            mapping_texture,
            shader_src: shader_src.to_string(),
            _shader: shader,
            compute_bind_group_layout,
            compute_bind_group,
            compute_pipeline_layout,
            compute_pipeline,
            mesh_bind_group_layout,
            mesh_bind_group,
            ray_samples_bind_group,
            overlay_bind_group_layout,
            overlay_bind_group,
            overlay_pipeline,
            settings_buffer,
            exponents_buffer,

            camera_buffer,
            camera,
            prev_pointer_pos: None,
            needs_redraw: true,
        }
    }

    pub fn update_model(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        vertices_buffer_binding: wgpu::BindingResource,
        faces_buffer_binding: wgpu::BindingResource,
        center: Vector3<f32>,
    ) {
        self.mesh_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.mesh_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: vertices_buffer_binding,
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: faces_buffer_binding,
                },
            ],
            label: Some("mesh_bind_group"),
        });

        let (width, height) = self.texture.dimensions;
        self.camera = ArcballCamera::new(center, 1.0, [width as f32, height as f32]);
        self.camera.zoom(-1.0, 1.0);
        self.update_camera(queue);
        self.needs_redraw = true;
    }

    pub fn update_camera(&mut self, queue: &wgpu::Queue) {
        let uniform = CameraUniform::moving(&self.camera);
        queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[uniform]));
        self.needs_redraw = true;
    }

    fn reset_camera(&mut self, queue: &wgpu::Queue) {
        let center = self.camera.center;
        let (width, height) = self.texture.dimensions;
        self.camera = ArcballCamera::new(center, 1.0, [width as f32, height as f32]);
        self.camera.zoom(-1.0, 1.0);
        self.update_camera(queue);
    }

    fn on_zoom(&mut self, queue: &wgpu::Queue, delta: f32) {
        #[cfg(not(target_arch = "wasm32"))]
        self.camera.zoom(delta, 1.0 / 60.0);
        #[cfg(target_arch = "wasm32")]
        self.camera.zoom(delta / 10.0, 1.0 / 60.0);
        self.update_camera(queue);
    }

    fn on_pointer_moved(
        &mut self,
        queue: &wgpu::Queue,
        camera_op: CameraOperation,
        pos: (f32, f32),
    ) {
        if self.prev_pointer_pos.is_none() {
            self.prev_pointer_pos = Some(pos);
            return;
        }
        let prev = self.prev_pointer_pos.unwrap();
        match camera_op {
            CameraOperation::Rotate => {
                self.camera.rotate(
                    Vector2::new(prev.0 as f32, prev.1 as f32),
                    Vector2::new(pos.0 as f32, pos.1 as f32),
                );
                self.update_camera(queue);
            }
            CameraOperation::Pan => {
                self.camera.pan(Vector2::new(
                    (pos.0 - prev.0) as f32,
                    (pos.1 - prev.1) as f32,
                ));
                self.update_camera(queue);
            }
            CameraOperation::None => {}
        }
        self.prev_pointer_pos = Some(pos);
    }

    pub fn update_settings(&mut self, queue: &wgpu::Queue, settings: Settings) {
        queue.write_buffer(&self.settings_buffer, 0, bytemuck::cast_slice(&[settings]));
        self.needs_redraw = true;
    }

    pub fn resize_texture(
        &mut self,
        rpass: &mut egui_wgpu_backend::RenderPass,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        width: u32,
        height: u32,
    ) {
        rpass.free(self.texture_id);
        self.texture = Texture::new(
            device,
            (width, height),
            Some("texture"),
            wgpu::TextureFormat::Rgba8Unorm,
            true,
        );
        self.texture_id = rpass.egui_texture_from_wgpu_texture(
            device,
            &self.texture.texture,
            wgpu::FilterMode::Linear,
        );
        self.ray_casting_texture = Texture::new(
            device,
            (width, height),
            Some("ray_casting_texture"),
            wgpu::TextureFormat::Rgba8Unorm,
            true,
        );
        self.mapping_texture = Texture::new(
            device,
            (width, height),
            Some("mapping_texture"),
            wgpu::TextureFormat::Rgba32Float,
            true,
        );
        self.exponents_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("exponents_buffer"),
            size: 4 * (width as u64) * (height as u64),
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        self.compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.compute_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&self.ray_casting_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&self.mapping_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.settings_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: self.exponents_buffer.as_entire_binding(),
                },
            ],
            label: Some("compute_bind_group"),
        });
        self.overlay_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.overlay_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&self.ray_casting_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&self.mapping_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&self.texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.settings_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: self.exponents_buffer.as_entire_binding(),
                },
            ],
            label: Some("overlay_bind_group"),
        });
        self.camera.update_screen(width as f32, height as f32);
        self.update_camera(queue);
        self.needs_redraw = true;
    }

    pub fn reload_shader(
        &mut self,
        device: &wgpu::Device,
        new_src: Option<&str>,
        field_function: String,
    ) -> Result<(), wgpu::Error> {
        let src = with_field_function(new_src.unwrap_or(&self.shader_src), &field_function);

        let (tx, rx) = channel::<wgpu::Error>();
        device.on_uncaptured_error(move |e: wgpu::Error| {
            tx.send(e).expect("sending error failed");
        });

        let compute_shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some("compute_shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(&src)),
        });
        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("compute_pipeline"),
            layout: Some(&self.compute_pipeline_layout),
            module: &compute_shader,
            entry_point: "main_view",
        });

        device.on_uncaptured_error(|e| panic!("{}", e));

        if let Ok(err) = rx.try_recv() {
            return Err(err);
        }

        if let Some(new_src) = new_src {
            self.shader_src = new_src.to_string();
        }
        self._shader = compute_shader;
        self.compute_pipeline = compute_pipeline;
        self.needs_redraw = true;

        Ok(())
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        rpass: &mut egui_wgpu_backend::RenderPass,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Option<[f32; 2]> {
        let size = ui.available_size();
        if self.texture.dimensions != (size.x as u32, size.y as u32) {
            self.resize_texture(rpass, device, queue, size.x as u32, size.y as u32);
        }
        let resp = ui.image(self.texture_id, size);
        let input = ui.input();
        let mut mouse_pos = None;
        if let Some(pos) = resp.hover_pos() {
            if input.key_pressed(egui::Key::Space) {
                self.reset_camera(queue);
            }
            if input.pointer.any_pressed() {
                mouse_pos = Some([
                    (pos.x - resp.rect.left()) / resp.rect.width(),
                    1.0 - (pos.y - resp.rect.top()) / resp.rect.height(),
                ]);
            }
            let camera_op = if input.pointer.button_down(egui::PointerButton::Primary) {
                CameraOperation::Rotate
            } else if input.pointer.button_down(egui::PointerButton::Secondary) {
                CameraOperation::Pan
            } else {
                CameraOperation::None
            };
            if input.pointer.is_moving() {
                self.on_pointer_moved(
                    queue,
                    camera_op,
                    (pos.x - resp.rect.left(), pos.y - resp.rect.top()),
                );
            }
            let scroll_delta = ui.input().scroll_delta;
            if scroll_delta.y != 0.0 {
                self.on_zoom(queue, scroll_delta.y);
            }
        }
        mouse_pos
    }

    pub fn render_high_accuracy(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        field_function: String,
    ) {
        let original_shader_src = self.shader_src.clone();
        let high_accuracy_shader_src = self
            .shader_src
            .replace("let h_initial = 0.1;", "let h_initial = 0.001;");
        self.reload_shader(
            device,
            Some(&high_accuracy_shader_src),
            field_function.clone(),
        )
        .unwrap();
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("high_accuracy_encoder"),
        });
        self.render(&mut encoder);
        queue.submit(Some(encoder.finish()));
        self.reload_shader(device, Some(&original_shader_src), field_function.clone())
            .unwrap();
        self.needs_redraw = false; // prevent redraw from shader reload
    }

    pub fn render_outline_rays(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        field_function: String,
    ) {
        let original_shader_src = self.shader_src.clone();
        let high_accuracy_shader_src = self.shader_src.replace(
            "let sample_outline_rays: bool = false;",
            "let sample_outline_rays: bool = true;",
        );
        self.reload_shader(
            device,
            Some(&high_accuracy_shader_src),
            field_function.clone(),
        )
        .unwrap();
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("outline_rays_encoder"),
        });
        self.render(&mut encoder);
        queue.submit(Some(encoder.finish()));
        self.reload_shader(device, Some(&original_shader_src), field_function.clone())
            .unwrap();
        self.needs_redraw = false; // prevent redraw from shader reload
    }

    pub fn render(&mut self, encoder: &mut wgpu::CommandEncoder) {
        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("cpass"),
        });
        let (width, height) = self.texture.dimensions;

        cpass.set_pipeline(&self.compute_pipeline);
        cpass.set_bind_group(0, &self.compute_bind_group, &[]);
        cpass.set_bind_group(1, &self.mesh_bind_group, &[]);
        cpass.set_bind_group(2, &self.ray_samples_bind_group, &[]);
        cpass.dispatch((width + 7) / 8, (height + 7) / 8, 1);

        cpass.set_pipeline(&self.overlay_pipeline);
        cpass.set_bind_group(0, &self.overlay_bind_group, &[]);
        cpass.dispatch((width + 7) / 8, (height + 7) / 8, 1);

        self.needs_redraw = false;
    }
}

fn with_field_function(shader_src: &str, field_function_body: &str) -> String {
    let field_function = format!(
        "fn field_function(p_prev: vec3<f32>, p: vec3<f32>, v0: vec3<f32>, v: vec3<f32>, t: f32) -> vec3<f32> {{\n{}\n}}",
        field_function_body,
    );
    shader_src.replace(
        "fn field_function(p_prev: vec3<f32>, p: vec3<f32>, v0: vec3<f32>, v: vec3<f32>, t: f32) -> vec3<f32> { return v; }",
        &field_function,
    )
}
