use anyhow::{Result, ensure};
use plat::Platform;

use crate::{
    deps::{
        hreads::{from_main, wait_for_next_frame},
        refs::Weak,
    },
    gm::color::{BLUE, RED, U8Color, WHITE, YELLOW},
    ui::{Container, Input, Label, Setup, Touch, UIManager, ViewData, ViewFrame, ViewTest, ViewTouch, view},
    ui_test::{check_colors, checkpoint, inject_touches},
    window::{AppWindow, WindowLevel, WindowPlace, WindowSpec},
};

/// The content of the second window. A red box at the same place as the blue
/// box of the main window, it turns yellow on a tap.
#[view]
struct SecondWindowContent {
    taps: u32,
    keys: u32,

    #[init]
    target: Container,
    title:  Label,
}

impl Setup for SecondWindowContent {
    fn setup(mut self: Weak<Self>) {
        self.set_color(WHITE);
        self.title.set_text("second window");
        self.title.set_frame((20, 0, 360, 40));

        // Turns touch on in `setup`, once it is in the root of its window, so
        // it lands in the touch stack of that window.
        self.target.set_color(RED);
        self.target.set_frame((50, 50, 200, 100));
        self.target.enable_touch();
        self.target.touch().up_inside.sub(self, move || {
            self.taps += 1;
            self.target.set_color(YELLOW);
        });
        UIManager::keymap().add(self, 'k', move || self.keys += 1);
    }
}

/// An extra window has its own root, its own touch stack and its own frame.
/// A tap into it reaches only its views, a tap into the main window only
/// the views there, and its frame shows only its own views.
#[view]
struct SecondWindow {
    main_taps: u32,
    main_keys: u32,
    content:   Weak<SecondWindowContent>,

    #[init]
    target: Container,
    title:  Label,
}

impl Setup for SecondWindow {
    fn setup(mut self: Weak<Self>) {
        self.title.set_text("main window");
        self.title.set_frame((20, 0, 360, 40));

        self.target.set_color(BLUE);
        self.target.set_frame((50, 50, 200, 100));
        self.target.enable_touch();
        self.target.touch().up_inside.sub(self, move || self.main_taps += 1);
        UIManager::keymap().add(self, 'k', move || self.main_keys += 1);
    }
}

/// A tap at `x y` into `window`, the way an event of that window comes.
fn tap_in(window: AppWindow, x: f32, y: f32) -> Result<()> {
    from_main(move || {
        window.within(|| {
            let scale = UIManager::scale();
            for mut touch in Touch::vec_from_str(&format!("{x} {y} b\n{x} {y} e")) {
                touch.position *= scale;
                Input::process_touch_event(touch);
            }
        })
    })
}

fn open(mut view: Weak<SecondWindow>) -> Result<AppWindow> {
    from_main(move || {
        AppWindow::open(
            WindowSpec {
                title:       "Second window".into(),
                place:       WindowPlace::Frame((0, 0, 400, 300).into()),
                level:       WindowLevel::Normal,
                decorations: true,
            },
            move || {
                let content = SecondWindowContent::new();
                view.content = content.weak();
                content
            },
        )
    })
}

/// A tap and a key reach only the window they came to.
fn each_window_has_its_input(view: Weak<SecondWindow>, window: AppWindow) -> Result<()> {
    inject_touches("150 100 b\n150 100 e");
    let taps = from_main(move || (view.main_taps, view.content.taps));
    ensure!(
        taps == (1, 0),
        "a tap into the main window must reach only its box, taps {taps:?}"
    );

    tap_in(window, 150.0, 100.0)?;
    let taps = from_main(move || (view.main_taps, view.content.taps));
    ensure!(
        taps == (1, 1),
        "a tap into the second window must reach only its box, taps {taps:?}"
    );

    // The keymap is 1 for the app, a key reaches the binding of the
    // window that has the keys only.
    from_main(|| Input::on_char('k', Some('k')));
    let keys = from_main(move || (view.main_keys, view.content.keys));
    ensure!(
        keys == (1, 0),
        "a key in the main window must reach only its binding, keys {keys:?}"
    );
    from_main(move || window.within(|| Input::on_char('k', Some('k'))))?;
    let keys = from_main(move || (view.main_keys, view.content.keys));
    ensure!(
        keys == (1, 1),
        "a key in the second window must reach only its binding, keys {keys:?}"
    );
    Ok(())
}

impl ViewTest for SecondWindow {
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
        checkpoint("a second window opened, the main window still shows its blue box")?;

        each_window_has_its_input(view, window)?;

        let shot = window.take_screenshot()?;
        let scale = from_main(UIManager::scale);
        for (x, y, color, what) in [
            (150.0, 100.0, YELLOW, "the tapped box"),
            (300.0, 250.0, WHITE, "the background"),
        ] {
            let seen = shot.get_pixel((x * scale, y * scale));
            ensure!(
                seen == U8Color::from(color),
                "{what} of the second window at {x} {y} is {}, not {}",
                seen.as_hex(),
                U8Color::from(color).as_hex()
            );
        }

        // The main window keeps its own picture, nothing of the second
        // window draws into it.
        check_colors(
            r"
             248    8 - #000000
             156   16 - #324553
             220   16 - #000000
             108   20 - #000000
             128   20 - #344958
             148   20 - #151e24
             156   20 - #324553
             188   20 - #000000
             204   20 - #000000
             244   20 - #597c95
             260   20 - #597c95
             128   24 - #344958
             140   24 - #597c95
             148   24 - #151e24
             156   24 - #324553
             240   24 - #597c95
             264   24 - #597c95
             128   28 - #344958
             156   28 - #324553
             172   28 - #000001
             220   28 - #000001
             276   28 - #000000
             288   28 - #000001
             248  100 - #0000e7
             120  108 - #0000e7
              52  148 - #0000e7
             188  148 - #0000e7
             592  228 - #597c95
               4  388 - #597c95
             352  392 - #597c95
             140  592 - #597c95
             592  592 - #597c95
            ",
        )?;

        from_main(move || window.close());
        ensure!(
            !from_main(move || window.is_open()),
            "a closed window is not open"
        );
        wait_for_next_frame();
        wait_for_next_frame();
        ensure!(
            !from_main(move || view.content.is_ok()),
            "the views of a closed window go with its next frame"
        );

        // A window in the slot of the closed one starts with a fresh touch
        // stack, the old box must not take the tap.
        let again = open(view)?;
        while !from_main(move || again.is_ready()) {
            wait_for_next_frame();
        }
        tap_in(again, 150.0, 100.0)?;
        let taps = from_main(move || (view.main_taps, view.content.taps));
        ensure!(
            taps == (1, 1),
            "the new window counts its own first tap, taps {taps:?}"
        );
        from_main(move || again.close());

        Ok(())
    }
}
