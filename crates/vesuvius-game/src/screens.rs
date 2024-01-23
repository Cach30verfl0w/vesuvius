use vesuvius_engine::screen::Screen;
use vesuvius_engine::App;
use vesuvius_engine::render::buffer::builder::BufferBuilder;
use vesuvius_engine::render::buffer::format::{Topology, VertexFormat};
use vesuvius_engine::render::GameRenderer;

pub struct MainMenuScreen;

impl Screen for MainMenuScreen {
    fn init(&mut self, _application: &App) {}

    fn render(&self, renderer: &mut GameRenderer) {
        BufferBuilder::builder(VertexFormat::PositionColor, Topology::Quad)
            .begin(0.0, 0.0)
            .color(1.0, 1.0, 1.0)
            .end()
            .begin(0.25, 0.0)
            .color(1.0, 1.0, 1.0)
            .end()
            .begin(0.25, 0.25)
            .color(1.0, 1.0, 1.0)
            .end()
            .begin(0.0, 0.25)
            .color(1.0, 1.0, 1.0)
            .end()
            .build(renderer);

        BufferBuilder::builder(VertexFormat::PositionColor, Topology::Quad)
            .begin(-1.0, -1.0)
            .color(1.0, 1.0, 1.0)
            .end()
            .begin(-0.5, -1.0)
            .color(1.0, 1.0, 1.0)
            .end()
            .begin(-0.5, -0.5)
            .color(1.0, 1.0, 1.0)
            .end()
            .begin(-1.0, -0.5)
            .color(1.0, 1.0, 1.0)
            .end()
            .build(renderer);

        renderer.queue_buffer_builder().unwrap();
    }
}
