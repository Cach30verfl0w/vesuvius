use crate::App;
use crate::Result;
use ash::vk;

use std::mem;
use vk_mem_alloc::{Allocation, AllocationCreateFlags, AllocationInfo};

/// This structure represents an allocated buffer with device memory. This struct contains a device, the buffer handle
/// itself, the allocation handle and the info about the allocation and allows a simple write function to write
/// arbitrary data into the buffer's memory.
#[derive(Clone)]
pub struct Buffer {
    app: App,
    pub(crate) buffer: vk::Buffer,
    alloc: Allocation,
    pub(crate) alloc_info: AllocationInfo,
    pub(crate) size: vk::DeviceSize,
}

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe {
            vk_mem_alloc::destroy_buffer(
                *self.app.main_device().allocator(),
                self.buffer,
                self.alloc,
            )
        };
    }
}

impl Buffer {
    /// This function creates a new buffer with the specified size or the specified usage. This buffer is created with
    /// the vk_mem_alloc crate.
    pub fn new(app: App, usage: vk::BufferUsageFlags, size: vk::DeviceSize) -> Result<Self> {
        let buffer_create_info = vk::BufferCreateInfo {
            usage,
            size,
            ..Default::default()
        };

        let alloc_create_info = vk_mem_alloc::AllocationCreateInfo {
            usage: vk_mem_alloc::MemoryUsage::AUTO_PREFER_HOST,
            flags: AllocationCreateFlags::HOST_ACCESS_RANDOM | AllocationCreateFlags::MAPPED,
            ..Default::default()
        };

        let (buffer, alloc, alloc_info) = unsafe {
            vk_mem_alloc::create_buffer(
                *app.main_device().allocator(),
                &buffer_create_info,
                &alloc_create_info,
            )
        }?;

        Ok(Self {
            app,
            buffer,
            alloc,
            alloc_info,
            size,
        })
    }

    /// This function allows to write arbitrary data into the buffer's memory. The input data can't be bigger than the
    /// size, specified in th allocation info.
    pub fn write<T>(&self, data: T) -> Result<()> {
        // Validate the size of the data
        let input_size = mem::size_of::<T>() as u64;
        if self.size < input_size {
            panic!(
                "Error while writing buffer => Input Size ({}) is bigger than Buffer Size ({})",
                input_size, self.size
            );
        }

        // Map memory into pointer
        unsafe {
            std::ptr::copy_nonoverlapping(&data as *const _, self.alloc_info.mapped_data.cast(), 1);
        }
        Ok(())
    }
}
