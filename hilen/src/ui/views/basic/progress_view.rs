use crate::{
    deps::refs::Weak,
    gm::{ToF32, color::BLUE},
    ui::{Container, Setup, ViewData, view},
};

#[view]
pub struct ProgressView {
    progress: f32,
    #[init]
    pub bar:  Container,
}

impl ProgressView {
    pub fn inc_progress(self: Weak<Self>, progress: impl ToF32) -> Weak<Self> {
        self.set_progress(self.progress + progress.to_f32())
    }

    pub fn set_progress(mut self: Weak<Self>, progress: impl ToF32) -> Weak<Self> {
        self.progress = progress.to_f32();
        self.bar.place().clear().tlb(0).relative_width(self, self.progress);
        self
    }
}

impl Setup for ProgressView {
    fn setup(self: Weak<Self>) {
        self.bar.set_color(BLUE).place().back();
    }
}
