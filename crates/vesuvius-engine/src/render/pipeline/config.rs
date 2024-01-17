use crate::render::pipeline::shader::ShaderKind;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub(crate) struct ShaderConfiguration {
    pub(crate) resource: String,
    pub(crate) kind: ShaderKind,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct PipelineConfiguration {
    pub(crate) name: String,
    pub(crate) shader: Vec<ShaderConfiguration>,
}
