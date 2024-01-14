pub mod game;

use log::info;
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
    let window = WindowBuilder::new().build(&&window_event_loop).unwrap();

    // Game Init
    info!("Initializing Vesuvius");
    let game = Game::new(&window).unwrap();
    info!("Successfully requested device '{}'", game.device());

    let mut renderer = GameRenderer::new(game.clone(), &window).unwrap();

    // Game Loop
    info!("Init game loop, display window");
    window_event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id
            } if window_id == window.id() => {
                *control_flow = ControlFlow::Exit;
            },
            Event::MainEventsCleared => {
                renderer.begin().unwrap();
                renderer.clear_color(0.0, 0.0, 0.0, 1.0);
                renderer.end().unwrap();
            }
            _ => {}
        }
    });
}
