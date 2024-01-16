use std::ffi::CStr;
use std::fmt::{Display, Formatter};
use std::rc::Rc;
use std::slice;
use ash::{Device, Instance, vk};
use ash::vk::PhysicalDevice;
use vk_mem_alloc::{Allocation, AllocationInfo, Allocator, AllocatorCreateFlags, AllocatorCreateInfo};
use crate::game::Result;

pub struct WrappedDeviceInner {
    instance: Instance,
    phy_device: PhysicalDevice,
    virtual_device: Device,
    allocator: Allocator
}

#[derive(Clone)]
pub struct WrappedDevice(Rc<WrappedDeviceInner>);

impl Display for WrappedDevice {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let device_name = unsafe { self.0.instance.get_physical_device_properties(self.0.phy_device) }.device_name;
        write!(formatter, "{}", unsafe { CStr::from_ptr(device_name.as_ptr()) }.to_str().unwrap())
    }
}

impl WrappedDevice {

    pub fn new(instance: Instance, phy_device: PhysicalDevice) -> Result<WrappedDevice> {
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

        let virtual_device = unsafe { instance.create_device(phy_device, &device_create_info, None) }?;
        Ok(Self(Rc::new(WrappedDeviceInner {
            allocator: unsafe {
                vk_mem_alloc::create_allocator(
                    &instance,
                    phy_device,
                    &virtual_device,
                    Some(&AllocatorCreateInfo {
                        flags: AllocatorCreateFlags::BUFFER_DEVICE_ADDRESS,
                        ..Default::default()
                    })
                )
            }?,
            virtual_device,
            phy_device,
            instance
        })))
    }

    pub fn create_buffer(&self, usage: vk::BufferUsageFlags, size: u64) -> Result<WrappedBuffer> {
        let buffer_create_info = vk::BufferCreateInfo {
            usage,
            size,
            ..Default::default()
        };

        let alloc_create_info = vk_mem_alloc::AllocationCreateInfo {
            usage: vk_mem_alloc::MemoryUsage::AUTO_PREFER_DEVICE,
            ..Default::default()
        };

        let (buffer, alloc, alloc_info) = unsafe {
            vk_mem_alloc::create_buffer(self.0.allocator, &buffer_create_info, &alloc_create_info)
        }?;
        
        Ok(WrappedBuffer {
            device: self.clone(),
            alloc_info,
            buffer,
            alloc
        })
    }

    #[inline]
    pub fn allocator(&self) -> &Allocator {
        &self.0.allocator
    }

    #[inline]
    pub fn physical_device(&self) -> &PhysicalDevice {
        &self.0.phy_device
    }

    #[inline]
    pub fn virtual_device(&self) -> &Device {
        &self.0.virtual_device
    }

}

#[derive(Clone)]
pub struct WrappedBuffer {
    device: WrappedDevice,
    buffer: vk::Buffer,
    alloc: Allocation,
    alloc_info: AllocationInfo
}

impl Drop for WrappedBuffer {
    fn drop(&mut self) {
        unsafe {
            vk_mem_alloc::destroy_buffer(*self.device.allocator(), self.buffer, self.alloc);
        }
    }
}
