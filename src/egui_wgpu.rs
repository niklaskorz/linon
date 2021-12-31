pub struct EguiWgpu {
    pub egui_ctx: egui::CtxRef,
    pub egui_winit: egui_winit::State,
    pub render_pass: egui_wgpu_backend::RenderPass,
}

impl EguiWgpu {
    pub fn new(
        window: &winit::window::Window,
        device: &wgpu::Device,
        output_format: wgpu::TextureFormat,
    ) -> Self {
        Self {
            egui_ctx: Default::default(),
            egui_winit: egui_winit::State::new(window),
            render_pass: egui_wgpu_backend::RenderPass::new(device, output_format, 1),
        }
    }

    /// Returns `true` if egui wants exclusive use of this event
    /// (e.g. a mouse click on an egui window, or entering text into a text field).
    /// For instance, if you use egui for a game, you want to first call this
    /// and only when this returns `false` pass on the events to your game.
    ///
    /// Note that egui uses `tab` to move focus between elements, so this will always return `true` for tabs.
    pub fn on_event(&mut self, event: &winit::event::WindowEvent<'_>) -> bool {
        self.egui_winit.on_event(&self.egui_ctx, event)
    }

    pub fn begin_frame(&mut self, window: &winit::window::Window) {
        let raw_input = self.egui_winit.take_egui_input(window);
        self.egui_ctx.begin_frame(raw_input);
    }

    /// Returns `needs_repaint` and shapes to draw.
    pub fn end_frame(
        &mut self,
        window: &winit::window::Window,
    ) -> (bool, Vec<egui::epaint::ClippedShape>) {
        let (egui_output, shapes) = self.egui_ctx.end_frame();
        let needs_repaint = egui_output.needs_repaint;
        self.egui_winit
            .handle_output(window, &self.egui_ctx, egui_output);
        (needs_repaint, shapes)
    }

    pub fn paint(
        &mut self,
        window: &winit::window::Window,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        color_attachment: &wgpu::TextureView,
        shapes: Vec<egui::epaint::ClippedShape>,
    ) {
        let clipped_meshes = self.egui_ctx.tessellate(shapes);

        let size = window.inner_size();
        let screen_descriptor = egui_wgpu_backend::ScreenDescriptor {
            physical_width: size.width,
            physical_height: size.height,
            scale_factor: self.egui_ctx.pixels_per_point(),
        };

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("egui_wgpu_encoder"),
        });

        self.render_pass
            .update_texture(device, queue, &self.egui_ctx.font_image());
        self.render_pass.update_user_textures(device, queue);
        self.render_pass
            .update_buffers(device, queue, &clipped_meshes, &screen_descriptor);

        self.render_pass
            .execute(
                &mut encoder,
                color_attachment,
                &clipped_meshes,
                &screen_descriptor,
                None,
            )
            .unwrap();

        queue.submit(Some(encoder.finish()));
    }
}
