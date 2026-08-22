use log::debug;

use crate::{deps::refs::main_lock::MainLock, render::UIRectPipeline};

static PIPELINES: MainLock<Pipelines> = MainLock::new();

pub(crate) struct Pipelines {
    pub rect: UIRectPipeline,
}

impl Pipelines {
    pub(crate) fn initialize() {
        assert!(!PIPELINES.is_set(), "Double pipelines init");

        PIPELINES.set(Pipelines {
            rect: UIRectPipeline::default(),
        });

        debug!("pipelines ready");
    }

    fn get() -> &'static mut Self {
        PIPELINES.try_get_mut().expect("Pipelines not initialized yet")
    }

    pub fn rect() -> &'static mut UIRectPipeline {
        &mut Self::get().rect
    }
}
