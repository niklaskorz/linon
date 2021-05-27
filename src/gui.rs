use std::time::Instant;

use egui::{FontDefinitions, Hyperlink, Layout, SidePanel, TopPanel};
use egui_wgpu_backend::{RenderPass, ScreenDescriptor};
use egui_winit_platform::{Platform, PlatformDescriptor};
use epi::{
    backend::{AppOutput, FrameBuilder},
    IntegrationInfo,
};
use wgpu::TextureView;
use winit::{dpi::PhysicalSize, event::Event, event_loop::EventLoop};

/// A custom event type for the winit app.
pub enum GuiEvent {
    RequestRedraw,
}

/// This is the repaint signal type that egui needs for requesting a repaint from another thread.
/// It sends the custom RequestRedraw event to the winit event loop.
struct RepaintSignal(std::sync::Mutex<winit::event_loop::EventLoopProxy<GuiEvent>>);

impl epi::RepaintSignal for RepaintSignal {
    fn request_repaint(&self) {
        self.0
            .lock()
            .unwrap()
            .send_event(GuiEvent::RequestRedraw)
            .ok();
    }
}

pub struct Gui {
    platform: Platform,
    rpass: RenderPass,
    start_time: Instant,
    previous_frame_time: Option<f32>,
    repaint_signal: std::sync::Arc<RepaintSignal>,
    label: String,
}

impl Gui {
    pub fn new(
        size: PhysicalSize<u32>,
        scale_factor: f64,
        device: &wgpu::Device,
        output_format: wgpu::TextureFormat,
        event_loop: &EventLoop<GuiEvent>,
    ) -> Self {
        let platform = Platform::new(PlatformDescriptor {
            physical_width: size.width,
            physical_height: size.height,
            scale_factor,
            font_definitions: FontDefinitions::default(),
            style: Default::default(),
        });
        let rpass = RenderPass::new(device, output_format, 1);
        let repaint_signal = std::sync::Arc::new(RepaintSignal(std::sync::Mutex::new(
            event_loop.create_proxy(),
        )));

        Self {
            platform,
            rpass,
            start_time: Instant::now(),
            previous_frame_time: None,
            repaint_signal,
            label: String::new(),
        }
    }

    pub fn handle_event<T>(&mut self, winit_event: &Event<T>) {
        self.platform.handle_event(winit_event)
    }

    pub fn captures_event<T>(&self, winit_event: &Event<T>) -> bool {
        self.platform.captures_event(winit_event)
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
        let mut app_output = AppOutput::default();
        let mut frame = FrameBuilder {
            info: IntegrationInfo {
                web_info: None,
                cpu_usage: self.previous_frame_time,
                seconds_since_midnight: None,
                native_pixels_per_point: Some(scale_factor),
            },
            tex_allocator: &mut self.rpass,
            output: &mut app_output,
            repaint_signal: self.repaint_signal.clone(),
        }
        .build();

        TopPanel::top("top_panel").show(&self.platform.context(), |ui| {
            egui::menu::bar(ui, |ui| {
                egui::menu::menu(ui, "File", |ui| {
                    if ui.button("Quit").clicked() {
                        frame.quit();
                    }
                });
            });
        });

        let Self { label, .. } = self;

        SidePanel::left("side_panel", 200.0).show(&self.platform.context(), |ui| {
            ui.heading("Side Panel");
            ui.horizontal(|ui| {
                ui.label("Write something: ");
                ui.text_edit_singleline(label);
            });
            ui.with_layout(Layout::bottom_up(egui::Align::Center), |ui| {
                ui.add(Hyperlink::new("https://github.com/niklaskorz/linon").text("linon"));
            });
        });

        // Draw application

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
