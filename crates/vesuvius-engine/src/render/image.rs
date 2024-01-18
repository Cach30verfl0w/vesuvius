use crate::render::buffer::Buffer;
use crate::{App, Result};
use ash::vk;
use std::path::Path;
use std::slice;
use vk_mem_alloc::{Allocation, AllocationCreateInfo, MemoryUsage};

pub struct Image {
    app: App,
    image: vk::Image,
    image_alloc: Allocation,
    pub(crate) image_view: vk::ImageView,
    pub(crate) sampler: vk::Sampler,
}

impl Drop for Image {
    fn drop(&mut self) {
        let device = self.app.main_device();
        let vk_device = device.virtual_device();
        unsafe {
            vk_device.destroy_image_view(self.image_view, None);
            vk_device.destroy_sampler(self.sampler, None);
            vk_mem_alloc::destroy_image(*device.allocator(), self.image, self.image_alloc);
        }
    }
}

impl Image {
    pub fn from_file<P: AsRef<Path>>(app: &App, path: P) -> Result<Self> {
        let device = app.main_device();
        let vk_device = device.virtual_device();

        // Read image
        let image = image::open(path)?.to_rgba8();
        let (width, height) = (image.width(), image.height());
        let pixels = image.pixels().collect::<Vec<_>>();

        // Prepare image creation
        let image_create_info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .extent(vk::Extent3D {
                width,
                height,
                depth: 1,
            })
            .mip_levels(1)
            .array_layers(1)
            .format(vk::Format::R8G8B8A8_SRGB)
            .tiling(vk::ImageTiling::OPTIMAL)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .usage(vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .samples(vk::SampleCountFlags::TYPE_1);

        let allocator = *device.allocator();
        let image_alloc_create_info = AllocationCreateInfo {
            usage: MemoryUsage::AUTO,
            ..Default::default()
        };

        // Create image and copy image data into it
        let staging_buffer = Buffer::new(
            app.clone(),
            vk::BufferUsageFlags::TRANSFER_SRC,
            (pixels.len() * 6) as _,
        )?;
        staging_buffer.write(pixels.as_slice())?;
        let (image, image_alloc, _image_alloc_info) = unsafe {
            vk_mem_alloc::create_image(allocator, &image_create_info, &image_alloc_create_info)
        }?;

        // Command Buffer move memory to image
        app.upload_single_time_command_buffer(|command_buffer| unsafe {
            device.memory_barrier(
                command_buffer,
                image,
                vk::ImageLayout::UNDEFINED,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            );

            let buffer_image_copy = vk::BufferImageCopy::default()
                .image_extent(vk::Extent3D {
                    width,
                    height,
                    depth: 1,
                })
                .image_subresource(
                    vk::ImageSubresourceLayers::default()
                        .aspect_mask(vk::ImageAspectFlags::COLOR)
                        .mip_level(0)
                        .base_array_layer(0)
                        .layer_count(1),
                );

            vk_device.cmd_copy_buffer_to_image(
                command_buffer,
                staging_buffer.buffer,
                image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                slice::from_ref(&buffer_image_copy),
            );

            device.memory_barrier(
                command_buffer,
                image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            );
        })?;

        // Create image view
        let image_view_create_info = vk::ImageViewCreateInfo::default()
            .image(image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(vk::Format::R8G8B8A8_SRGB)
            .components(vk::ComponentMapping::default())
            .subresource_range(
                vk::ImageSubresourceRange::default()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .layer_count(1)
                    .level_count(1),
            );
        let image_view = unsafe { vk_device.create_image_view(&image_view_create_info, None) }?;

        // Create sampler
        let sampler_create_info = vk::SamplerCreateInfo::default()
            .mag_filter(vk::Filter::LINEAR)
            .min_filter(vk::Filter::LINEAR)
            .address_mode_u(vk::SamplerAddressMode::REPEAT)
            .address_mode_v(vk::SamplerAddressMode::REPEAT)
            .address_mode_w(vk::SamplerAddressMode::REPEAT)
            .anisotropy_enable(true)
            .max_anisotropy(16.0)
            .border_color(vk::BorderColor::INT_OPAQUE_BLACK)
            .mipmap_mode(vk::SamplerMipmapMode::LINEAR);
        let sampler = unsafe { vk_device.create_sampler(&sampler_create_info, None) }?;

        Ok(Self {
            app: app.clone(),
            image,
            image_view,
            image_alloc,
            sampler,
        })
    }
}
