use crate::render::GameRenderer;
use crate::App;

pub trait Screen {
    fn init(&mut self, application: &App);
    fn on_close(&mut self, _application: &App) {}
    fn render(&self, renderer: &mut GameRenderer);
}
