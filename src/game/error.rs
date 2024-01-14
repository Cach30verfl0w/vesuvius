use ash::{LoadingError, vk};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum EngineError {
    #[error("Loading Error => {0}")]
    Load(#[from] LoadingError),

    #[error("Vulkan Error => {0}")]
    Vulkan(#[from] vk::Result)
}