use crate::render::buffer::Buffer;
use crate::{App, Result};
use ash::vk;
use std::fs::File;
use std::path::Path;
use vk_mem_alloc::{Allocation, AllocationCreateInfo, MemoryUsage};

pub struct Image {
    app: App,
    image: vk::Image,
    image_alloc: Allocation,
    image_view: vk::ImageView,
}

impl Drop for Image {
    fn drop(&mut self) {
        let device = self.app.main_device();
        unsafe {
            device
                .virtual_device()
                .destroy_image_view(self.image_view, None);
            vk_mem_alloc::destroy_image(*device.allocator(), self.image, self.image_alloc);
        }
    }
}

impl Image {
    pub fn from_file<P: AsRef<Path>>(app: &App, path: P) -> Result<Self> {
        let device = app.main_device().virtual_device();

        // Read image
        let image = File::open(path)?;
        let decoder = png::Decoder::new(image);
        let mut reader = decoder.read_info()?;
        let mut pixels = vec![0; reader.info().raw_bytes()];
        reader.next_frame(&mut pixels)?;
        let size = reader.info().raw_bytes() as u64;
        let (width, height) = reader.info().size();

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
            .usage(vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .samples(vk::SampleCountFlags::TYPE_1);

        let allocator = *app.main_device().allocator();
        let image_alloc_create_info = AllocationCreateInfo {
            usage: MemoryUsage::AUTO,
            ..Default::default()
        };

        // Create image and copy image data into it
        let staging_buffer = Buffer::new(app.clone(), vk::BufferUsageFlags::TRANSFER_SRC, size)?;
        staging_buffer.write(pixels.as_slice())?;
        let (image, image_alloc, _image_alloc_info) = unsafe {
            vk_mem_alloc::create_image(allocator, &image_create_info, &image_alloc_create_info)
        }?;

        // TODO: Finish

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
                    .level_count(1)
            );
        let image_view = unsafe { device.create_image_view(&image_view_create_info, None) }?;

        Ok(Self {
            app: app.clone(),
            image,
            image_view,
            image_alloc,
        })
    }
}
