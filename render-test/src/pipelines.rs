use hilen::{
    refs::main_lock::MainLock,
    render::{UIImageRectPipeline, UIPathPipeline, UIRectPipeline},
};

pub(crate) static UI_RECT: MainLock<UIRectPipeline> = MainLock::new();
pub(crate) static IMAGE_DRAWER: MainLock<UIImageRectPipeline> = MainLock::new();

pub(crate) static PATH: MainLock<UIPathPipeline> = MainLock::new();
