pub mod buffer;
pub mod image;
pub mod pipeline;

use crate::render::buffer::Buffer;
use crate::render::pipeline::config::PipelineConfiguration;
use ash::extensions::khr::{Surface, Swapchain};
use ash::vk;
use glam::{Vec2, Vec3};
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use std::fmt::Debug;
use std::sync::Arc;
use std::{fs, mem, slice};

use crate::render::image::Image;
use crate::render::pipeline::{DescriptorSet, RenderPipeline};
use crate::App;
use crate::Result;

struct GameRendererInner {
    application: App,

    // Surface
    surface_loader: Surface,
    surface: vk::SurfaceKHR,

    // Images
    images: Vec<vk::Image>,
    image_views: Vec<vk::ImageView>,
    current_image_index: u32,

    // Swapchain
    swapchain_loader: Swapchain,
    swapchain: Option<vk::SwapchainKHR>,

    // Command Pool and Buffer
    command_pool: vk::CommandPool,
    command_buffer: vk::CommandBuffer,

    // Semaphores
    submit_semaphore: vk::Semaphore,
    present_semaphore: vk::Semaphore,

    // Other things
    pipelines: Vec<RenderPipeline>,
    descriptor_pool: vk::DescriptorPool,
    queued_buffer_builder: Vec<BufferBuilder>
}

impl Drop for GameRendererInner {
    fn drop(&mut self) {
        let device = self.application.main_device().virtual_device();
        let surface_loader = Surface::new(self.application.entry(), self.application.instance());
        unsafe {
            device.destroy_descriptor_pool(self.descriptor_pool, None);
            device.destroy_semaphore(self.submit_semaphore, None);
            device.destroy_semaphore(self.present_semaphore, None);
            for image_view in self.image_views.iter() {
                device.destroy_image_view(*image_view, None);
            }

            if let Some(swapchain) = self.swapchain.as_ref() {
                self.swapchain_loader.destroy_swapchain(*swapchain, None);
            }

            surface_loader.destroy_surface(self.surface, None);
            device.free_command_buffers(self.command_pool, slice::from_ref(&self.command_buffer));
            device.destroy_command_pool(self.command_pool, None);
        }
    }
}

#[derive(Clone)]
pub struct GameRenderer(Arc<GameRendererInner>);

impl GameRenderer {
    pub fn new(application: App) -> Result<Self> {
        let device = application.main_device().virtual_device();
        let window = application.window();
        let surface = unsafe {
            ash_window::create_surface(
                application.entry(),
                application.instance(),
                window.raw_display_handle(),
                window.raw_window_handle(),
                None,
            )
        }?;

        // Command Pool and Command Buffer
        let command_pool_create_info = vk::CommandPoolCreateInfo::default()
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER) // Reset at begin
            .queue_family_index(0);
        let command_pool = unsafe { device.create_command_pool(&command_pool_create_info, None) }?;

        let command_buffer_alloc_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(command_pool)
            .command_buffer_count(1);
        let command_buffer =
            unsafe { device.allocate_command_buffers(&command_buffer_alloc_info) }?[0];

        // Create descriptor pool
        // TODO
        let descriptor_pool_sizes = [
            vk::DescriptorPoolSize::default()
                .descriptor_count(1)
                .ty(vk::DescriptorType::UNIFORM_BUFFER),
            vk::DescriptorPoolSize::default()
                .descriptor_count(1)
                .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER),
        ];
        let descriptor_pool_create_info = vk::DescriptorPoolCreateInfo::default()
            .pool_sizes(&descriptor_pool_sizes)
            .flags(vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET)
            .max_sets(1024);
        let descriptor_pool =
            unsafe { device.create_descriptor_pool(&descriptor_pool_create_info, None) }?;

        // Create swapchain loader and return game renderer to caller
        let swapchain_loader = Swapchain::new(application.instance(), device);
        let surface_loader = Surface::new(application.entry(), application.instance());
        Ok(Self(Arc::new(GameRendererInner {
            submit_semaphore: unsafe {
                device.create_semaphore(&vk::SemaphoreCreateInfo::default(), None)
            }?,
            present_semaphore: unsafe {
                device.create_semaphore(&vk::SemaphoreCreateInfo::default(), None)
            }?,
            surface_loader,
            swapchain_loader,
            swapchain: None,
            images: Vec::new(),
            image_views: Vec::new(),
            command_pool,
            command_buffer,
            current_image_index: 0,
            application,
            surface,
            pipelines: Vec::new(),
            descriptor_pool,
            queued_buffer_builder: Vec::new()
        })))
    }

    pub fn reload(&mut self, recompile_pipelines: bool) -> Result<()> {
        let inner = unsafe { Arc::get_mut_unchecked(&mut self.0) };

        // Create swapchain and images
        let device = inner.application.main_device().virtual_device();
        unsafe { device.device_wait_idle() }?;

        for image_view in &inner.image_views {
            unsafe { device.destroy_image_view(*image_view, None) };
        }

        if let Some(swapchain) = inner.swapchain.as_ref() {
            unsafe { inner.swapchain_loader.destroy_swapchain(*swapchain, None) };
        }
        let surface_capabilities = unsafe {
            inner
                .surface_loader
                .get_physical_device_surface_capabilities(
                    inner.application.main_device().physical_device(),
                    inner.surface,
                )
        }?;
        let swapchain_create_info = vk::SwapchainCreateInfoKHR::default()
            .surface(inner.surface)
            .min_image_count(2)
            .image_format(vk::Format::B8G8R8A8_UNORM)
            .image_color_space(vk::ColorSpaceKHR::SRGB_NONLINEAR)
            .image_extent(surface_capabilities.current_extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(vk::SurfaceTransformFlagsKHR::IDENTITY)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(vk::PresentModeKHR::FIFO);
        let swapchain = unsafe {
            inner
                .swapchain_loader
                .create_swapchain(&swapchain_create_info, None)
        }?;

        let images = unsafe { inner.swapchain_loader.get_swapchain_images(swapchain) }?;
        let image_views = images
            .iter()
            .map(|image| {
                let image_view_create_info = vk::ImageViewCreateInfo::default()
                    .image(*image)
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(vk::Format::B8G8R8A8_UNORM)
                    .components(vk::ComponentMapping::default())
                    .subresource_range(
                        vk::ImageSubresourceRange::default()
                            .aspect_mask(vk::ImageAspectFlags::COLOR)
                            .layer_count(1)
                            .level_count(1),
                    );
                unsafe { device.create_image_view(&image_view_create_info, None) }.unwrap()
            })
            .collect::<Vec<_>>();

        inner.swapchain = Some(swapchain);
        inner.images = images;
        inner.image_views = image_views;

        // (Re)compile pipelines
        if recompile_pipelines {
            for pipeline_configurations in
                fs::read_dir("assets/pipelines").expect("Unable to find pipeline configs")
            {
                // Filter invalid configuration files
                let config_file = pipeline_configurations.unwrap().path();
                if !config_file
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .ends_with(".json")
                {
                    continue;
                }

                // Create pipeline or recompile it
                let file_content = String::from_utf8(fs::read(&config_file)?)?;
                let pipeline_config: PipelineConfiguration = serde_json::from_str(&file_content)
                    .expect("Unable to read pipeline configuration");
                match inner
                    .pipelines
                    .iter_mut()
                    .find(|pipeline| pipeline.name == pipeline_config.name)
                {
                    Some(pipeline) => pipeline.compile()?, // TODO: Reload only if changes are detected
                    None => {
                        let mut pipeline =
                            RenderPipeline::new(inner.application.clone(), pipeline_config)?;
                        pipeline.compile()?;
                        inner.pipelines.push(pipeline);
                    }
                }
            }
        }

        Ok(())
    }

    pub fn begin(&mut self) -> Result<()> {
        let inner = unsafe { Arc::get_mut_unchecked(&mut self.0) };
        inner.current_image_index = unsafe {
            inner.swapchain_loader.acquire_next_image(
                inner.swapchain.unwrap(),
                u64::MAX,
                inner.submit_semaphore,
                vk::Fence::null(),
            )
        }?
        .0;

        let device = inner.application.main_device().virtual_device();
        unsafe {
            device.reset_command_pool(
                inner.command_pool,
                vk::CommandPoolResetFlags::RELEASE_RESOURCES,
            )?;
            device.reset_command_buffer(
                inner.command_buffer,
                vk::CommandBufferResetFlags::RELEASE_RESOURCES,
            )?;
            device.begin_command_buffer(
                inner.command_buffer,
                &vk::CommandBufferBeginInfo::default(),
            )?;
        };

        inner.application.main_device().memory_barrier(
            inner.command_buffer,
            inner.images[inner.current_image_index as usize],
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        );
        Ok(())
    }

    pub fn clear_color(&self, red: f32, green: f32, blue: f32, alpha: f32) {
        let inner = &self.0;

        let rendering_attachment_info = vk::RenderingAttachmentInfo::default()
            .image_view(inner.image_views[inner.current_image_index as usize])
            .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .clear_value(vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [red, green, blue, alpha],
                },
            });

        let window_size = inner.application.window().inner_size();
        let rendering_info = vk::RenderingInfo::default()
            .layer_count(1)
            .render_area(vk::Rect2D {
                offset: vk::Offset2D::default(),
                extent: vk::Extent2D {
                    width: window_size.width,
                    height: window_size.height,
                },
            })
            .color_attachments(slice::from_ref(&rendering_attachment_info));
        unsafe {
            inner
                .application
                .main_device()
                .virtual_device()
                .cmd_begin_rendering(inner.command_buffer, &rendering_info);
        }
    }

    pub fn queue_buffer_builder(&mut self) -> Result<()> {
        // Create groups of equal buffer builders
        let mut grouped_buffer_builders = Vec::new();
        for buffer_builder in self.0.queued_buffer_builder.iter() {
            // Push first buffer into grouped buffer builders list
            if grouped_buffer_builders.is_empty() {
                grouped_buffer_builders.push(vec![buffer_builder.clone()]);
                continue;
            }

            // Check if current buffer and last buffer moved into grouped buffer builders are equal, if yes add this
            // buffer into the buffer group.
            let buffer_builder_groups = grouped_buffer_builders.len();
            let last_group = grouped_buffer_builders
                .get_mut(buffer_builder_groups - 1)
                .unwrap();
            let last_buffer_builder = last_group.get(last_group.len() - 1).unwrap();
            if last_buffer_builder.eq(buffer_builder) {
                last_group.push(buffer_builder.clone());
                continue;
            }

            // If not equal, create a new buffer group and add this buffer into the group
            grouped_buffer_builders.push(vec![buffer_builder.clone()]);
        }

        // Process groups into buffer and vertex format
        let app = &self.0.application;
        let mut grouped_buffers: Vec<(Buffer, Buffer, VertexFormat)> = Vec::new();
        for buffer_builder_group in grouped_buffer_builders {
            let (mut vertices, mut indices) = (Vec::new(), Vec::new());
            let vertex_format = buffer_builder_group.get(0).unwrap().vertex_format.clone();

            // Fill buffer data
            for buffer_builder in buffer_builder_group.into_iter() {
                vertices.push(buffer_builder.vertices);
                indices.push(buffer_builder.indices);
            }

            // Create buffer
            let (vertex_buffer, index_buffer) = (
                Buffer::new(
                    app.clone(),
                    vk::BufferUsageFlags::VERTEX_BUFFER,
                    (vertex_format.vertex_size() * vertices.len()) as vk::DeviceSize,
                    None,
                )?,
                Buffer::new(
                    app.clone(),
                    vk::BufferUsageFlags::INDEX_BUFFER,
                    (mem::size_of::<u16>() * indices.len()) as vk::DeviceSize,
                    None,
                )?,
            );

            // Write buffer and push
            vertex_buffer.write_ptr(vertices.as_ptr(), vertices.len())?;
            index_buffer.write_ptr(indices.as_ptr(), indices.len())?;
            grouped_buffers.push((vertex_buffer, index_buffer, vertex_format));
        }

        // Bind and draw
        for (vertex_buffer, index_buffer, vertex_format) in grouped_buffers {
            self.bind_pipeline(self.find_pipeline(vertex_format.pipeline_name()).unwrap(), &[]);
            self.bind_vertex_buffer(&vertex_buffer);
            self.draw_indexed(&index_buffer);
        }
        Ok(())
    }

    pub fn end(&mut self) -> Result<()> {
        // Memory barrier
        let device = &self.0.application.main_device().virtual_device();
        unsafe { device.cmd_end_rendering(self.0.command_buffer) };
        self.0.application.main_device().memory_barrier(
            self.0.command_buffer,
            self.0.images[self.0.current_image_index as usize],
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            vk::ImageLayout::PRESENT_SRC_KHR,
        );

        // Move command buffer into executable state
        unsafe { device.end_command_buffer(self.0.command_buffer) }?;

        // Submit and present queued commands
        let submit_info = vk::SubmitInfo::default()
            .wait_semaphores(slice::from_ref(&self.0.submit_semaphore))
            .wait_dst_stage_mask(slice::from_ref(
                &vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            ))
            .command_buffers(slice::from_ref(&self.0.command_buffer))
            .signal_semaphores(slice::from_ref(&self.0.present_semaphore));
        unsafe {
            device.queue_submit(
                *self.0.application.main_device().queue(),
                slice::from_ref(&submit_info),
                vk::Fence::null(),
            )
        }?;

        let present_info = vk::PresentInfoKHR::default()
            .image_indices(slice::from_ref(&self.0.current_image_index))
            .wait_semaphores(slice::from_ref(&self.0.present_semaphore))
            .swapchains(slice::from_ref(self.0.swapchain.as_ref().unwrap()));

        match unsafe {
            self.0
                .swapchain_loader
                .queue_present(*self.0.application.main_device().queue(), &present_info)
        } {
            Ok(_) => Ok(()),
            Err(error) => {
                if error == vk::Result::ERROR_OUT_OF_DATE_KHR {
                    self.reload(false)?;
                    return Ok(());
                }
                Err(error)
            }
        }
        .unwrap();

        // Wait for finish operations
        unsafe { device.device_wait_idle() }?;
        Ok(())
    }

    pub fn bind_pipeline(&self, pipeline: &RenderPipeline, descriptor_sets: &[DescriptorSet]) {
        let inner = &self.0;
        let device = inner.application.main_device().virtual_device();
        let window_size = inner.application.window().inner_size();
        unsafe {
            device.cmd_bind_pipeline(
                inner.command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline.vulkan_pipeline.unwrap(),
            );

            let viewport = vk::Viewport::default()
                .width(window_size.width as f32)
                .height(window_size.height as f32);
            device.cmd_set_viewport(inner.command_buffer, 0, slice::from_ref(&viewport));

            let scissor = vk::Rect2D::default().extent(vk::Extent2D {
                width: window_size.width,
                height: window_size.height,
            });
            device.cmd_set_scissor(inner.command_buffer, 0, slice::from_ref(&scissor));
        }

        if !descriptor_sets.is_empty() {
            let raw_descriptor_sets = descriptor_sets
                .iter()
                .map(|value| value.vk_descriptor_set)
                .collect::<Vec<_>>();
            unsafe {
                device.cmd_bind_descriptor_sets(
                    inner.command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    pipeline.vulkan_pipeline_layout.unwrap(),
                    0,
                    raw_descriptor_sets.as_slice(),
                    &[],
                );
            }
        }
    }

    pub fn bind_vertex_buffer(&self, buffer: &Buffer) {
        let inner = &self.0;
        unsafe {
            inner
                .application
                .main_device()
                .virtual_device()
                .cmd_bind_vertex_buffers(
                    inner.command_buffer,
                    0,
                    slice::from_ref(&buffer.buffer),
                    slice::from_ref(&vk::DeviceSize::from(0u32)),
                );
        }
    }

    pub fn draw(&self, vertices: u32) {
        let inner = &self.0;
        unsafe {
            inner.application.main_device().virtual_device().cmd_draw(
                inner.command_buffer,
                vertices,
                4,
                1,
                0,
            );
        }
    }

    pub fn draw_indexed(&self, index_buffer: &Buffer) {
        let inner = &self.0;
        let device = inner.application.main_device().virtual_device();
        let indices = (index_buffer.alloc_info.size / mem::size_of::<u16>() as u64) as u32;
        unsafe {
            device.cmd_bind_index_buffer(
                inner.command_buffer,
                index_buffer.buffer,
                vk::DeviceSize::from(0u32),
                vk::IndexType::UINT16,
            );
            device.cmd_draw_indexed(inner.command_buffer, indices, 1, 0, 0, 0);
        }
    }

    #[inline]
    pub fn find_pipeline(&self, pipeline_name: &str) -> Option<&RenderPipeline> {
        self.0
            .pipelines
            .iter()
            .find(|pipeline| pipeline.name == pipeline_name)
    }
}

/// This enum describes the topology of the project. The topology defines the values for the index buffer
#[derive(Clone, PartialEq)]
pub enum VertexFormat {
    TriangleCoordColor,
    QuadCoordColor,
    QuadCoordImage(Image)
}

impl VertexFormat {
    #[inline]
    pub fn add_indices(&self, indices: &mut Vec<u16>) {
        match self {
            VertexFormat::TriangleCoordColor => indices.extend(vec![0, 1, 2]),
            VertexFormat::QuadCoordColor | VertexFormat::QuadCoordImage(_) => indices.extend(vec![0, 1, 2, 2, 3, 0]),
        }
    }

    #[inline]
    pub const fn vertex_size(&self) -> usize {
        match self {
            VertexFormat::TriangleCoordColor | VertexFormat::QuadCoordColor => mem::size_of::<Vec2>() + mem::size_of::<Vec3>(),
            VertexFormat::QuadCoordImage(_) => mem::size_of::<Vec2>() * 2
        }
    }

    #[inline]
    pub const fn pipeline_name(&self) -> &'static str {
        match self {
            VertexFormat::TriangleCoordColor | VertexFormat::QuadCoordColor => "position_color",
            VertexFormat::QuadCoordImage(_) => "position_texcoord"
        }
    }
}

/// This struct describes the data of a single vertex. The vertex contains the position and the color or uv coordinates.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) struct Vertex {
    position: Vec2,
    color: Option<Vec3>,
    uv: Option<Vec2>,
}

/// This struct represents the buffer builder. The buffer builder allows the renderer to draw batched render calls when
/// possible or non-batched when needed.
#[derive(Clone)]
pub struct BufferBuilder {
    vertices: Vec<Vertex>,
    indices: Vec<u16>,
    current_vertex: Option<Vertex>,
    vertex_format: VertexFormat
}

impl PartialEq for BufferBuilder {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.vertex_format == other.vertex_format
    }
}

impl BufferBuilder {
    #[inline]
    pub fn builder(vertex_format: VertexFormat) -> Self {
        Self {
            vertices: vec![],
            indices: vec![],
            current_vertex: None,
            vertex_format,
        }
    }

    pub fn begin(mut self, x: f32, y: f32) -> Self {
        if let Some(vertex) = self.current_vertex.as_ref() {
            panic!(
                "Error while using buffer builder => The previous vertex ({:?}) has not end",
                vertex
            );
        }

        self.current_vertex = Some(Vertex {
            position: Vec2::new(x, y),
            color: None,
            uv: None,
        });
        self
    }

    pub fn color(mut self, red: f32, green: f32, blue: f32) -> Self {
        let Some(vertex) = self.current_vertex.as_mut() else {
            panic!("Error while using buffer builder => No vertex building has begun, use position before this");
        };

        if vertex.uv.is_some() {
            panic!("Error while using buffer builder => Unable to set color while uv coordinates are set");
        }

        vertex.color = Some(Vec3::new(red, green, blue));
        self
    }

    pub fn uv(mut self, u: f32, v: f32) -> Self {
        let Some(vertex) = self.current_vertex.as_mut() else {
            panic!("Error while using buffer builder => No vertex building has begun, use position before this");
        };

        if vertex.color.is_some() {
            panic!("Error while using buffer builder => Unable to set color while color is set");
        }

        vertex.uv = Some(Vec2::new(u, v));
        self
    }

    pub fn end(mut self) -> Self {
        let Some(vertex) = self.current_vertex else {
            panic!("Error while using buffer builder => No vertex is in building");
        };

        self.vertices.push(vertex);
        self.current_vertex = None;
        self
    }

    #[inline]
    pub fn build(mut self, renderer: &mut GameRenderer) {
        self.vertex_format.add_indices(&mut self.indices);
        unsafe { Arc::get_mut_unchecked(&mut renderer.0) }
            .queued_buffer_builder
            .push(self);
    }
}
