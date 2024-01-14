use std::ffi::CStr;
use std::fmt::{Display, Formatter};
use std::slice;
use ash::{Device, Instance, vk};
use ash::vk::PhysicalDevice;
use crate::game::Result;

#[derive(Clone)]
pub struct WrappedDevice<'a> {
    instance: &'a Instance,
    physical_device: PhysicalDevice,
    virtual_device: Device
}

impl Display for WrappedDevice<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let device_name = unsafe { self.instance.get_physical_device_properties(self.physical_device) }.device_name;
        write!(formatter, "{}", unsafe { CStr::from_ptr(device_name.as_ptr()) }.to_str().unwrap())
    }
}

impl Drop for WrappedDevice<'_> {
    fn drop(&mut self) {
        unsafe {
            self.virtual_device.destroy_device(None);
        }
    }
}

impl<'a> WrappedDevice<'a> {

    pub fn new(instance: &'a Instance, physical_device: PhysicalDevice) -> Result<WrappedDevice> {
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
        Ok(Self {
            virtual_device: unsafe { instance.create_device(physical_device, &device_create_info, None) }?,
            physical_device,
            instance
        })
    }

}