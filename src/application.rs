use std::fmt::Display;

use crate::cornell_box as cbox;
use crate::functions::PredefinedFunction;
use crate::main_view::{MainView, Settings};
use crate::reference_view::ReferenceView;
use crate::vertices::{get_center, normalize_vertices};
use anyhow::{Context, Result};
use wgpu::util::DeviceExt;
use winit::window::Window;

pub const INITIAL_SIDEBAR_WIDTH: f32 = 500.0;

#[derive(Debug, PartialEq, Clone, Copy)]
enum OverlayMode {
    Disabled = 0,
    LyapunovExponents = 1,
}

impl Display for OverlayMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            Self::Disabled => "Disabled",
            Self::LyapunovExponents => "Lyapunov exponents",
        };
        write!(f, "{}", text)
    }
}

pub struct Application {
    _instance: wgpu::Instance,
    surface_config: wgpu::SurfaceConfiguration,
    surface: wgpu::Surface,
    _adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    main_view: MainView,
    reference_view: ReferenceView,
    vertices_buffer: wgpu::Buffer,
    faces_buffer: wgpu::Buffer,
    indices: u32,
    ray_samples_buffer: wgpu::Buffer,
    // egui
    platform: egui_winit_platform::Platform,
    rpass: egui_wgpu_backend::RenderPass,
    #[cfg(not(target_arch = "wasm32"))]
    start_time: std::time::Instant,
    previous_frame_time: Option<f32>,
    // gui state
    shader_error: Option<String>,
    field_weight: f32,
    mouse_pos: [f32; 2],
    overlay_mode: OverlayMode,
    central_difference_delta: i32,
    lyapunov_scaling: f32,
    predefined_function: PredefinedFunction,
    field_function: String,
}

impl Application {
    pub async fn new(window: &Window) -> Result<Self> {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::Backends::PRIMARY);
        let surface = unsafe { instance.create_surface(window) };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await
            .context("no compatible adapter found")?;
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: wgpu::Features::default(),
                    limits: wgpu::Limits::default(),
                },
                None,
            )
            .await?;

        let surface_format = surface
            .get_preferred_format(&adapter)
            .context("no compatible surface format found")?;
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };
        surface.configure(&device, &surface_config);

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
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX,
        });
        let indices = cbox::INDICES;
        let faces_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("faces_buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::INDEX,
        });
        let center = get_center(&vertices);

        let ray_samples_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ray_samples_buffer"),
            size: std::mem::size_of::<[[[[f32; 4]; 2]; 100]; 8]>() as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX,
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
        let mut rpass = egui_wgpu_backend::RenderPass::new(&device, surface_format, 1);

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
            surface_config,
            surface,
            _adapter: adapter,
            device,
            queue,
            main_view,
            reference_view,
            vertices_buffer,
            faces_buffer,
            indices: indices.len() as u32,
            ray_samples_buffer,
            // egui
            platform,
            rpass,
            #[cfg(not(target_arch = "wasm32"))]
            start_time: std::time::Instant::now(),
            previous_frame_time: None,
            // gui state
            shader_error: None,
            field_weight: 1.0,
            mouse_pos: [0.5, 0.5],
            overlay_mode: OverlayMode::Disabled,
            central_difference_delta: 1,
            lyapunov_scaling: 50.0,
            predefined_function: PredefinedFunction::MirageSphericalSigmoid,
            field_function: PredefinedFunction::MirageSphericalSigmoid.to_code(),
        })
    }

    pub fn handle_event<T>(&mut self, winit_event: &winit::event::Event<T>) {
        self.platform.handle_event(winit_event)
    }

    pub fn captures_event<T>(&self, winit_event: &winit::event::Event<T>) -> bool {
        self.platform.captures_event(winit_event)
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.surface_config.width = width;
        self.surface_config.height = height;
        self.surface.configure(&self.device, &self.surface_config);
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
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX,
            });
        self.faces_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("faces_buffer"),
                contents: bytemuck::cast_slice(indices),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::INDEX,
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
            mouse_pos,
            overlay_mode,
            central_difference_delta,
            lyapunov_scaling,
            field_function,
            predefined_function,
            ..
        } = self;
        let mut field_function_changed = false;
        let device = &self.device;
        let queue = &self.queue;
        egui::SidePanel::left("Settings").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if egui::ComboBox::from_label("Overlay")
                    .selected_text(*overlay_mode)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            overlay_mode,
                            OverlayMode::Disabled,
                            OverlayMode::Disabled,
                        )
                        .clicked()
                            || ui
                                .selectable_value(
                                    overlay_mode,
                                    OverlayMode::LyapunovExponents,
                                    OverlayMode::LyapunovExponents,
                                )
                                .clicked()
                    })
                    .inner
                    .unwrap_or(false)
                {
                    main_view.update_settings(
                        queue,
                        Settings {
                            field_weight: *field_weight,
                            mouse_pos: *mouse_pos,
                            overlay_mode: *overlay_mode as i32,
                            central_difference_delta: *central_difference_delta,
                            lyapunov_scaling: *lyapunov_scaling,
                        },
                    );
                }
                if ui.button("Enhance").clicked() {
                    main_view.render_high_accuracy(device, queue, field_function.clone());
                }
                if ui.button("Outline").clicked() {
                    main_view.render_outline_rays(device, queue, field_function.clone());
                }
            });
            if *overlay_mode != OverlayMode::Disabled {
                ui.horizontal(|ui| {
                    ui.label("Central difference delta:");
                    if ui
                        .add(egui::Slider::new(central_difference_delta, 1..=10))
                        .changed()
                    {
                        main_view.update_settings(
                            queue,
                            Settings {
                                field_weight: *field_weight,
                                mouse_pos: *mouse_pos,
                                overlay_mode: *overlay_mode as i32,
                                central_difference_delta: *central_difference_delta,
                                lyapunov_scaling: *lyapunov_scaling,
                            },
                        );
                    }
                });
            }
            if *overlay_mode == OverlayMode::LyapunovExponents {
                ui.horizontal(|ui| {
                    ui.label("Lyapunov scaling:");
                    if ui
                        .add(egui::Slider::new(lyapunov_scaling, 1.0..=100.0))
                        .changed()
                    {
                        main_view.update_settings(
                            queue,
                            Settings {
                                field_weight: *field_weight,
                                mouse_pos: *mouse_pos,
                                overlay_mode: *overlay_mode as i32,
                                central_difference_delta: *central_difference_delta,
                                lyapunov_scaling: *lyapunov_scaling,
                            },
                        );
                    }
                });
            }
            ui.horizontal(|ui| {
                ui.label("Field weight:");
                if ui.add(egui::Slider::new(field_weight, 0.0..=1.0)).changed() {
                    main_view.update_settings(
                        queue,
                        Settings {
                            field_weight: *field_weight,
                            mouse_pos: *mouse_pos,
                            overlay_mode: *overlay_mode as i32,
                            central_difference_delta: *central_difference_delta,
                            lyapunov_scaling: *lyapunov_scaling,
                        },
                    );
                }
            });
            egui::ComboBox::from_label("Predefined function")
                .selected_text(predefined_function.to_string())
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_value(
                            predefined_function,
                            PredefinedFunction::MirageSpherical,
                            PredefinedFunction::MirageSpherical.to_string(),
                        )
                        .clicked()
                    {
                        *field_function = PredefinedFunction::MirageSpherical.to_code();
                        field_function_changed = true;
                    }
                    if ui
                        .selectable_value(
                            predefined_function,
                            PredefinedFunction::MiragePlane,
                            PredefinedFunction::MiragePlane.to_string(),
                        )
                        .clicked()
                    {
                        *field_function = PredefinedFunction::MiragePlane.to_code();
                        field_function_changed = true;
                    }
                    if ui
                        .selectable_value(
                            predefined_function,
                            PredefinedFunction::MirageSphericalSigmoid,
                            PredefinedFunction::MirageSphericalSigmoid.to_string(),
                        )
                        .clicked()
                    {
                        *field_function = PredefinedFunction::MirageSphericalSigmoid.to_code();
                        field_function_changed = true;
                    }
                    if ui
                        .selectable_value(
                            predefined_function,
                            PredefinedFunction::MiragePlaneSigmoid,
                            PredefinedFunction::MiragePlaneSigmoid.to_string(),
                        )
                        .clicked()
                    {
                        *field_function = PredefinedFunction::MiragePlaneSigmoid.to_code();
                        field_function_changed = true;
                    }
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
            reference_view.show(ui, device, queue);
        });
        let device = &self.device;
        let queue = &self.queue;
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(new_pos) = main_view.show(ui, rpass, device, queue) {
                *mouse_pos = new_pos;
                main_view.update_settings(
                    queue,
                    Settings {
                        field_weight: *field_weight,
                        mouse_pos: *mouse_pos,
                        overlay_mode: *overlay_mode as i32,
                        central_difference_delta: *central_difference_delta,
                        lyapunov_scaling: *lyapunov_scaling,
                    },
                );
            }
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

    pub fn render(&mut self, scale_factor: f32) -> Result<(), wgpu::SurfaceError> {
        let frame = self.surface.get_current_frame()?.output;
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        #[cfg(not(target_arch = "wasm32"))]
        self.platform
            .update_time(self.start_time.elapsed().as_secs_f64());

        // Begin frame
        #[cfg(not(target_arch = "wasm32"))]
        let start = std::time::Instant::now();
        self.platform.begin_frame();

        // Show UI
        self.show();

        // End frame
        let (_, paint_commands) = self.platform.end_frame(None);
        let paint_jobs = self.platform.context().tessellate(paint_commands);

        #[cfg(not(target_arch = "wasm32"))]
        {
            let frame_time = (std::time::Instant::now() - start).as_secs_f32();
            self.previous_frame_time = Some(frame_time);
        }

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
            physical_width: self.surface_config.width,
            physical_height: self.surface_config.height,
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

        self.rpass
            .execute(&mut encoder, &view, &paint_jobs, &screen_descriptor, None)
            .unwrap();

        self.queue.submit(Some(encoder.finish()));

        Ok(())
    }
}
