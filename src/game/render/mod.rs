use std::slice;
use ash::extensions::khr::Swapchain;
use ash::vk;
use log::info;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use crate::game::Game;
use crate::game::Result;

pub(crate) struct GameRenderer {
    game: Game,
    swapchain_loader: Swapchain,
    swapchain: vk::SwapchainKHR,
    image_views: Vec<vk::ImageView>,
    images: Vec<vk::Image>,
    command_pool: vk::CommandPool,
    command_buffer: vk::CommandBuffer,
    submit_semaphore: vk::Semaphore,
    present_semaphore: vk::Semaphore,
    queue: vk::Queue,
    current_image_index: u32
}

impl Drop for GameRenderer {
    fn drop(&mut self) {
        let device = &self.game.0.device;
        unsafe {
            device.virtual_device.destroy_semaphore(self.submit_semaphore, None);
            device.virtual_device.destroy_semaphore(self.present_semaphore, None);
            for image_view in &self.image_views {
                device.virtual_device.destroy_image_view(*image_view, None);
            }

            self.swapchain_loader.destroy_swapchain(self.swapchain, None);
            device.virtual_device.free_command_buffers(self.command_pool, slice::from_ref(&self.command_buffer));
            device.virtual_device.destroy_command_pool(self.command_pool, None);
        }
    }
}

impl<'a> GameRenderer {

    pub(crate) fn new(game: Game) -> Result<Self> {
        let window = game.window();
        let surface = unsafe { ash_window::create_surface(&game.0.entry, &game.0.instance, window.raw_display_handle(),
                                                          window.raw_window_handle(), None)? };

        // Create swapchain
        let swapchain_loader = Swapchain::new(&game.0.instance, &game.0.device.virtual_device);
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
        info!("Swapchain created by Game renderer");

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
            unsafe { game.0.device.virtual_device.create_image_view(&image_view_create_info, None) }.unwrap()
        }).collect::<Vec<_>>();

        // Command Pool and Command Buffer
        let command_pool_create_info = vk::CommandPoolCreateInfo::default()
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER) // Reset at begin
            .queue_family_index(0);
        let command_pool = unsafe { game.0.device.virtual_device.create_command_pool(&command_pool_create_info, None) }?;

        let command_buffer_alloc_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(command_pool)
            .command_buffer_count(1);
        let command_buffer = unsafe { game.0.device.virtual_device.allocate_command_buffers(&command_buffer_alloc_info) }?[0];

        let virtual_device = &game.0.device.virtual_device;
        Ok(Self {
            submit_semaphore: unsafe { virtual_device.create_semaphore(&vk::SemaphoreCreateInfo::default(), None) }?,
            present_semaphore: unsafe { virtual_device.create_semaphore(&vk::SemaphoreCreateInfo::default(), None) }?,
            queue: unsafe { virtual_device.get_device_queue(0, 0) },
            game,
            swapchain_loader,
            swapchain,
            images,
            image_views,
            command_pool,
            command_buffer,
            current_image_index: 0
        })
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

        let device = &self.game.0.device.virtual_device;
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

        let window_size = self.game.window().inner_size();
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
            self.game.0.device.virtual_device.cmd_begin_rendering(self.command_buffer, &rendering_info);
            self.game.0.device.virtual_device.cmd_end_rendering(self.command_buffer);
        }

    }

    pub fn end(&self) -> Result<()> {
        let device = &self.game.0.device.virtual_device;

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

        // Submit and present queue
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

}