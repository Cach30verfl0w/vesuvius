use std::mem::size_of;
use ash::vk;
use glam::{Vec2, Vec3};
use Vertex;
use vesuvius_engine::App;
use vesuvius_engine::render::buffer::Buffer;
use vesuvius_engine::render::GameRenderer;
use vesuvius_engine::screen::Screen;

#[derive(Default)]
pub struct MainMenuScreen {
    vertex_buffer: Option<Buffer>,
    index_buffer: Option<Buffer>
}

impl Screen for MainMenuScreen {
    fn init(&mut self, application: &App) {
        let vertex_buffer = Buffer::new(application.clone(), vk::BufferUsageFlags::VERTEX_BUFFER, size_of::<Vertex>() * 4)
            .unwrap();
        vertex_buffer.write([
            Vertex { position: Vec2::new(-0.5, -0.5), color: Vec3::new(1.0, 0.0, 0.0) },
            Vertex { position: Vec2::new(0.5, -0.5), color: Vec3::new(1.0, 1.0, 0.0) },
            Vertex { position: Vec2::new(0.5, 0.5), color: Vec3::new(0.0, 1.0, 0.0) },
            Vertex { position: Vec2::new(-0.5, 0.5), color: Vec3::new(0.0, 0.0, 1.0) }
        ]).unwrap();

        let index_buffer = Buffer::new(application.clone(), vk::BufferUsageFlags::INDEX_BUFFER, size_of::<u16>() * 6)
            .unwrap();
        index_buffer.write([
            0u16,
            1u16,
            2u16,
            2u16,
            3u16,
            0u16
        ]).unwrap();

        self.vertex_buffer = Some(vertex_buffer);
        self.index_buffer = Some(index_buffer);
    }

    fn render(&self, renderer: &mut GameRenderer) {
        renderer.bind_pipeline(renderer.find_pipeline("draw").unwrap());
        renderer.bind_vertex_buffer(self.vertex_buffer.as_ref().unwrap());
        renderer.draw_indexed(self.index_buffer.as_ref().unwrap());
    }
}