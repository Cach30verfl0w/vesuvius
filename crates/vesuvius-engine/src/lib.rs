pub mod error;
pub mod device;
pub mod render;

extern crate ash;
extern crate thiserror;
extern crate vk_mem_alloc;
extern crate itertools;
extern crate winit;
extern crate raw_window_handle;
extern crate serde;
extern crate spirv_reflect;
extern crate shaderc;
extern crate log;

use std::mem::ManuallyDrop;
use std::sync::Arc;
use ash::{Entry, Instance, vk};
use ash::vk::{MemoryHeapFlags, PhysicalDevice};
use itertools::Itertools;
use raw_window_handle::HasRawDisplayHandle;
use winit::window::Window;
use device::WrappedDevice;
use error::Error;

pub mod vesuvius_winit {
    pub use winit::*;
}

pub type Result<T> = std::result::Result<T, Error>;

/// This struct represents the internal handles for the Vulkan API. This struct is stored in the App structure.
struct AppInner {
    /// Holder of instance-independent functions
    entry: Entry,

    /// Handle to the Vulkan instance
    instance: Instance,

    /// Reference to the main graphics device
    main_device: ManuallyDrop<WrappedDevice>,

    // The game window itself
    window: Window
}

impl Drop for AppInner {
    fn drop(&mut self) {
        unsafe {
            ManuallyDrop::drop(&mut self.main_device);
            self.instance.destroy_instance(None);
        }
    }
}

/// This struct represents a single instance of the game. This game instance is the core of the engine and stores the
/// vulkan handles etc.
#[derive(Clone)]
pub struct App(Arc<AppInner>);

impl App {

    /// This function creates a new instance of the engine application
    pub fn new(window: Window) -> Result<Self> {
        let entry = unsafe { Entry::load() }?;

        // Add validation layer if enabled
        let mut layers = Vec::new();
        if let Ok(value) = std::env::var("VALIDATION_LAYER") {
            if value.parse::<bool>().expect("Unable to wrap VALIDATE_LAYER env var into boolean") {
                layers.push(b"VK_LAYER_KHRONOS_validation\0".as_ptr().cast());
            }
        }

        // Create Vulkan instance
        let extensions = ash_window::enumerate_required_extensions(window.raw_display_handle())?;
        let application_info = vk::ApplicationInfo::default()
            .api_version(vk::API_VERSION_1_3)
            .engine_version(vk::make_api_version(0, 1, 0, 0));
        let instance_create_info = vk::InstanceCreateInfo::default()
            .application_info(&application_info)
            .enabled_extension_names(extensions)
            .enabled_layer_names(layers.as_slice());
        let instance = unsafe { entry.create_instance(&instance_create_info, None) }?;

        // Create device and application
        Ok(Self(Arc::new(AppInner {
            main_device: ManuallyDrop::new(
                WrappedDevice::new(instance.clone(), unsafe { instance.enumerate_physical_devices() }?
                    .into_iter()
                    .sorted_by(|a, b| local_heap_size_of(&instance, a).cmp(&local_heap_size_of(&instance, b)))
                    .next().unwrap())?
            ),
            entry,
            instance,
            window
        })))
    }

    #[inline]
    pub(crate) fn instance(&self) -> &Instance {
        &self.0.instance
    }

    #[inline]
    pub(crate) fn entry(&self) -> &Entry {
        &self.0.entry
    }

    #[inline]
    pub fn main_device(&self) -> &WrappedDevice {
        &self.0.main_device
    }

    #[inline]
    pub fn window(&self) -> &Window {
        &self.0.window
    }

}


#[inline]
fn local_heap_size_of(instance: &Instance, physical_device: &PhysicalDevice) -> u64 {
    unsafe { instance.get_physical_device_memory_properties(*physical_device) }
        .memory_heaps.iter()
        .filter(|heap| (heap.flags & MemoryHeapFlags::DEVICE_LOCAL) == MemoryHeapFlags::DEVICE_LOCAL)
        .map(|heap| heap.size)
        .sum()
}
