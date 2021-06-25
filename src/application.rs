use crate::cornell_box as cbox;
use crate::gui::{Gui, INITIAL_SIDEBAR_WIDTH};
use crate::main_view::MainView;
use crate::reference_view::ReferenceView;
use anyhow::{Context, Result};
use cgmath::Vector3;
use wgpu::util::DeviceExt;
use winit::window::Window;

#[repr(C)]
#[derive(Debug, Clone)]
struct Vertices {
    length: u32,
    data: Vec<u32>,
}

#[repr(C)]
#[derive(Debug, Clone)]
struct Faces {
    length: u32,
    data: Vec<u32>,
}

pub struct Application {
    _instance: wgpu::Instance,
    surface: wgpu::Surface,
    _adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    sc_desc: wgpu::SwapChainDescriptor,
    swap_chain: wgpu::SwapChain,
    main_view: MainView,
    reference_view: ReferenceView,
    pub gui: Gui,
}

impl Application {
    pub async fn new(window: &Window) -> Result<Self> {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
        let surface = unsafe { instance.create_surface(window) };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
            })
            .await
            .context("no compatible adapter found")?;
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::default(),
                },
                None,
            )
            .await?;

        let swapchain_format = adapter
            .get_swap_chain_preferred_format(&surface)
            .context("no compatible swap chain format found")?;
        let sc_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
            format: swapchain_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };
        let swap_chain = device.create_swap_chain(&surface, &sc_desc);

        let mut vertices = cbox::VERTICES;
        for i in 0..(vertices.len() / 3) {
            // Invert x and z axis
            vertices[3 * i] = -vertices[3 * i];
            vertices[3 * i + 2] = -vertices[3 * i + 2];
        }
        normalize_vertices(&mut vertices);
        let vertices_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("vertices_buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsage::STORAGE,
        });
        let faces_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("faces_buffer"),
            contents: bytemuck::cast_slice(&cbox::INDICES),
            usage: wgpu::BufferUsage::STORAGE,
        });
        let center = get_center(&vertices);

        let main_view = MainView::new(
            &device,
            vertices_buffer.as_entire_binding(),
            faces_buffer.as_entire_binding(),
            center,
            size.width - INITIAL_SIDEBAR_WIDTH as u32,
            size.height,
        );
        let reference_view = ReferenceView::new(&device);

        let gui = Gui::new(
            size,
            window.scale_factor(),
            &device,
            swapchain_format,
            &main_view.texture.texture,
            &reference_view.texture.texture,
        );

        Ok(Self {
            _instance: instance,
            surface,
            _adapter: adapter,
            device,
            queue,
            sc_desc,
            swap_chain,
            main_view,
            reference_view,
            gui,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.sc_desc.width = width;
        self.sc_desc.height = height;
        self.swap_chain = self.device.create_swap_chain(&self.surface, &self.sc_desc);
    }

    pub fn load_default_model(&mut self) {
        let mut vertices = cbox::VERTICES;
        self.load_model(&mut vertices, &cbox::INDICES);
    }

    pub fn load_model(&mut self, vertices: &mut [f32], indices: &[u32]) {
        normalize_vertices(vertices);

        let vertices_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("vertices_buffer"),
                contents: bytemuck::cast_slice(vertices),
                usage: wgpu::BufferUsage::STORAGE,
            });
        let faces_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("faces_buffer"),
                contents: bytemuck::cast_slice(indices),
                usage: wgpu::BufferUsage::STORAGE,
            });
        let center = get_center(vertices);

        self.main_view.update_model(
            &self.device,
            &self.queue,
            vertices_buffer.as_entire_binding(),
            faces_buffer.as_entire_binding(),
            center,
        );
    }

    pub fn render(&mut self, scale_factor: f32) {
        if self.main_view.needs_redraw {
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("encoder"),
                });
            encoder.push_debug_group("render main view");
            self.main_view.render(&mut encoder);
            encoder.pop_debug_group();
            encoder.push_debug_group("render reference view");
            self.reference_view.render(&mut encoder);
            encoder.pop_debug_group();
            self.queue.submit(Some(encoder.finish()));
        }

        let frame = self
            .swap_chain
            .get_current_frame()
            .expect("failed to acquire next swap chain texture")
            .output;

        let dimensions = self.gui.draw(
            &frame.view,
            self.sc_desc.width,
            self.sc_desc.height,
            scale_factor,
            &self.device,
            &self.queue,
        );
        if self.main_view.texture.dimensions != dimensions {
            self.main_view
                .resize_texture(&self.device, dimensions.0, dimensions.1);
            self.gui
                .change_texture(&self.device, &self.main_view.texture.texture);
        }

        if let Some(pos) = self.gui.cursor_pos {
            self.main_view
                .on_cursor_moved(&self.queue, self.gui.camera_op, pos);
        }
        if self.gui.scroll_delta.y != 0.0 {
            self.main_view
                .on_mouse_wheel(&self.queue, self.gui.scroll_delta.y);
        }

        if self.gui.rotate_scene_changed {
            self.gui.rotate_scene_changed = false;
            self.main_view.rotate_scene = self.gui.rotate_scene;
            self.main_view.update_camera(&self.queue);
        }
        if self.gui.field_weight_changed {
            self.gui.field_weight_changed = false;
            self.main_view
                .update_settings(&self.queue, self.gui.field_weight);
        }
        if self.gui.field_function_changed {
            self.gui.field_function_changed = false;
            if let Err(e) =
                self.main_view
                    .reload_compute_shader(&self.device, None, self.gui.field_function)
            {
                self.gui.shader_error = Some(e.to_string());
            } else {
                self.gui.shader_error = None;
            }
        }
    }
}

fn normalize_vertices(vertices: &mut [f32]) {
    let mut max: f32 = 1.0;
    let mut min: f32 = -1.0;
    for (i, x) in vertices.iter().enumerate() {
        if i == 0 || *x > max {
            max = *x;
        }
        if i == 0 || *x < min {
            min = *x;
        }
    }
    for x in vertices.iter_mut() {
        *x = (*x - min) / (max - min) * 2.0 - 1.0;
    }
}

fn get_center(vertices: &[f32]) -> Vector3<f32> {
    let mut min_x = vertices[0];
    let mut min_y = vertices[1];
    let mut min_z = vertices[2];
    let mut max_x = vertices[0];
    let mut max_y = vertices[1];
    let mut max_z = vertices[2];

    let num_vertices = vertices.len() / 3;
    for i in 1..num_vertices {
        let x = vertices[3 * i];
        if x < min_x {
            min_x = x;
        }
        if x > max_x {
            max_x = x;
        }
        let y = vertices[3 * i + 1];
        if y < min_y {
            min_y = y;
        }
        if y > max_y {
            max_y = y;
        }
        let z = vertices[3 * i + 2];
        if z < min_z {
            min_z = z;
        }
        if z > max_z {
            max_z = z;
        }
    }

    Vector3::new(
        (min_x + max_x) / 2.0,
        (min_y + max_y) / 2.0,
        (min_z + max_z) / 2.0,
    )
}
