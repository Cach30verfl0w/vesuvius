use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct Bounds {
    left: f32,
    top: f32,
    right: f32,
    bottom: f32
}

#[derive(Deserialize, Serialize)]
pub(crate) struct Glyph {
    unicode: u8,
    advance: f32,
    #[serde(rename = "planeBounds")]
    plane_bounds: Bounds,
    #[serde(rename = "atlasBounds")]
    atlas_bounds: Bounds
}

#[derive(Deserialize, Serialize)]
pub(crate) struct Metrics {
    #[serde(rename = "emSize")]
    em_size: u8,
    #[serde(rename = "lineHeight")]
    line_height: f32,
    ascender: f32,
    descender: f32,
    #[serde(rename = "underlineY")]
    underline_y: f32,
    #[serde(rename = "underlineThickness")]
    underline_thickness: f32
}

#[derive(Deserialize, Serialize)]
pub(crate) struct AtlasMeta {
    #[serde(rename = "type")]
    kind: String,
    #[serde(rename = "distanceRange")]
    distance_range: u8,
    size: u8,
    width: u16,
    height: u16,
    #[serde(rename = "yOrigin")]
    origin_y: String
}

#[derive(Deserialize, Serialize)]
pub(crate) struct FontAtlas {
    atlas: AtlasMeta,
    metrics: Metrics,
    glyphs: Vec<Glyph>
}