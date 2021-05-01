mod application;
mod texture;

use anyhow::Result;
use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};
use application::Application;

async fn run(event_loop: EventLoop<()>, window: Window) {
    let mut app = Application::new(&window).await.expect("creation of application failed");

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

fn main() -> Result<()> {
    #[cfg(not(target_arch = "wasm32"))]
    env_logger::init();
    #[cfg(target_arch = "wasm32")]
    {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        console_log::init().expect("could not initialize logger");
    }

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("linon")
        .with_inner_size(PhysicalSize {
            width: 1280,
            height: 720,
        })
        .build(&event_loop)?;

    #[cfg(target_arch = "wasm32")]
    {
        use anyhow::Context;
        use winit::platform::web::WindowExtWebSys;
        // On wasm, append the canvas to the document body
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| doc.body())
            .and_then(|body| {
                body.append_child(&web_sys::Element::from(window.canvas()))
                    .ok()
            }).context("could not append canvas to document")?;
    }

    #[cfg(not(target_arch = "wasm32"))]
    futures::executor::block_on(run(event_loop, window));
    #[cfg(target_arch = "wasm32")]
    wasm_bindgen_futures::spawn_local(run(event_loop, window));

    Ok(())
}
