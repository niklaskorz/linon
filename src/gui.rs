use std::time::Instant;

use egui::FontDefinitions;
use egui_wgpu_backend::{RenderPass, ScreenDescriptor};
use egui_winit_platform::{Platform, PlatformDescriptor};
use wgpu::TextureView;
use winit::{dpi::PhysicalSize, event::Event};

use crate::functions::PredefinedFunction;

pub struct Gui {
    platform: Platform,
    rpass: RenderPass,
    start_time: Instant,
    previous_frame_time: Option<f32>,
    field_strength: f32,
    predefined_function: PredefinedFunction,
    field_function: String,
}

impl Gui {
    pub fn new(
        size: PhysicalSize<u32>,
        scale_factor: f64,
        device: &wgpu::Device,
        output_format: wgpu::TextureFormat,
    ) -> Self {
        let platform = Platform::new(PlatformDescriptor {
            physical_width: size.width,
            physical_height: size.height,
            scale_factor,
            font_definitions: FontDefinitions::default(),
            style: Default::default(),
        });
        let rpass = RenderPass::new(device, output_format, 1);

        Self {
            platform,
            rpass,
            start_time: Instant::now(),
            previous_frame_time: None,
            field_strength: 0.05,
            predefined_function: PredefinedFunction::LorenzAttractor,
            field_function: PredefinedFunction::LorenzAttractor.to_code(),
        }
    }

    pub fn handle_event<T>(&mut self, winit_event: &Event<T>) {
        self.platform.handle_event(winit_event)
    }

    pub fn captures_event<T>(&self, winit_event: &Event<T>) -> bool {
        self.platform.captures_event(winit_event)
    }

    fn show(&mut self, ctx: &egui::CtxRef) {
        let Self {
            field_strength,
            predefined_function,
            field_function,
            ..
        } = self;
        egui::Window::new("Settings").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Field strength:");
                ui.add(egui::Slider::new(field_strength, 0.0..=1.0));
            });
            egui::ComboBox::from_label("Predefined function")
                .selected_text(predefined_function.to_string())
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_value(
                            predefined_function,
                            PredefinedFunction::LorenzAttractor,
                            PredefinedFunction::LorenzAttractor.to_string(),
                        )
                        .clicked()
                    {
                        *field_function = PredefinedFunction::LorenzAttractor.to_code();
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
                    }
                });
            ui.vertical(|ui| {
                ui.label("Custom function:");
                if ui.code_editor(field_function).changed() {
                    *predefined_function = PredefinedFunction::Custom;
                }
            });
        });
    }

    pub fn draw(
        &mut self,
        output_view: &TextureView,
        width: u32,
        height: u32,
        scale_factor: f32,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        self.platform
            .update_time(self.start_time.elapsed().as_secs_f64());

        // Begin frame
        let start = Instant::now();
        self.platform.begin_frame();

        // Show UI
        self.show(&self.platform.context());

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
    }
}
