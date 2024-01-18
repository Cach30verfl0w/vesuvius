pub mod config;
pub mod shader;

use crate::render::buffer::Buffer;
use crate::render::image::Image;
use crate::render::pipeline::config::PipelineConfiguration;
use crate::render::pipeline::shader::{ShaderKind, ShaderModule};
use crate::render::GameRenderer;
use crate::App;
use crate::Result;
use ash::vk;
use log::info;
use std::path::PathBuf;
use std::slice;
use std::str::FromStr;

/// This structure represents a render pipeline. The complete pipeline is re-compilable, when the
/// source code or the configuration file changes. The re-compilation feature is used by the file
/// watcher in the Game Renderer.
#[derive(Clone)]
pub struct RenderPipeline {
    shader_modules: Vec<ShaderModule>,
    application: App,
    pub(crate) vulkan_pipeline_layout: Option<vk::PipelineLayout>,
    descriptor_set_layouts: Option<Vec<(vk::DescriptorSetLayout, Vec<vk::DescriptorType>)>>,
    pub(crate) vulkan_pipeline: Option<vk::Pipeline>,
    pub(crate) name: String,
}

impl Drop for RenderPipeline {
    fn drop(&mut self) {
        let device = self.application.main_device().virtual_device();
        if let Some(descriptor_set_layouts) = self.descriptor_set_layouts.as_ref() {
            for descriptor_set_layout in descriptor_set_layouts {
                unsafe { device.destroy_descriptor_set_layout(descriptor_set_layout.0, None) };
            }
        }

        if let Some(vulkan_pipeline_layout) = self.vulkan_pipeline_layout {
            unsafe { device.destroy_pipeline_layout(vulkan_pipeline_layout, None) };
        }

        if let Some(vulkan_pipeline) = self.vulkan_pipeline {
            unsafe { device.destroy_pipeline(vulkan_pipeline, None) };
        }
    }
}

impl RenderPipeline {
    pub(crate) fn new(application: App, pipeline_config: PipelineConfiguration) -> Result<Self> {
        // Create shader from file
        let mut shader_modules = Vec::new();
        for shader_configuration in pipeline_config.shader.iter() {
            // Get shader path and validate
            let shader_path = PathBuf::from_str(&shader_configuration.resource).unwrap();
            if !shader_path.exists() || !shader_path.is_file() {
                panic!(
                    "Unable to create shader module => The path '{}' doesn't points to a file",
                    shader_path.to_str().unwrap()
                );
            }

            // Push shader into list
            shader_modules.push(ShaderModule {
                application: application.clone(),
                shader_source_path: shader_path,
                vulkan_shader_module: None,
                kind: shader_configuration.kind,
                shader_ir_code: Vec::new(),
            })
        }
        info!(
            "Internally created '{}' render pipeline with {} shaders",
            pipeline_config.name,
            shader_modules.len()
        );

        Ok(Self {
            application,
            shader_modules,
            descriptor_set_layouts: None,
            vulkan_pipeline_layout: None,
            vulkan_pipeline: None,
            name: pipeline_config.name,
        })
    }

    pub fn compile(&mut self) -> Result<()> {
        let window_size = self.application.window().inner_size();
        let device = self.application.main_device().virtual_device();

        for shader in self.shader_modules.iter_mut() {
            shader.compile()?;
        }

        // Viewport and scissor
        let viewport = vk::Viewport::default()
            .x(0.0)
            .y(0.0)
            .width(window_size.width as f32)
            .height(window_size.height as f32)
            .min_depth(0.0)
            .max_depth(1.0);
        let scissor = vk::Rect2D::default().extent(vk::Extent2D {
            width: window_size.width,
            height: window_size.height,
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
            .line_width(1.0);
        let multisample_stage_create_info = vk::PipelineMultisampleStateCreateInfo::default()
            .rasterization_samples(vk::SampleCountFlags::TYPE_1)
            .sample_shading_enable(false)
            .alpha_to_coverage_enable(false)
            .alpha_to_one_enable(false);

        // Color Blend infos
        let pipeline_color_blend_attachment_info = vk::PipelineColorBlendAttachmentState::default()
            .blend_enable(true)
            .color_write_mask(vk::ColorComponentFlags::RGBA);
        let pipeline_color_blend_state_create_info =
            vk::PipelineColorBlendStateCreateInfo::default()
                .attachments(slice::from_ref(&pipeline_color_blend_attachment_info));

        // Create descriptor sets and pipeline layout
        let mut descriptor_sets = Vec::new();
        for shader in self.shader_modules.iter() {
            for descriptor_set in shader.create_descriptor_sets() {
                let descriptor_set_layout_info = vk::DescriptorSetLayoutCreateInfo::default()
                    .bindings(descriptor_set.as_slice());
                let descriptor_set_layout = unsafe {
                    device.create_descriptor_set_layout(&descriptor_set_layout_info, None)
                }?;
                descriptor_sets.push((
                    descriptor_set_layout,
                    descriptor_set
                        .iter()
                        .map(|desc| desc.descriptor_type)
                        .collect(),
                ));
            }
        }

        let raw_descriptor_sets = descriptor_sets
            .iter()
            .map(|value| value.0)
            .collect::<Vec<_>>();
        let layout_create_info =
            vk::PipelineLayoutCreateInfo::default().set_layouts(raw_descriptor_sets.as_slice());
        let layout = unsafe { device.create_pipeline_layout(&layout_create_info, None) }?;

        // Create pipeline with recompiled shader modules
        let mut pipeline_rendering_create_info = vk::PipelineRenderingCreateInfo::default()
            .color_attachment_formats(&[vk::Format::B8G8R8A8_UNORM]);
        let dynamic_state_create_info = vk::PipelineDynamicStateCreateInfo::default();
        let input_assembly_state_create_info = vk::PipelineInputAssemblyStateCreateInfo::default()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST) // Weather draw the stuff as triangles, lines etc.
            .primitive_restart_enable(false); // Ignore lol

        // Configure pipeline input state
        let vertex_shader = self
            .shader_modules
            .iter()
            .find(|module| module.kind == ShaderKind::Vertex)
            .unwrap();
        let (input_attrs, binding_desc) = vertex_shader.reflect_input_attributes();

        let vertex_input_state_create_info = vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_attribute_descriptions(input_attrs.as_slice())
            .vertex_binding_descriptions(slice::from_ref(&binding_desc));

        // Create pipeline with recompiled shader modules
        let stages = self
            .shader_modules
            .iter()
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
        if let Some(descriptor_set_layouts) = self.descriptor_set_layouts.as_ref() {
            for descriptor_set_layout in descriptor_set_layouts {
                unsafe { device.destroy_descriptor_set_layout(descriptor_set_layout.0, None) };
            }
        }

        if let Some(old_pipeline) = self.vulkan_pipeline {
            unsafe { device.destroy_pipeline(old_pipeline, None) };
        }

        if let Some(old_layout_handle) = self.vulkan_pipeline_layout {
            unsafe { device.destroy_pipeline_layout(old_layout_handle, None) };
        }

        // Replace old handles with new handles
        self.descriptor_set_layouts = Some(descriptor_sets);
        self.vulkan_pipeline_layout = Some(layout);
        self.vulkan_pipeline = Some(
            unsafe {
                device.create_graphics_pipelines(
                    vk::PipelineCache::null(),
                    slice::from_ref(&graphics_pipeline_create_info),
                    None,
                )
            }
            .unwrap()[0],
        );
        Ok(())
    }
}

#[derive(Clone)]
pub struct DescriptorSet {
    pub(crate) vk_descriptor_set: vk::DescriptorSet,
    renderer: GameRenderer,
    binding_types: Vec<vk::DescriptorType>,
}

impl Drop for DescriptorSet {
    fn drop(&mut self) {
        unsafe {
            self.renderer
                .0
                .application
                .main_device()
                .virtual_device()
                .free_descriptor_sets(
                    self.renderer.0.descriptor_pool,
                    slice::from_ref(&self.vk_descriptor_set),
                )
                .expect("Unable to free descriptor set");
        }
    }
}

impl DescriptorSet {
    pub fn allocate(renderer: &GameRenderer, pipeline: &str, set_index: usize) -> Result<Self> {
        let found_pipeline = renderer
            .find_pipeline(pipeline)
            .unwrap_or_else(|| panic!("Invalid pipeline name '{}'", pipeline));
        let (descriptor_set, binding_types) = found_pipeline
            .descriptor_set_layouts
            .as_ref()
            .unwrap()
            .get(set_index)
            .unwrap_or_else(|| {
                panic!(
                    "Unable to find descriptor set by index '{}' in pipeline '{}'",
                    set_index, pipeline
                )
            });

        let descriptor_set_allocate_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(renderer.0.descriptor_pool)
            .set_layouts(slice::from_ref(descriptor_set));
        let device = renderer.0.application.main_device();
        let descriptor_set = unsafe {
            device
                .virtual_device()
                .allocate_descriptor_sets(&descriptor_set_allocate_info)
        }?[0];

        Ok(Self {
            vk_descriptor_set: descriptor_set,
            renderer: renderer.clone(),
            binding_types: binding_types.clone(),
        })
    }
}

pub trait WriteDescriptorSet {
    fn write_to_set(&self, descriptor_set: &DescriptorSet, binding: u32);
}

impl WriteDescriptorSet for Buffer {
    fn write_to_set(&self, descriptor_set: &DescriptorSet, binding: u32) {
        let descriptor_buffer_info = vk::DescriptorBufferInfo::default()
            .buffer(self.buffer)
            .range(vk::WHOLE_SIZE);
        let write_descriptor_set = vk::WriteDescriptorSet::default()
            .descriptor_count(1)
            .descriptor_type(descriptor_set.binding_types[binding as usize])
            .buffer_info(slice::from_ref(&descriptor_buffer_info))
            .dst_set(descriptor_set.vk_descriptor_set)
            .dst_binding(binding);

        unsafe {
            descriptor_set
                .renderer
                .0
                .application
                .main_device()
                .virtual_device()
                .update_descriptor_sets(slice::from_ref(&write_descriptor_set), &[]);
        }
    }
}

impl WriteDescriptorSet for Image {
    fn write_to_set(&self, descriptor_set: &DescriptorSet, binding: u32) {
        let descriptor_image_info = vk::DescriptorImageInfo::default()
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .image_view(self.image_view)
            .sampler(self.sampler);
        let write_descriptor_set = vk::WriteDescriptorSet::default()
            .dst_set(descriptor_set.vk_descriptor_set)
            .dst_binding(binding)
            .dst_array_element(0)
            .descriptor_type(descriptor_set.binding_types[binding as usize])
            .image_info(slice::from_ref(&descriptor_image_info));

        unsafe {
            descriptor_set
                .renderer
                .0
                .application
                .main_device()
                .virtual_device()
                .update_descriptor_sets(slice::from_ref(&write_descriptor_set), &[]);
        }
    }
}
