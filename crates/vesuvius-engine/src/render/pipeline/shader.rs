use crate::error::Error;
use crate::App;
use crate::Result;
use ash::vk;
use serde::{Deserialize, Serialize};
use shaderc::{CompileOptions, Compiler};
use spirv_reflect::types::{ReflectDescriptorType, ReflectFormat};
use std::ffi::CStr;
use std::fs;
use std::path::PathBuf;

/// This structure represents a shader module. This shader module is re-compilable, when the source
/// code of the shader changes. The re-compilation features is used by the render pipeline while
/// rebuilding the pipeline.
#[derive(Clone)]
pub(crate) struct ShaderModule {
    /// Reference to the internal application
    pub(crate) application: App,

    /// This field contains the path to the shader source file in the assets folder
    pub(crate) shader_source_path: PathBuf,

    /// The SPIR-V IR code of the compiled shader
    pub(crate) shader_ir_code: Vec<u8>,

    /// This field contains the handle of the compiled shader module
    pub(crate) vulkan_shader_module: Option<vk::ShaderModule>,

    /// This field contains the kind of the shader (like fragment or vertex)
    pub(crate) kind: ShaderKind,
}

impl Drop for ShaderModule {
    fn drop(&mut self) {
        unsafe {
            if let Some(shader_module) = self.vulkan_shader_module {
                self.application
                    .main_device()
                    .virtual_device()
                    .destroy_shader_module(shader_module, None);
            }
        }
    }
}

/// Convert reference of shader module into [vk::PipelineShaderStageCreateInfo]
impl From<&ShaderModule> for vk::PipelineShaderStageCreateInfo<'_> {
    fn from(value: &ShaderModule) -> Self {
        vk::PipelineShaderStageCreateInfo::default()
            .stage(value.kind.into())
            .module(value.vulkan_shader_module.unwrap())
            .name(unsafe { CStr::from_ptr(b"main\0".as_ptr().cast()) })
    }
}

impl ShaderModule {
    pub(crate) fn compile(&mut self) -> Result<()> {
        let file_content = String::from_utf8(fs::read(&self.shader_source_path)?)?;
        let file_name = self
            .shader_source_path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap();

        // Compile Shader
        let compiler = Compiler::new().ok_or(Error::CompilerCreation)?;
        let options = CompileOptions::new().ok_or(Error::CompilerCreation)?;
        let result = compiler.compile_into_spirv(
            &file_content,
            self.kind.into(),
            file_name,
            "main",
            Some(&options),
        )?;
        self.shader_ir_code = result.as_binary_u8().to_vec();

        // Create shader
        let device = self.application.main_device().virtual_device();
        if let Some(old_shader_module) = self.vulkan_shader_module {
            unsafe { device.destroy_shader_module(old_shader_module, None) };
        }

        let shader_module_create_info =
            vk::ShaderModuleCreateInfo::default().code(result.as_binary());
        let shader = unsafe { device.create_shader_module(&shader_module_create_info, None) }?;
        self.vulkan_shader_module = Some(shader);
        Ok(())
    }

    pub(crate) fn reflect_input_attributes(
        &self,
    ) -> (
        Vec<vk::VertexInputAttributeDescription>,
        vk::VertexInputBindingDescription,
    ) {
        let reflected_module =
            spirv_reflect::create_shader_module(self.shader_ir_code.as_slice()).unwrap();
        let mut input_attributes = Vec::new();
        let mut offset = 0;

        for input_variable in reflected_module.enumerate_input_variables(None).unwrap() {
            input_attributes.push(
                vk::VertexInputAttributeDescription::default()
                    .location(input_variable.location)
                    .format(reflect_to_vulkan_format(input_variable.format))
                    .offset(offset),
            );
            offset += reflect_format_to_offset(input_variable.format);
        }

        (
            input_attributes,
            vk::VertexInputBindingDescription::default()
                .stride(offset)
                .input_rate(vk::VertexInputRate::VERTEX),
        )
    }

    pub(crate) fn create_descriptor_sets(&self) -> Vec<Vec<vk::DescriptorSetLayoutBinding>> {
        let reflected_module =
            spirv_reflect::create_shader_module(self.shader_ir_code.as_slice()).unwrap();

        let mut vulkan_descriptor_sets = Vec::new();
        for descriptor_set in reflected_module
            .enumerate_descriptor_sets(Some("main"))
            .unwrap()
            .iter()
        {
            let mut descriptor_set_bindings = Vec::new();
            for descriptor_binding in &descriptor_set.bindings {
                let descriptor_set_layout_binding = vk::DescriptorSetLayoutBinding::default()
                    .descriptor_type(reflect_to_vulkan_descriptor_type(
                        descriptor_binding.descriptor_type,
                    ))
                    .binding(descriptor_binding.binding)
                    .descriptor_count(descriptor_binding.count)
                    .stage_flags(self.kind.into());
                descriptor_set_bindings.push(descriptor_set_layout_binding);
            }
            vulkan_descriptor_sets.push(descriptor_set_bindings);
        }
        vulkan_descriptor_sets
    }
}

/// This enum represents all supported kinds of shader in the Vesuvius game engine. Currently only
/// vertex and fragment shader are supported, because we only need them now.
#[derive(Serialize, Deserialize, Clone, Copy, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub(crate) enum ShaderKind {
    #[serde(rename = "fragment")]
    Fragment,
    #[serde(rename = "vertex")]
    Vertex,
}

/// Convert own shader kind into [shaderc::ShaderKind] of the shaderc crate
impl From<ShaderKind> for shaderc::ShaderKind {
    #[inline]
    fn from(value: ShaderKind) -> Self {
        match value {
            ShaderKind::Vertex => Self::Vertex,
            ShaderKind::Fragment => Self::Fragment,
        }
    }
}

/// Convert own shader kind into [vk::ShaderStageFlags] of the vulkan crate
impl From<ShaderKind> for vk::ShaderStageFlags {
    #[inline]
    fn from(value: ShaderKind) -> Self {
        match value {
            ShaderKind::Vertex => Self::VERTEX,
            ShaderKind::Fragment => Self::FRAGMENT,
        }
    }
}

const fn reflect_to_vulkan_descriptor_type(
    descriptor_type: ReflectDescriptorType,
) -> vk::DescriptorType {
    match descriptor_type {
        ReflectDescriptorType::Undefined => panic!("Unable to convert undefined descriptor type"),
        ReflectDescriptorType::Sampler => vk::DescriptorType::SAMPLER,
        ReflectDescriptorType::CombinedImageSampler => vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
        ReflectDescriptorType::SampledImage => vk::DescriptorType::SAMPLED_IMAGE,
        ReflectDescriptorType::StorageImage => vk::DescriptorType::STORAGE_IMAGE,
        ReflectDescriptorType::UniformTexelBuffer => vk::DescriptorType::UNIFORM_TEXEL_BUFFER,
        ReflectDescriptorType::StorageTexelBuffer => vk::DescriptorType::STORAGE_TEXEL_BUFFER,
        ReflectDescriptorType::UniformBuffer => vk::DescriptorType::UNIFORM_BUFFER,
        ReflectDescriptorType::StorageBuffer => vk::DescriptorType::STORAGE_BUFFER,
        ReflectDescriptorType::UniformBufferDynamic => vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC,
        ReflectDescriptorType::StorageBufferDynamic => vk::DescriptorType::STORAGE_BUFFER_DYNAMIC,
        ReflectDescriptorType::InputAttachment => vk::DescriptorType::INPUT_ATTACHMENT,
        ReflectDescriptorType::AccelerationStructureNV => {
            vk::DescriptorType::ACCELERATION_STRUCTURE_NV
        }
    }
}

/// This function converts the format, provided by the spirv-reflect crate, into the vulkan equivalent
#[inline]
const fn reflect_to_vulkan_format(format: ReflectFormat) -> vk::Format {
    match format {
        ReflectFormat::Undefined => vk::Format::UNDEFINED,
        ReflectFormat::R32_UINT => vk::Format::R32_UINT,
        ReflectFormat::R32_SINT => vk::Format::R32_SINT,
        ReflectFormat::R32_SFLOAT => vk::Format::R32_SFLOAT,
        ReflectFormat::R32G32_UINT => vk::Format::R32G32_UINT,
        ReflectFormat::R32G32_SINT => vk::Format::R32G32_UINT,
        ReflectFormat::R32G32_SFLOAT => vk::Format::R32G32_SFLOAT,
        ReflectFormat::R32G32B32_UINT => vk::Format::R32G32B32_UINT,
        ReflectFormat::R32G32B32_SINT => vk::Format::R32G32B32_SINT,
        ReflectFormat::R32G32B32_SFLOAT => vk::Format::R32G32B32_SFLOAT,
        ReflectFormat::R32G32B32A32_UINT => vk::Format::R32G32B32A32_UINT,
        ReflectFormat::R32G32B32A32_SINT => vk::Format::R32G32B32A32_SINT,
        ReflectFormat::R32G32B32A32_SFLOAT => vk::Format::R32G32B32A32_SFLOAT,
    }
}

/// This function returns the size of the specified format
#[inline]
const fn reflect_format_to_offset(format: ReflectFormat) -> u32 {
    match format {
        ReflectFormat::Undefined => 0,
        ReflectFormat::R32_UINT | ReflectFormat::R32_SINT | ReflectFormat::R32_SFLOAT => 4,
        ReflectFormat::R32G32_UINT | ReflectFormat::R32G32_SINT | ReflectFormat::R32G32_SFLOAT => 8,
        ReflectFormat::R32G32B32_UINT | ReflectFormat::R32G32B32_SINT => 12,
        ReflectFormat::R32G32B32_SFLOAT => 12,
        ReflectFormat::R32G32B32A32_UINT | ReflectFormat::R32G32B32A32_SINT => 16,
        ReflectFormat::R32G32B32A32_SFLOAT => 16,
    }
}
