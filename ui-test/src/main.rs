use std::{collections::BTreeMap, env::var, hint::black_box, panic::set_hook, path::PathBuf, process::exit};

use anyhow::Result;
use clap::Parser;
use hilen::{
    AppRunner, Window,
    dispatch::{from_main, is_main_thread},
    ui::{Label, UIManager},
    ui_test::{
        TestFailure, UITest, UITestEntry, capture_requested_screenshot, clear_failures, current_test_name,
        enable_color_recording, enable_fps_report, enable_human_mode, enable_screenshot_capture,
        enable_shots, present_test, push_failure, run_test, spaced_test_name, take_failures,
    },
};
use log::info;

#[derive(Parser)]
struct Args {
    /// One test, or a comma separated subset, `make smoke` uses that.
    #[arg(long, short)]
    test_name: Option<String>,

    /// Print every registered test and the total, then exit without running.
    #[arg(long)]
    list: bool,

    #[command(flatten)]
    run: RunArgs,

    #[command(flatten)]
    display: DisplayArgs,
}

/// How the run reacts to failures and what it reports.
#[derive(clap::Args)]
struct RunArgs {
    #[arg(long)]
    fps_report: bool,

    /// Print ready to paste `check_colors` blocks instead of asserting them.
    #[arg(long)]
    record_colors: bool,
}

/// Where and how the frames are shown.
#[derive(clap::Args)]
struct DisplayArgs {
    #[arg(long)]
    headless: bool,

    /// Watchable run: slows injections, shows touches, holds after
    /// each test until ctrl is pressed.
    #[arg(long)]
    human: bool,

    /// Save a test screenshot without opening a window. Requires --test-name.
    #[arg(long, value_name = "PATH")]
    screenshot: Option<PathBuf>,

    /// Save a clean frame into this dir at every `check_colors` and every
    /// checkpoint, no window, no probe markers. For agents that need to
    /// see every verified state of a run.
    #[arg(long, value_name = "DIR")]
    shots: Option<PathBuf>,

    /// Presentation mode: build one test's view over the whole window and
    /// hand it over. Nothing is injected and `perform_test` never runs, so
    /// a human can play with the view. Requires exactly one --test-name.
    #[arg(long)]
    present: bool,
}

/// Names the crates whose tests this runner covers, so a linker keeps them.
///
/// Every test registers through a `ctor` and nothing calls it by name, so a
/// linker drops a whole rlib and takes its tests with it. Nothing reports that,
/// the suite just quietly runs fewer tests. This is the same trap that hid
/// every test on iOS, see `keep_ctor_linked` in
/// `hilen/src/app_starter.rs`.
fn keep_tests_linked() {
    ui_test_suite::keep_linked();
    black_box(demo::DemoApp);
}

/// Every registered test, from the corpus, the app and the engine. They all
/// register into the one engine owned map, so there is nothing to merge.
fn all_tests() -> BTreeMap<String, UITestEntry> {
    keep_tests_linked();
    hilen::UI_TESTS.lock().clone()
}

/// Validates the mode flags against each other and switches the chosen
/// ones on.
fn apply_modes(args: &Args) -> Result<()> {
    if args.run.fps_report {
        enable_fps_report();
    }

    if args.display.human {
        if args.display.headless || args.display.screenshot.is_some() || args.display.shots.is_some() {
            anyhow::bail!("--human requires a window, remove --headless, --screenshot and --shots");
        }
        enable_human_mode();
    }

    if let Some(path) = args.display.screenshot.clone() {
        anyhow::ensure!(
            args.test_name.as_ref().is_some_and(|names| !names.contains(',')),
            "--screenshot requires exactly one --test-name"
        );
        enable_screenshot_capture(path);
    }

    if let Some(dir) = args.display.shots.clone() {
        enable_shots(dir)?;
    }

    if args.run.record_colors {
        enable_color_recording();
    }

    if args.display.present {
        anyhow::ensure!(
            !args.display.headless
                && args.display.screenshot.is_none()
                && args.display.shots.is_none()
                && !args.display.human,
            "--present is its own mode, remove --headless, --screenshot, --shots and --human"
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

    // A suite that runs nothing otherwise reports success, which looks exactly
    // like a suite that passes. Registration is a ctor nothing calls by name,
    // so an empty map means the `ui-tests` feature is off or a linker
    // dropped a whole crate, never that there are no tests.
    anyhow::ensure!(
        !tests.is_empty(),
        "No UI tests registered. Either the `hilen/ui-tests` feature is off, or a linker \
         dropped a test crate whose ctors nothing references, see `keep_tests_linked`.",
    );

    if args.list {
        for name in tests.keys() {
            println!("{name}");
        }
        println!("\n{} UI tests", tests.len());
        return Ok(());
    }

    let test_name = args.test_name;
    let human = args.display.human;

    if args.display.present {
        let name = test_name.expect("checked above");
        anyhow::ensure!(
            tests.contains_key(&spaced_test_name(&name)),
            "UI test not found: {name}. Run `cargo run -p ui-test -- --list` to see every registered test."
        );
        return AppRunner::start_with_actor(async move {
            present_test(&name)?;
            println!(
                "{}: presented, close the window to finish",
                spaced_test_name(&name)
            );
            Ok(())
        });
    }

    let total = test_name.as_ref().map_or(tests.len(), |names| names.split(',').count());

    let actor = async move {
        Label::set_default_text_size(32);
        UIManager::set_display_touches(human);

        from_main(move || {
            UIManager::override_scale(1.0);

            if !human {
                Window::set_vsync(false);
                Window::set_max_frame_latency(3);
            }
        });

        clear_failures();

        if let Some(test_name) = test_name {
            // Also accept the struct ident, so a tool reading `impl ViewTest
            // for ScrollViewTest` off the source can pass what it
            // sees without deriving the spaced name itself.
            // `spaced_test_name` is the one place that rule lives,
            // and drifting from it is what made the old
            // generated `#[test]` pass a name the runner rejected.
            let names: Vec<&str> = test_name.split(',').map(str::trim).collect();

            // Every name resolves before anything runs, so a typo in a
            // subset never looks like a shorter green run.
            let mut missing = false;
            for name in &names {
                if !tests.contains_key(&spaced_test_name(name)) {
                    eprintln!("UI test not found: {name}");
                    missing = true;
                }
            }
            if missing {
                eprintln!("Run `cargo run -p ui-test -- --list` to see every registered test.");
                exit(1);
            }

            for name in names {
                let key = spaced_test_name(name);
                run_test(&key, tests[&key].run);

                if let Err(error) = capture_requested_screenshot() {
                    push_failure(&key, format!("screenshot capture failed: {error}"));
                }
            }

            UITest::finish();
            AppRunner::stop();
            return Ok(());
        }

        let cycles: u32 = var("UI_TEST_CYCLES").unwrap_or("2".to_string()).parse().unwrap();

        for i in 1..=cycles {
            for (name, test) in &tests {
                run_test(name, test.run);
            }
            info!("Cycle {i}: OK");
        }

        UITest::finish();
        AppRunner::stop();

        Ok(())
    };

    if args.display.headless || args.display.screenshot.is_some() || args.display.shots.is_some() {
        AppRunner::start_headless_with_actor(actor)?;
    } else {
        AppRunner::start_with_actor(actor)?;
    }

    let failures = take_failures();

    if failures.is_empty() {
        println!("{total} UI tests passed");
        return Ok(());
    }

    report_failures(total, &failures);
    exit(1);
}

/// A panic inside a `from_main` closure runs on the main thread and kills the
/// frame loop, so `CatchUnwind` on the actor never sees it and any pending
/// `from_main` hangs. Detect that case, report everything gathered so far plus
/// the fatal test, and exit. Actor thread panics are left to `CatchUnwind`.
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

/// Print every failed test once, most useful line first, then the full detail.
fn report_failures(total: usize, failures: &[TestFailure]) {
    let mut seen = std::collections::BTreeSet::new();
    let unique: Vec<&TestFailure> = failures.iter().filter(|f| seen.insert(f.name.clone())).collect();

    eprintln!("\n{} of {total} UI test(s) failed:", unique.len());
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
