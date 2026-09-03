use anyhow::Result;
use ui_proc::view;

use crate::{
    LEVEL_TESTS,
    deps::{hreads::from_main, refs::Weak},
    level::{Level, LevelManager},
    ui::Setup,
    ui_test::{UITest, UITestEntry, get_test_name},
};

/// Implemented by `#[level]` for every level it can name concretely, so a
/// test on a generic level is a compile error instead of one that never
/// registers. Same reason as `Registrable` for views.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is a generic level, so it cannot be a level test",
    label = "generic level",
    note = "a test is registered by a ctor, and a ctor cannot name a generic type, so this test \
            would compile and then never run",
    note = "wrap it in a plain non generic level and put the `impl LevelTest` on that instead"
)]
pub trait LevelRegistrable {}

/// A level test. `impl LevelTest for X` on a `#[level]` struct is the
/// whole declaration, the macro registers it. The runner installs an
/// empty root on the test canvas, starts the level at scale 1, hands
/// it over and stops it afterwards.
pub trait LevelTest: Level + LevelRegistrable + Default {
    fn perform_test(level: Weak<Self>) -> Result<()>;

    /// Screen pixels the test draws in, see `ViewTest::canvas`.
    fn canvas() -> (u32, u32) {
        (600, 600)
    }
}

/// The root a level test runs under. The level draws beneath the UI, so
/// the root only pins the canvas.
#[view]
pub struct LevelTestView {}

/// Lets the `level` macro ask a type whether it is a test, like
/// `MaybeUITest` does for views.
pub trait MaybeLevelTest {
    fn __level_test() -> Option<fn() -> Result<()>>;
    fn __level_present() -> Option<fn()>;
}

impl<T: Level> MaybeLevelTest for T {
    default fn __level_test() -> Option<fn() -> Result<()>> {
        None
    }

    default fn __level_present() -> Option<fn()> {
        None
    }
}

impl<T: Level + LevelTest + 'static> MaybeLevelTest for T {
    fn __level_test() -> Option<fn() -> Result<()>> {
        Some(|| {
            let (width, height) = T::canvas();

            UITest::set(LevelTestView::new(), width, height, true, get_test_name::<T>());

            // A window on a retina screen puts the level at the display
            // scale, and a level probe drifts with it like a layout does.
            let level = from_main(|| {
                let level = LevelManager::set_level(T::default());
                LevelManager::set_scale(1.0);
                level
            });

            let result = T::perform_test(level);

            from_main(LevelManager::stop_level);

            result
        })
    }

    fn __level_present() -> Option<fn()> {
        Some(|| {
            UITest::present_root(LevelTestView::new());
            from_main(|| {
                LevelManager::set_level(T::default());
            });
        })
    }
}

/// Registers `T` if, and only if, it implements [`LevelTest`]. Called
/// from the ctor `#[level]` puts on every level.
pub fn register_if_level_test<T: Level + 'static>(file: &'static str) {
    let (Some(run), Some(present)) = (
        <T as MaybeLevelTest>::__level_test(),
        <T as MaybeLevelTest>::__level_present(),
    ) else {
        return;
    };

    let name = get_test_name::<T>();

    assert!(
        LEVEL_TESTS
            .lock()
            .insert(name.clone(), UITestEntry { run, present, file })
            .is_none(),
        "Duplicate level test: {name}. The registry keys on the type name alone.",
    );
}
