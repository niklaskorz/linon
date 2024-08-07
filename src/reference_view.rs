use cgmath::{Vector2, Vector3};
use egui::{load::SizedTexture, Image, ImageSource, Sense, Widget};
use egui_wgpu as egui_wgpu_backend;
use wgpu::util::DeviceExt;

use crate::{
    application::INITIAL_SIDEBAR_WIDTH,
    arcball::{ArcballCamera, CameraOperation},
    ray_samples::{create_indices, vertex_desc},
    texture::{DepthTexture, Texture},
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
    camera_pos: [f32; 4],
    view_projection: [[f32; 4]; 4],
}

pub struct ReferenceView {
    texture: Texture,
    texture_id: egui::TextureId,
    depth_texture: DepthTexture,
    render_pipeline: wgpu::RenderPipeline,
    camera: ArcballCamera<f32>,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    mesh_bind_group: wgpu::BindGroup,

    sample_index_buffer: wgpu::Buffer,
    sample_num_indices: u32,
    sample_render_pipeline: wgpu::RenderPipeline,

    prev_pointer_pos: Option<(f32, f32)>,
    pub needs_redraw: bool,
}

impl ReferenceView {
    pub fn new(
        rpass: &mut egui_wgpu_backend::Renderer,
        device: &wgpu::Device,
        vertices_buffer_binding: wgpu::BindingResource,
        faces_buffer_binding: wgpu::BindingResource,
        center: Vector3<f32>,
    ) -> Self {
        let shader_src = include_str!("reference_view.wgsl");
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("reference_shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(shader_src)),
        });

        let dimensions = (INITIAL_SIDEBAR_WIDTH as u32, INITIAL_SIDEBAR_WIDTH as u32);

        let texture = Texture::new(
            device,
            dimensions,
            Some("reference_texture"),
            wgpu::TextureFormat::Rgba8UnormSrgb,
            false,
        );
        let texture_id =
            rpass.register_native_texture(device, &texture.view, wgpu::FilterMode::Linear);

        let depth_texture = DepthTexture::new(device, dimensions, Some("depth_texture"));

        let mut camera =
            ArcballCamera::new(center, 1.0, [INITIAL_SIDEBAR_WIDTH, INITIAL_SIDEBAR_WIDTH]);
        camera.zoom(-1.0, 1.0);
        let eye_pos = camera.eye_pos();

        let uniforms = Uniforms {
            camera_pos: [eye_pos.x, eye_pos.y, eye_pos.z, 0.0],
            view_projection: {
                let view = camera.get_mat4();
                let proj = cgmath::perspective(cgmath::Deg(45.0), 1.0, 0.1, 100.0);
                (OPENGL_TO_WGPU_MATRIX * proj * view).into()
            },
        };
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("uniform_buffer"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("uniform_bind_group_layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
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

        let mesh_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
                label: Some("mesh_bind_group_layout"),
            });
        let mesh_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &mesh_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: vertices_buffer_binding,
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: faces_buffer_binding,
                },
            ],
            label: Some("mesh_bind_group"),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("render_pipeline_layout"),
                bind_group_layouts: &[&uniform_bind_group_layout, &mesh_bind_group_layout],
                push_constant_ranges: &[],
            });
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("render_pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "main_vertex",
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                #[cfg(not(target_arch = "wasm32"))]
                entry_point: "main_fragment",
                #[cfg(target_arch = "wasm32")]
                entry_point: "main_fragment_web",
                targets: &[Some(wgpu::ColorTargetState {
                    format: texture.format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent::REPLACE,
                        alpha: wgpu::BlendComponent::REPLACE,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Cw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let sample_indices = create_indices();
        let sample_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("sample_index_buffer"),
            contents: bytemuck::cast_slice(&sample_indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let sample_render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("sample_render_pipeline_layout"),
                bind_group_layouts: &[&uniform_bind_group_layout],
                push_constant_ranges: &[],
            });
        let sample_render_pipeline = create_sample_render_pipeline(
            device,
            &sample_render_pipeline_layout,
            &shader,
            texture.format,
            false,
        );

        Self {
            texture,
            texture_id,
            depth_texture,
            render_pipeline,
            camera,
            uniform_buffer,
            uniform_bind_group,
            mesh_bind_group,
            prev_pointer_pos: None,

            sample_index_buffer,
            sample_num_indices: sample_indices.len() as u32 * 3,
            sample_render_pipeline,

            needs_redraw: true,
        }
    }

    fn update_camera(&mut self, queue: &wgpu::Queue) {
        let eye_pos = self.camera.eye_pos();
        let uniforms = Uniforms {
            camera_pos: [eye_pos.x, eye_pos.y, eye_pos.z, 0.0],
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
        #[cfg(not(target_arch = "wasm32"))]
        self.camera.zoom(delta, 1.0 / 60.0);
        #[cfg(target_arch = "wasm32")]
        self.camera.zoom(delta / 10.0, 1.0 / 60.0);
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
                self.camera
                    .rotate(Vector2::new(prev.0, prev.1), Vector2::new(pos.0, pos.1));
                self.update_camera(queue);
            }
            CameraOperation::Pan => {
                self.camera
                    .pan(Vector2::new(pos.0 - prev.0, pos.1 - prev.1));
                self.update_camera(queue);
            }
            CameraOperation::None => {}
        }
        self.prev_pointer_pos = Some(pos);
    }

    pub fn show(&mut self, ui: &mut egui::Ui, _device: &wgpu::Device, queue: &wgpu::Queue) {
        let resp = Image::new(ImageSource::Texture(SizedTexture::new(
            self.texture_id,
            (INITIAL_SIDEBAR_WIDTH, INITIAL_SIDEBAR_WIDTH),
        )))
        .sense(Sense::drag())
        .ui(ui);
        if resp.contains_pointer() {
            if ui.input(|i| i.key_pressed(egui::Key::Space)) {
                self.reset_camera(queue);
            }
            let scroll_delta = ui.input(|i| i.smooth_scroll_delta);
            if scroll_delta.y != 0.0 {
                self.on_zoom(queue, scroll_delta.y);
            }
        }
        if resp.dragged() {
            let Some(pos) = resp.interact_pointer_pos() else {
                return;
            };
            let camera_op = if resp.dragged_by(egui::PointerButton::Primary) {
                CameraOperation::Rotate
            } else if resp.dragged_by(egui::PointerButton::Secondary) {
                CameraOperation::Pan
            } else {
                CameraOperation::None
            };
            self.on_pointer_moved(
                queue,
                camera_op,
                (pos.x - resp.rect.left(), pos.y - resp.rect.top()),
            );
        } else {
            self.prev_pointer_pos = None;
        }
    }

    pub fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        indices: u32,
        vertex_buffer_slice: wgpu::BufferSlice,
    ) {
        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("rpass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.texture.view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth_texture.view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        rpass.set_pipeline(&self.render_pipeline);
        rpass.set_bind_group(0, &self.uniform_bind_group, &[]);
        rpass.set_bind_group(1, &self.mesh_bind_group, &[]);
        rpass.draw(0..indices, 0..1);

        rpass.set_pipeline(&self.sample_render_pipeline);
        rpass.set_bind_group(0, &self.uniform_bind_group, &[]);
        rpass.set_vertex_buffer(0, vertex_buffer_slice);
        rpass.set_index_buffer(
            self.sample_index_buffer.slice(..),
            wgpu::IndexFormat::Uint16,
        );
        rpass.draw_indexed(0..self.sample_num_indices, 0, 0..1);
    }
}

fn create_sample_render_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    texture_format: wgpu::TextureFormat,
    wireframe: bool,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("sample_render_pipeline"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: "sample_vertex",
            buffers: &[vertex_desc()],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            #[cfg(not(target_arch = "wasm32"))]
            entry_point: "sample_fragment",
            #[cfg(target_arch = "wasm32")]
            entry_point: "sample_fragment_web",
            targets: &[Some(wgpu::ColorTargetState {
                format: texture_format,
                blend: Some(wgpu::BlendState {
                    color: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::SrcAlpha,
                        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                        operation: wgpu::BlendOperation::Add,
                    },
                    alpha: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::One,
                        dst_factor: wgpu::BlendFactor::One,
                        operation: wgpu::BlendOperation::Max,
                    },
                }),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Cw,
            cull_mode: None,
            polygon_mode: if wireframe {
                wgpu::PolygonMode::Line
            } else {
                wgpu::PolygonMode::Fill
            },
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    })
}
