extern crate vesuvius_engine;
extern crate log;
extern crate glam;
extern crate ash;

pub mod screens;

use glam::{Vec2, Vec3};
use log::{debug, info};
use screens::MainMenuScreen;
use vesuvius_engine::vesuvius_winit::dpi::PhysicalSize;
use vesuvius_engine::vesuvius_winit::event::{Event, WindowEvent};
use vesuvius_engine::vesuvius_winit::event_loop::{ControlFlow, EventLoop};
use vesuvius_engine::vesuvius_winit::window::WindowBuilder;
use vesuvius_engine::App;
use vesuvius_engine::debug::DebugExtension;
use vesuvius_engine::render::GameRenderer;

#[repr(C)]
pub struct Vertex {
    position: Vec2,
    color: Vec3
}

fn main() {
    simple_logger::init().unwrap();

    // Create window
    info!("Create 1200x800 game window");
    let window_event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title(concat!("Vesuvius v", env!("CARGO_PKG_VERSION"), " by ", env!("CARGO_PKG_AUTHORS")))
        .with_inner_size(PhysicalSize::new(1200, 800))
        .build(&window_event_loop)
        .unwrap();

    // Create application
    let mut app = App::new(window).unwrap();
    let mut renderer = GameRenderer::new(app.clone()).unwrap();
    renderer.reload().unwrap();

    app.open_screen(Box::new(MainMenuScreen {
        vertex_buffer: None,
        index_buffer: None,
        alpha_buffer: None,
        renderer: renderer.clone(),
        descriptor_set: None,
    }));
    info!("Successfully created application and renderer");

    #[cfg(feature = "debug_extensions")]
    {
        debug!("Game-internal debug extensions enabled (Game is compiled for debug)");
        let debug_extension = DebugExtension::new(renderer.clone());
    }

    // Game Loop
    info!("Init game loop and display game");
    window_event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id
            } if window_id == app.window().id() => {
                *control_flow = ControlFlow::Exit;
            },
            Event::MainEventsCleared => app.window().request_redraw(),
            Event::RedrawRequested(_window_id) => {
                renderer.begin().unwrap();
                renderer.clear_color(0.0, 0.0, 0.0, 1.0);

                if let Some(screen) = app.screen() {
                    screen.render(&mut renderer);
                }

                renderer.end().unwrap();
            }
            _ => {}
        }
    });
}