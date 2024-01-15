pub mod game;

use log::info;
use winit::dpi::PhysicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;
use crate::game::Game;
use crate::game::render::GameRenderer;

fn main() {
    simple_logger::init().unwrap();

    // Window Init
    info!("Initializing game window");
    let window_event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title(concat!("Vesuvious v", env!("CARGO_PKG_VERSION"), " by Cach30verfl0w"))
        .with_inner_size(PhysicalSize::new(1200, 800))
        .build(&window_event_loop)
        .unwrap();

    // Game Init
    info!("Initializing Vesuvius");
    let game = Game::new(window).unwrap();
    info!("Successfully requested device '{}'", game.device());
    let mut renderer = GameRenderer::new(game.clone()).unwrap();
    renderer.init_pipelines().unwrap();

    // Game Loop
    info!("Init game loop and display window");
    window_event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id
            } if window_id == game.window().id() => {
                *control_flow = ControlFlow::Exit;
            },
            Event::MainEventsCleared => game.window().request_redraw(),
            Event::RedrawRequested(_window_id) => {
                renderer.begin().unwrap();
                renderer.clear_color(0.0, 0.0, 0.0, 1.0);

                renderer.apply_pipeline("triangle");
                renderer.draw();

                renderer.end().unwrap();
            }
            _ => {}
        }
    });
}
