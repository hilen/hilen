use std::f32::consts::PI;

use chrono::Utc;
use hreads::{from_main, on_main};
use log::{trace, warn};
use parking_lot::{Mutex, MutexGuard};
use refs::Weak;
use ui_proc::view;
use vents::OnceEvent;

use crate::{
    self as test_engine,
    gm::{
        Animation, LossyConvert,
        color::{BLACK, Color, GRAY, LIGHT_BLUE},
        flat::{Size, point_on_circle},
    },
    ui::{
        Container, ModalView, Setup, SpinnerLockGlobal, SpinnerLockOnView, TouchStack, UIAnimation, View,
        ViewCallbacks, ViewData, ViewFrame, ViewSubviews, WeakView,
    },
};

static CIRCLES_N: u32 = 6;
static SPINNER: Mutex<Weak<Spinner>> = Mutex::new(Weak::const_default());

#[view]
pub struct Spinner {
    pub dot_color:      Color,
    pub rotation_speed: f32,

    circles: Vec<Weak<Container>>,
    event:   OnceEvent,
}

impl Spinner {
    fn current() -> MutexGuard<'static, Weak<Spinner>> {
        SPINNER.lock()
    }

    // fn set_alpha(&mut self, alpha: impl IntoF32) {
    //     self.set_color(self.color().with_alpha(alpha));
    //     for cir in &mut self.circles {
    //         let c = *cir.color();
    //         cir.set_color(c.with_alpha(alpha));
    //     }
    // }
}

impl Setup for Spinner {
    fn setup(mut self: Weak<Self>) {
        if self.dot_color.is_default() {
            self.dot_color = BLACK;
        }
        self.rotation_speed = 1.5;

        for _ in 0..CIRCLES_N {
            let circle = self.add_view::<Container>();
            self.circles.push(circle);
        }
    }
}

impl ViewCallbacks for Spinner {
    fn update(&mut self) {
        let duration_scale = self.rotation_speed;
        let microseconds_in_one_second = 1_000_000.0;

        let cycle_duration: i64 = (microseconds_in_one_second * duration_scale).lossy_convert();

        let current_time: i64 = Utc::now().timestamp_micros();

        let val: f32 = (current_time % cycle_duration).lossy_convert() / cycle_duration.lossy_convert();

        let span = PI * 2.0;
        let start = -PI;

        let angle = start + span * val;

        let step = 2.0 * PI / CIRCLES_N.lossy_convert();

        let parent_size = self.size().smallest_side();
        let size = parent_size * 0.115;
        let half_size = size / 2.0;

        let points: Vec<_> = (0..CIRCLES_N)
            .map(|index| {
                point_on_circle(
                    parent_size * 0.285,
                    angle + step * index.lossy_convert(),
                    self.size().center(),
                )
            })
            .collect();

        for (circle, point) in self.circles.iter_mut().zip(points) {
            circle
                .set_size(size, size)
                .set_corner_radius(half_size)
                .set_position((point.x - half_size, point.y - half_size))
                .set_color(self.dot_color);
        }
    }
}

impl Spinner {
    pub fn lock() -> SpinnerLockGlobal {
        trace!("Lock spinner");
        Self::start();
        SpinnerLockGlobal { stopped: false }
    }

    // pub fn

    fn start() {
        trace!("Start spinner");

        if Self::current().is_ok() {
            warn!("Spinner already started");
            return;
        }

        on_main(|| {
            let mut spinner = Spinner::default();

            spinner.set_color(GRAY.with_alpha(0.8));
            spinner.set_corner_radius(20);
            spinner.dot_color = LIGHT_BLUE;

            *Self::current() = Self::show_modally(spinner);
        });
    }

    pub fn stop() {
        trace!("Stop spinner");

        if Self::current().is_null() {
            warn!("Spinner already stopped");
            return;
        }

        on_main(|| {
            let spinner = Self::current();

            if spinner.is_null() {
                warn!("Spinner already stopped");
                return;
            }

            TouchStack::pop_layer(spinner.weak_view());

            let animation = UIAnimation::new(|sp, val| {
                let color = sp.color();
                sp.set_color(color.with_alpha(val));
                for dot in sp.subviews_weak() {
                    let color = *dot.color();
                    dot.set_color(color.with_alpha(val));
                }
            })
            .animation(Animation::new(0.8, 0.0, 0.4));

            animation.on_finish.sub(|| {
                let mut spinner = Self::current();
                let mut scrim = *spinner.superview();
                scrim.remove_from_superview();
                *spinner = Weak::default();
            });

            spinner.add_animation(animation);
        });
    }

    pub(crate) fn instant_stop() {
        trace!("Instant stop spinner");

        let mut spinner = Self::current();

        if spinner.is_null() {
            return;
        }

        spinner.hide_modal(());
        *spinner = Weak::default();
    }

    pub fn start_on(view: WeakView) -> SpinnerLockOnView {
        from_main(move || {
            if view.is_null() {
                return SpinnerLockOnView {
                    spinner: Weak::default(),
                };
            }
            let spinner = view.add_view::<Spinner>();
            spinner.place().back();
            SpinnerLockOnView { spinner }
        })
    }
}

impl ModalView for Spinner {
    fn modal_event(&self) -> &OnceEvent<()> {
        &self.event
    }

    fn modal_size() -> Size {
        (140, 140).into()
    }
}

#[cfg(feature = "ui-tests")]
mod test {

    use anyhow::Result;
    use hreads::from_main;
    use refs::Weak;

    use crate::{
        self as test_engine,
        gm::color::{BLACK, LIGHTER_GRAY, WHITE},
        ui::{Container, Setup, Spinner, ViewData, ViewFrame, ViewSubviews, ViewTest, view},
        ui_test::check_colors,
    };

    #[view]
    struct TestSpinner {
        #[init]
        content: Container,
    }

    impl Setup for TestSpinner {
        fn setup(self: Weak<Self>) {
            self.content.set_color(LIGHTER_GRAY).set_frame((20, 20, 50, 50));
        }
    }

    impl ViewTest for TestSpinner {
        fn perform_test(view: refs::Weak<Self>) -> Result<()> {
            let lock = Spinner::lock();

            check_colors(
                r"
             221  290 - #597c95
             242  290 - #adb1b5
             309  306 - #adb1b5
             362  301 - #adb1b5
             397  301 - #597c95
                    ",
            )?;

            lock.stop();

            check_colors(
                r"
             136  288 - #597c95
             215  288 - #597c95
             269  287 - #597c95
             340  284 - #597c95
             395  284 - #597c95
             465  284 - #597c95
                    ",
            )?;

            let _lock = Spinner::lock();

            from_main(move || {
                let mut spinner = view.content.add_view::<Spinner>();
                spinner.place().back();
                spinner.dot_color = BLACK;
                spinner.rotation_speed = 2.2;

                let mut spinner = view.add_view::<Spinner>();
                spinner.place().back();
                spinner.dot_color = WHITE;
                spinner.rotation_speed = 5.0;
            });

            Ok(())
        }
    }
}
