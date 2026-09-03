use std::collections::BTreeMap;

use super::{TestFailure, UITest, UITestEntry, clear_failures, run_test, take_failures};
#[cfg(feature = "level")]
use crate::level::LevelManager;
use crate::{
    deps::hreads::from_main,
    ui::{Label, Style, UIColor, UIManager, ViewData, style::GlobalStyles},
};

pub struct TestRunReport {
    pub total:    usize,
    pub failures: Vec<TestFailure>,
}

/// Everything a run takes from the app, kept so it can be handed back.
///
/// A run is not read only. It pins scale 1, forces its own text size, paints
/// its own background and tears the app's root view down. Leave any of it
/// behind and the app carries on wrong: at scale 1 on a phone that is really 2,
/// boxed into the test canvas, or with no root view at all.
struct AppState {
    styles:         GlobalStyles,
    text_size:      f32,
    scale_override: f32,
    clear_color:    UIColor,
    bug_animation:  Option<&'static [u8]>,
}

/// Tests expect scale 1 and 32 point text. Any host that runs them must match,
/// or every layout and color check drifts.
fn prepare_harness() -> AppState {
    let state = from_main(|| AppState {
        styles:         Style::take_globals(),
        text_size:      Label::default_text_size(),
        scale_override: UIManager::scale_override(),
        clear_color:    UIManager::clear_color(),
        bug_animation:  crate::BugReport::animation(),
    });

    Label::set_default_text_size(32);

    from_main(|| {
        UIManager::override_scale(1.0);
    });

    state
}

/// Give the app back everything the run took, and a root view to live in.
fn restore_app(state: AppState) {
    Label::set_default_text_size(state.text_size);
    crate::BugReport::restore_animation(state.bug_animation);

    from_main(move || {
        Style::restore_globals(state.styles);
        UIManager::restore_scale_override(state.scale_override);
        UIManager::set_clear_color(state.clear_color);

        let mut root = UIManager::root_view();
        root.clear_root();
        root.reset_background();
        root.clear_test_canvas();
        #[cfg(feature = "level")]
        LevelManager::stop_level();
        #[cfg(feature = "scene")]
        crate::scene::SceneManager::stop_scene();
        root.add_subview_to_root(crate::app::app().make_root_view()).place().back();
    });
}

/// The UI map, then the scene map. Two maps, a view and a scene may share
/// a name, `Transparency` does.
pub fn registered_test_maps() -> Vec<BTreeMap<String, UITestEntry>> {
    let mut maps = vec![crate::UI_TESTS.lock().clone()];
    #[cfg(feature = "scene")]
    maps.push(crate::SCENE_TESTS.lock().clone());
    maps
}

/// `HILEN_TEST_ONLY`, a comma separated list in either spelling.
pub fn keep_named(maps: &mut [BTreeMap<String, UITestEntry>], names: &str) {
    let keep = spaced_names(names);
    for tests in maps {
        tests.retain(|name, _| keep.contains(name));
    }
}

/// `hilen_test_skip`, the same list taken out.
pub fn drop_named(maps: &mut [BTreeMap<String, UITestEntry>], names: &str) {
    let drop = spaced_names(names);
    for tests in maps {
        tests.retain(|name, _| !drop.contains(name));
    }
}

fn spaced_names(names: &str) -> Vec<String> {
    names.split(',').map(|name| super::spaced_test_name(name.trim())).collect()
}

/// Runs the maps in turn under one harness into one report. Must not run
/// on the main thread, the tests drive it through `from_main`.
pub fn run_test_maps(maps: &[BTreeMap<String, UITestEntry>]) -> TestRunReport {
    let state = prepare_harness();
    clear_failures();

    #[cfg(not_wasm)]
    super::watchdog::start_run();

    for tests in maps {
        for (name, test) in tests {
            run_test(name, test.run);
        }
    }

    // The last test's OK line and the human mode hold both live in
    // `finish`. Only the desktop runner called it, so in-app runs
    // dropped the last OK and a browser human run never held. Before
    // `restore_app`, the held view must still be on screen.
    UITest::finish();

    let report = TestRunReport {
        total:    maps.iter().map(BTreeMap::len).sum(),
        failures: take_failures(),
    };

    restore_app(state);

    report
}

/// Show one registered test's view full screen for a human to play with.
/// Takes the test text size so labels look like they do under test, but
/// keeps the display scale, a test's scale 1 renders half size on a retina
/// screen. The app's styles go like in a run. Nothing is handed back, the
/// window is the user's now. Must not run on the main thread, like
/// `run_test_maps`. A name shared by a view and a scene presents the view.
pub fn present_test(name: &str) -> anyhow::Result<()> {
    let key = super::spaced_test_name(name.trim());
    let entry = registered_test_maps().iter().find_map(|tests| tests.get(&key).copied());
    let Some(entry) = entry else {
        anyhow::bail!("Test not found: {name}");
    };

    from_main(Style::take_globals);
    Label::set_default_text_size(32);
    (entry.present)();

    Ok(())
}

/// Every registered test, the UI tests then the scene tests, with no help
/// from the app.
pub fn run_all_tests() -> TestRunReport {
    run_test_maps(&registered_test_maps())
}
