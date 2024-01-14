pub mod error;
pub mod device;

use std::ffi::CStr;
use ash::{Entry, Instance, vk};
use ash::vk::{MemoryHeapFlags, PhysicalDevice};
use itertools::Itertools;
use raw_window_handle::HasRawDisplayHandle;
use winit::window::Window;
use crate::game::device::WrappedDevice;
use crate::game::error::EngineError;

pub type Result<T> = std::result::Result<T, EngineError>;

pub(crate) struct Game {
    entry: Entry,
    instance: Instance
}

impl Game {

    pub(crate) fn new(window: &Window) -> Result<Self> {
        let entry = unsafe { Entry::load() }?;

        // Generate instance create info etc.
        let mut layers = Vec::new();
        if let Ok(value) = std::env::var("VALIDATION_LAYER") {
            if value.parse::<bool>().expect("Unable to wrap VALIDATE_LAYER env var into boolean") {
                layers.push(b"VK_LAYER_KHRONOS_validation\0".as_ptr().cast());
            }
        }

        let extensions = ash_window::enumerate_required_extensions(window.raw_display_handle())?;
        let application_info = vk::ApplicationInfo::default()
            .application_version(vk::make_api_version(0, 1, 0, 0))
            .application_name(unsafe { CStr::from_ptr(b"Vesuvius\0".as_ptr() as _) })
            .engine_version(vk::make_api_version(0, 1, 0, 0))
            .api_version(vk::API_VERSION_1_3);
        let instance_create_info = vk::InstanceCreateInfo::default()
            .application_info(&application_info)
            .enabled_extension_names(extensions)
            .enabled_layer_names(layers.as_slice());

        // Create instance and return value
        let instance = unsafe { entry.create_instance(&instance_create_info, None) }?;
        Ok(Self {
            entry,
            instance
        })
    }

    pub fn request_best_device(&self) -> Result<WrappedDevice> {
        WrappedDevice::new(&self.instance, unsafe { self.instance.enumerate_physical_devices() }?.into_iter()
            .sorted_by(|a, b| local_heap_size_of(&self.instance, a).cmp(&local_heap_size_of(&self.instance, b)))
            .next().unwrap())
    }

}

#[inline]
fn local_heap_size_of(instance: &Instance, physical_device: &PhysicalDevice) -> u64 {
    unsafe { instance.get_physical_device_memory_properties(*physical_device) }
        .memory_heaps.into_iter()
        .filter(|heap| (heap.flags & MemoryHeapFlags::DEVICE_LOCAL) == MemoryHeapFlags::DEVICE_LOCAL)
        .map(|heap| heap.size)
        .sum()
}