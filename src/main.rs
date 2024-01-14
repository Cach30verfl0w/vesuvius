pub mod game;

use log::info;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;
use crate::game::Game;

fn main() {
    simple_logger::init().unwrap();

    // Window Init
    info!("Initializing game window");
    let window_event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&&window_event_loop).unwrap();

    // Game Init
    info!("Initializing Vesuvius");
    let game = Game::new(&window).unwrap();
    let device = game.request_best_device().unwrap();
    info!("Successfully requested device '{}'", device);

    // Game Loop
    info!("Init game loop, display window");
    window_event_loop.run(move |event, _, control_flow| {
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id
            } if window_id == window.id() => {
                *control_flow = ControlFlow::Exit;
            },
            Event::MainEventsCleared => {
                // Render
            }
            _ => {}
        }
    });
}
