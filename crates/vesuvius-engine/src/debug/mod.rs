#![allow(dead_code)] // TODO: Work in progress
use crate::render::GameRenderer;

pub struct DebugExtension {
    renderer: GameRenderer,
}

impl DebugExtension {
    pub fn new(renderer: GameRenderer) -> Self {
        Self { renderer }
    }
}
