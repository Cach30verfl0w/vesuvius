use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Default)]
pub struct Bounds {
    pub(crate) left: f32,
    pub(crate) top: f32,
    pub(crate) right: f32,
    pub(crate) bottom: f32,
}

#[derive(Deserialize, Serialize)]
pub(crate) struct Glyph {
    pub(crate) unicode: u8,
    pub(crate) advance: f32,
    #[serde(rename = "planeBounds", default = "Bounds::default")]
    pub(crate) plane_bounds: Bounds,
    #[serde(rename = "atlasBounds", default = "Bounds::default")]
    pub(crate) atlas_bounds: Bounds,
}

#[derive(Deserialize, Serialize)]
pub(crate) struct Metrics {
    #[serde(rename = "emSize")]
    pub(crate) em_size: u8,
    #[serde(rename = "lineHeight")]
    pub(crate) line_height: f32,
    pub(crate) ascender: f32,
    pub(crate) descender: f32,
    #[serde(rename = "underlineY")]
    pub(crate) underline_y: f32,
    #[serde(rename = "underlineThickness")]
    pub(crate) underline_thickness: f32,
}

#[derive(Deserialize, Serialize)]
pub(crate) struct AtlasMeta {
    #[serde(rename = "type")]
    pub(crate) kind: String,
    #[serde(rename = "distanceRange")]
    pub(crate) distance_range: u8,
    pub(crate) size: u8,
    pub(crate) width: u16,
    pub(crate) height: u16,
    #[serde(rename = "yOrigin")]
    pub(crate) origin_y: String,
}

#[derive(Deserialize, Serialize)]
pub(crate) struct FontAtlas {
    pub(crate) atlas: AtlasMeta,
    pub(crate) metrics: Metrics,
    pub(crate) glyphs: Vec<Glyph>,
}
