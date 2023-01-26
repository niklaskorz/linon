use std::time::Duration;

use egui::FullOutput;
use egui_wgpu::renderer as egui_wgpu_backend;
use winit::event_loop::EventLoop;

pub struct EguiWgpu {
    pub egui_ctx: egui::Context,
    pub egui_winit: egui_winit::State,
    pub renderer: egui_wgpu_backend::Renderer,
}

impl EguiWgpu {
    pub fn new(
        event_loop: &EventLoop<()>,
        device: &wgpu::Device,
        output_format: wgpu::TextureFormat,
    ) -> Self {
        Self {
            egui_ctx: Default::default(),
            egui_winit: egui_winit::State::new(event_loop),
            renderer: egui_wgpu_backend::Renderer::new(device, output_format, None, 1),
        }
    }

    /// Returns `true` if egui wants exclusive use of this event
    /// (e.g. a mouse click on an egui window, or entering text into a text field).
    /// For instance, if you use egui for a game, you want to first call this
    /// and only when this returns `false` pass on the events to your game.
    ///
    /// Note that egui uses `tab` to move focus between elements, so this will always return `true` for tabs.
    pub fn on_event(&mut self, event: &winit::event::WindowEvent<'_>) -> bool {
        self.egui_winit.on_event(&self.egui_ctx, event).consumed
    }

    pub fn begin_frame(&mut self, window: &winit::window::Window) {
        let raw_input = self.egui_winit.take_egui_input(window);
        self.egui_ctx.begin_frame(raw_input);
    }

    /// Returns `needs_repaint` and shapes to draw.
    pub fn end_frame(&mut self, window: &winit::window::Window) -> (bool, FullOutput) {
        let output = self.egui_ctx.end_frame();
        let needs_repaint = Duration::is_zero(&output.repaint_after);
        self.egui_winit.handle_platform_output(
            window,
            &self.egui_ctx,
            output.platform_output.clone(),
        );
        (needs_repaint, output)
    }

    pub fn paint(
        &mut self,
        window: &winit::window::Window,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        color_attachment: &wgpu::TextureView,
        output: FullOutput,
    ) {
        let clipped_meshes = self.egui_ctx.tessellate(output.shapes);

        self.egui_ctx.set_pixels_per_point(window.scale_factor() as f32);
        let size = window.inner_size();
        let screen_descriptor = egui_wgpu_backend::ScreenDescriptor {
            size_in_pixels: [size.width, size.height],
            pixels_per_point: self.egui_ctx.pixels_per_point(),
        };

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("egui_wgpu_encoder"),
        });

        for (id, delta) in output.textures_delta.set {
            self.renderer.update_texture(device, queue, id, &delta);
        }

        self.renderer.update_buffers(
            device,
            queue,
            &mut encoder,
            &clipped_meshes,
            &screen_descriptor,
        );

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &color_attachment,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
                label: Some("egui_render"),
            });
            self.renderer
                .render(&mut render_pass, &clipped_meshes, &screen_descriptor);
        }

        for id in output.textures_delta.free {
            self.renderer.free_texture(&id);
        }

        queue.submit(Some(encoder.finish()));
    }
}
