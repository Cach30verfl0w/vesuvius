pub mod pipeline;
pub mod buffer;

use std::{fs, mem, slice};
use std::collections::HashMap;
use ash::extensions::khr::{Surface, Swapchain};
use ash::vk;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use render::buffer::Buffer;
use render::pipeline::config::PipelineConfiguration;

use render::pipeline::RenderPipeline;
use crate::App;
use crate::Result;

#[derive(Clone)]
pub struct GameRenderer {
    application: App,
    surface: vk::SurfaceKHR,

    // Images
    images: Vec<vk::Image>,
    image_views: Vec<vk::ImageView>,
    current_image_index: u32,

    // Swapchain
    swapchain_loader: Swapchain,
    swapchain: vk::SwapchainKHR,

    // Command Pool and Buffer
    command_pool: vk::CommandPool,
    command_buffer: vk::CommandBuffer,

    // Semaphores
    submit_semaphore: vk::Semaphore,
    present_semaphore: vk::Semaphore,

    // Other things
    queue: vk::Queue,
    pipelines: Vec<RenderPipeline>,
    descriptor_pool: Option<vk::DescriptorPool>
}

impl Drop for GameRenderer {
    fn drop(&mut self) {
        let device = self.application.main_device().virtual_device();
        let surface_loader = Surface::new(self.application.entry(), self.application.instance());
        unsafe {
            device.destroy_semaphore(self.submit_semaphore, None);
            device.destroy_semaphore(self.present_semaphore, None);
            for image_view in self.image_views.iter() {
                device.destroy_image_view(*image_view, None);
            }

            self.swapchain_loader.destroy_swapchain(self.swapchain, None);
            surface_loader.destroy_surface(self.surface, None);
            device.free_command_buffers(self.command_pool, slice::from_ref(&self.command_buffer));
            device.destroy_command_pool(self.command_pool, None);
        }
    }
}

impl GameRenderer {

    pub fn new(application: App) -> Result<Self> {
        let device = application.main_device().virtual_device();
        let window = application.window();
        let surface = unsafe {
            ash_window::create_surface(application.entry(), application.instance(), window.raw_display_handle(),
                                       window.raw_window_handle(), None)
        }?;


        // Create swapchain
        let swapchain_loader = Swapchain::new(application.instance(), device);
        let swapchain_create_info = vk::SwapchainCreateInfoKHR::default()
            .surface(surface)
            .min_image_count(2)
            .image_format(vk::Format::B8G8R8A8_UNORM)
            .image_color_space(vk::ColorSpaceKHR::SRGB_NONLINEAR)
            .image_extent(vk::Extent2D { width: window.inner_size().width, height: window.inner_size().height })
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(vk::SurfaceTransformFlagsKHR::IDENTITY)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(vk::PresentModeKHR::FIFO);
        let swapchain = unsafe { swapchain_loader.create_swapchain(&swapchain_create_info, None) }?;

        // Create image views
        let images = unsafe { swapchain_loader.get_swapchain_images(swapchain) }?;
        let image_views = images.iter().map(|image| {
            let image_view_create_info = vk::ImageViewCreateInfo::default()
                .image(*image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(vk::Format::B8G8R8A8_UNORM)
                .components(vk::ComponentMapping::default())
                .subresource_range(vk::ImageSubresourceRange::default().aspect_mask(vk::ImageAspectFlags::COLOR)
                    .layer_count(1).level_count(1));
            unsafe { device.create_image_view(&image_view_create_info, None) }.unwrap()
        }).collect::<Vec<_>>();

        // Command Pool and Command Buffer
        let command_pool_create_info = vk::CommandPoolCreateInfo::default()
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER) // Reset at begin
            .queue_family_index(0);
        let command_pool = unsafe { device.create_command_pool(&command_pool_create_info, None) }?;

        let command_buffer_alloc_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(command_pool)
            .command_buffer_count(1);
        let command_buffer = unsafe { device.allocate_command_buffers(&command_buffer_alloc_info) }?[0];

        // Return game renderer to caller
        Ok(Self {
            submit_semaphore: unsafe { device.create_semaphore(&vk::SemaphoreCreateInfo::default(), None) }?,
            present_semaphore: unsafe { device.create_semaphore(&vk::SemaphoreCreateInfo::default(), None) }?,
            queue: unsafe { device.get_device_queue(0, 0) },
            swapchain_loader,
            swapchain,
            images,
            image_views,
            command_pool,
            command_buffer,
            current_image_index: 0,
            application,
            surface,
            pipelines: Vec::new(),
            descriptor_pool: None
        })
    }

    pub fn reload(&mut self) -> Result<()> {
        // (Re)compile pipelines
        for pipeline_configurations in fs::read_dir("assets/pipelines").expect("Unable to find pipeline configs") {
            // Filter invalid configuration files
            let config_file = pipeline_configurations.unwrap().path();
            if !config_file.file_name().unwrap().to_str().unwrap().ends_with(".json") {
                continue;
            }

            // Create pipeline or recompile it
            let file_content = String::from_utf8(fs::read(&config_file)?)?;
            let pipeline_config: PipelineConfiguration = serde_json::from_str(&file_content)
                .expect("Unable to read pipeline configuration");
            match self.pipelines.iter_mut().find(|pipeline| pipeline.name == pipeline_config.name) {
                Some(pipeline) => pipeline.compile()?, // TODO: Reload only if changes are detected
                None => {
                    let mut pipeline = RenderPipeline::new(self.application.clone(), pipeline_config)?;
                    pipeline.compile()?;
                    self.pipelines.push(pipeline);
                }
            }
        }

        // (Re)create descriptor pool on reflection information of shaders in pipeline
        let device = self.application.main_device().virtual_device();
        if let Some(descriptor_pool) = self.descriptor_pool {
            unsafe {
                device.destroy_descriptor_pool(descriptor_pool, None);
            }
        }

        let descriptor_pool_sizes = self.pipelines.iter()
            .map(|pipeline| pipeline.get_descriptor_count())
            .flatten()
            .fold(HashMap::new(), |mut descriptor_sizes, (descriptor_type, count)| {
                *descriptor_sizes.entry(descriptor_type).or_insert(0) += count;
                descriptor_sizes
            })
            .iter()
            .map(|(descriptor_type, count)| {
                vk::DescriptorPoolSize::default()
                    .descriptor_count(*count as u32)
                    .ty(*descriptor_type)
            })
            .collect::<Vec<vk::DescriptorPoolSize>>();
        let descriptor_pool_create_info = vk::DescriptorPoolCreateInfo::default()
            .pool_sizes(descriptor_pool_sizes.as_slice())
            .max_sets(1024);

        self.descriptor_pool = Some(unsafe {
            device.create_descriptor_pool(&descriptor_pool_create_info, None)
        }?);
        Ok(())
    }

    pub fn begin(&mut self) -> Result<()> {
        self.current_image_index = unsafe {
            self.swapchain_loader.acquire_next_image(
                self.swapchain,
                u64::MAX,
                self.submit_semaphore,
                vk::Fence::null()
            )
        }?.0;

        let device = self.application.main_device().virtual_device();
        unsafe { device.reset_command_pool(self.command_pool, vk::CommandPoolResetFlags::RELEASE_RESOURCES) }?;
        unsafe { device.reset_command_buffer(self.command_buffer, vk::CommandBufferResetFlags::RELEASE_RESOURCES) }?;
        unsafe { device.begin_command_buffer(self.command_buffer, &vk::CommandBufferBeginInfo::default()) }?;

        let image_memory_barrier = vk::ImageMemoryBarrier::default()
            .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
            .old_layout(vk::ImageLayout::UNDEFINED)
            .new_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .image(self.images[self.current_image_index as usize])
            .subresource_range(vk::ImageSubresourceRange::default().aspect_mask(vk::ImageAspectFlags::COLOR).level_count(1).layer_count(1));

        unsafe {
            device.cmd_pipeline_barrier(
                self.command_buffer,
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                slice::from_ref(&image_memory_barrier)
            )
        };
        Ok(())
    }

    pub fn clear_color(&self, red: f32, green: f32, blue: f32, alpha: f32) {
        let rendering_attachment_info = vk::RenderingAttachmentInfo::default()
            .image_view(self.image_views[self.current_image_index as usize])
            .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .clear_value(vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [red, green, blue, alpha]
                }
            });

        let window_size = self.application.window().inner_size();
        let rendering_info = vk::RenderingInfo::default()
            .layer_count(1)
            .render_area(vk::Rect2D {
                offset: vk::Offset2D::default(), extent: vk::Extent2D {
                    width: window_size.width,
                    height: window_size.height
                }
            })
            .color_attachments(slice::from_ref(&rendering_attachment_info));
        unsafe {
            self.application.main_device().virtual_device().cmd_begin_rendering(self.command_buffer, &rendering_info);
        }

    }

    pub fn end(&self) -> Result<()> {
        let device = &self.application.main_device().virtual_device();
        unsafe { device.cmd_end_rendering(self.command_buffer) };

        let image_memory_barrier = vk::ImageMemoryBarrier::default()
            .src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
            .old_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
            .image(self.images[self.current_image_index as usize])
            .subresource_range(vk::ImageSubresourceRange::default().aspect_mask(vk::ImageAspectFlags::COLOR).level_count(1).layer_count(1));

        unsafe {
            device.cmd_pipeline_barrier(
                self.command_buffer,
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                slice::from_ref(&image_memory_barrier)
            )
        };

        // Move command buffer into executable state
        unsafe { device.end_command_buffer(self.command_buffer) }?;

        // Submit and present queued commands
        let submit_info = vk::SubmitInfo::default()
            .wait_semaphores(slice::from_ref(&self.submit_semaphore))
            .wait_dst_stage_mask(slice::from_ref(&vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT))
            .command_buffers(slice::from_ref(&self.command_buffer))
            .signal_semaphores(slice::from_ref(&self.present_semaphore));
        unsafe { device.queue_submit(self.queue, slice::from_ref(&submit_info), vk::Fence::null()) }?;

        let present_info = vk::PresentInfoKHR::default()
            .image_indices(slice::from_ref(&self.current_image_index))
            .wait_semaphores(slice::from_ref(&self.present_semaphore))
            .swapchains(slice::from_ref(&self.swapchain));
        unsafe { self.swapchain_loader.queue_present(self.queue, &present_info) }?;

        // Wait for finish operations
        unsafe { device.device_wait_idle() }?;
        Ok(())
    }

    pub fn bind_pipeline(&self, pipeline: &RenderPipeline) {
        unsafe {
            self.application.main_device().virtual_device().cmd_bind_pipeline(
                self.command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline.vulkan_pipeline.unwrap()
            );
        }
    }

    pub fn bind_vertex_buffer(&self, buffer: &Buffer) {
        unsafe {
            self.application.main_device().virtual_device().cmd_bind_vertex_buffers(
                self.command_buffer,
                0,
                slice::from_ref(&buffer.buffer),
                slice::from_ref(&vk::DeviceSize::from(0u32))
            );
        }
    }

    pub fn draw(&self, vertices: u32) {
        unsafe {
            self.application.main_device().virtual_device().cmd_draw(self.command_buffer, vertices, 4, 1, 0);
        }
    }

    pub fn draw_indexed(&self, index_buffer: &Buffer) {
        let device = self.application.main_device().virtual_device();
        let indices = (index_buffer.alloc_info.size / mem::size_of::<u16>() as u64) as u32;
        unsafe {
            device.cmd_bind_index_buffer(self.command_buffer, index_buffer.buffer, vk::DeviceSize::from(0u32),
                vk::IndexType::UINT16);
            device.cmd_draw_indexed(self.command_buffer, indices, 1, 0, 0, 0);
        }
    }

    #[inline]
    pub fn find_pipeline(&self, pipeline_name: &str) -> Option<&RenderPipeline> {
        self.pipelines.iter().find(|pipeline| pipeline.name == pipeline_name)
    }

}