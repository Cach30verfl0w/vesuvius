use render::GameRenderer;

pub struct DebugExtension {
    renderer: GameRenderer
}

impl DebugExtension {
    pub fn new(renderer: GameRenderer) -> Self {
        Self {
            renderer: renderer
        }
    }
}