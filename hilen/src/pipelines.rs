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

    /// Whether the pipelines of the app are built.
    pub(crate) fn ready() -> bool {
        PIPELINES.is_set()
    }

    /// Makes the pipelines again, for a new sample count.
    pub(crate) fn rebuild() {
        PIPELINES.set(Pipelines {
            rect: UIRectPipeline::default(),
        });
    }

    fn get() -> &'static mut Self {
        PIPELINES.try_get_mut().expect("Pipelines not initialized yet")
    }

    pub fn rect() -> &'static mut UIRectPipeline {
        &mut Self::get().rect
    }
}
