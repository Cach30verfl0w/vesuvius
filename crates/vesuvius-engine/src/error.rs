use ash::{vk, LoadingError};
use std::io;
use std::string::FromUtf8Error;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Error while operating with Vulkan => {0}")]
    Vulkan(#[from] vk::Result),

    #[error("Error while loading Vulkan => {0}")]
    Loading(#[from] LoadingError),

    #[error("Error while doing IO operation => {0}")]
    IO(#[from] io::Error),

    #[error("Error while converting from UTF-8 => {0}")]
    FromUtf8(#[from] FromUtf8Error),

    #[error("Error while creating shader => Unable to create SPIR-V compiler")]
    CompilerCreation,

    #[error("Error while creating shader => {0}")]
    ShaderCompiler(#[from] shaderc::Error),

    #[error("Error while decoding image resource => {0}")]
    Image(#[from] image::ImageError),
}
