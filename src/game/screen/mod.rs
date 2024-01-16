use std::mem::size_of;
use ash::vk::BufferUsageFlags;
use glam::{Vec2, Vec3};
use crate::game::device::WrappedBuffer;
use crate::game::Game;
use crate::game::render::GameRenderer;
use crate::Vertex;

pub(crate) trait Screen {
    fn init(&mut self, game: &mut Game);

    fn on_close(&mut self, game: &mut Game);

    fn render(&self, renderer: &mut GameRenderer);

    fn title<'a>() -> &'a str where Self: Sized;
}

#[derive(Default)]
pub struct MainMenuScreen {
    vertex_buffer: Option<WrappedBuffer>,
    index_buffer: Option<WrappedBuffer>
}

impl Screen for MainMenuScreen {
    fn init(&mut self, game: &mut Game) {
        let vertex_buffer = game.device_mut().new_buffer(BufferUsageFlags::VERTEX_BUFFER, size_of::<Vertex>() * 4).unwrap();
        vertex_buffer.write([
            Vertex { position: Vec2::new(-0.5, -0.5), color: Vec3::new(1.0, 0.0, 0.0) },
            Vertex { position: Vec2::new(0.5, -0.5), color: Vec3::new(1.0, 1.0, 0.0) },
            Vertex { position: Vec2::new(0.5, 0.5), color: Vec3::new(0.0, 1.0, 0.0) },
            Vertex { position: Vec2::new(-0.5, 0.5), color: Vec3::new(0.0, 0.0, 1.0) }
        ]).unwrap();
        self.vertex_buffer = Some(vertex_buffer);

        let index_buffer = game.device_mut().new_buffer(BufferUsageFlags::INDEX_BUFFER, size_of::<u16>() * 6).unwrap();
        index_buffer.write([
            0u16,
            1u16,
            2u16,
            2u16,
            3u16,
            0u16
        ]).unwrap();
        self.index_buffer = Some(index_buffer);
    }

    fn on_close(&mut self, _game: &mut Game) {}

    fn render(&self, renderer: &mut GameRenderer) {
        renderer.apply_pipeline("triangle");
        renderer.bind_vertex_buffer(&self.vertex_buffer.as_ref().unwrap());
        renderer.draw_indexed(&self.index_buffer.as_ref().unwrap());
    }

    fn title<'a>() -> &'a str {
        "Main Menu"
    }
}
