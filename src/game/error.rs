use std::string::FromUtf8Error;
use ash::{LoadingError, vk};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum EngineError {
    #[error("Loading Error => {0}")]
    Load(#[from] LoadingError),

    #[error("Vulkan Error => {0}")]
    Vulkan(#[from] vk::Result),

    #[error("File Watcher Error => {0}")]
    FileWatcher(#[from] notify::Error),

    #[error("IO Error => {0}")]
    Io(#[from] std::io::Error),

    #[error("From UTF-8 Error => {0}")]
    Utf8(#[from] FromUtf8Error),

    #[error("Shader Compiler Error => {0}")]
    ShaderCompiler(#[from] shaderc::Error),

    #[error("Creation of SpirV Compiler Instance failed")]
    CompilerCreation
}