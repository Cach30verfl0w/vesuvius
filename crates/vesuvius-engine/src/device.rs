use crate::Result;
use ash::vk::PhysicalDevice;
use ash::{vk, Device, Instance};
use std::ffi::CStr;
use std::fmt::{Display, Formatter};
use std::slice;
use std::sync::Arc;
use vk_mem_alloc::{Allocator, AllocatorCreateInfo};

pub struct WrappedDeviceInner {
    vk_instance: Instance,
    physical_device: vk::PhysicalDevice,
    virtual_device: Device,
    allocator: Allocator,
    queue: vk::Queue,
}

impl Drop for WrappedDeviceInner {
    fn drop(&mut self) {
        unsafe {
            vk_mem_alloc::destroy_allocator(self.allocator);
            self.virtual_device.destroy_device(None);
        }
    }
}

#[derive(Clone)]
pub struct WrappedDevice(Arc<WrappedDeviceInner>);

impl Display for WrappedDevice {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let device_name = unsafe {
            self.0
                .vk_instance
                .get_physical_device_properties(self.0.physical_device)
        }
        .device_name;
        write!(
            formatter,
            "{}",
            unsafe { CStr::from_ptr(device_name.as_ptr()) }
                .to_str()
                .unwrap()
        )
    }
}

impl WrappedDevice {
    pub fn new(vk_instance: Instance, physical_device: vk::PhysicalDevice) -> Result<Self> {
        let queue_create_info = vk::DeviceQueueCreateInfo::default()
            .queue_family_index(0)
            .queue_priorities(slice::from_ref(&1.0));

        let mut vulkan13_features =
            vk::PhysicalDeviceVulkan13Features::default().dynamic_rendering(true);
        let features = vk::PhysicalDeviceFeatures::default().sampler_anisotropy(true);
        let mut features2 = vk::PhysicalDeviceFeatures2::default()
            .push_next(&mut vulkan13_features)
            .features(features);

        let device_extensions = [b"VK_KHR_swapchain\0".as_ptr().cast()];
        let device_create_info = vk::DeviceCreateInfo::default()
            .push_next(&mut features2)
            .enabled_extension_names(&device_extensions)
            .queue_create_infos(slice::from_ref(&queue_create_info));

        let virtual_device =
            unsafe { vk_instance.create_device(physical_device, &device_create_info, None) }?;
        Ok(Self(Arc::new(WrappedDeviceInner {
            allocator: unsafe {
                vk_mem_alloc::create_allocator(
                    &vk_instance,
                    physical_device,
                    &virtual_device,
                    Some(&AllocatorCreateInfo::default()),
                )?
            },
            queue: unsafe { virtual_device.get_device_queue(0, 0) },
            physical_device,
            virtual_device,
            vk_instance,
        })))
    }

    pub(crate) fn memory_barrier(
        &self,
        command_buffer: vk::CommandBuffer,
        image: vk::Image,
        old_layout: vk::ImageLayout,
        new_layout: vk::ImageLayout,
    ) {
        let (src_access_mask, dst_access_mask, src_stage_mask, dst_stage_mask) =
            match (old_layout, new_layout) {
                (vk::ImageLayout::UNDEFINED, vk::ImageLayout::TRANSFER_DST_OPTIMAL) => (
                    vk::AccessFlags::empty(),
                    vk::AccessFlags::TRANSFER_WRITE,
                    vk::PipelineStageFlags::TOP_OF_PIPE,
                    vk::PipelineStageFlags::TRANSFER,
                ),
                (vk::ImageLayout::UNDEFINED, vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL) => (
                    vk::AccessFlags::empty(),
                    vk::AccessFlags::empty(),
                    vk::PipelineStageFlags::TOP_OF_PIPE,
                    vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                ),
                (
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                ) => (
                    vk::AccessFlags::TRANSFER_WRITE,
                    vk::AccessFlags::SHADER_READ,
                    vk::PipelineStageFlags::TRANSFER,
                    vk::PipelineStageFlags::FRAGMENT_SHADER,
                ),
                (vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL, vk::ImageLayout::PRESENT_SRC_KHR) => (
                    vk::AccessFlags::empty(),
                    vk::AccessFlags::empty(),
                    vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                    vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                ),
                _ => panic!("Unsupported layouts"),
            };

        let memory_barrier = vk::ImageMemoryBarrier::default()
            .src_access_mask(src_access_mask)
            .dst_access_mask(dst_access_mask)
            .new_layout(new_layout)
            .old_layout(old_layout)
            .image(image)
            .subresource_range(
                vk::ImageSubresourceRange::default()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .level_count(1)
                    .layer_count(1),
            );
        unsafe {
            self.virtual_device().cmd_pipeline_barrier(
                command_buffer,
                src_stage_mask,
                dst_stage_mask,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                slice::from_ref(&memory_barrier),
            );
        }
    }

    #[inline]
    pub(crate) fn queue(&self) -> &vk::Queue {
        &self.0.queue
    }

    #[inline]
    pub(crate) fn allocator(&self) -> &Allocator {
        &self.0.allocator
    }

    #[inline]
    pub(crate) fn virtual_device(&self) -> &Device {
        &self.0.virtual_device
    }

    #[inline]
    pub(crate) fn physical_device(&self) -> PhysicalDevice {
        self.0.physical_device
    }
}
