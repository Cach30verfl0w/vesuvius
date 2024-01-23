use std::sync::Arc;
use glam::{Vec2, Vec3};
use crate::render::buffer::format::{Topology, VertexFormat};
use crate::render::GameRenderer;

/// This struct describes the data of a single vertex. The vertex contains the position and the color or uv coordinates.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) struct Vertex {
    pub(crate) position: Vec2,
    pub(crate) color: Option<Vec3>,
    pub(crate) uv: Option<Vec2>,
}

/// This struct represents the buffer builder. The buffer builder allows the renderer to draw batched render calls when
/// possible or non-batched when needed.
#[derive(Clone, Debug)]
pub struct BufferBuilder {
    pub(crate) vertices: Vec<Vertex>,
    pub(crate) vertex_format: VertexFormat,
    pub(crate) topology: Topology,
    current_vertex: Option<Vertex>
}

impl PartialEq for BufferBuilder {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.vertex_format == other.vertex_format && self.topology == other.topology
    }
}

impl BufferBuilder {
    #[inline]
    pub fn builder(vertex_format: VertexFormat, topology: Topology) -> Self {
        Self {
            vertices: vec![],
            current_vertex: None,
            vertex_format,
            topology
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
    pub fn build(self, renderer: &mut GameRenderer) {
        unsafe { Arc::get_mut_unchecked(&mut renderer.0) }
            .queued_buffer_builder
            .push(self);
    }
}
