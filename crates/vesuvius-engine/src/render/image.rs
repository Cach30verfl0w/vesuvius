use crate::render::buffer::Buffer;
use crate::{App, Result};
use ash::vk;
use log::{debug, info};
use std::path::Path;
use std::slice;
use std::sync::Arc;
use vk_mem_alloc::{Allocation, AllocationCreateFlags, AllocationCreateInfo, MemoryUsage};

pub struct ImageInner {
    app: App,
    image: vk::Image,
    image_alloc: Allocation,
    pub(crate) image_view: vk::ImageView,
    pub(crate) sampler: vk::Sampler,
}

impl Drop for ImageInner {
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

#[derive(Clone)]
pub struct Image(pub(crate) Arc<ImageInner>);

impl PartialEq for Image {
    fn eq(&self, other: &Self) -> bool {
        self.0.image == other.0.image
    }
}

impl Image {
    pub fn from_file<P: AsRef<Path>>(app: &App, path: P) -> Result<Self> {
        info!(
            "Loading resource '{}' as image",
            path.as_ref().file_name().unwrap().to_str().unwrap()
        );
        let device = app.main_device();
        let vk_device = device.virtual_device();

        // Read image
        let image = image::open(path)?.to_rgba8();
        let (width, height) = (image.width(), image.height());
        let pixels = image.pixels().map(|pixel| pixel.0).collect::<Vec<_>>();

        // Create image and image buffer
        let image_create_info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .extent(vk::Extent3D {
                width,
                height,
                depth: 1,
            })
            .mip_levels(1)
            .array_layers(1)
            .format(vk::Format::R8G8B8A8_UNORM)
            .tiling(vk::ImageTiling::OPTIMAL)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .usage(vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .samples(vk::SampleCountFlags::TYPE_1);

        let image_alloc_create_info = AllocationCreateInfo {
            usage: MemoryUsage::AUTO,
            ..Default::default()
        };
        let allocator = *device.allocator();
        let (image, image_alloc, image_alloc_info) = unsafe {
            vk_mem_alloc::create_image(allocator, &image_create_info, &image_alloc_create_info)
        }?;

        debug!("Initialize and write staging buffer");
        let staging_buffer = Buffer::new(
            app.clone(),
            vk::BufferUsageFlags::TRANSFER_SRC,
            image_alloc_info.size,
            Some(
                AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE | AllocationCreateFlags::MAPPED,
            ),
        )?;
        staging_buffer.write_ptr(pixels.as_ptr(), pixels.len())?;

        // Command Buffer move memory to image
        debug!("Use staging buffer to upload pixel data into resource image");
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
        debug!("Create image view and sampler by resource");
        let image_view_create_info = vk::ImageViewCreateInfo::default()
            .image(image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(vk::Format::R8G8B8A8_UNORM)
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

        Ok(Self(Arc::new(ImageInner {
            app: app.clone(),
            image,
            image_view,
            sampler,
            image_alloc,
        })))
    }
}

pub fn get_memory_type_index(
    app: &App,
    type_filter: Option<u32>,
    properties: vk::MemoryPropertyFlags,
) -> u32 {
    let memory_properties = unsafe {
        app.instance()
            .get_physical_device_memory_properties(app.main_device().physical_device())
    };
    for i in 0..memory_properties.memory_type_count as usize {
        if type_filter
            .map(|filter| (filter & (1 << i)) != 0)
            .unwrap_or(true)
            && !(memory_properties.memory_types[i].property_flags & properties).is_empty()
        {
            return i as u32;
        }
    }
    panic!("No support ig... ._.")
}
