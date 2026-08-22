#![allow(incomplete_features)]
#![feature(specialization)]

mod occlusion;
mod path;
mod pipelines;
pub(crate) mod render;

use anyhow::Result;
use hilen::AppRunner;

use crate::render::test_render;

fn main() -> Result<()> {
    AppRunner::start_with_actor(async {
        hilen::ui::UIManager::set_display_touches(true);

        test_render().await?;

        AppRunner::stop();

        Ok(())
    })
}
