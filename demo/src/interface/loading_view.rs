use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::Result;
use hilen::{
    dispatch::{on_main, spawn},
    filesystem::Assets,
    refs::Weak,
    ui::{
        CLEAR, Container, Label, ProgressView, Setup, Spinner, UIManager, ViewData, ViewSubviews, ViewTest,
        view,
    },
};

use crate::interface::{
    HomeView,
    palette::{ACCENT, BG, TEXT_DIM},
};

/// The boot preload touches the view on every progress tick, so a test
/// that returns before it finishes leaves it dereferencing a freed
/// pointer.
static LOADED: AtomicBool = AtomicBool::new(false);

#[view]
pub struct LoadingView {
    #[init]
    spinner:  Container,
    label:    Label,
    progress: ProgressView,
}

impl Setup for LoadingView {
    fn setup(self: Weak<Self>) {
        LOADED.store(false, Ordering::Relaxed);
        UIManager::set_app_ready(false);
        UIManager::set_clear_color(BG);

        self.spinner.place().center().size(200, 200);

        self.label
            .set_text("Loading...")
            .set_color(CLEAR)
            .set_text_color(TEXT_DIM)
            .place()
            .above(self.spinner, 20)
            .h(40);

        self.progress.place().lrb(0).h(4);

        let mut spinner = self.spinner.add_view::<Spinner>();
        spinner.place().back();
        spinner.dot_color = ACCENT.dark;

        Assets::load_progress().val(move |progress| {
            self.progress.set_progress(progress);
        });

        spawn(async move {
            Assets::await_boot().await;

            on_main(|| {
                // The event outlives this view, and the next LoadingView
                // asserts on a leftover subscriber.
                Assets::load_progress().remove_subscribers();

                UIManager::set_view(HomeView::new());

                LOADED.store(true, Ordering::Relaxed);
                UIManager::set_app_ready(true);
            });
        });
    }
}

impl ViewTest for LoadingView {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        // The boot wait touches the view from its task. Returning before
        // it is done frees the view under it and the next test dies on
        // the dangling deref, so hold until the swap to HomeView landed.
        while !LOADED.load(Ordering::Relaxed) {
            std::hint::spin_loop();
        }

        Ok(())
    }
}
