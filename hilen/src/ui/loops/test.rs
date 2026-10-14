//! `Declared loop`: 3 declared loops, a bar that slides, a production
//! `RingSpinner`, and a square with 2 pictures from change times.
//!
//! Everywhere it pins the pixels of the frame path at fixed phases on frame
//! stepped time, and that a pose with change times is called only at them.
//! Where a layer of the system exists, a window on macOS, iOS and tvOS, it
//! then turns the layers on and checks that each loop is on a layer with
//! its animation, that the app draws no frame while only the loops run, and
//! that a view put over the ring sends the ring back to the frame, which
//! draws it again at the right phase.

use anyhow::{Result, ensure};

use crate::{
    self as hilen,
    deps::{hreads::from_main, refs::Weak},
    gm::{Clock, color::Color},
    ui::{
        Container, Label, LoopAnimation, RingSpinner, Setup, TextAlignment, UIAnimation, View, ViewData,
        ViewFrame, ViewSubviews, ViewTest, view,
    },
    ui_test::{check_colors, step_frames},
    window::continuous_render_active,
};

const TRACK: Color = Color::hex("#d1d5db");
const BAR: Color = Color::hex("#2563eb");
const RING: Color = Color::hex("#374151");
const FIRST: Color = Color::hex("#dc2626");
const SECOND: Color = Color::hex("#16a34a");
const COVER: Color = Color::hex("#f59e0b");

/// Frames of 1/60 s on stepped time, a quarter of the loops of 1 s.
const QUARTER: u32 = 15;

#[view]
struct DeclaredLoop {
    /// Every phase the pose of `pictures` was called with.
    poses: Vec<f32>,

    #[init]
    track:          Container,
    track_label:    Label,
    ring:           RingSpinner,
    ring_label:     Label,
    pictures:       Container,
    pictures_label: Label,
    status:         Label,
    cover:          Container,
}

impl Setup for DeclaredLoop {
    fn setup(mut self: Weak<Self>) {
        self.track.set_color(TRACK).set_frame((20, 40, 300, 40));
        let bar = self.track.add_view::<Container>();
        bar.set_color(BAR).set_frame((0, 0, 40, 40));

        self.ring.set_ring_color(RING).set_line_width(16);
        self.ring.set_frame((20, 130, 120, 120));

        self.pictures.set_frame((340, 130, 80, 80));

        for (label, text, frame) in [
            (self.track_label, "loop bar", (340, 40, 240, 40)),
            (self.ring_label, "ring", (160, 170, 120, 40)),
            (self.pictures_label, "2 pictures", (430, 150, 160, 40)),
        ] {
            label.set_text(text).set_text_size(20).set_alignment(TextAlignment::Left);
            label.set_frame(frame);
        }

        self.status.set_text_size(20).set_alignment(TextAlignment::Left);
        self.status.set_frame((20, 300, 560, 40));

        // In the corner of the ring frame, off the stroke.
        self.cover.set_color(COVER).set_frame((20, 130, 26, 26));
        self.cover.set_hidden(true);

        self.start_loops();
    }
}

impl DeclaredLoop {
    /// Every loop from phase 0 at the time of this frame.
    fn start_loops(mut self: Weak<Self>) {
        self.poses.clear();
        self.track.run_loop(LoopAnimation::new(1.0, move |phase| self.slide(phase)));
        self.pictures
            .run_loop(LoopAnimation::new(1.0, move |phase| self.paint(phase)).changes_at([0.5]));
        self.ring.set_angle(0.0);
    }

    fn slide(mut self: Weak<Self>, phase: f32) {
        let bar = self.track.subviews_weak()[0];
        bar.set_position((phase * 260.0, 0.0));
    }

    fn paint(mut self: Weak<Self>, phase: f32) {
        self.poses.push(phase);
        self.pictures.set_color(if phase < 0.5 { FIRST } else { SECOND });
    }

    fn say(self: Weak<Self>, text: &str) {
        let text = text.to_string();
        from_main(move || {
            self.status.set_text(text);
        });
    }
}

/// The bar a quarter along, the gap of the ring a quarter turned, the
/// first picture.
const QUARTER_PROBES: &str = r"
      80   40 - #d1d5db
     120   40 - #2563eb
     264   40 - #d1d5db
     368   60 - #597c95
     404   60 - #597c95
      88   76 - #2563eb
     124   76 - #2563eb
     200   76 - #d1d5db
     396  132 - #dc2626
     108  152 - #374151
     464  168 - #000000
     508  168 - #000000
     476  172 - #172127
      24  176 - #374151
     452  176 - #12191e
     340  180 - #dc2626
      36  184 - #485f73
     200  192 - #000001
     396  208 - #dc2626
     116  232 - #374151
     308  316 - #000000
     216  320 - #597c95
     252  320 - #0f151a
     260  320 - #314552
      44  324 - #222f39
      76  324 - #374d5c
     124  324 - #000000
     156  324 - #40596b
     232  324 - #40596a
       4  592 - #597c95
     312  592 - #597c95
     592  592 - #597c95
";

/// The bar 3 quarters along, the gap 3 quarters turned, the second
/// picture.
const THREE_QUARTERS_PROBES: &str = r"
     100   40 - #d1d5db
     216   40 - #2563eb
     252   52 - #2563eb
     368   60 - #597c95
     404   60 - #597c95
      20   76 - #d1d5db
     152   76 - #d1d5db
     216   76 - #2563eb
     396  132 - #16a34a
      64  144 - #374151
     464  168 - #000000
     508  168 - #000000
     476  172 - #172127
     452  176 - #12191e
     136  180 - #374151
     340  180 - #16a34a
     204  192 - #597c95
     396  208 - #16a34a
      48  224 - #374151
     308  316 - #000000
     224  320 - #000000
     252  320 - #0f151a
     260  320 - #314552
      44  324 - #222f39
      76  324 - #374d5c
     124  324 - #000000
     156  324 - #40596b
     232  324 - #40596a
     308  324 - #000000
       4  592 - #597c95
     312  592 - #597c95
     592  592 - #597c95
";

/// The ring back on the frame under the cover, a quarter turned, the other
/// loops drawn by the frame for the check.
#[cfg(apple)]
const COVERED_PROBES: &str = r"
     168    4 - #597c95
      88   52 - #2563eb
     368   60 - #597c95
     404   60 - #597c95
      20   64 - #d1d5db
     124   76 - #2563eb
     224   76 - #d1d5db
     316   76 - #d1d5db
      28  132 - #f59e0b
     376  136 - #dc2626
      48  140 - #374151
      20  152 - #f59e0b
     476  168 - #172127
     528  168 - #597c95
     492  172 - #212e38
     452  176 - #12191e
      36  184 - #485f73
     200  192 - #000001
     340  208 - #dc2626
     116  232 - #374151
      72  320 - #314552
     280  320 - #000000
     308  320 - #19232b
     348  320 - #344856
     440  320 - #000000
     148  324 - #070a0c
     192  324 - #456073
     232  324 - #40596a
     372  324 - #0a0f12
     404  324 - #3c5465
       4  592 - #597c95
     592  592 - #597c95
";

impl ViewTest for DeclaredLoop {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        view.say("frame path, phases on stepped time");
        from_main(move || {
            Clock::enter_stepped();
            view.start_loops();
        });

        step_frames(QUARTER);
        check_colors(QUARTER_PROBES)?;
        let phase = from_main(move || view.track.loop_phase());
        ensure!(
            phase.is_some_and(|phase| (phase - 0.25).abs() < 0.001),
            "the loop must stand at a quarter, it is at {phase:?}"
        );

        step_frames(QUARTER * 2);
        check_colors(THREE_QUARTERS_PROBES)?;

        let poses = from_main(move || view.poses.clone());
        ensure!(
            poses
                .iter()
                .all(|phase| phase.to_bits() == 0.0_f32.to_bits() || phase.to_bits() == 0.5_f32.to_bits()),
            "a loop with change times is posed only at them, got {poses:?}"
        );
        ensure!(
            from_main(continuous_render_active),
            "loops on the frame path keep the loop of the app drawing"
        );

        #[cfg(apple)]
        layers::check(view)?;

        from_main(Clock::exit_stepped);

        // A closure animation that repeats is a loop nobody declared. Last,
        // it keeps frames until the view is gone.
        let warned = from_main(move || {
            view.status.add_animation(UIAnimation::new(|_, _| {}).repeat());
            view.status.__base_view().endless_warned
        });
        ensure!(warned, "a closure animation that repeats must be warned about");
        Ok(())
    }
}

/// The layers of the system, where a window has them.
#[cfg(apple)]
mod layers {
    use std::{thread::sleep, time::Duration};

    use anyhow::{Result, bail, ensure};

    use super::{COVER, COVERED_PROBES, DeclaredLoop, QUARTER, RING};
    use crate::{
        deps::{
            hreads::{from_main, wait_for_next_frame},
            refs::Weak,
        },
        gm::{Clock, flat::Rect},
        ui::{
            UIManager, View, ViewData, ViewFrame, WeakView,
            loops::{on_layer::layer_state, set_test_layers},
        },
        ui_test::{
            check_colors, step_frames,
            system_input::{screen_ink, system_input, wait_until},
        },
        window::{Window, continuous_render_active},
    };

    /// Most frames the app may draw in `IDLE` while only the loops run. The
    /// read of the frame count asks for 1 itself.
    const IDLE_FRAMES: u64 = 3;
    const IDLE: Duration = Duration::from_millis(600);
    const IDLE_TRIES: u32 = 3;

    pub(super) fn check(view: Weak<DeclaredLoop>) -> Result<()> {
        if Window::headless() {
            return Ok(());
        }

        view.say("layers on, the app draws no frame");
        from_main(move || {
            Clock::exit_stepped();
            set_test_layers(Some(true));
        });

        wait_until("every loop on a layer", move || {
            view.track.__base_view().loop_layer
                && view.ring.__base_view().loop_layer
                && view.pictures.__base_view().loop_layer
        })?;

        check_places(view)?;
        check_idle()?;
        check_rebuilds(view)?;
        check_cover(view)
    }

    /// Every loop on a layer with its animation, at its view, not upside
    /// down.
    fn check_places(view: Weak<DeclaredLoop>) -> Result<()> {
        let views = [
            ("loop bar", view.track.weak_view()),
            ("ring", view.ring.weak_view()),
            ("2 pictures", view.pictures.weak_view()),
        ];
        for (name, loop_view) in views {
            let state = from_main(move || layer_state(loop_view));
            let Some(state) = state else {
                bail!("the {name} loop has no layer");
            };
            ensure!(state.animated, "the layer of the {name} loop has no animation");
            ensure!(
                !state.flipped,
                "the layer of the {name} loop shows its pictures upside down"
            );
            let expected = from_main(move || expected_frame(loop_view, state.y_down));
            ensure!(
                near(state.frame, expected),
                "the layer of the {name} loop is at {:?}, the view at {expected:?}",
                state.frame
            );
            log::info!(
                "{name}: {} pictures on a layer at {:?}",
                state.pictures,
                state.frame
            );
        }
        let pictures = from_main(move || layer_state(view.pictures.weak_view())).map(|state| state.pictures);
        ensure!(
            pictures == Some(2),
            "2 change times are 2 pictures, got {pictures:?}"
        );

        Ok(())
    }

    /// The app sleeps while only loops on layers run, and the screen shows
    /// the ring of the layer.
    fn check_idle() -> Result<()> {
        // Nothing of the test may ask for a frame here, a read of the count
        // wakes the loop. A window on a desktop also draws for a pointer
        // that moves over it, so the quietest of a few tries counts. A loop
        // on the frame path draws in every refresh of every try.
        let drawn = (0..IDLE_TRIES)
            .map(|_| {
                let before = from_main(Window::render_frame);
                sleep(IDLE);
                from_main(Window::render_frame) - before
            })
            .min()
            .unwrap_or(u64::MAX);
        ensure!(
            drawn <= IDLE_FRAMES,
            "the app drew {drawn} frames in {} ms while only loops on layers ran",
            IDLE.as_millis()
        );
        ensure!(
            !from_main(continuous_render_active),
            "loops on layers must let the loop of the app sleep"
        );

        // The screen itself shows the ring of the layer. The box of a ring
        // with a quarter gap is at least 3 quarters of its frame, at any
        // angle.
        if system_input() {
            let ink = screen_ink(20.0, 130.0, 120.0, 120.0)?;
            let ring = [RING.r, RING.g, RING.b].map(|channel| channel * 255.0);
            let same = ink
                .color
                .iter()
                .zip(ring)
                .all(|(ink, ring)| (f32::from(*ink) - ring).abs() <= 3.0);
            ensure!(
                same && ink.right - ink.left > 80.0 && ink.bottom - ink.top > 80.0,
                "the screen must show the ring of the layer, it shows {ink:?}"
            );
            log::info!("the screen shows the ring of the layer: {ink:?}");
        } else {
            log::info!("no system input helper, the screen itself is not read");
        }

        Ok(())
    }

    fn check_rebuilds(view: Weak<DeclaredLoop>) -> Result<()> {
        // A move of a view moves its layer, with no new pictures.
        let pictures_view = view.pictures.weak_view();
        let before = builds(pictures_view);
        let left = from_main(move || layer_state(pictures_view).map(|state| state.frame.x()));
        view.say("the 2 pictures move, no new pictures");
        from_main(move || {
            view.pictures.set_frame((344, 130, 80, 80));
        });
        wait_until("the layer of the 2 pictures moved", move || {
            layer_state(pictures_view).is_some_and(|state| {
                Some(state.frame.x()) != left
                    && near(state.frame, expected_frame(pictures_view, state.y_down))
            })
        })?;
        ensure!(
            builds(pictures_view) == before,
            "a move of the view must not make new pictures"
        );
        from_main(move || {
            view.pictures.set_frame((340, 130, 80, 80));
        });

        // A new content makes the pictures again.
        let ring_view = view.ring.weak_view();
        let before = builds(ring_view);
        view.say("a new ring color, new pictures");
        from_main(move || {
            let mut this = view;
            this.ring.set_ring_color(COVER);
        });
        wait_until("new pictures of the ring", move || {
            layer_state(ring_view).is_some_and(|state| state.builds > before)
        })?;
        from_main(move || {
            let mut this = view;
            this.ring.set_ring_color(RING);
        });

        Ok(())
    }

    fn check_cover(view: Weak<DeclaredLoop>) -> Result<()> {
        view.say("a view over the ring, the ring back on the frame");
        from_main(move || {
            view.cover.set_hidden(false);
        });
        wait_for_next_frame();
        wait_for_next_frame();
        let (ring, track) = from_main(move || {
            (
                view.ring.__base_view().loop_layer,
                view.track.__base_view().loop_layer,
            )
        });
        ensure!(!ring, "a covered ring must go back to the frame");
        ensure!(track, "a loop nothing covers stays on its layer");
        ensure!(
            from_main(continuous_render_active),
            "a loop back on the frame keeps the loop of the app drawing"
        );

        from_main(move || {
            Clock::enter_stepped();
            view.start_loops();
        });
        step_frames(QUARTER);
        check_colors(COVERED_PROBES)?;

        from_main(move || {
            set_test_layers(Some(false));
            Clock::exit_stepped();
        });
        Ok(())
    }

    /// The rect of the layer of a loop view, in the points of the window
    /// view: the pixels of the view with the soft edge, over the screen
    /// scale.
    fn expected_frame(view: WeakView, y_down: bool) -> Rect {
        let scale = UIManager::scale();
        let screen = Window::screen_scale();
        let frame = *view.absolute_frame() * scale;
        let left = (frame.x() - 2.0).floor();
        let top = (frame.y() - 2.0).floor();
        let width = (frame.max_x() + 2.0).ceil() - left;
        let height = (frame.max_y() + 2.0).ceil() - top;
        let top = if y_down {
            top
        } else {
            UIManager::window_resolution().height - top - height
        };
        Rect::new(left / screen, top / screen, width / screen, height / screen)
    }

    /// How many times the pictures of the loop of `view` were made.
    fn builds(view: WeakView) -> u32 {
        from_main(move || layer_state(view).map_or(0, |state| state.builds))
    }

    fn near(a: Rect, b: Rect) -> bool {
        (a.x() - b.x()).abs() < 0.5
            && (a.y() - b.y()).abs() < 0.5
            && (a.width() - b.width()).abs() < 0.5
            && (a.height() - b.height()).abs() < 0.5
    }
}
