use crate::Vertex;
use ash::vk;
use glam::{Vec2, Vec3};
use std::mem::size_of;
use std::slice;
use vesuvius_engine::render::buffer::Buffer;
use vesuvius_engine::render::pipeline::DescriptorSet;
use vesuvius_engine::render::GameRenderer;
use vesuvius_engine::screen::Screen;
use vesuvius_engine::App;

pub struct MainMenuScreen {
    pub(crate) vertex_buffer: Option<Buffer>,
    pub(crate) index_buffer: Option<Buffer>,
    pub(crate) alpha_buffer: Option<Buffer>,
    pub(crate) renderer: GameRenderer,
    pub(crate) descriptor_set: Option<DescriptorSet>,
}

impl Screen for MainMenuScreen {
    fn init(&mut self, application: &App) {
        let vertex_buffer = Buffer::new(
            application.clone(),
            vk::BufferUsageFlags::VERTEX_BUFFER,
            (size_of::<Vertex>() * 4) as vk::DeviceSize,
        )
        .unwrap();
        vertex_buffer
            .write([
                Vertex {
                    position: Vec2::new(-0.5, -0.5),
                    color: Vec3::new(1.0, 0.0, 0.0),
                },
                Vertex {
                    position: Vec2::new(0.5, -0.5),
                    color: Vec3::new(1.0, 1.0, 0.0),
                },
                Vertex {
                    position: Vec2::new(0.5, 0.5),
                    color: Vec3::new(0.0, 1.0, 0.0),
                },
                Vertex {
                    position: Vec2::new(-0.5, 0.5),
                    color: Vec3::new(0.0, 0.0, 1.0),
                },
            ])
            .unwrap();

        let index_buffer = Buffer::new(
            application.clone(),
            vk::BufferUsageFlags::INDEX_BUFFER,
            (size_of::<u16>() * 6) as vk::DeviceSize,
        )
        .unwrap();
        index_buffer
            .write([0u16, 1u16, 2u16, 2u16, 3u16, 0u16])
            .unwrap();

        let alpha_buffer = Buffer::new(
            application.clone(),
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            size_of::<f32>() as vk::DeviceSize,
        )
        .unwrap();
        alpha_buffer.write([1.0f32]).unwrap();

        let descriptor_set = DescriptorSet::allocate(&self.renderer, "draw", 0).unwrap();
        descriptor_set.write(0, &alpha_buffer);

        self.vertex_buffer = Some(vertex_buffer);
        self.index_buffer = Some(index_buffer);
        self.alpha_buffer = Some(alpha_buffer);
        self.descriptor_set = Some(descriptor_set);
    }

    fn render(&self, renderer: &mut GameRenderer) {
        renderer.bind_pipeline(
            renderer.find_pipeline("draw").unwrap(),
            slice::from_ref(self.descriptor_set.as_ref().unwrap()),
        );
        renderer.bind_vertex_buffer(self.vertex_buffer.as_ref().unwrap());
        renderer.draw_indexed(self.index_buffer.as_ref().unwrap());
    }
}
