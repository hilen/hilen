//! The level test runner. Every `impl LevelTest` registers into
//! `hilen::LEVEL_TESTS` through a ctor, the same way a UI test registers
//! into `UI_TESTS`, and this binary runs that map. The corpus lives in
//! this crate, so nothing has to keep a separate rlib linked.

#![allow(incomplete_features)]
#![feature(specialization)]

mod cutout;

use std::{collections::BTreeMap, panic::set_hook, path::PathBuf, process::exit};

use anyhow::Result;
use clap::Parser;
use hilen::{
    AppRunner, Window,
    dispatch::{from_main, is_main_thread},
    ui::UIManager,
    ui_test::{
        TestFailure, UITest, UITestEntry, capture_requested_screenshot, clear_failures, current_test_name,
        enable_color_recording, enable_human_mode, enable_screenshot_capture, push_failure, run_test,
        spaced_test_name, take_failures,
    },
};

#[derive(Parser)]
struct Args {
    /// One test, or a comma separated subset.
    #[arg(long, short)]
    test_name: Option<String>,

    /// Print every registered test and the total, then exit without running.
    #[arg(long)]
    list: bool,

    /// Print ready to paste `check_colors` blocks instead of asserting them.
    #[arg(long)]
    record_colors: bool,

    #[command(flatten)]
    display: DisplayArgs,
}

/// Where and how the frames are shown.
#[derive(clap::Args)]
struct DisplayArgs {
    #[arg(long)]
    headless: bool,

    /// Watchable run, holds after each check until ctrl is pressed.
    #[arg(long)]
    human: bool,

    /// Save a test screenshot without opening a window. Requires --test-name.
    #[arg(long, value_name = "PATH")]
    screenshot: Option<PathBuf>,

    /// Build one test's level over the whole window and hand it over,
    /// `perform_test` never runs. Requires exactly one --test-name.
    #[arg(long)]
    present: bool,
}

fn all_tests() -> BTreeMap<String, UITestEntry> {
    hilen::LEVEL_TESTS.lock().clone()
}

fn apply_modes(args: &Args) -> Result<()> {
    if args.display.human {
        anyhow::ensure!(
            !args.display.headless && args.display.screenshot.is_none(),
            "--human requires a window, remove --headless and --screenshot"
        );
        enable_human_mode();
    }

    if let Some(path) = args.display.screenshot.clone() {
        anyhow::ensure!(
            args.test_name.as_ref().is_some_and(|names| !names.contains(',')),
            "--screenshot requires exactly one --test-name"
        );
        enable_screenshot_capture(path);
    }

    if args.record_colors {
        enable_color_recording();
    }

    if args.display.present {
        anyhow::ensure!(
            !args.display.headless && args.display.screenshot.is_none() && !args.display.human,
            "--present is its own mode, remove --headless, --screenshot and --human"
        );
        anyhow::ensure!(
            args.test_name.as_ref().is_some_and(|names| !names.contains(',')),
            "--present requires exactly one --test-name"
        );
    }

    Ok(())
}

fn run(args: Args) -> Result<()> {
    apply_modes(&args)?;

    install_fatal_panic_hook();

    let tests = all_tests();

    anyhow::ensure!(
        !tests.is_empty(),
        "No level tests registered. The `hilen/ui-tests` feature is off.",
    );

    if args.list {
        for name in tests.keys() {
            println!("{name}");
        }
        println!("\n{} level tests", tests.len());
        return Ok(());
    }

    let names: Vec<String> = match &args.test_name {
        Some(names) => names.split(',').map(|name| spaced_test_name(name.trim())).collect(),
        None => tests.keys().cloned().collect(),
    };

    let mut missing = false;
    for name in &names {
        if !tests.contains_key(name) {
            eprintln!("Level test not found: {name}");
            missing = true;
        }
    }
    if missing {
        eprintln!("Run `cargo run -p level-test -- --list` to see every registered test.");
        exit(1);
    }

    if args.display.present {
        let name = names[0].clone();
        let present = tests[&name].present;
        return AppRunner::start_with_actor(async move {
            present();
            println!("{name}: presented, close the window to finish");
            Ok(())
        });
    }

    let total = names.len();
    let human = args.display.human;

    let actor = async move {
        from_main(move || {
            UIManager::override_scale(1.0);

            if !human {
                Window::set_vsync(false);
                Window::set_max_frame_latency(3);
            }
        });

        clear_failures();

        for name in &names {
            run_test(name, tests[name].run);

            if let Err(error) = capture_requested_screenshot() {
                push_failure(name, format!("screenshot capture failed: {error}"));
            }
        }

        UITest::finish();
        AppRunner::stop();

        Ok(())
    };

    if args.display.headless || args.display.screenshot.is_some() {
        AppRunner::start_headless_with_actor(actor)?;
    } else {
        AppRunner::start_with_actor(actor)?;
    }

    let failures = take_failures();

    if failures.is_empty() {
        println!("{total} level tests passed");
        return Ok(());
    }

    report_failures(total, &failures);
    exit(1);
}

/// A panic inside a `from_main` closure kills the frame loop and hangs
/// the actor, see the same hook in `ui-test`.
fn install_fatal_panic_hook() {
    set_hook(Box::new(move |info| {
        if !is_main_thread() {
            return;
        }

        let name = current_test_name();
        let name = if name.is_empty() {
            "unknown".to_string()
        } else {
            name
        };
        push_failure(&name, format!("main thread panic: {info}"));
        report_failures(all_tests().len(), &take_failures());
        exit(1);
    }));
}

fn report_failures(total: usize, failures: &[TestFailure]) {
    let mut seen = std::collections::BTreeSet::new();
    let unique: Vec<&TestFailure> = failures.iter().filter(|f| seen.insert(f.name.clone())).collect();

    eprintln!("\n{} of {total} level test(s) failed:", unique.len());
    for f in &unique {
        eprintln!("  - {}", f.name);
    }

    for f in &unique {
        eprintln!("\n===== {} =====\n{}", f.name, f.detail);
    }
}

fn main() -> Result<()> {
    run(Args::parse())
}
