use crate::cornell_box as cbox;
use crate::functions::PredefinedFunction;
use crate::main_view::MainView;
use crate::reference_view::ReferenceView;
use crate::vertices::{get_center, normalize_vertices};
use anyhow::{Context, Result};
use wgpu::util::DeviceExt;
use winit::window::Window;

pub const INITIAL_SIDEBAR_WIDTH: f32 = 500.0;

pub struct Application {
    _instance: wgpu::Instance,
    surface: wgpu::Surface,
    _adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    sc_desc: wgpu::SwapChainDescriptor,
    swap_chain: wgpu::SwapChain,
    main_view: MainView,
    reference_view: ReferenceView,
    vertices_buffer: wgpu::Buffer,
    faces_buffer: wgpu::Buffer,
    indices: u32,
    ray_samples_buffer: wgpu::Buffer,
    // egui
    platform: egui_winit_platform::Platform,
    rpass: egui_wgpu_backend::RenderPass,
    start_time: std::time::Instant,
    previous_frame_time: Option<f32>,
    // gui state
    shader_error: Option<String>,
    field_weight: f32,
    predefined_function: PredefinedFunction,
    field_function: String,
    wireframe: bool,
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
                    features: wgpu::Features::NON_FILL_POLYGON_MODE,
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

        let mut vertices = cbox::VERTICES;
        for i in 0..(vertices.len() / 3) {
            // Invert x and z axis
            vertices[3 * i] = -vertices[3 * i];
            vertices[3 * i + 2] = -vertices[3 * i + 2];
        }
        normalize_vertices(&mut vertices);
        let vertices_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("vertices_buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsage::STORAGE | wgpu::BufferUsage::VERTEX,
        });
        let indices = cbox::INDICES;
        let faces_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("faces_buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsage::STORAGE | wgpu::BufferUsage::INDEX,
        });
        let center = get_center(&vertices);

        let ray_samples_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ray_samples_buffer"),
            size: std::mem::size_of::<[[[[f32; 4]; 2]; 100]; 8]>() as u64,
            usage: wgpu::BufferUsage::STORAGE | wgpu::BufferUsage::VERTEX,
            mapped_at_creation: false,
        });

        let platform =
            egui_winit_platform::Platform::new(egui_winit_platform::PlatformDescriptor {
                physical_width: size.width,
                physical_height: size.height,
                scale_factor: window.scale_factor(),
                font_definitions: egui::FontDefinitions::default(),
                style: Default::default(),
            });
        let mut rpass = egui_wgpu_backend::RenderPass::new(&device, swapchain_format, 1);

        let main_view = MainView::new(
            &mut rpass,
            &device,
            vertices_buffer.as_entire_binding(),
            faces_buffer.as_entire_binding(),
            center,
            ray_samples_buffer.as_entire_binding(),
            size.width - INITIAL_SIDEBAR_WIDTH as u32,
            size.height,
        );
        let reference_view = ReferenceView::new(
            &mut rpass,
            &device,
            vertices_buffer.as_entire_binding(),
            faces_buffer.as_entire_binding(),
            center,
        );

        Ok(Self {
            _instance: instance,
            surface,
            _adapter: adapter,
            device,
            queue,
            sc_desc,
            swap_chain,
            main_view,
            reference_view,
            vertices_buffer,
            faces_buffer,
            indices: indices.len() as u32,
            ray_samples_buffer,
            // egui
            platform,
            rpass,
            start_time: std::time::Instant::now(),
            previous_frame_time: None,
            // gui state
            shader_error: None,
            field_weight: 0.001,
            predefined_function: PredefinedFunction::TranslationX,
            field_function: PredefinedFunction::TranslationX.to_code(),
            wireframe: false,
        })
    }

    pub fn handle_event<T>(&mut self, winit_event: &winit::event::Event<T>) {
        self.platform.handle_event(winit_event)
    }

    pub fn captures_event<T>(&self, winit_event: &winit::event::Event<T>) -> bool {
        self.platform.captures_event(winit_event)
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.sc_desc.width = width;
        self.sc_desc.height = height;
        self.swap_chain = self.device.create_swap_chain(&self.surface, &self.sc_desc);
    }

    pub fn load_default_model(&mut self) {
        let mut vertices = cbox::VERTICES;
        self.load_model(&mut vertices, &cbox::INDICES);
    }

    pub fn load_model(&mut self, vertices: &mut [f32], indices: &[u32]) {
        normalize_vertices(vertices);

        self.vertices_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("vertices_buffer"),
                contents: bytemuck::cast_slice(vertices),
                usage: wgpu::BufferUsage::STORAGE | wgpu::BufferUsage::VERTEX,
            });
        self.faces_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("faces_buffer"),
                contents: bytemuck::cast_slice(indices),
                usage: wgpu::BufferUsage::STORAGE | wgpu::BufferUsage::INDEX,
            });
        self.indices = indices.len() as u32;
        let center = get_center(vertices);

        self.main_view.update_model(
            &self.device,
            &self.queue,
            self.vertices_buffer.as_entire_binding(),
            self.faces_buffer.as_entire_binding(),
            center,
        );
    }

    pub fn reload_compute_shader(&mut self, new_src: &str) -> Result<(), wgpu::Error> {
        self.main_view
            .reload_shader(&self.device, Some(new_src), self.field_function.clone())
    }

    fn show(&mut self) {
        let ctx = &self.platform.context();
        let Self {
            main_view,
            reference_view,
            rpass,
            shader_error,
            field_weight,
            field_function,
            predefined_function,
            wireframe,
            ..
        } = self;
        let mut field_function_changed = false;
        let device = &self.device;
        let queue = &self.queue;
        egui::SidePanel::left("Settings", INITIAL_SIDEBAR_WIDTH).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Field weight:");
                if ui.add(egui::Slider::new(field_weight, 0.0..=1.0)).changed() {
                    main_view.update_settings(queue, *field_weight);
                }
            });
            egui::ComboBox::from_label("Predefined function")
                .selected_text(predefined_function.to_string())
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_value(
                            predefined_function,
                            PredefinedFunction::TranslationX,
                            PredefinedFunction::TranslationX.to_string(),
                        )
                        .clicked()
                    {
                        *field_function = PredefinedFunction::TranslationX.to_code();
                        field_function_changed = true;
                    }
                    if ui
                        .selectable_value(
                            predefined_function,
                            PredefinedFunction::TranslationZ,
                            PredefinedFunction::TranslationZ.to_string(),
                        )
                        .clicked()
                    {
                        *field_function = PredefinedFunction::TranslationZ.to_code();
                        field_function_changed = true;
                    }
                    if ui
                        .selectable_value(
                            predefined_function,
                            PredefinedFunction::Rotation,
                            PredefinedFunction::Rotation.to_string(),
                        )
                        .clicked()
                    {
                        *field_function = PredefinedFunction::Rotation.to_code();
                        field_function_changed = true;
                    }
                    if ui
                        .selectable_value(
                            predefined_function,
                            PredefinedFunction::LorenzAttractor,
                            PredefinedFunction::LorenzAttractor.to_string(),
                        )
                        .clicked()
                    {
                        *field_function = PredefinedFunction::LorenzAttractor.to_code();
                        field_function_changed = true;
                    }
                    if ui
                        .selectable_value(
                            predefined_function,
                            PredefinedFunction::RoesslerAttractor,
                            PredefinedFunction::RoesslerAttractor.to_string(),
                        )
                        .clicked()
                    {
                        *field_function = PredefinedFunction::RoesslerAttractor.to_code();
                        field_function_changed = true;
                    }
                });
            ui.vertical(|ui| {
                ui.label("Custom function:");
                if ui.code_editor(field_function).lost_focus() {
                    *predefined_function = PredefinedFunction::Custom;
                    field_function_changed = true;
                }
                if let Some(shader_error) = shader_error {
                    ui.label(format!("Shader error: {}", shader_error));
                }
            });
            ui.horizontal(|ui| {
                if ui.checkbox(wireframe, "Ray outline as wireframe").changed() {
                    reference_view.update_sample_pipeline(device, *wireframe);
                }
            });
            reference_view.show(ui, device, queue);
        });
        let device = &self.device;
        let queue = &self.queue;
        egui::CentralPanel::default().show(ctx, |ui| {
            main_view.show(ui, rpass, device, queue);
        });
        if field_function_changed {
            if let Err(e) = self
                .main_view
                .reload_shader(device, None, field_function.clone())
            {
                self.shader_error = Some(e.to_string());
            } else {
                self.shader_error = None;
            }
        }
    }

    pub fn render(&mut self, scale_factor: f32) -> Result<(), wgpu::SwapChainError> {
        let frame = self.swap_chain.get_current_frame()?.output;

        self.platform
            .update_time(self.start_time.elapsed().as_secs_f64());

        // Begin frame
        let start = std::time::Instant::now();
        self.platform.begin_frame();

        // Show UI
        self.show();

        // End frame
        let (_, paint_commands) = self.platform.end_frame();
        let paint_jobs = self.platform.context().tessellate(paint_commands);

        let frame_time = (std::time::Instant::now() - start).as_secs_f32();
        self.previous_frame_time = Some(frame_time);

        if self.main_view.needs_redraw || self.reference_view.needs_redraw {
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("encoder"),
                });
            if self.main_view.needs_redraw {
                encoder.push_debug_group("render main view");
                self.main_view.render(&mut encoder);
                encoder.pop_debug_group();
            }
            encoder.push_debug_group("render reference view");
            self.reference_view.render(
                &mut encoder,
                self.indices,
                self.ray_samples_buffer.slice(..),
            );
            encoder.pop_debug_group();
            self.queue.submit(Some(encoder.finish()));
        }

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("gui_encoder"),
            });

        let screen_descriptor = egui_wgpu_backend::ScreenDescriptor {
            physical_width: self.sc_desc.width,
            physical_height: self.sc_desc.height,
            scale_factor,
        };
        self.rpass.update_texture(
            &self.device,
            &self.queue,
            &self.platform.context().texture(),
        );
        self.rpass.update_user_textures(&self.device, &self.queue);
        self.rpass
            .update_buffers(&self.device, &self.queue, &paint_jobs, &screen_descriptor);

        self.rpass.execute(
            &mut encoder,
            &frame.view,
            &paint_jobs,
            &screen_descriptor,
            None,
        );

        self.queue.submit(Some(encoder.finish()));

        Ok(())
    }
}
