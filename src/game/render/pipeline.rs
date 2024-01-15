use std::ffi::CStr;
use std::{fs, slice};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use ash::vk;
use log::info;
use serde::{Deserialize, Serialize};
use shaderc::{CompileOptions, Compiler};
use spirv_reflect::types::ReflectFormat;
use crate::game::error::EngineError;
use crate::game::{Game, Result};

/// This structure represents a render pipeline. The complete pipeline is re-compilable, when the
/// source code or the configuration file changes. The re-compilation feature is used by the file
/// watcher in the Game Renderer.
#[derive(Clone, PartialOrd, PartialEq, Debug)]
pub(crate) struct RenderPipeline {
    /// All shader modules, which are used for the compilation of the pipeline
    pub(crate) shader_modules: Vec<ShaderModule>,

    /// The handle of the compiled graphics pipeline
    pub(crate) vulkan_pipeline: Option<vk::Pipeline>,

    /// The handle of the compiled graphics pipeline handle
    pub(crate) vulkan_pipeline_layout: Option<vk::PipelineLayout>,

    /// The configuration of the rasterization state
    rasterizer_configuration: RasterizerConfiguration,

    /// The name of the pipeline for querying etc.
    pub(crate) name: String
}

impl RenderPipeline {

    /// This function reads the pipeline configuration file and builds the complete pipeline with
    /// the shaders.
    pub(crate) fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        if !path.is_file() {
            panic!("Unable to create render pipeline => The path '{}' doesn't points to a file",
                   path.to_str().unwrap());
        }

        // Read configuration from file
        let file_content = String::from_utf8(fs::read(path)?)?;
        let pipeline_configuration = serde_json::from_str::<PipelineConfiguration>(&file_content)
            .expect("Illegal pipeline configuration file specified");

        // Create shader from file
        let mut shader_modules = Vec::new();
        for shader_configuration in pipeline_configuration.shader.iter() {
            // Get shader path and validate
            let shader_path = PathBuf::from_str(&shader_configuration.file).unwrap();
            if !shader_path.exists() || !shader_path.is_file() {
                panic!("Unable to create shader module => The path '{}' doesn't points to a file",
                       shader_path.to_str().unwrap());
            }

            // Push shader into list
            shader_modules.push(ShaderModule {
                shader_source_path: shader_path,
                vulkan_shader_module: None,
                kind: shader_configuration.kind,
                shader_ir_code: Vec::new()
            })
        }
        info!("Internally created '{}' render pipeline with {} shaders",
            pipeline_configuration.name, shader_modules.len());

        // Return shader list
        Ok(Self {
            rasterizer_configuration: pipeline_configuration.rasterizer,
            vulkan_pipeline: None,
            vulkan_pipeline_layout: None,
            shader_modules,
            name: pipeline_configuration.name,
        })
    }

    pub(crate) fn compile(&mut self, game: &Game) -> Result<()> {
        let window_size = game.window().inner_size();
        let device = game.device().virtual_device();

        // Recompile all shader
        for shader_module in self.shader_modules.iter_mut() {
            shader_module.compile(game)?;
        }

        // Viewport and scissor
        let viewport = vk::Viewport::default().x(0.0).y(0.0)
            .width(window_size.width as f32)
            .height(window_size.height as f32)
            .min_depth(0.0)
            .max_depth(1.0);
        let scissor = vk::Rect2D::default().extent(vk::Extent2D {
            width: window_size.width,
            height: window_size.height
        });
        let viewport_state_create_info = vk::PipelineViewportStateCreateInfo::default()
            .scissors(slice::from_ref(&scissor))
            .viewports(slice::from_ref(&viewport));

        // Some stage infos
        let rasterization_stage_create_info = vk::PipelineRasterizationStateCreateInfo::default()
            .rasterizer_discard_enable(false)
            .depth_clamp_enable(false)
            .polygon_mode(vk::PolygonMode::FILL) // TODO: Read from config
            .cull_mode(vk::CullModeFlags::NONE)
            .front_face(vk::FrontFace::CLOCKWISE)
            .depth_bias_enable(false)
            .line_width(self.rasterizer_configuration.line_width);
        let multisample_stage_create_info = vk::PipelineMultisampleStateCreateInfo::default()
            .rasterization_samples(vk::SampleCountFlags::TYPE_1)
            .sample_shading_enable(false)
            .alpha_to_coverage_enable(false)
            .alpha_to_one_enable(false);

        // Color Blend infos
        let pipeline_color_blend_attachment_info = vk::PipelineColorBlendAttachmentState::default()
            .color_write_mask(vk::ColorComponentFlags::RGBA);
        let pipeline_color_blend_state_create_info = vk::PipelineColorBlendStateCreateInfo::default()
            .attachments(slice::from_ref(&pipeline_color_blend_attachment_info));

        // Create pipeline layout
        let layout_create_info = vk::PipelineLayoutCreateInfo::default();
        let layout = unsafe { game.device().virtual_device()
            .create_pipeline_layout(&layout_create_info, None) }?;

        // Configure pipeline input
        let vertex_shader = self.shader_modules.iter()
            .find(|module| module.kind == ShaderKind::Vertex).unwrap();
        let (input_attrs, binding_desc) = vertex_shader.reflect_input_attributes();

        let vertex_input_state_create_info = vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_attribute_descriptions(input_attrs.as_slice())
            .vertex_binding_descriptions(slice::from_ref(&binding_desc));

        // Create pipeline with recompiled shader modules
        let mut pipeline_rendering_create_info = vk::PipelineRenderingCreateInfo::default()
            .color_attachment_formats(&[vk::Format::B8G8R8A8_UNORM]);
        let dynamic_state_create_info = vk::PipelineDynamicStateCreateInfo::default();
        let input_assembly_state_create_info = vk::PipelineInputAssemblyStateCreateInfo::default()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST) // Weather draw the stuff as triangles, lines etc.
            .primitive_restart_enable(false); // Ignore lol

        let stages = self.shader_modules.iter()
            .map(|module| module.into())
            .collect::<Vec<_>>();
        let graphics_pipeline_create_info = vk::GraphicsPipelineCreateInfo::default()
            .push_next(&mut pipeline_rendering_create_info)
            .vertex_input_state(&vertex_input_state_create_info)
            .input_assembly_state(&input_assembly_state_create_info)
            .color_blend_state(&pipeline_color_blend_state_create_info)
            .rasterization_state(&rasterization_stage_create_info)
            .multisample_state(&multisample_stage_create_info)
            .viewport_state(&viewport_state_create_info)
            .dynamic_state(&dynamic_state_create_info)
            .stages(stages.as_slice())
            .base_pipeline_handle(vk::Pipeline::null())
            .layout(layout);

        // Destroy old handles in memory
        if let Some(old_pipeline) = self.vulkan_pipeline {
            unsafe { device.destroy_pipeline(old_pipeline, None) };
        }

        if let Some(old_layout_handle) = self.vulkan_pipeline_layout {
            unsafe { device.destroy_pipeline_layout(old_layout_handle, None) };
        }

        // Replace old handles with new handles
        self.vulkan_pipeline_layout = Some(layout);
        self.vulkan_pipeline = Some(unsafe {
            device.create_graphics_pipelines(
                vk::PipelineCache::null(),
                slice::from_ref(&graphics_pipeline_create_info),
                None
            )
        }.unwrap()[0]);

        Ok(())
    }

}

/// This structure represents a shader module. This shader module is re-compilable, when the source
/// code of the shader changes. The re-compilation features is used by the render pipeline while
/// rebuilding the pipeline.
#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub(crate) struct ShaderModule {
    /// This field contains the path to the shader source file in the assets folder
    shader_source_path: PathBuf,

    /// The SPIR-V IR code of the compiled shader
    shader_ir_code: Vec<u8>,

    /// This field contains the handle of the compiled shader module
    pub(crate) vulkan_shader_module: Option<vk::ShaderModule>,

    /// This field contains the kind of the shader (like fragment or vertex)
    kind: ShaderKind
}

impl From<&ShaderModule> for vk::PipelineShaderStageCreateInfo<'_> {
    fn from(value: &ShaderModule) -> Self {
        vk::PipelineShaderStageCreateInfo::default()
            .stage(value.kind.into())
            .module(value.vulkan_shader_module.unwrap())
            .name(unsafe { CStr::from_ptr(b"main\0".as_ptr().cast()) })
    }
}

impl ShaderModule {

    pub(crate) fn compile(&mut self, game: &Game) -> Result<()> {
        let file_content = String::from_utf8(fs::read(&self.shader_source_path)?)?;
        let file_name = self.shader_source_path.file_name().unwrap().to_str().unwrap();

        // Compile Shader
        let compiler = Compiler::new().ok_or(EngineError::CompilerCreation)?;
        let options = CompileOptions::new().ok_or(EngineError::CompilerCreation)?;
        let result = compiler.compile_into_spirv(&file_content, self.kind.into(), file_name,
                                                 "main", Some(&options))?;
        self.shader_ir_code = result.as_binary_u8().to_vec();

        // Create shader
        let device = game.device().virtual_device();
        if let Some(old_shader_module) = self.vulkan_shader_module {
            unsafe { device.destroy_shader_module(old_shader_module, None) };
        }

        let shader_module_create_info = vk::ShaderModuleCreateInfo::default()
            .code(result.as_binary());
        let shader = unsafe { device.create_shader_module(&shader_module_create_info, None) }?;
        self.vulkan_shader_module = Some(shader);
        Ok(())
    }

    pub(crate) fn reflect_input_attributes(&self) -> (Vec<vk::VertexInputAttributeDescription>,
                                                      vk::VertexInputBindingDescription) {
        let reflected_module = spirv_reflect::create_shader_module(self.shader_ir_code.as_slice())
            .unwrap();

        let mut input_attributes = Vec::new();
        let mut offset = 0;
        for input_variable in reflected_module.enumerate_input_variables(None).unwrap() {
            input_attributes.push(vk::VertexInputAttributeDescription::default()
                .location(input_variable.location)
                .format(reflect_to_vulkan_format(input_variable.format))
                .offset(offset));
            offset += reflect_format_to_offset(input_variable.format);
        }
        (
            input_attributes,
            vk::VertexInputBindingDescription::default()
                .stride(offset)
                .input_rate(vk::VertexInputRate::VERTEX)
        )
    }

}

/// This enum represents all supported kinds of shader in the Vesuvius game engine. Currently only
/// vertex and fragment shader are supported, because we only need them now.
#[derive(Serialize, Deserialize, Clone, Copy, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
enum ShaderKind {
    #[serde(rename = "fragment")]
    Fragment,
    #[serde(rename = "vertex")]
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

/// This struct represents the main configuration structure as json config wrapper for the pipeline
/// configuration
#[derive(Serialize, Deserialize)]
struct PipelineConfiguration {
    /// The system-internal name of the pipeline
    name: String,

    /// The list of shader with configuration
    shader: Vec<ShaderConfiguration>,

    /// A configuration section for the rasterization state in the pipeline
    rasterizer: RasterizerConfiguration
}

#[derive(Serialize, Deserialize)]
struct ShaderConfiguration {
    file: String,
    kind: ShaderKind
}

#[derive(Serialize, Deserialize, Clone, PartialOrd, PartialEq, Debug)]
struct RasterizerConfiguration {
    polygon_mode: String,
    line_width: f32
}

#[inline]
fn reflect_to_vulkan_format(format: ReflectFormat) -> vk::Format {
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
        ReflectFormat::R32G32B32A32_SFLOAT => vk::Format::R32G32B32A32_SFLOAT
    }
}

#[inline]
fn reflect_format_to_offset(format: ReflectFormat) -> u32 {
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

fn memory_type_index(game: &Game, type_filter: u32, props: vk::MemoryPropertyFlags) -> u32 {
    let mem_props = unsafe {
        game.0.instance.get_physical_device_memory_properties(*game.device().physical_device())
    };
    for i in 0..mem_props.memory_type_count as usize {
        if (type_filter & (1 << i)) != 1 && !(mem_props.memory_types[i].property_flags & props)
            .is_empty() {
            return i as u32;
        }
    }
    panic!("No support ig... ._.")
}