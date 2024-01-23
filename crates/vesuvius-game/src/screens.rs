use vesuvius_engine::render::{BufferBuilder, GameRenderer, VertexFormat};
use vesuvius_engine::screen::Screen;
use vesuvius_engine::App;

pub struct MainMenuScreen;

impl Screen for MainMenuScreen {
    fn init(&mut self, _application: &App) {}

    fn render(&self, renderer: &mut GameRenderer) {
        BufferBuilder::builder(VertexFormat::QuadCoordColor)
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
        renderer.queue_buffer_builder().unwrap();
    }
}
