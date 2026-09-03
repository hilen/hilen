mod shader_lab;
/// The uniform instance path, the WebGL2 way to read instances.
#[cfg(feature = "ui-tests")]
mod uniform_instances_test;

pub use shader_lab::{ShaderLab, ShaderVariant};
