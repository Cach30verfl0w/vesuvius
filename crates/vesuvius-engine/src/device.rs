use std::ffi::CStr;
use std::fmt::{Display, Formatter};
use std::slice;
use std::sync::Arc;
use ash::{Device, Instance, vk};
use vk_mem_alloc::{Allocator, AllocatorCreateInfo};
use crate::Result;

pub struct WrappedDeviceInner {
    vk_instance: Instance,
    physical_device: vk::PhysicalDevice,
    virtual_device: Device,
    allocator: Allocator
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
            self.0.vk_instance.get_physical_device_properties(self.0.physical_device)
        }.device_name;
        write!(formatter, "{}", unsafe { CStr::from_ptr(device_name.as_ptr()) }.to_str().unwrap())
    }
}

impl WrappedDevice {

    pub fn new(vk_instance: Instance, physical_device: vk::PhysicalDevice) -> Result<Self> {
        let queue_create_info = vk::DeviceQueueCreateInfo::default()
            .queue_family_index(0)
            .queue_priorities(slice::from_ref(&1.0));
        let mut vulkan13_features = vk::PhysicalDeviceVulkan13Features::default()
            .dynamic_rendering(true);
        let mut features = vk::PhysicalDeviceFeatures2::default()
            .push_next(&mut vulkan13_features);
        let device_extensions = [b"VK_KHR_swapchain\0".as_ptr().cast()];
        let device_create_info = vk::DeviceCreateInfo::default()
            .push_next(&mut features)
            .enabled_extension_names(&device_extensions)
            .queue_create_infos(slice::from_ref(&queue_create_info));

        let virtual_device = unsafe { vk_instance.create_device(physical_device, &device_create_info, None) }?;
        Ok(Self(Arc::new(WrappedDeviceInner {
            allocator: unsafe {
                vk_mem_alloc::create_allocator(
                    &vk_instance,
                    physical_device,
                    &virtual_device,
                    Some(&AllocatorCreateInfo::default())
                )?
            },
            physical_device,
            virtual_device,
            vk_instance
        })))
    }

    #[inline]
    pub(crate) fn allocator(&self) -> &Allocator {
        &self.0.allocator
    }

    #[inline]
    pub(crate) fn virtual_device(&self) -> &Device {
        &self.0.virtual_device
    }

}