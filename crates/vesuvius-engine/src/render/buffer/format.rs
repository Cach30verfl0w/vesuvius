use crate::render::buffer::builder::Vertex;
use glam::{Vec2, Vec3};
use std::mem;

/// This enum represents the format of a single vertex in the buffer. The engine in Vesuvius
/// implements a few vertex formats like the [VertexFormat::PositionColor] format. The renderer
/// can use these formats to determine the size of the buffer.
#[derive(Clone, Copy, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub enum VertexFormat {
    PositionColor,
    PositionTexCoord,
    PositionTexCoordColor,
}

impl VertexFormat {
    /// This function returns the evaluated size of the specified vertex format
    pub const fn vertex_size(&self) -> usize {
        match self {
            Self::PositionColor => mem::size_of::<Vec2>() + mem::size_of::<Vec3>(),
            Self::PositionTexCoord => mem::size_of::<Vec2>() * 2,
            Self::PositionTexCoordColor => mem::size_of::<Vec2>() * 2 + mem::size_of::<Vec3>(),
        }
    }

    /// This function converts the specified vertex into the raw byte structure
    pub(crate) fn extend_raw_data(&self, raw_data: &mut Vec<u8>, vertex: Vertex) {
        raw_data.extend(unsafe { mem::transmute::<Vec2, [u8; 8]>(vertex.position) });
        match self {
            Self::PositionColor => {
                raw_data.extend(unsafe { mem::transmute::<Vec3, [u8; 12]>(vertex.color.unwrap()) });
            }
            Self::PositionTexCoord => {
                raw_data.extend(unsafe { mem::transmute::<Vec2, [u8; 8]>(vertex.uv.unwrap()) });
            }
            Self::PositionTexCoordColor => {
                raw_data.extend(unsafe { mem::transmute::<Vec3, [u8; 12]>(vertex.color.unwrap()) });
                raw_data.extend(unsafe { mem::transmute::<Vec2, [u8; 8]>(vertex.uv.unwrap()) });
            }
        }
    }
}

/// The topology is used to create the content for the index buffer. The engine uses the topology
/// in combination with the vertex format to generate the information for the pipeline
#[derive(Clone, Copy, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub enum Topology {
    Quad,
    Triangle,
}

impl Topology {
    /// This function returns a vector filled with the indices
    #[inline]
    pub fn indices(&self, offset: u16) -> Vec<u16> {
        match self {
            Self::Quad => vec![
                offset,
                offset + 1,
                offset + 3,
                offset + 3,
                offset + 1,
                offset + 2,
            ],
            Self::Triangle => vec![offset, offset + 1, offset + 2],
        }
    }

    pub const fn vertex_count(&self) -> usize {
        match self {
            Self::Quad => 4,
            Self::Triangle => 3
        }
    }
}
