use hilen::{
    App, Window,
    refs::Own,
    ui::{Button, Setup, Size, View},
};
#[cfg(not_wasm)]
use hilen::{PinnedFuture, net::SecretsManager};

#[cfg(feature = "bench")]
use crate::interface::dev::UIBenchmarkView;
use crate::interface::{BUTTON, loading_view::LoadingView};

#[cfg(not_wasm)]
async fn secrets() -> anyhow::Result<&'static SecretsManager> {
    use std::env::var;

    use anyhow::Context;
    use tokio::sync::OnceCell;

    static SECRETS: OnceCell<SecretsManager> = OnceCell::const_new();

    SECRETS
        .get_or_try_init(|| async {
            let client_secret = var("INFISICAL_TE").context("INFISICAL_TE")?;

            let manager = SecretsManager::new(
                "49d67108-3678-45de-b28c-912519d5d3a0",
                client_secret,
                "d8a0c826-859b-406f-b876-ddf98cb5a6f6",
                "dev",
            )
            .await
            .context("Secrets Manager init")?;

            Ok(manager)
        })
        .await
}

#[derive(Default)]
pub struct DemoApp;

impl App for DemoApp {
    fn before_launch(&self) {
        BUTTON.apply_globally::<Button>();

        // demo is the app the suite runs on a device, so it carries the
        // whole corpus. The tests register through `ctor`s that nothing calls
        // by name, so without this the linker drops them and the device
        // quietly runs a fraction of the suite.
        ui_test_suite::keep_linked();
    }

    fn after_launch(&self) {
        Window::set_quit_on_escape(true);
        Window::set_icon(include_bytes!("../../assets/images/engine.png"));
    }

    fn make_root_view(&self) -> Own<dyn View> {
        #[cfg(feature = "bench")]
        {
            use std::env::{args, var};

            use crate::interface::dev::guard_benchmark;

            if var("UI_BENCHMARK").is_ok() {
                let force = args().any(|arg| arg == "--no-guard");
                guard_benchmark(force);
                return UIBenchmarkView::new();
            }
        }
        LoadingView::new()
    }

    fn initial_size(&self) -> Size {
        (1500, 1200).into()
    }

    #[cfg(not_wasm)]
    fn sentry_url(&self) -> PinnedFuture<Option<String>> {
        Box::pin(async {
            dotenvy::dotenv()?;
            let url = secrets().await?.get("SENTRY_URL").await?;
            Ok(Some(url))
        })
    }
}
