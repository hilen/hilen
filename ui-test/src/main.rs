use std::{collections::BTreeMap, hint::black_box};

use anyhow::Result;
use hilen::ui_test::{UITestEntry, runner::run_cli};

/// Every registered test, from `ui-test-suite`, the app and the engine. They
/// all register into the one engine owned map, so there is nothing to merge.
///
/// Every test registers through a `ctor` and nothing calls it by name, so a
/// linker drops a whole rlib and takes its tests with it. Nothing reports that,
/// the suite just quietly runs fewer tests. This is the same trap that hid
/// every test on iOS, see `keep_ctor_linked` in
/// `hilen/src/app_starter.rs`. So the crates are named here first.
fn all_tests() -> BTreeMap<String, UITestEntry> {
    ui_test_suite::keep_linked();
    black_box(demo::DemoApp);
    hilen::UI_TESTS.lock().clone()
}

fn main() -> Result<()> {
    run_cli(all_tests)
}
