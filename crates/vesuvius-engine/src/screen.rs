use crate::render::GameRenderer;
use crate::App;
use winit::dpi::PhysicalPosition;
use winit::event::{ModifiersState, VirtualKeyCode};

pub trait Screen {
    fn init(&mut self, application: &App);
    fn on_close(&mut self, _application: &App) {}
    fn on_key_released(&mut self, _key: VirtualKeyCode, _modifiers: ModifiersState) {}
    fn on_key_pressed(&mut self, _key: VirtualKeyCode, _modifiers: ModifiersState) {}
    fn on_mouse_moved(&mut self, _position: PhysicalPosition<f64>) {}
    fn render(&self, renderer: &mut GameRenderer);
}
