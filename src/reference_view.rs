use std::borrow::Cow;

use crate::{gui::INITIAL_SIDEBAR_WIDTH, texture::Texture};

pub struct ReferenceView {
    pub texture: Texture,
    render_pipeline_layout: wgpu::PipelineLayout,
    render_pipeline: wgpu::RenderPipeline,
}

impl ReferenceView {
    pub fn new(device: &wgpu::Device) -> Self {
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
                buffers: &[],
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
            render_pipeline_layout,
            render_pipeline,
        }
    }

    pub fn render(&mut self, encoder: &mut wgpu::CommandEncoder) {
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
        // rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        // rpass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        // rpass.draw_indexed(0..self.num_indices, 0, 0..1);
    }
}
