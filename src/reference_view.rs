use cgmath::{Vector2, Vector3};
use wgpu::util::DeviceExt;

use crate::{
    application::INITIAL_SIDEBAR_WIDTH,
    arcball::{ArcballCamera, CameraOperation},
    texture::Texture,
};
use std::borrow::Cow;

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    view_projection: [[f32; 4]; 4],
}

pub struct ReferenceView {
    texture: Texture,
    texture_id: egui::TextureId,
    render_pipeline: wgpu::RenderPipeline,
    camera: ArcballCamera<f32>,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    prev_pointer_pos: Option<(f32, f32)>,
    pub needs_redraw: bool,
}

impl ReferenceView {
    pub fn new(
        rpass: &mut egui_wgpu_backend::RenderPass,
        device: &wgpu::Device,
        center: Vector3<f32>,
    ) -> Self {
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

        let mut camera =
            ArcballCamera::new(center, 1.0, [INITIAL_SIDEBAR_WIDTH, INITIAL_SIDEBAR_WIDTH]);
        camera.zoom(-1.0, 1.0);

        let uniforms = Uniforms {
            view_projection: {
                let view = camera.get_mat4();
                let proj = cgmath::perspective(cgmath::Deg(45.0), 1.0, 0.1, 100.0);
                (OPENGL_TO_WGPU_MATRIX * proj * view).into()
            },
        };
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("uniform_buffer"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        });
        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("uniform_bind_group_layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("uniform_bind_group"),
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

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
                bind_group_layouts: &[&uniform_bind_group_layout],
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
            render_pipeline,
            camera,
            uniform_buffer,
            uniform_bind_group,
            prev_pointer_pos: None,
            needs_redraw: true,
        }
    }

    fn update_camera(&mut self, queue: &wgpu::Queue) {
        let uniforms = Uniforms {
            view_projection: {
                let view = self.camera.get_mat4();
                let proj = cgmath::perspective(cgmath::Deg(45.0), 1.0, 0.1, 100.0);
                (OPENGL_TO_WGPU_MATRIX * proj * view).into()
            },
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
        self.needs_redraw = true;
    }

    fn reset_camera(&mut self, queue: &wgpu::Queue) {
        let center = self.camera.center;
        let (width, height) = self.texture.dimensions;
        self.camera = ArcballCamera::new(center, 1.0, [width as f32, height as f32]);
        self.camera.zoom(-1.0, 1.0);
        self.update_camera(queue);
    }

    fn on_zoom(&mut self, queue: &wgpu::Queue, delta: f32) {
        self.camera.zoom(delta, 1.0 / 60.0);
        self.update_camera(queue);
    }

    fn on_pointer_moved(
        &mut self,
        queue: &wgpu::Queue,
        camera_op: CameraOperation,
        pos: (f32, f32),
    ) {
        if self.prev_pointer_pos.is_none() {
            self.prev_pointer_pos = Some(pos);
            return;
        }
        let prev = self.prev_pointer_pos.unwrap();
        match camera_op {
            CameraOperation::Rotate => {
                self.camera.rotate(
                    Vector2::new(prev.0 as f32, prev.1 as f32),
                    Vector2::new(pos.0 as f32, pos.1 as f32),
                );
                self.update_camera(queue);
            }
            CameraOperation::Pan => {
                self.camera.pan(Vector2::new(
                    (pos.0 - prev.0) as f32,
                    (pos.1 - prev.1) as f32,
                ));
                self.update_camera(queue);
            }
            CameraOperation::None => {}
        }
        self.prev_pointer_pos = Some(pos);
    }

    pub fn show(&mut self, ui: &mut egui::Ui, _device: &wgpu::Device, queue: &wgpu::Queue) {
        let resp = ui.image(
            self.texture_id,
            (INITIAL_SIDEBAR_WIDTH, INITIAL_SIDEBAR_WIDTH),
        );
        let input = ui.input();
        if let Some(pos) = resp.hover_pos() {
            if input.key_pressed(egui::Key::Space) {
                self.reset_camera(queue);
            }
            let camera_op = if input.pointer.button_down(egui::PointerButton::Primary) {
                CameraOperation::Rotate
            } else if input.pointer.button_down(egui::PointerButton::Secondary) {
                CameraOperation::Pan
            } else {
                CameraOperation::None
            };
            if input.pointer.is_moving() {
                self.on_pointer_moved(
                    queue,
                    camera_op,
                    (pos.x - resp.rect.left(), pos.y - resp.rect.top()),
                );
            }
            let scroll_delta = ui.input().scroll_delta;
            if scroll_delta.y != 0.0 {
                self.on_zoom(queue, scroll_delta.y);
            }
        }
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
        rpass.set_bind_group(0, &self.uniform_bind_group, &[]);
        rpass.set_vertex_buffer(0, vertices);
        rpass.set_index_buffer(faces, wgpu::IndexFormat::Uint32);
        rpass.draw_indexed(0..(num_faces * 3), 0, 0..1);
    }
}
