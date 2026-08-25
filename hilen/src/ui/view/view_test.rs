use anyhow::Result;

use crate::{
    deps::refs::{Own, Weak},
    ui::{Setup, View},
};

/// Implemented by `#[view]` for every view it can name concretely, which is
/// every view that is not generic.
///
/// A ctor has to name one type, and a generic view has none until something
/// instantiates it somewhere the macro cannot see, so no ctor is generated and
/// nothing would ever register the test. This is a [`ViewTest`] supertrait to
/// turn that into a compile error rather than a test that silently never runs.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is a generic view, so it cannot be a UI test",
    label = "generic view",
    note = "a test is registered by a ctor, and a ctor cannot name a generic type, so this test \
            would compile and then never run",
    note = "wrap it in a plain non generic view and put the `impl ViewTest` on that instead"
)]
pub trait Registrable {}

pub trait ViewTest: View + Registrable {
    fn perform_test(view: Weak<Self>) -> Result<()>;

    /// Runs before the harness builds the view.
    ///
    /// Anything a view reads while it is being built has to be set here rather
    /// than in [`ViewTest::perform_test`], which only runs once the view
    /// already exists. A global [`Style`](crate::ui::Style) is the case this
    /// is for: it is read in `setup`, so applying it later has no effect and
    /// the test renders unstyled against styled expectations.
    fn before_start() {}

    /// The view the harness installs as the root, given the test view.
    ///
    /// Defaults to the test view itself. Override when the thing under test
    /// only works inside a host, such as a view that has to sit in a
    /// [`NavigationView`](crate::ui::NavigationView) before it can present.
    /// `perform_test` still receives the test view, not the host.
    fn make_root(view: Own<Self>) -> Own<dyn View>
    where Self: Sized + 'static {
        view
    }

    /// Screen pixels the test draws in. Override when the default is too small
    /// for the view, but keep it within the smallest supported screen, which is
    /// 640 by 1136 on an iPhone 5S.
    fn canvas() -> (u32, u32) {
        (600, 600)
    }
}

/// Lets the `view` macro ask a type whether it is a test, with no attribute to
/// forget. A proc macro cannot see a trait impl written elsewhere, so the
/// question is answered here, where the impl is visible.
pub trait MaybeUITest {
    /// `Some` only for a type that implements [`ViewTest`].
    fn __ui_test() -> Option<fn() -> Result<()>>;

    /// Builds the test view full screen and hands it over untouched, for
    /// presentation mode. `Some` only for a type that implements [`ViewTest`].
    fn __ui_present() -> Option<fn()>;
}

impl<T: View> MaybeUITest for T {
    default fn __ui_test() -> Option<fn() -> Result<()>> {
        None
    }

    default fn __ui_present() -> Option<fn()> {
        None
    }
}

impl<T: View + ViewTest + Default + 'static> MaybeUITest for T {
    fn __ui_present() -> Option<fn()> {
        Some(|| {
            T::before_start();
            crate::ui_test::UITest::present_root(T::make_root(T::new()));
        })
    }

    fn __ui_test() -> Option<fn() -> Result<()>> {
        Some(|| {
            T::before_start();

            let (width, height) = T::canvas();

            // The test view is built here, not by `make_root`, so the handle
            // handed to the test is always the test view even when the root
            // installed above it is something else.
            let view = T::new();
            let weak = view.weak();

            crate::ui_test::UITest::set_root(
                T::make_root(view),
                width,
                height,
                crate::ui_test::get_test_name::<T>(),
            );

            T::perform_test(weak)
        })
    }
}
