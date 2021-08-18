mod application;
mod arcball;
mod cornell_box;
mod functions;
mod main_view;
mod ray_samples;
mod reference_view;
mod texture;
mod vertices;

use anyhow::Result;
use application::Application;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::sync::mpsc::{channel, Sender};
use std::{ffi::OsStr, fs};
use winit::dpi::LogicalSize;
use winit::{
    event::{Event, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

fn start_watcher(tx: Sender<notify::Result<notify::Event>>) -> Result<RecommendedWatcher> {
    let mut watcher: RecommendedWatcher = Watcher::new_immediate(move |res| {
        tx.send(res).expect("sending watch event failed");
    })?;
    watcher.watch("src/main_view.wgsl", RecursiveMode::NonRecursive)?;
    Ok(watcher)
}

async fn run(event_loop: EventLoop<()>, window: Window) {
    let mut app = Application::new(&window)
        .await
        .expect("creation of application failed");

    let (tx, rx) = channel::<notify::Result<notify::Event>>();
    let watcher = start_watcher(tx);
    if let Err(e) = watcher {
        println!("Hot reloading disabled, watcher creation failed: {:?}", e);
    }

    event_loop.run(move |event, _, control_flow| {
        app.handle_event(&event);
        if app.captures_event(&event) {
            return;
        }

        match event {
            Event::WindowEvent {
                event: WindowEvent::DroppedFile(path),
                ..
            } => {
                println!("File dropped: {:?}", path);
                if path.extension().and_then(OsStr::to_str) == Some("obj") {
                    println!("Loading object...");
                    let (models, _) = tobj::load_obj(
                        &path,
                        &tobj::LoadOptions {
                            triangulate: true,
                            ..Default::default()
                        },
                    )
                    .expect("failed to load obj file");
                    println!("Number of models: {}", models.len());
                    println!("Loading first model...");
                    let mesh = &models[0].mesh;
                    app.load_model(&mut mesh.positions.clone(), &mesh.indices);
                    println!("Finished loading");
                }
            }
            Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } => app.resize(size.width, size.height),
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            Event::WindowEvent {
                event: WindowEvent::KeyboardInput { input, .. },
                ..
            } => match input.virtual_keycode {
                Some(VirtualKeyCode::R) => {
                    app.load_default_model();
                }
                _ => {}
            },
            Event::RedrawRequested(_) => {
                if let Err(e) = app.render(window.scale_factor() as f32) {
                    if e == wgpu::SurfaceError::Outdated {
                        let size = window.inner_size();
                        app.resize(size.width, size.height);
                    } else {
                        panic!("SwapChainError: {}", e);
                    }
                }
            }
            Event::MainEventsCleared => {
                let mut reload_compute_shader = false;
                for result in rx.try_iter() {
                    if let Ok(_event) = result {
                        reload_compute_shader = true;
                    }
                }
                if reload_compute_shader {
                    println!("Compute shader has changed");
                    let source = fs::read_to_string("src/main_view.wgsl")
                        .expect("reading compute shader failed");
                    if let Err(e) = app.reload_compute_shader(&source) {
                        println!("Shader reload failed: {:?}", e);
                    }
                }
                window.request_redraw()
            }
            _ => {}
        }
    });
}

fn main() -> Result<()> {
    env_logger::init();

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("linon")
        .with_inner_size(LogicalSize {
            width: 1400,
            height: 900,
        })
        .build(&event_loop)?;

    futures::executor::block_on(run(event_loop, window));

    Ok(())
}
