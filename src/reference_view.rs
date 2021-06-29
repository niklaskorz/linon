use std::borrow::Cow;

use crate::{application::INITIAL_SIDEBAR_WIDTH, texture::Texture};

pub struct ReferenceView {
    texture: Texture,
    texture_id: egui::TextureId,
    _render_pipeline_layout: wgpu::PipelineLayout,
    render_pipeline: wgpu::RenderPipeline,
}

impl ReferenceView {
    pub fn new(rpass: &mut egui_wgpu_backend::RenderPass, device: &wgpu::Device) -> Self {
        let shader_src = include_str!("reference_view.wgsl");
        let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some("reference_shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(shader_src)),
            #[cfg(not(target_arch = "wasm32"))]
            flags: wgpu::ShaderFlags::all(),
            #[cfg(target_arch = "wasm32")]
            flags: wgpu::ShaderFlags::VALIDATION,
        });

        let texture = Texture::new(
            &device,
            (INITIAL_SIDEBAR_WIDTH as u32, INITIAL_SIDEBAR_WIDTH as u32),
            Some("reference_texture"),
        );
        let texture_id = rpass.egui_texture_from_wgpu_texture(
            device,
            &texture.texture,
            wgpu::FilterMode::Linear,
        );

        let vertex_buffer_layout = wgpu::VertexBufferLayout {
            array_stride: 3 * 32,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x3,
            }],
        };

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("render_pipeline_layout"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("render_pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "main",
                buffers: &[vertex_buffer_layout],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "main",
                targets: &[wgpu::ColorTargetState {
                    format: texture.format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent::REPLACE,
                        alpha: wgpu::BlendComponent::REPLACE,
                    }),
                    write_mask: wgpu::ColorWrite::ALL,
                }],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
        });

        Self {
            texture,
            texture_id,
            _render_pipeline_layout: render_pipeline_layout,
            render_pipeline,
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        ui.image(
            self.texture_id,
            (INITIAL_SIDEBAR_WIDTH, INITIAL_SIDEBAR_WIDTH),
        );
    }

    pub fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        vertices: wgpu::BufferSlice,
        faces: wgpu::BufferSlice,
        num_faces: u32,
    ) {
        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("rpass"),
            color_attachments: &[wgpu::RenderPassColorAttachment {
                view: &self.texture.view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 1.0,
                    }),
                    store: true,
                },
            }],
            depth_stencil_attachment: None,
        });
        rpass.set_pipeline(&self.render_pipeline);
        // rpass.set_bind_group(0, &self.diffuse_bind_group, &[]);
        // rpass.set_bind_group(1, &self.uniform_bind_group, &[]);
        rpass.set_vertex_buffer(0, vertices);
        rpass.set_index_buffer(faces, wgpu::IndexFormat::Uint32);
        rpass.draw_indexed(0..(num_faces * 3), 0, 0..1);
    }
}
