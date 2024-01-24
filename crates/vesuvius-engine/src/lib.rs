#![feature(stmt_expr_attributes)]
#![feature(get_mut_unchecked)]

extern crate core;

#[cfg(feature = "debug_extensions")]
pub mod debug;
pub mod device;
pub mod error;
pub mod render;
pub mod screen;

use ash::vk::{CommandBuffer, MemoryHeapFlags, PhysicalDevice};
use ash::{vk, Entry, Instance};
use device::WrappedDevice;
use error::Error;
use itertools::Itertools;
use raw_window_handle::HasRawDisplayHandle;
use screen::Screen;
use std::mem::ManuallyDrop;
use std::slice;
use std::sync::Arc;
use winit::window::Window;

/// Reexport egui if debug extensions enabled
#[cfg(feature = "debug_extensions")]
pub mod vesuvius_egui {
    pub use egui::*;
}

/// Reexport winit
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

    /// The game window itself
    window: Window,

    /// The current screen (game state) of the application
    current_screen: Option<Box<dyn Screen>>,
}

unsafe impl Send for AppInner {}
unsafe impl Sync for AppInner {}

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
            if value
                .parse::<bool>()
                .expect("Unable to wrap VALIDATE_LAYER env var into boolean")
            {
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
            main_device: ManuallyDrop::new(WrappedDevice::new(
                instance.clone(),
                unsafe { instance.enumerate_physical_devices() }?
                    .into_iter()
                    .sorted_by(|a, b| {
                        local_heap_size_of(&instance, a).cmp(&local_heap_size_of(&instance, b))
                    })
                    .next()
                    .unwrap(),
            )?),
            entry,
            instance,
            window,
            current_screen: None,
        })))
    }

    pub fn open_screen(&mut self, screen: Box<dyn Screen>) {
        let immutable_clone = self.clone();
        let inner_application = unsafe { Arc::get_mut_unchecked(&mut self.0) };
        if let Some(previous_screen) = inner_application.current_screen.as_mut() {
            previous_screen.on_close(&immutable_clone);
        }

        inner_application.current_screen = Some(screen);
        inner_application
            .current_screen
            .as_mut()
            .unwrap()
            .init(&immutable_clone);
    }

    pub fn upload_single_time_command_buffer<F: FnOnce(CommandBuffer)>(
        &self,
        operation: F,
    ) -> Result<()> {
        let device = self.main_device().virtual_device();

        // Allocate command buffer
        let command_pool_create_info = vk::CommandPoolCreateInfo::default().queue_family_index(0);
        let command_pool = unsafe { device.create_command_pool(&command_pool_create_info, None) }?;
        let command_buffer_allocate_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(command_pool)
            .command_buffer_count(1);
        let command_buffer =
            unsafe { device.allocate_command_buffers(&command_buffer_allocate_info) }?[0];

        // Create fence
        let submit_fence = unsafe { device.create_fence(&vk::FenceCreateInfo::default(), None) }?;

        // Begin and perform operation
        unsafe {
            device.begin_command_buffer(command_buffer, &vk::CommandBufferBeginInfo::default())
        }?;
        operation(command_buffer);

        // End, submit and free
        unsafe {
            device.end_command_buffer(command_buffer)?;

            let submit_info =
                vk::SubmitInfo::default().command_buffers(slice::from_ref(&command_buffer));
            device.queue_submit(
                *self.main_device().queue(),
                slice::from_ref(&submit_info),
                submit_fence,
            )?;
            device.wait_for_fences(slice::from_ref(&submit_fence), true, u64::MAX)?;

            device.destroy_fence(submit_fence, None);
            device.free_command_buffers(command_pool, slice::from_ref(&command_buffer));
            device.destroy_command_pool(command_pool, None);
        }
        Ok(())
    }

    #[inline]
    pub fn screen(&self) -> Option<&dyn Screen> {
        self.0.current_screen.as_ref().map(|value| value.as_ref())
    }

    #[inline]
    pub fn screen_mut(&mut self) -> Option<&mut dyn Screen> {
        // Yeah ik, there is the map function in Option... But when I use this, the borrow checker doesn't like me
        // anymore
        match unsafe { Arc::get_mut_unchecked(&mut self.0) }
            .current_screen
            .as_mut()
        {
            Some(value) => Some(value.as_mut()),
            None => None,
        }
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
        .memory_heaps
        .iter()
        .filter(|heap| {
            (heap.flags & MemoryHeapFlags::DEVICE_LOCAL) == MemoryHeapFlags::DEVICE_LOCAL
        })
        .map(|heap| heap.size)
        .sum()
}
