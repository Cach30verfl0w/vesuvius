extern crate vesuvius_engine;
extern crate log;
extern crate glam;
extern crate ash;

use std::mem::size_of;
use ash::vk;
use glam::{Vec2, Vec3};
use log::info;
use vesuvius_engine::vesuvius_winit::dpi::PhysicalSize;
use vesuvius_engine::vesuvius_winit::event::{Event, WindowEvent};
use vesuvius_engine::vesuvius_winit::event_loop::{ControlFlow, EventLoop};
use vesuvius_engine::vesuvius_winit::window::WindowBuilder;
use vesuvius_engine::App;
use vesuvius_engine::render::buffer::Buffer;
use vesuvius_engine::render::GameRenderer;
use vesuvius_engine::render::pipeline::RenderPipeline;

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
    let app = App::new(window).unwrap();
    let mut renderer = GameRenderer::new(app.clone()).unwrap();
    info!("Successfully created application and renderer");

    let mut pipeline = RenderPipeline::new(app.clone(), "assets/pipelines/draw.json").unwrap();
    pipeline.compile().unwrap();

    let vertex_buffer = Buffer::new(app.clone(), vk::BufferUsageFlags::VERTEX_BUFFER, size_of::<Vertex>() * 4)
        .unwrap();
    vertex_buffer.write([
        Vertex { position: Vec2::new(-0.5, -0.5), color: Vec3::new(1.0, 0.0, 0.0) },
        Vertex { position: Vec2::new(0.5, -0.5), color: Vec3::new(1.0, 1.0, 0.0) },
        Vertex { position: Vec2::new(0.5, 0.5), color: Vec3::new(0.0, 1.0, 0.0) },
        Vertex { position: Vec2::new(-0.5, 0.5), color: Vec3::new(0.0, 0.0, 1.0) }
    ]).unwrap();

    let index_buffer = Buffer::new(app.clone(), vk::BufferUsageFlags::INDEX_BUFFER, size_of::<u16>() * 6)
        .unwrap();
    index_buffer.write([
        0u16,
        1u16,
        2u16,
        2u16,
        3u16,
        0u16
    ]).unwrap();

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
                renderer.bind_pipeline(&pipeline);
                renderer.bind_vertex_buffer(&vertex_buffer);
                renderer.draw_indexed(&index_buffer);
                renderer.end().unwrap();
            }
            _ => {}
        }
    });
}