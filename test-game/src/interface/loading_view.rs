use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::{Ok, Result};
use log::error;
use test_engine::{
    Platform,
    dispatch::{from_main, on_main, spawn},
    gm::LossyConvert,
    refs::{Weak, manage::DataManager},
    ui::{
        AlertErr, CLEAR, Container, Image, LIGHT_BLUE, Label, ProgressView, Setup, Spinner, UIManager,
        ViewData, ViewSubviews, ViewTest, view,
    },
};

use crate::interface::HomeView;

/// The load task touches the view on every asset, so a test that returns before
/// it finishes leaves it dereferencing a freed pointer.
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

        self.spinner.place().center().size(200, 200);

        self.label
            .set_text("Loading...")
            .set_color(CLEAR)
            .place()
            .above(self.spinner, 20)
            .h(40);

        self.progress.place().lrb(0).h(20);

        let mut spinner = self.spinner.add_view::<Spinner>();
        spinner.place().back();
        spinner.dot_color = LIGHT_BLUE;

        spawn(async move {
            self.load(vec![
                "frisk.png",
                "board.png",
                "shop.png",
                "stone_floor.png",
                "triangle.png",
                "sky.png",
                "square.png",
                "bullet.png",
                "cat.png",
                "crate_box.png",
            ])
            .await
            .alert_err();
        });
    }
}

impl LoadingView {
    async fn load(self: Weak<Self>, assets: Vec<&str>) -> Result<()> {
        let part = 1.0 / assets.len().lossy_convert();

        for asset in assets {
            if Platform::WASM {
                if let Err(err) = Self::download_asset(asset).await {
                    error!("{err}");
                }
            } else {
                Self::load_asset(asset.to_owned());
            }

            on_main(move || {
                self.progress.inc_progress(part);
            });
        }

        UIManager::set_view(HomeView::new());

        LOADED.store(true, Ordering::Relaxed);
        UIManager::set_app_ready(true);

        Ok(())
    }

    fn load_asset(path: String) {
        from_main(move || {
            Image::get(path);
        });
    }

    async fn download_asset(path: &str) -> Result<()> {
        Image::download(&path, &format!("/assets/images/{path}")).await?;

        Ok(())
    }
}

impl ViewTest for LoadingView {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        // Every asset touches the view from the load task. Returning before it
        // is done frees the view under it and the next test dies on the
        // dangling deref, so hold until the last asset has landed.
        while !LOADED.load(Ordering::Relaxed) {
            std::hint::spin_loop();
        }

        Ok(())
    }
}
