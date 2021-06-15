use std::time::Instant;

use egui::FontDefinitions;
use egui_wgpu_backend::{epi::TextureAllocator, RenderPass, ScreenDescriptor};
use egui_winit_platform::{Platform, PlatformDescriptor};
use wgpu::TextureView;
use winit::{dpi::PhysicalSize, event::Event};

use crate::{arcball::CameraOperation, functions::PredefinedFunction};

pub const INITIAL_SIDEBAR_WIDTH: f32 = 200.0;

pub struct Gui {
    platform: Platform,
    rpass: RenderPass,
    start_time: Instant,
    previous_frame_time: Option<f32>,

    pub shader_error: Option<String>,

    pub rotate_scene: bool,
    pub field_weight: f32,
    pub predefined_function: PredefinedFunction,
    pub field_function: String,

    pub rotate_scene_changed: bool,
    pub field_weight_changed: bool,
    pub field_function_changed: bool,

    pub texture_id: egui::TextureId,
    pub reference_view_texture_id: egui::TextureId,

    pub cursor_pos: Option<(f32, f32)>,
    pub scroll_delta: egui::Vec2,
    pub camera_op: CameraOperation,
}

impl Gui {
    pub fn new(
        size: PhysicalSize<u32>,
        scale_factor: f64,
        device: &wgpu::Device,
        output_format: wgpu::TextureFormat,
        texture: &wgpu::Texture,
        reference_view_texture: &wgpu::Texture,
    ) -> Self {
        let platform = Platform::new(PlatformDescriptor {
            physical_width: size.width,
            physical_height: size.height,
            scale_factor,
            font_definitions: FontDefinitions::default(),
            style: Default::default(),
        });
        let mut rpass = RenderPass::new(device, output_format, 1);

        let texture_id =
            rpass.egui_texture_from_wgpu_texture(device, texture, wgpu::FilterMode::Linear);
        let reference_view_texture_id = rpass.egui_texture_from_wgpu_texture(
            device,
            reference_view_texture,
            wgpu::FilterMode::Linear,
        );

        Self {
            platform,
            rpass,
            start_time: Instant::now(),
            previous_frame_time: None,

            shader_error: None,

            rotate_scene: false,
            field_weight: 0.05,
            predefined_function: PredefinedFunction::Rotation,
            field_function: PredefinedFunction::Rotation.to_code(),

            rotate_scene_changed: false,
            field_weight_changed: false,
            field_function_changed: false,

            texture_id,
            reference_view_texture_id,

            cursor_pos: None,
            scroll_delta: egui::Vec2::new(0.0, 0.0),
            camera_op: CameraOperation::None,
        }
    }

    pub fn handle_event<T>(&mut self, winit_event: &Event<T>) {
        self.platform.handle_event(winit_event)
    }

    pub fn captures_event<T>(&self, winit_event: &Event<T>) -> bool {
        self.platform.captures_event(winit_event)
    }

    pub fn change_texture(&mut self, device: &wgpu::Device, texture: &wgpu::Texture) {
        let texture_id =
            self.rpass
                .egui_texture_from_wgpu_texture(device, texture, wgpu::FilterMode::Linear);
        self.rpass.free(self.texture_id);
        self.texture_id = texture_id;
    }

    fn show(&mut self, ctx: &egui::CtxRef) -> egui::Vec2 {
        let Self {
            shader_error,

            rotate_scene,
            field_weight,
            predefined_function,
            field_function,

            rotate_scene_changed,
            field_weight_changed,
            field_function_changed,

            cursor_pos,
            scroll_delta,
            camera_op,
            ..
        } = self;
        let texture_id = self.texture_id;
        let reference_view_texture_id = self.reference_view_texture_id;
        egui::SidePanel::left("Settings", INITIAL_SIDEBAR_WIDTH).show(ctx, |ui| {
            if ui
                .checkbox(rotate_scene, "Rotate scene instead of camera")
                .changed()
            {
                *rotate_scene_changed = true;
            }
            ui.horizontal(|ui| {
                ui.label("Field weight:");
                if ui.add(egui::Slider::new(field_weight, 0.0..=0.1)).changed() {
                    *field_weight_changed = true;
                }
            });
            egui::ComboBox::from_label("Predefined function")
                .selected_text(predefined_function.to_string())
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_value(
                            predefined_function,
                            PredefinedFunction::Rotation,
                            PredefinedFunction::Rotation.to_string(),
                        )
                        .clicked()
                    {
                        *field_function = PredefinedFunction::Rotation.to_code();
                        *field_function_changed = true;
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
                        *field_function_changed = true;
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
                        *field_function_changed = true;
                    }
                });
            ui.vertical(|ui| {
                ui.label("Custom function:");
                if ui.code_editor(field_function).lost_focus() {
                    *predefined_function = PredefinedFunction::Custom;
                    *field_function_changed = true;
                }
                if let Some(shader_error) = shader_error {
                    ui.label(format!("Shader error: {}", shader_error));
                }
            });
            ui.image(reference_view_texture_id, (200.0, 200.0));
        });
        egui::CentralPanel::default()
            .show(ctx, |ui| {
                let resp = ui.image(texture_id, ui.available_size());
                if let Some(pos) = resp.hover_pos() {
                    *cursor_pos = Some((pos.x - resp.rect.left(), pos.y - resp.rect.top()));
                } else {
                    *cursor_pos = None
                }
                let input = ui.input();
                *camera_op = if input.pointer.button_down(egui::PointerButton::Primary) {
                    CameraOperation::Rotate
                } else if input.pointer.button_down(egui::PointerButton::Secondary) {
                    CameraOperation::Pan
                } else {
                    CameraOperation::None
                };
                *scroll_delta = ui.input().scroll_delta;
                resp.rect.size()
            })
            .inner
    }

    pub fn draw(
        &mut self,
        output_view: &TextureView,
        width: u32,
        height: u32,
        scale_factor: f32,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> (u32, u32) {
        self.platform
            .update_time(self.start_time.elapsed().as_secs_f64());

        // Begin frame
        let start = Instant::now();
        self.platform.begin_frame();

        // Show UI
        let size = self.show(&self.platform.context());

        // End frame
        let (_, paint_commands) = self.platform.end_frame();
        let paint_jobs = self.platform.context().tessellate(paint_commands);

        let frame_time = (Instant::now() - start).as_secs_f32();
        self.previous_frame_time = Some(frame_time);

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("gui_encoder"),
        });

        let screen_descriptor = ScreenDescriptor {
            physical_width: width,
            physical_height: height,
            scale_factor,
        };
        self.rpass
            .update_texture(device, queue, &self.platform.context().texture());
        self.rpass.update_user_textures(device, queue);
        self.rpass
            .update_buffers(device, queue, &paint_jobs, &screen_descriptor);

        self.rpass.execute(
            &mut encoder,
            output_view,
            &paint_jobs,
            &screen_descriptor,
            None,
        );

        queue.submit(Some(encoder.finish()));

        (size.x as u32, size.y as u32)
    }
}
