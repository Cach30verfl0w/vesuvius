use crate::render::buffer::builder::BufferBuilder;
use crate::render::buffer::format::{Topology, VertexFormat};
use crate::render::text::config::FontAtlas;
use crate::render::GameRenderer;
use crate::Result;
use glam::Vec3;
use std::borrow::Cow;
use std::cell::RefCell;
use std::fs;
use std::path::Path;
use crate::render::image::Image;

pub mod config;

pub struct FontRenderer {
    renderer: RefCell<GameRenderer>,
    atlas: FontAtlas,
    atlas_image: Image
}

impl FontRenderer {
    pub fn new<P: AsRef<Path>>(renderer: GameRenderer, path: P) -> Result<Self> {
        let path = path.as_ref();
        let atlas: FontAtlas = {
            let atlas_config_path = path.join("atlas.json");
            if !atlas_config_path.exists() {
                panic!("Atlas config file 'atlas.json' not found in '{:?}'", path);
            }

            serde_json::from_slice(fs::read(atlas_config_path)?.as_slice())?
        };
        Ok(Self {
            atlas_image: Image::from_file(&renderer.0.application, path.join("atlas.png"))?,
            renderer: RefCell::new(renderer),
            atlas,
        })
    }

    pub fn draw(&self, x: f32, y: f32, text: Cow<str>, size: f32, color: Vec3) -> Result<()> {
        let mut builder = BufferBuilder::builder(
            VertexFormat::PositionTexCoordColor,
            Topology::Quad,
            "msdf_font",
        );
        builder.image(&self.atlas_image);
        let mut text_x = x;

        // Enumerate characters
        #[rustfmt::skip]
        for character in text.chars() {
            text_x += self.visit(&mut builder, text_x, y, character, size, color);
        }

        builder.build(&mut self.renderer.borrow_mut());
        Ok(())
    }

    fn visit(&self, buffer_builder: &mut BufferBuilder, x: f32, y: f32, character: char, size: f32, color: Vec3) -> f32 {
        let Some(glyph) = self.atlas.glyphs.iter()
            .find(|value| value.unicode.eq(&(character as u8)))
            else {
                panic!(
                    "Unable to draw char => Character '{}' not found in font atlas",
                    character
                );
            };

        let plane_bounds = &glyph.plane_bounds;
        let atlas_bounds = &glyph.atlas_bounds;
        let font_metrics = &self.atlas.metrics;
        let atlas_meta = &self.atlas.atlas;

        if plane_bounds.right - plane_bounds.left != 0.0 {
            let x0 = x + plane_bounds.left * size;
            let x1 = x + plane_bounds.right * size;

            let y0 = y + font_metrics.ascender * size - plane_bounds.top * size;
            let y1 = y + font_metrics.ascender * size - plane_bounds.bottom * size;

            let u0 = atlas_bounds.left / atlas_meta.width as f32;
            let u1 = atlas_bounds.right / atlas_meta.width as f32;

            let v0 = atlas_bounds.top / atlas_meta.height as f32;
            let v1 = atlas_bounds.bottom / atlas_meta.height as f32;

            buffer_builder.begin(x0, y0).uv(u0, 1.0 - v0).color(color.x, color.y, color.z).end();
            buffer_builder.begin(x0, y1).uv(u0, 1.0 - v1).color(color.x, color.y, color.z).end();
            buffer_builder.begin(x1, y1).uv(u1, 1.0 - v1).color(color.x, color.y, color.z).end();
            buffer_builder.begin(x1, y0).uv(u1, 1.0 - v0).color(color.x, color.y, color.z).end();
        }
        return size * glyph.advance;
    }
}
