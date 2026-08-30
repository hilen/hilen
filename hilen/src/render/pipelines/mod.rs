#[cfg(feature = "level")]
use crate::render::{
    SpriteView,
    data::{SpriteInstance, TexturedSpriteInstance},
};
use crate::render::{
    data::{RectView, UIRectInstance},
    pipelines::{pipeline_type::PipelineType, rect_pipeline::RectPipeline},
};

#[cfg(feature = "level")]
mod background_pipeline;
mod lab_pipeline;
mod pipeline_type;
#[cfg(feature = "level")]
mod polygon_pipeline;
mod rect_pipeline;
mod ui_backdrop_pipeline;
mod ui_blur_pipeline;
mod ui_clip_pipeline;
mod ui_path_pipeline;

#[cfg(feature = "level")]
const SPRITE_CODE: &str = include_str!("shaders/sprite.wgsl");
#[cfg(feature = "level")]
const TEXTURED_SPRITE_CODE: &str = include_str!("shaders/sprite_textured.wgsl");
const UI_CODE: &str = include_str!("shaders/ui_rect.wgsl");
const UI_IMAGE_CODE: &str = include_str!("shaders/ui_image.wgsl");
const UI_GRADIENT_CODE: &str = include_str!("shaders/ui_gradient.wgsl");
const UI_SHADOW_CODE: &str = include_str!("shaders/ui_shadow.wgsl");

#[cfg(feature = "level")]
pub(crate) type SpriteBoxPipeline =
    RectPipeline<{ PipelineType::Color }, "sprite_box", SPRITE_CODE, SpriteView, SpriteInstance>;
#[cfg(feature = "level")]
pub(crate) type TexturedSpriteBoxPipeline = RectPipeline<
    { PipelineType::Image },
    "textured_sprite_box",
    TEXTURED_SPRITE_CODE,
    SpriteView,
    TexturedSpriteInstance,
>;

pub type UIRectPipeline = RectPipeline<{ PipelineType::Color }, "ui_rect", UI_CODE, RectView, UIRectInstance>;

pub type UIImageRectPipeline =
    RectPipeline<{ PipelineType::Image }, "ui_image_rect", UI_IMAGE_CODE, RectView, UIImageInstance>;

pub(crate) type UIGradientPipeline =
    RectPipeline<{ PipelineType::Color }, "ui_gradient", UI_GRADIENT_CODE, RectView, UIGradientInstance>;

pub(crate) type UIShadowPipeline =
    RectPipeline<{ PipelineType::Color }, "ui_shadow", UI_SHADOW_CODE, RectView, UIShadowInstance>;

#[cfg(feature = "level")]
pub use background_pipeline::BackgroundPipeline;
pub(crate) use lab_pipeline::LabPipeline;
#[cfg(feature = "level")]
pub use polygon_pipeline::PolygonPipeline;
pub use rect_pipeline::ImageKey;
pub use ui_backdrop_pipeline::UIBackdropPipeline;
pub use ui_blur_pipeline::UIBlurPipeline;
pub(crate) use ui_clip_pipeline::UIClipPipeline;
pub use ui_path_pipeline::UIPathPipeline;

use crate::render::data::{UIGradientInstance, UIImageInstance, UIShadowInstance};
