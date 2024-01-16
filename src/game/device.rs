use std::ffi::CStr;
use std::fmt::{Display, Formatter};
use std::rc::Rc;
use std::slice;
use ash::{Device, Instance, vk};
use ash::vk::PhysicalDevice;
use vk_mem_alloc::{Allocation, AllocationCreateFlags, AllocationInfo, Allocator, AllocatorCreateInfo};
use crate::game::Result;

pub struct WrappedDeviceInner {
    instance: Instance,
    phy_device: PhysicalDevice,
    virtual_device: Device,
    allocator: Allocator,
    pub allocated_buffers: Vec<WrappedBuffer>
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
                    Some(&AllocatorCreateInfo::default())
                )
            }?,
            virtual_device,
            phy_device,
            instance,
            allocated_buffers: Vec::new()
        })))
    }

    pub fn new_buffer(&mut self, usage: vk::BufferUsageFlags, size: usize) -> Result<WrappedBuffer> {
        let buffer_create_info = vk::BufferCreateInfo {
            usage,
            size: size as u64,
            ..Default::default()
        };

        let alloc_create_info = vk_mem_alloc::AllocationCreateInfo {
            usage: vk_mem_alloc::MemoryUsage::AUTO_PREFER_HOST,
            flags: AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE,
            ..Default::default()
        };

        let (buffer, alloc, alloc_info) = unsafe {
            vk_mem_alloc::create_buffer(self.0.allocator, &buffer_create_info, &alloc_create_info)
        }?;

        let buffer = WrappedBuffer {
            device: self.clone(),
            alloc_info,
            vk_buffer: buffer,
            alloc
        };
        unsafe { Rc::get_mut_unchecked(&mut self.0) }.allocated_buffers.push(buffer.clone());
        Ok(buffer)
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

    #[inline]
    pub fn allocated_buffers(&self) -> &Vec<WrappedBuffer> {
        &self.0.allocated_buffers
    }

}

#[derive(Clone)]
pub struct WrappedBuffer {
    device: WrappedDevice,
    pub(crate) vk_buffer: vk::Buffer,
    pub(crate) alloc: Allocation,
    pub(crate) alloc_info: AllocationInfo
}

impl WrappedBuffer {
    pub fn write<T>(&self, data: T) -> Result<()> {
        let allocator = *self.device.allocator();
        unsafe {
            let memory_ptr = vk_mem_alloc::map_memory(allocator, self.alloc)?;
            std::ptr::copy_nonoverlapping(&data as *const T, memory_ptr as *mut T, 1);
            vk_mem_alloc::unmap_memory(allocator, self.alloc);
        }
        Ok(())
    }
}