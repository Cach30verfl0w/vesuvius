pub mod screens;

use screens::MainMenuScreen;
#[cfg(feature = "debug_extensions")]
use vesuvius_engine::debug::DebugExtension;
use vesuvius_engine::render::text::FontRenderer;
use vesuvius_engine::render::GameRenderer;
use vesuvius_engine::vesuvius_winit::dpi::PhysicalSize;
use vesuvius_engine::vesuvius_winit::event::{ElementState, Event, ModifiersState, WindowEvent};
use vesuvius_engine::vesuvius_winit::event_loop::{ControlFlow, EventLoop};
use vesuvius_engine::vesuvius_winit::window::WindowBuilder;
use vesuvius_engine::App;

fn main() {
    simple_logger::init().unwrap();

    // Create window
    log::info!("Create 1200x800 game window");
    let window_event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title(concat!(
            "Vesuvius v",
            env!("CARGO_PKG_VERSION"),
            " by ",
            env!("CARGO_PKG_AUTHORS")
        ))
        .with_min_inner_size(PhysicalSize::new(600, 600))
        .with_inner_size(PhysicalSize::new(1200, 800))
        .with_visible(false)
        .build(&window_event_loop)
        .unwrap();

    // Create application
    let mut app = App::new(window).unwrap();
    let mut renderer = GameRenderer::new(app.clone()).unwrap();
    renderer.reload(true).unwrap();

    let font_renderer =
        FontRenderer::new(renderer.clone(), "assets/resources/fonts/roboto-thin").unwrap();

    app.open_screen(Box::new(MainMenuScreen {
        image: None,
        font_renderer,
    }));
    log::info!("Successfully created application and renderer");

    #[cfg(feature = "debug_extensions")]
    {
        log::debug!("Game-internal debug extensions enabled (Game is compiled for debug)");
        let _debug_extension = DebugExtension::new(renderer.clone());
    }

    // Game Loop
    app.window().set_visible(true);
    log::info!("Init game loop and display game");
    let mut current_modifiers_state = ModifiersState::empty();
    window_event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::MainEventsCleared => app.window().request_redraw(),
            Event::WindowEvent { event, window_id } if window_id == app.window().id() => {
                match event {
                    WindowEvent::ModifiersChanged(modifiers) => current_modifiers_state = modifiers,
                    WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                    WindowEvent::Resized(_resized_size) => renderer.reload(false).unwrap(),
                    WindowEvent::KeyboardInput { input, .. } => {
                        if let Some(keycode) = input.virtual_keycode {
                            if let Some(screen) = app.screen_mut() {
                                match input.state {
                                    ElementState::Pressed => {
                                        screen.on_key_pressed(keycode, current_modifiers_state)
                                    }
                                    ElementState::Released => {
                                        screen.on_key_released(keycode, current_modifiers_state)
                                    }
                                }
                            }
                        }
                    }
                    WindowEvent::CursorMoved { position, .. } => {
                        if let Some(screen) = app.screen_mut() {
                            screen.on_mouse_moved(position);
                        }
                    }
                    _ => {}
                }
            }
            Event::RedrawRequested(_window_id) => {
                renderer.begin().unwrap();
                renderer.clear_color(0.0, 0.0, 0.0, 1.0);

                if let Some(screen) = app.screen() {
                    screen.render(&mut renderer);
                }

                renderer.end().unwrap();
            }
            _ => {}
        }
    });
}
