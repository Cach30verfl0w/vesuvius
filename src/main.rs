#![feature(get_mut_unchecked)]

pub mod game;

use std::mem::size_of;
use ash::vk::BufferUsageFlags;
use glam::{Vec2, Vec3};
use log::info;
use winit::dpi::PhysicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;
use crate::game::Game;
use crate::game::render::GameRenderer;

#[repr(C)]
pub struct Vertex {
    position: Vec2,
    color: Vec3
}

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
    let mut game = Game::new(window).unwrap();
    info!("Successfully requested device '{}'", game.device());
    let mut renderer = GameRenderer::new(game.clone()).unwrap();
    renderer.init_pipelines().unwrap();

    let vertex_buffer = game.device_mut().new_buffer(BufferUsageFlags::VERTEX_BUFFER, size_of::<Vertex>() * 4).unwrap();
    vertex_buffer.write([
        Vertex { position: Vec2::new(-0.5, -0.5), color: Vec3::new(1.0, 0.0, 0.0) },
        Vertex { position: Vec2::new(0.5, -0.5), color: Vec3::new(1.0, 1.0, 0.0) },
        Vertex { position: Vec2::new(0.5, 0.5), color: Vec3::new(0.0, 1.0, 0.0) },
        Vertex { position: Vec2::new(-0.5, 0.5), color: Vec3::new(0.0, 0.0, 1.0) }
    ]).unwrap();

    let index_buffer = game.device_mut().new_buffer(BufferUsageFlags::INDEX_BUFFER, size_of::<u16>() * 6).unwrap();
    index_buffer.write([
        0u16,
        1u16,
        2u16,
        2u16,
        3u16,
        0u16
    ]).unwrap();

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
                renderer.bind_vertex_buffer(&vertex_buffer);
                renderer.draw_indexed(&index_buffer);

                renderer.end().unwrap();
            }
            _ => {}
        }
    });
}
