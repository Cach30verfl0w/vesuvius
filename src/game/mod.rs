pub mod error;
pub mod device;
pub mod render;
pub mod screen;

use std::ffi::CStr;
use std::rc::Rc;
use ash::{Entry, Instance, vk};
use ash::vk::{MemoryHeapFlags, PhysicalDevice};
use itertools::Itertools;
use log::debug;
use raw_window_handle::HasRawDisplayHandle;
use winit::window::Window;
use crate::game::device::WrappedDevice;
use crate::game::error::EngineError;
use crate::game::screen::Screen;

pub type Result<T> = std::result::Result<T, EngineError>;

/// This struct is the internal store of the vulkan instance, the main graphics device and the game window. This is the
/// internal holder of the data in the Game struct (which is internally an reference counter).
pub(crate) struct GameInner<'a> {
    entry: Entry,
    instance: Instance,
    device: WrappedDevice,
    window: Window,
    pub(crate) current_screen: Option<Box<dyn Screen + 'a>>
}

impl Drop for GameInner<'_> {
    fn drop(&mut self) {
        unsafe {
            for buffer in self.device.allocated_buffers() {
                vk_mem_alloc::destroy_buffer(*self.device.allocator(), buffer.vk_buffer, buffer.alloc);
            }

            vk_mem_alloc::destroy_allocator(*self.device.allocator());
            self.device.virtual_device().destroy_device(None);
        }
    }
}

#[derive(Clone)]
pub(crate) struct Game<'a>(pub(crate) Rc<GameInner<'a>>);

impl<'a> Game<'a> {


    /// This function creates the vulkan part of the game instance with some data, provided by the specified window, and
    /// returns the instance itself.
    pub(crate) fn new(window: Window) -> Result<Self> {
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
            instance,
            window,
            current_screen: None
        })))
    }

    pub fn open_screen<S: Screen + 'a>(&mut self, mut screen: S) {
        let mut mutable_clone = self.clone();
        let game = unsafe { Rc::get_mut_unchecked(&mut self.0) };

        if let Some(last_screen) = game.current_screen.as_mut() {
            last_screen.on_close(&mut mutable_clone);
        }

        screen.init(&mut mutable_clone);
        debug!("Opening to screen '{}'", S::title());
        game.current_screen = Some(Box::new(screen));
        self.window().set_title(&format!("{} - {}",
                                         concat!("Vesuvious v", env!("CARGO_PKG_VERSION"), " by Cach30verfl0w"),
                                         S::title()));
    }

    #[inline]
    pub fn device_mut(&mut self) -> &mut WrappedDevice {
        &mut unsafe { Rc::get_mut_unchecked(&mut self.0) }.device
    }

    #[inline]
    pub fn device(&self) -> &WrappedDevice {
        &self.0.device
    }

    #[inline]
    pub fn window(&self) -> &Window {
        &self.0.window
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