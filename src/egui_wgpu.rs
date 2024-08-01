use egui::FullOutput;
use egui_wgpu;

pub struct EguiWgpu {
    pub egui_ctx: egui::Context,
    pub egui_winit: egui_winit::State,
    pub renderer: egui_wgpu::Renderer,
}

impl EguiWgpu {
    pub fn new(
        window: &winit::window::Window,
        device: &wgpu::Device,
        output_format: wgpu::TextureFormat,
    ) -> Self {
        let egui_ctx = egui::Context::default();
        let viewport_id = egui_ctx.viewport_id();
        Self {
            egui_ctx: egui_ctx.clone(),
            egui_winit: egui_winit::State::new(
                egui_ctx,
                viewport_id,
                window,
                Some(window.scale_factor() as f32),
                None,
            ),
            renderer: egui_wgpu::Renderer::new(device, output_format, None, 1, false),
        }
    }

    /// Returns `true` if egui wants exclusive use of this event
    /// (e.g. a mouse click on an egui window, or entering text into a text field).
    /// For instance, if you use egui for a game, you want to first call this
    /// and only when this returns `false` pass on the events to your game.
    ///
    /// Note that egui uses `tab` to move focus between elements, so this will always return `true` for tabs.
    pub fn on_event(
        &mut self,
        window: &winit::window::Window,
        event: &winit::event::WindowEvent,
    ) -> bool {
        self.egui_winit.on_window_event(window, event).consumed
    }

    pub fn begin_frame(&mut self, window: &winit::window::Window) {
        let raw_input = self.egui_winit.take_egui_input(window);
        self.egui_ctx.begin_frame(raw_input);
    }

    /// Returns `needs_repaint` and shapes to draw.
    pub fn end_frame(&mut self, window: &winit::window::Window) -> FullOutput {
        let output = self.egui_ctx.end_frame();
        self.egui_winit
            .handle_platform_output(window, output.platform_output.clone());
        output
    }

    pub fn paint(
        &mut self,
        window: &winit::window::Window,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        color_attachment: &wgpu::TextureView,
        output: FullOutput,
    ) {
        let clipped_meshes = self
            .egui_ctx
            .tessellate(output.shapes, output.pixels_per_point);

        let pixels_per_point = window.scale_factor() as f32;
        self.egui_ctx.set_pixels_per_point(pixels_per_point);
        let size = window.inner_size();
        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [size.width, size.height],
            pixels_per_point,
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
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                label: Some("egui_render"),
                timestamp_writes: None,
                occlusion_query_set: None,
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
