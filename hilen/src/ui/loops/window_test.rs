//! `Loop in second window`: a declared loop on a view of an extra window
//! moves with the frames of that window and keeps the app drawing. Where a
//! window has layers, a desktop run on macOS, the loop then goes on a layer
//! of that window.

use anyhow::{Result, ensure};
use plat::Platform;

use crate::{
    deps::{
        hreads::{from_main, wait_for_next_frame},
        refs::Weak,
    },
    gm::{
        Clock,
        color::{Color, U8Color, WHITE},
    },
    ui::{
        Container, Label, LoopAnimation, Setup, UIManager, ViewData, ViewFrame, ViewSubviews, ViewTest, view,
    },
    ui_test::{checkpoint, step_frames},
    window::{AppWindow, Screenshot, WindowLevel, WindowPlace, WindowSpec, continuous_render_active},
};

const TRACK: Color = Color::hex("#d1d5db");
const BAR: Color = Color::hex("#2563eb");

/// Frames of 1/60 s on stepped time, a quarter of the loop of 1 s.
const QUARTER: u32 = 15;

/// The left edge of the track and how far the bar slides on it.
const TRACK_X: f32 = 20.0;
const SLIDE: f32 = 260.0;
const BAR_WIDTH: f32 = 40.0;
/// A row through the middle of the track.
const ROW: f32 = 60.0;

/// The content of the second window, a bar that slides along a track.
#[view]
struct LoopWindowContent {
    #[init]
    track: Container,
    title: Label,
}

impl Setup for LoopWindowContent {
    fn setup(self: Weak<Self>) {
        self.set_color(WHITE);
        self.title.set_text("loop in a second window");
        // Clear of the picture of the loop, which reaches 2 pixels above
        // the track. A view over it keeps the loop off a layer.
        self.title.set_frame((20, 0, 360, 30));

        self.track.set_color(TRACK).set_frame((TRACK_X, 40.0, SLIDE + BAR_WIDTH, 40.0));
        let bar = self.track.add_view::<Container>();
        bar.set_color(BAR).set_frame((0.0, 0.0, BAR_WIDTH, 40.0));
    }
}

impl LoopWindowContent {
    /// The loop from phase 0 at the time of this frame.
    fn start(mut self: Weak<Self>) {
        self.track.run_loop(LoopAnimation::new(1.0, move |phase| {
            let bar = self.track.subviews_weak()[0];
            bar.set_position((phase * SLIDE, 0.0));
        }));
    }
}

/// The main window shows only a label, the loop is in the second window.
#[view]
struct LoopInSecondWindow {
    content: Weak<LoopWindowContent>,

    #[init]
    title: Label,
}

impl Setup for LoopInSecondWindow {
    fn setup(self: Weak<Self>) {
        self.title.set_text("main window, no loop");
        self.title.set_frame((20, 0, 360, 40));
    }
}

fn open(mut view: Weak<LoopInSecondWindow>) -> Result<AppWindow> {
    from_main(move || {
        AppWindow::open(
            WindowSpec {
                title:       "Loop window".into(),
                place:       WindowPlace::Frame((0, 0, 400, 200).into()),
                level:       WindowLevel::Normal,
                decorations: true,
            },
            move || {
                let content = LoopWindowContent::new();
                view.content = content.weak();
                content
            },
        )
    })
}

/// The bar of the picture of the second window covers `phase` of its slide,
/// and the track shows where the bar is not.
fn check_bar(shot: &Screenshot, phase: f32) -> Result<()> {
    let scale = from_main(UIManager::scale);
    let bar_x = TRACK_X + phase * SLIDE + BAR_WIDTH / 2.0;
    // Half a slide away from the bar, on the track either way.
    let free_x = TRACK_X + ((phase + 0.5) % 1.0) * SLIDE + BAR_WIDTH / 2.0;
    for (x, color, what) in [(bar_x, BAR, "the bar"), (free_x, TRACK, "the free track")] {
        let seen = shot.get_pixel((x * scale, ROW * scale));
        ensure!(
            seen == U8Color::from(color),
            "at phase {phase} {what} of the second window at {x} {ROW} is {}, not {}",
            seen.as_hex(),
            U8Color::from(color).as_hex()
        );
    }
    Ok(())
}

impl ViewTest for LoopInSecondWindow {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        let opened = open(view);

        if !Platform::DESKTOP {
            ensure!(opened.is_err(), "A phone and a page have 1 window");
            return Ok(());
        }
        let window = opened?;

        while !from_main(move || window.is_ready()) {
            wait_for_next_frame();
        }

        from_main(move || {
            Clock::enter_stepped();
            view.content.start();
        });

        // The picture of the second window is its next frame, which steps
        // its loops at the time of that frame.
        step_frames(QUARTER);
        check_bar(&window.take_screenshot()?, 0.25)?;
        checkpoint("the bar of the second window a quarter along")?;

        step_frames(QUARTER * 2);
        check_bar(&window.take_screenshot()?, 0.75)?;

        ensure!(
            from_main(continuous_render_active),
            "a loop on the frame path of the second window keeps the app drawing"
        );

        #[cfg(macos)]
        layer::check(view, window)?;

        from_main(Clock::exit_stepped);

        from_main(move || window.close());
        wait_for_next_frame();
        wait_for_next_frame();
        ensure!(
            !from_main(move || view.content.is_ok()),
            "the views of a closed window go with its next frame"
        );
        ensure!(
            !from_main(continuous_render_active),
            "the loop of a closed window must not keep the app drawing"
        );

        Ok(())
    }
}

/// In a window on a Mac the loop goes on a layer of the second window, not
/// of the main one.
#[cfg(macos)]
mod layer {
    use anyhow::{Result, bail, ensure};

    use super::LoopInSecondWindow;
    use crate::{
        deps::{hreads::from_main, refs::Weak},
        gm::Clock,
        ui::{
            View,
            loops::{
                on_layer::{layer_state, window_root_layer},
                set_test_layers,
            },
        },
        ui_test::system_input::wait_until,
        window::{AppWindow, Window},
    };

    pub(super) fn check(view: Weak<LoopInSecondWindow>, window: AppWindow) -> Result<()> {
        if Window::headless() {
            return Ok(());
        }

        from_main(move || {
            Clock::exit_stepped();
            set_test_layers(Some(true));
        });

        let result = on_layer(view, window);
        from_main(|| set_test_layers(Some(false)));
        result
    }

    fn on_layer(view: Weak<LoopInSecondWindow>, window: AppWindow) -> Result<()> {
        wait_until("the loop of the second window on a layer", move || {
            view.content.track.__base_view().loop_layer
        })?;

        let track = view.content.track.weak_view();
        let Some(state) = from_main(move || layer_state(track)) else {
            bail!("the loop of the second window has no layer");
        };
        ensure!(state.animated, "the layer of the loop has no animation");

        let second = from_main(move || window.within(window_root_layer))?;
        let main = from_main(window_root_layer);
        ensure!(
            second == Some(state.root),
            "the layer is on {:#x}, not on the second window {second:x?}, the main one is {main:x?}",
            state.root
        );
        Ok(())
    }
}
