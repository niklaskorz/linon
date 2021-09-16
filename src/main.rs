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
#[cfg(not(target_arch = "wasm32"))]
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::rc::Rc;
use std::sync::mpsc::{channel, Sender};
use std::{ffi::OsStr, fs};
use winit::dpi::LogicalSize;
use winit::{
    event::{Event, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

#[cfg(not(target_arch = "wasm32"))]
fn start_watcher(tx: Sender<notify::Result<notify::Event>>) -> Result<RecommendedWatcher> {
    let mut watcher: RecommendedWatcher = RecommendedWatcher::new(move |res| {
        tx.send(res).expect("sending watch event failed");
    })?;
    watcher.watch("src/main_view.wgsl".as_ref(), RecursiveMode::NonRecursive)?;
    Ok(watcher)
}

async fn run(event_loop: EventLoop<()>, window: Rc<Window>) {
    let mut app = Application::new(&window)
        .await
        .expect("creation of application failed");

    #[cfg(not(target_arch = "wasm32"))]
    let (rx, _watcher) = {
        let (tx, rx) = channel::<notify::Result<notify::Event>>();
        match start_watcher(tx) {
            Ok(watcher) => {
                println!("Watching shader main_view.wgsl for changes");
                (rx, Some(watcher))
            }
            Err(e) => {
                println!("Hot reloading disabled, watcher creation failed: {:?}", e);
                (rx, None)
            }
        }
    };

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
                        panic!("{}", e);
                    }
                }
            }
            Event::MainEventsCleared => {
                #[cfg(not(target_arch = "wasm32"))]
                {
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
                }
                window.request_redraw()
            }
            _ => {}
        }
    });
}

fn main() -> Result<()> {
    #[cfg(not(target_arch = "wasm32"))]
    env_logger::init();

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("linon")
        .with_inner_size(LogicalSize {
            width: 1400,
            height: 900,
        })
        .build(&event_loop)?;
    let window = Rc::new(window); // needed for resize closure on web

    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;
        use winit::platform::web::WindowExtWebSys;
        console_log::init().expect("could not initialize logger");
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        // On wasm, append the canvas to the document body
        let window = window.clone();
        let web_window = web_sys::window().expect("couldn't retrieve website window");
        let body = web_window
            .document()
            .and_then(|doc| doc.body())
            .expect("couldn't retrieve document body");
        body.append_child(&web_sys::Element::from(window.canvas()))
            .ok()
            .expect("couldn't append canvas to body");
        window.set_inner_size(winit::dpi::LogicalSize::new(
            body.client_width(),
            body.client_height(),
        ));
        let resize_closure =
            wasm_bindgen::closure::Closure::wrap(Box::new(move |e: web_sys::Event| {
                window.set_inner_size(winit::dpi::LogicalSize::new(
                    body.client_width(),
                    body.client_height(),
                ));
            }) as Box<dyn FnMut(_)>);
        web_window
            .add_event_listener_with_callback("resize", resize_closure.as_ref().unchecked_ref())
            .unwrap();
        resize_closure.forget();
    }

    #[cfg(not(target_arch = "wasm32"))]
    futures::executor::block_on(run(event_loop, window));
    #[cfg(target_arch = "wasm32")]
    wasm_bindgen_futures::spawn_local(run(event_loop, window));

    Ok(())
}
