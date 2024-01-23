use vesuvius_engine::screen::Screen;
use vesuvius_engine::App;
use vesuvius_engine::render::buffer::builder::BufferBuilder;
use vesuvius_engine::render::buffer::format::{Topology, VertexFormat};
use vesuvius_engine::render::GameRenderer;
use vesuvius_engine::render::image::Image;

pub struct MainMenuScreen {
    pub(crate) image: Option<Image>
}

impl Screen for MainMenuScreen {
    fn init(&mut self, application: &App) {
        self.image = Some(Image::from_file(application, "assets/resources/images/image.png").unwrap());
    }

    fn render(&self, renderer: &mut GameRenderer) {
        BufferBuilder::builder(VertexFormat::PositionTexCoord, Topology::Quad)
            .image(self.image.as_ref().unwrap())
            .begin(-1.0, -1.0)
            .uv(0.0, 0.0)
            .end()
            .begin(-0.5, -1.0)
            .uv(1.0, 0.0)
            .end()
            .begin(-0.5, -0.5)
            .uv(1.0, 1.0)
            .end()
            .begin(-1.0, -0.5)
            .uv(0.0, 1.0)
            .end()
            .build(renderer);

        BufferBuilder::builder(VertexFormat::PositionColor, Topology::Quad)
            .begin(0.0, 0.0).color(1.0, 0.0, 0.0).end()
            .begin(0.5, 0.0).color(0.0, 1.0, 0.0).end()
            .begin(0.5, 0.5).color(0.0, 0.0, 1.0).end()
            .begin(0.0, 0.5).color(1.0, 1.0, 0.0).end()
            .build(renderer);

        renderer.queue_buffer_builder().unwrap();
    }
}
