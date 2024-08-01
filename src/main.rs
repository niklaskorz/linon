mod application;
mod arcball;
mod cornell_box;
mod egui_wgpu;
mod functions;
mod main_view;
mod ray_samples;
mod reference_view;
mod syntax_highlighting;
mod texture;
mod vertices;

use anyhow::Result;
use application::Application;
#[cfg(not(target_arch = "wasm32"))]
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::ffi::OsStr;
use std::sync::mpsc::Receiver;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::mpsc::{channel, Sender};
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, KeyEvent};
use winit::window::Fullscreen;
use winit::{event::WindowEvent, event_loop::EventLoop, keyboard::Key, window::Window};

#[cfg(not(target_arch = "wasm32"))]
fn start_watcher(tx: Sender<notify::Result<notify::Event>>) -> Result<RecommendedWatcher> {
    let mut watcher = notify::recommended_watcher(move |res| {
        tx.send(res).expect("sending watch event failed");
    })?;
    watcher.watch("src/main_view.wgsl".as_ref(), RecursiveMode::NonRecursive)?;
    Ok(watcher)
}

struct ApplicationWindow<'a> {
    app: Option<Application<'a>>,
    window: Option<Arc<Window>>,
    close_requested: bool,
    #[cfg(not(target_arch = "wasm32"))]
    shader_rx: Receiver<notify::Result<notify::Event>>,
}

impl<'a> ApplicationHandler for ApplicationWindow<'a> {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window_attributes = Window::default_attributes()
            .with_title("linon")
            .with_inner_size(LogicalSize {
                width: 1400,
                height: 900,
            });
        let window = Arc::new(event_loop.create_window(window_attributes).unwrap()); // needed for resize closure on web

        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::JsCast;
            use winit::platform::web::WindowExtWebSys;
            console_log::init().expect("could not initialize logger");
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
            // On wasm, append the canvas to the document body
            let window = window.clone();
            let canvas = window.canvas().expect("couldn't retrieve canvas");
            let web_window = web_sys::window().expect("couldn't retrieve website window");
            let body = web_window
                .document()
                .and_then(|doc| doc.body())
                .expect("couldn't retrieve document body");
            body.append_child(&web_sys::Element::from(canvas))
                .ok()
                .expect("couldn't append canvas to body");
            let _ = window.request_inner_size(winit::dpi::LogicalSize::new(
                body.client_width(),
                body.client_height(),
            ));
            let resize_closure =
                wasm_bindgen::closure::Closure::wrap(Box::new(move |_e: web_sys::Event| {
                    let _ = window.request_inner_size(winit::dpi::LogicalSize::new(
                        body.client_width(),
                        body.client_height(),
                    ));
                }) as Box<dyn FnMut(_)>);
            web_window
                .add_event_listener_with_callback("resize", resize_closure.as_ref().unchecked_ref())
                .unwrap();
            resize_closure.forget();
        }

        self.app = Some(create_application(window.clone()));
        self.window = Some(window);
    }

    fn window_event(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let (Some(app), Some(window)) = (&mut self.app, &self.window) else {
            return;
        };
        if app.handle_event(&window, &event) {
            return;
        }

        match event {
            WindowEvent::DroppedFile(path) => {
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
            WindowEvent::Resized(size) => app.resize(size.width, size.height),
            WindowEvent::CloseRequested => {
                self.close_requested = true;
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state: ElementState::Pressed,
                        logical_key: Key::Character(chr),
                        ..
                    },
                ..
            } => match chr.as_str() {
                "r" => {
                    app.load_default_model();
                }
                "f" => {
                    window.set_fullscreen(if window.fullscreen().is_some() {
                        None
                    } else {
                        Some(Fullscreen::Borderless(None))
                    });
                }
                _ => {}
            },
            WindowEvent::RedrawRequested => {
                if let Err(e) = app.render(&window) {
                    if e == wgpu::SurfaceError::Outdated {
                        let size = window.inner_size();
                        app.resize(size.width, size.height);
                    } else {
                        panic!("{}", e);
                    }
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.close_requested {
            event_loop.exit();
            return;
        }
        let (Some(app), Some(window)) = (&mut self.app, &self.window) else {
            return;
        };

        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut reload_compute_shader = false;
            for result in self.shader_rx.try_iter() {
                if let Ok(_event) = result {
                    reload_compute_shader = true;
                }
            }
            if reload_compute_shader {
                println!("Compute shader has changed");
                let source = std::fs::read_to_string("src/main_view.wgsl")
                    .expect("reading compute shader failed");
                if let Err(e) = app.reload_compute_shader(&source) {
                    println!("Shader reload failed: {:?}", e);
                }
            }
        }

        window.request_redraw()
    }
}

fn create_application<'a>(window: Arc<Window>) -> Application<'a> {
    #[cfg(not(target_arch = "wasm32"))]
    let app = futures::executor::block_on(Application::new(window));
    #[cfg(target_arch = "wasm32")]
    let app = wasm_bindgen_futures::spawn_local(async move { Application::new(window).await });
    app.expect("creation of application failed")
}

fn main() -> Result<()> {
    #[cfg(not(target_arch = "wasm32"))]
    env_logger::init();

    #[cfg(not(target_arch = "wasm32"))]
    let (shader_rx, _watcher) = {
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

    let mut app = ApplicationWindow {
        window: None,
        app: None,
        close_requested: false,
        #[cfg(not(target_arch = "wasm32"))]
        shader_rx,
    };

    let event_loop = EventLoop::new()?;
    event_loop.run_app(&mut app)?;

    Ok(())
}
