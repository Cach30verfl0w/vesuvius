use std::borrow::Cow;
use glam::Vec3;
use vesuvius_engine::render::image::Image;
use vesuvius_engine::render::text::FontRenderer;
use vesuvius_engine::render::GameRenderer;
use vesuvius_engine::screen::Screen;
use vesuvius_engine::App;

pub struct MainMenuScreen {
    pub(crate) image: Option<Image>,
    pub(crate) font_renderer: FontRenderer,
}

impl Screen for MainMenuScreen {
    fn init(&mut self, application: &App) {
        self.image =
            Some(Image::from_file(application, "assets/resources/images/image.png").unwrap());
    }

    fn render(&self, renderer: &mut GameRenderer) {
        self.font_renderer
            .draw(
                0.1,
                0.1,
                Cow::Borrowed("It's working"),
                100.0,
                Vec3::new(1.0, 1.0, 1.0),
            )
            .unwrap();
        renderer.queue_buffer_builder().unwrap();
    }
}
