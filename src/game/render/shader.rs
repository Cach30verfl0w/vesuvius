use std::borrow::Cow;
use std::fs;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use ash::vk;
use shaderc::{CompileOptions, Compiler, ShaderKind};
use crate::game::device::WrappedDevice;
use crate::game::error::EngineError;

use crate::game::Result;

pub struct Shader {
    watch_file_path: Option<PathBuf>,
    source_code: String,
    vk_shader_module: Option<vk::ShaderModule>,
}

impl Deref for Shader {
    type Target = vk::ShaderModule;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.vk_shader_module.as_ref().unwrap()
    }
}

impl<'a> Shader {

    pub fn from_file<P: AsRef<Path>>(path: P) -> Self {
        Self {
            watch_file_path: Some(path.as_ref().to_path_buf()),
            source_code: String::new(),
            vk_shader_module: None,
        }
    }

    pub fn from_source_code(source_code: Cow<'a, str>) -> Self {
        Self {
            source_code: source_code.to_string(),
            watch_file_path: None,
            vk_shader_module: None
        }
    }

    pub fn update(&mut self, device: &WrappedDevice) -> Result<()> {
        if let Some(file_path) = self.watch_file_path.as_ref() {
            // Read file content and name
            let file_content = String::from_utf8(fs::read(file_path)?)?;
            let file_name = file_path.file_name().unwrap().to_str().unwrap();
            self.source_code = file_content.clone();

            // Compile shader into SpirV code
            let compiler = Compiler::new().ok_or(EngineError::CompilerCreation)?;
            let compiler_options = CompileOptions::new().ok_or(EngineError::CompilerCreation)?;
            let compile_result = compiler.compile_into_spirv(file_content.as_str(), ShaderKind::Vertex, file_name,
                                                            "main", Some(&compiler_options))?;

            // Compile into Vulkan shader module
            if self.vk_shader_module.is_some() {
                unsafe { device.virtual_device.destroy_shader_module(self.vk_shader_module.unwrap(), None) };
            }

            let shader_module_create_info = vk::ShaderModuleCreateInfo::default().code(compile_result.as_binary());
            self.vk_shader_module = Some(unsafe {
                device.virtual_device.create_shader_module(&shader_module_create_info, None)
            }?);
        }
        Ok(())
    }

}