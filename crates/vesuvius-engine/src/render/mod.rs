pub mod buffer;
pub mod image;
pub mod pipeline;

use crate::render::buffer::Buffer;
use crate::render::pipeline::config::PipelineConfiguration;
use ash::extensions::khr::{Surface, Swapchain};
use ash::vk;
use ash::vk::DescriptorType;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use std::sync::Arc;
use std::{fs, mem, slice};

use crate::render::pipeline::{DescriptorSet, RenderPipeline};
use crate::App;
use crate::Result;

struct GameRendererInner {
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
    descriptor_pool: vk::DescriptorPool,
}

impl Drop for GameRendererInner {
    fn drop(&mut self) {
        let device = self.application.main_device().virtual_device();
        let surface_loader = Surface::new(self.application.entry(), self.application.instance());
        unsafe {
            device.destroy_descriptor_pool(self.descriptor_pool, None);
            device.destroy_semaphore(self.submit_semaphore, None);
            device.destroy_semaphore(self.present_semaphore, None);
            for image_view in self.image_views.iter() {
                device.destroy_image_view(*image_view, None);
            }

            self.swapchain_loader
                .destroy_swapchain(self.swapchain, None);
            surface_loader.destroy_surface(self.surface, None);
            device.free_command_buffers(self.command_pool, slice::from_ref(&self.command_buffer));
            device.destroy_command_pool(self.command_pool, None);
        }
    }
}

#[derive(Clone)]
pub struct GameRenderer(Arc<GameRendererInner>);

impl GameRenderer {
    pub fn new(application: App) -> Result<Self> {
        let device = application.main_device().virtual_device();
        let window = application.window();
        let surface = unsafe {
            ash_window::create_surface(
                application.entry(),
                application.instance(),
                window.raw_display_handle(),
                window.raw_window_handle(),
                None,
            )
        }?;

        // Create swapchain
        let swapchain_loader = Swapchain::new(application.instance(), device);
        let swapchain_create_info = vk::SwapchainCreateInfoKHR::default()
            .surface(surface)
            .min_image_count(2)
            .image_format(vk::Format::B8G8R8A8_UNORM)
            .image_color_space(vk::ColorSpaceKHR::SRGB_NONLINEAR)
            .image_extent(vk::Extent2D {
                width: window.inner_size().width,
                height: window.inner_size().height,
            })
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(vk::SurfaceTransformFlagsKHR::IDENTITY)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(vk::PresentModeKHR::FIFO);
        let swapchain = unsafe { swapchain_loader.create_swapchain(&swapchain_create_info, None) }?;

        // Create image views
        let images = unsafe { swapchain_loader.get_swapchain_images(swapchain) }?;
        let image_views = images
            .iter()
            .map(|image| {
                let image_view_create_info = vk::ImageViewCreateInfo::default()
                    .image(*image)
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(vk::Format::B8G8R8A8_UNORM)
                    .components(vk::ComponentMapping::default())
                    .subresource_range(
                        vk::ImageSubresourceRange::default()
                            .aspect_mask(vk::ImageAspectFlags::COLOR)
                            .layer_count(1)
                            .level_count(1),
                    );
                unsafe { device.create_image_view(&image_view_create_info, None) }.unwrap()
            })
            .collect::<Vec<_>>();

        // Command Pool and Command Buffer
        let command_pool_create_info = vk::CommandPoolCreateInfo::default()
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER) // Reset at begin
            .queue_family_index(0);
        let command_pool = unsafe { device.create_command_pool(&command_pool_create_info, None) }?;

        let command_buffer_alloc_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(command_pool)
            .command_buffer_count(1);
        let command_buffer =
            unsafe { device.allocate_command_buffers(&command_buffer_alloc_info) }?[0];

        // Create descriptor pool
        // TODO
        let descriptor_pool_sizes = [
            vk::DescriptorPoolSize::default()
                .descriptor_count(1)
                .ty(DescriptorType::UNIFORM_BUFFER),
            vk::DescriptorPoolSize::default()
                .descriptor_count(1)
                .ty(DescriptorType::COMBINED_IMAGE_SAMPLER),
        ];
        let descriptor_pool_create_info = vk::DescriptorPoolCreateInfo::default()
            .pool_sizes(&descriptor_pool_sizes)
            .flags(vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET)
            .max_sets(1024);
        let descriptor_pool =
            unsafe { device.create_descriptor_pool(&descriptor_pool_create_info, None) }?;

        // Return game renderer to caller
        Ok(Self(Arc::new(GameRendererInner {
            submit_semaphore: unsafe {
                device.create_semaphore(&vk::SemaphoreCreateInfo::default(), None)
            }?,
            present_semaphore: unsafe {
                device.create_semaphore(&vk::SemaphoreCreateInfo::default(), None)
            }?,
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
            descriptor_pool,
        })))
    }

    pub fn reload(&mut self) -> Result<()> {
        let inner = unsafe { Arc::get_mut_unchecked(&mut self.0) };

        // (Re)compile pipelines
        for pipeline_configurations in
            fs::read_dir("assets/pipelines").expect("Unable to find pipeline configs")
        {
            // Filter invalid configuration files
            let config_file = pipeline_configurations.unwrap().path();
            if !config_file
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .ends_with(".json")
            {
                continue;
            }

            // Create pipeline or recompile it
            let file_content = String::from_utf8(fs::read(&config_file)?)?;
            let pipeline_config: PipelineConfiguration =
                serde_json::from_str(&file_content).expect("Unable to read pipeline configuration");
            match inner
                .pipelines
                .iter_mut()
                .find(|pipeline| pipeline.name == pipeline_config.name)
            {
                Some(pipeline) => pipeline.compile()?, // TODO: Reload only if changes are detected
                None => {
                    let mut pipeline =
                        RenderPipeline::new(inner.application.clone(), pipeline_config)?;
                    pipeline.compile()?;
                    inner.pipelines.push(pipeline);
                }
            }
        }

        Ok(())
    }

    pub fn begin(&mut self) -> Result<()> {
        let inner = unsafe { Arc::get_mut_unchecked(&mut self.0) };

        inner.current_image_index = unsafe {
            inner.swapchain_loader.acquire_next_image(
                inner.swapchain,
                u64::MAX,
                inner.submit_semaphore,
                vk::Fence::null(),
            )
        }?
        .0;

        let device = inner.application.main_device().virtual_device();
        unsafe {
            device.reset_command_pool(
                inner.command_pool,
                vk::CommandPoolResetFlags::RELEASE_RESOURCES,
            )
        }?;
        unsafe {
            device.reset_command_buffer(
                inner.command_buffer,
                vk::CommandBufferResetFlags::RELEASE_RESOURCES,
            )
        }?;
        unsafe {
            device
                .begin_command_buffer(inner.command_buffer, &vk::CommandBufferBeginInfo::default())
        }?;

        let image_memory_barrier = vk::ImageMemoryBarrier::default()
            .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
            .old_layout(vk::ImageLayout::UNDEFINED)
            .new_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .image(inner.images[inner.current_image_index as usize])
            .subresource_range(
                vk::ImageSubresourceRange::default()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .level_count(1)
                    .layer_count(1),
            );

        unsafe {
            device.cmd_pipeline_barrier(
                inner.command_buffer,
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                slice::from_ref(&image_memory_barrier),
            )
        };
        Ok(())
    }

    pub fn clear_color(&self, red: f32, green: f32, blue: f32, alpha: f32) {
        let inner = &self.0;

        let rendering_attachment_info = vk::RenderingAttachmentInfo::default()
            .image_view(inner.image_views[inner.current_image_index as usize])
            .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .clear_value(vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [red, green, blue, alpha],
                },
            });

        let window_size = inner.application.window().inner_size();
        let rendering_info = vk::RenderingInfo::default()
            .layer_count(1)
            .render_area(vk::Rect2D {
                offset: vk::Offset2D::default(),
                extent: vk::Extent2D {
                    width: window_size.width,
                    height: window_size.height,
                },
            })
            .color_attachments(slice::from_ref(&rendering_attachment_info));
        unsafe {
            inner
                .application
                .main_device()
                .virtual_device()
                .cmd_begin_rendering(inner.command_buffer, &rendering_info);
        }
    }

    pub fn end(&self) -> Result<()> {
        let inner = &self.0;
        let device = &inner.application.main_device().virtual_device();
        unsafe { device.cmd_end_rendering(inner.command_buffer) };

        let image_memory_barrier = vk::ImageMemoryBarrier::default()
            .src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
            .old_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
            .image(inner.images[inner.current_image_index as usize])
            .subresource_range(
                vk::ImageSubresourceRange::default()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .level_count(1)
                    .layer_count(1),
            );

        unsafe {
            device.cmd_pipeline_barrier(
                inner.command_buffer,
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                slice::from_ref(&image_memory_barrier),
            )
        };

        // Move command buffer into executable state
        unsafe { device.end_command_buffer(inner.command_buffer) }?;

        // Submit and present queued commands
        let submit_info = vk::SubmitInfo::default()
            .wait_semaphores(slice::from_ref(&inner.submit_semaphore))
            .wait_dst_stage_mask(slice::from_ref(
                &vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            ))
            .command_buffers(slice::from_ref(&inner.command_buffer))
            .signal_semaphores(slice::from_ref(&inner.present_semaphore));
        unsafe {
            device.queue_submit(
                inner.queue,
                slice::from_ref(&submit_info),
                vk::Fence::null(),
            )
        }?;

        let present_info = vk::PresentInfoKHR::default()
            .image_indices(slice::from_ref(&inner.current_image_index))
            .wait_semaphores(slice::from_ref(&inner.present_semaphore))
            .swapchains(slice::from_ref(&inner.swapchain));
        unsafe {
            inner
                .swapchain_loader
                .queue_present(inner.queue, &present_info)
        }?;

        // Wait for finish operations
        unsafe { device.device_wait_idle() }?;
        Ok(())
    }

    pub fn bind_pipeline(&self, pipeline: &RenderPipeline, descriptor_sets: &[DescriptorSet]) {
        let inner = &self.0;
        unsafe {
            inner
                .application
                .main_device()
                .virtual_device()
                .cmd_bind_pipeline(
                    inner.command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    pipeline.vulkan_pipeline.unwrap(),
                );
        }

        if !descriptor_sets.is_empty() {
            let raw_descriptor_sets = descriptor_sets
                .iter()
                .map(|value| value.vk_descriptor_set)
                .collect::<Vec<_>>();
            unsafe {
                inner
                    .application
                    .main_device()
                    .virtual_device()
                    .cmd_bind_descriptor_sets(
                        inner.command_buffer,
                        vk::PipelineBindPoint::GRAPHICS,
                        pipeline.vulkan_pipeline_layout.unwrap(),
                        0,
                        raw_descriptor_sets.as_slice(),
                        &[],
                    );
            }
        }
    }

    pub fn bind_vertex_buffer(&self, buffer: &Buffer) {
        let inner = &self.0;
        unsafe {
            inner
                .application
                .main_device()
                .virtual_device()
                .cmd_bind_vertex_buffers(
                    inner.command_buffer,
                    0,
                    slice::from_ref(&buffer.buffer),
                    slice::from_ref(&vk::DeviceSize::from(0u32)),
                );
        }
    }

    pub fn draw(&self, vertices: u32) {
        let inner = &self.0;
        unsafe {
            inner.application.main_device().virtual_device().cmd_draw(
                inner.command_buffer,
                vertices,
                4,
                1,
                0,
            );
        }
    }

    pub fn draw_indexed(&self, index_buffer: &Buffer) {
        let inner = &self.0;
        let device = inner.application.main_device().virtual_device();
        let indices = (index_buffer.alloc_info.size / mem::size_of::<u16>() as u64) as u32;
        unsafe {
            device.cmd_bind_index_buffer(
                inner.command_buffer,
                index_buffer.buffer,
                vk::DeviceSize::from(0u32),
                vk::IndexType::UINT16,
            );
            device.cmd_draw_indexed(inner.command_buffer, indices, 1, 0, 0, 0);
        }
    }

    #[inline]
    pub fn find_pipeline(&self, pipeline_name: &str) -> Option<&RenderPipeline> {
        self.0
            .pipelines
            .iter()
            .find(|pipeline| pipeline.name == pipeline_name)
    }
}
