mod application;
mod texture;

use anyhow::Result;
use futures::executor::block_on;
use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

fn main() -> Result<()> {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("linon")
        .with_inner_size(PhysicalSize {
            width: 1280,
            height: 720,
        })
        .build(&event_loop)?;
    env_logger::init();
    let mut app = block_on(application::Application::new(&window))?;
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } => app.resize(size.width, size.height),
            Event::RedrawRequested(_) => app.render(),
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            Event::MainEventsCleared => window.request_redraw(),
            _ => {}
        }
    });
}
