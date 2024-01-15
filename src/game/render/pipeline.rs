use std::fs;
use std::path::{Path, PathBuf};
use ash::vk;
use serde::{Deserialize, Serialize};
use crate::game::render::GameRenderer;
use crate::game::Result;

/// This structure represents a render pipeline. The complete pipeline is re-compilable, when the
/// source code or the configuration file changes. The re-compilation feature is used by the file
/// watcher in the Game Renderer.
#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub(crate) struct RenderPipeline {
    /// This field contains all shader modules, which are used for the compilation of the pipeline
    shader_modules: Vec<ShaderModule>,

    /// This field contains the handle of the compiled graphics pipeline.
    vulkan_pipeline: Option<vk::Pipeline>
}

impl RenderPipeline {

    /// This function reads the pipeline configuration file and builds the complete pipeline with
    /// the shaders.
    pub(crate) fn from_file<P: AsRef<Path>>(game_renderer: &mut GameRenderer, path: P) -> Result<Self> {
        let path = path.as_ref();
        if !path.is_file() {
            panic!("Unable to create render pipeline => The path '{}' doesn't points to a file",
                   path.to_str().unwrap());
        }

        let file_content = String::from_utf8(fs::read(path)?)?;
        todo!()
    }

}

/// This structure represents a shader module. This shader module is re-compilable, when the source
/// code of the shader changes. The re-compilation features is used by the render pipeline while
/// rebuilding the pipeline.
#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
struct ShaderModule {
    /// This field contains the path to the shader source file in the assets folder
    shader_source_path: Option<PathBuf>,

    /// This field contains the handle of the compiled shader module
    vulkan_shader_module: Option<vk::ShaderModule>,

    /// This field contains the kind of the shader (like fragment or vertex)
    kind: ShaderKind
}

/// This enum represents all supported kinds of shader in the Vesuvius game engine. Currently only
/// vertex and fragment shader are supported, because we only need them now.
#[derive(Serialize, Deserialize, Clone, Copy, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
enum ShaderKind {
    Fragment,
    Vertex
}

impl From<ShaderKind> for shaderc::ShaderKind {
    #[inline]
    fn from(value: ShaderKind) -> Self {
        match value {
            ShaderKind::Vertex => Self::Vertex,
            ShaderKind::Fragment => Self::Fragment
        }
    }
}

impl From<ShaderKind> for vk::ShaderStageFlags {
    #[inline]
    fn from(value: ShaderKind) -> Self {
        match value {
            ShaderKind::Vertex => Self::VERTEX,
            ShaderKind::Fragment => Self::FRAGMENT
        }
    }
}