use serde::{Deserialize, Serialize};
use render::pipeline::shader::ShaderKind;

#[derive(Serialize, Deserialize)]
pub(crate) struct ShaderConfiguration {
    pub(crate) resource: String,
    pub(crate) kind: ShaderKind
}

#[derive(Serialize, Deserialize)]
pub(crate) struct PipelineConfiguration {
    pub(crate) name: String,
    pub(crate) shader: Vec<ShaderConfiguration>
}