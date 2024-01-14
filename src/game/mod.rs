pub mod error;
pub mod device;
pub mod render;

use std::ffi::CStr;
use std::rc::Rc;
use ash::{Entry, Instance, vk};
use ash::vk::{MemoryHeapFlags, PhysicalDevice};
use itertools::Itertools;
use raw_window_handle::HasRawDisplayHandle;
use winit::window::Window;
use crate::game::device::WrappedDevice;
use crate::game::error::EngineError;

pub type Result<T> = std::result::Result<T, EngineError>;

struct GameInner {
    entry: Entry,
    instance: Instance,
    device: WrappedDevice
}

#[derive(Clone)]
pub(crate) struct Game(Rc<GameInner>);

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

        // Get best device
        Ok(Self(Rc::new(GameInner {
            entry,
            device: WrappedDevice::new(instance.clone(), unsafe { instance.enumerate_physical_devices() }?.into_iter()
                .sorted_by(|a, b| local_heap_size_of(&instance, a).cmp(&local_heap_size_of(&instance, b)))
                .next().unwrap())?,
            instance
        })))
    }

    pub fn device(&self) -> &WrappedDevice {
        &self.0.device
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