use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{
        Anchor, Button, Label, ScreenKeyboard, ScreenKeyboardMove, Setup, TextAlignment, TextField, UIEvents,
        View, ViewData, ViewFrame, ViewTest, ViewTouch, WeakView, view,
    },
    ui_test::{
        checkpoint,
        system_input::{screen_keyboard, tap, type_text, wait_until},
    },
};

use super::keyboard_marker::KeyboardMarker;

const COMPOSE: (f32, f32) = (300.0, 955.0);
const KEYBOARD_TOP: f32 = 814.0;
const MARGIN: f32 = 20.0;
const ENDS_TOP: f32 = 134.0;
const CHAT_TOP: f32 = 198.0;
/// The row of the 2 views that keep the selection, above the box.
const ROW_BOTTOM: f32 = 80.0;

/// The frames the field is watched for after a tap, 1 s of the clock of a
/// test run. A phone took the keyboard away 40 ms after the tap that ended
/// the editing.
const WATCHED_FRAMES: usize = 60;

/// A text area in edit with the keyboard up, and taps on 3 views around
/// it. A button and a plain view marked with `set_keeps_selection` get
/// their tap while the field stays edited and the keyboard stays where it
/// is. A button without the mark ends the editing, as every view does.
///
/// A send button of a chat once closed the keyboard at every message: the
/// press selected the button, so the field was not the selected view any
/// more.
#[view]
struct ScreenKeyboardKeptByTap {
    keeps_taps: usize,
    view_taps:  usize,
    ends_taps:  usize,

    /// Where the keyboard stood when the watch began.
    stands:  Option<f32>,
    /// The moves away from that place that started since.
    moves:   Vec<ScreenKeyboardMove>,
    watched: bool,

    // The box is laid out first, the row and the chat read its frame.
    #[init]
    compose:    TextField,
    keeps:      Button,
    keeps_view: Label,
    ends:       Button,
    title:      Label,
    taps:       Label,
    status:     Label,
    chat:       Label,
    marker:     KeyboardMarker,
}

impl Setup for ScreenKeyboardKeptByTap {
    fn setup(mut self: Weak<Self>) {
        let line = |label: Weak<Label>, y: f32| {
            label.set_text_size(18);
            label.set_alignment(TextAlignment::Left);
            label.place().t(y).lr(20).h(26);
        };

        line(self.title, 47.0);
        self.title.set_text("a tap on a view that keeps the selection");
        line(self.taps, 73.0);
        line(self.status, 99.0);

        self.compose.set_text_size(24).set_multiline(true);
        self.compose.set_alignment(TextAlignment::Left);
        self.compose.set_placeholder("message");
        self.compose.place().lr(MARGIN).h(50).b_keyboard(MARGIN);

        self.keeps.set_text("button, keeps").set_text_size(18);
        self.keeps.set_color((170, 215, 170));
        self.keeps.set_keeps_selection(true);
        self.keeps.place().l(MARGIN).w(180).h(44).b_keyboard(ROW_BOTTOM);
        self.keeps.on_tap(move || {
            self.keeps_taps += 1;
            self.show_status();
        });

        self.keeps_view.set_text("view, keeps").set_text_size(18);
        self.keeps_view.set_color((170, 215, 170));
        self.keeps_view.enable_touch();
        self.keeps_view.set_keeps_selection(true);
        self.keeps_view.place().l(210).w(180).h(44).b_keyboard(ROW_BOTTOM);
        self.keeps_view.touch().up_inside.sub(self, move || {
            self.view_taps += 1;
            self.show_status();
        });

        // Not on the keyboard. The keyboard of a phone leaves while the
        // finger is still down, and a view that rides on it moves away
        // from under the finger, so the touch ends outside and is no tap.
        self.ends.set_text("button, ends").set_text_size(18);
        self.ends.set_color((235, 180, 170));
        self.ends.place().t(ENDS_TOP).l(MARGIN).w(180).h(44);
        self.ends.on_tap(move || {
            self.ends_taps += 1;
            self.show_status();
        });

        self.chat.set_text("the chat ends above the row").set_text_size(18);
        self.chat.set_color((230, 236, 240));
        self.chat.place().t(CHAT_TOP).lr(MARGIN).anchor(Anchor::Bot, self.keeps, MARGIN);

        UIEvents::screen_keyboard().val(self, move |moved| {
            // The system also says again where the keyboard stands, that
            // moves nothing.
            let stays = match (moved.top, self.stands) {
                (Some(top), Some(stands)) => (top - stands).abs() < 0.5,
                _ => false,
            };
            if self.watched && !stays {
                self.moves.push(moved);
                self.show_status();
            }
        });
        self.show_status();
    }
}

impl ScreenKeyboardKeptByTap {
    fn show_status(self: Weak<Self>) {
        self.taps.set_text(format!(
            "taps: button keeps {}, view keeps {}, button ends {}",
            self.keeps_taps, self.view_taps, self.ends_taps
        ));
        let edited = if self.compose.is_editing() {
            "the box is edited"
        } else {
            "the box is not edited"
        };
        self.status.set_text(format!("{edited}, keyboard moves: {}", self.moves.len()));
    }

    fn watch(mut self: Weak<Self>) {
        self.stands = ScreenKeyboard::top();
        self.moves.clear();
        self.watched = true;
        self.show_status();
    }

    /// The field after a tap on a view that keeps the selection: still
    /// edited, and the keyboard started no move away from its place.
    fn kept(self: Weak<Self>, tapped: &'static str) -> Result<()> {
        for _ in 0..WATCHED_FRAMES {
            wait_for_next_frame();
        }
        from_main(move || {
            self.show_status();
            ensure!(
                self.compose.is_editing() && self.moves.is_empty(),
                "the tap on {tapped} changed the selection: the box is edited: {}, keyboard moves: {:?}",
                self.compose.is_editing(),
                self.moves
            );
            ensure!(
                ScreenKeyboard::top() == self.stands,
                "the keyboard stood at {:?} and is at {:?} after the tap on {tapped}",
                self.stands,
                ScreenKeyboard::top()
            );
            Ok(())
        })
    }
}

/// A tap in the middle of `target`, where it stands now.
fn tap_on(target: WeakView) -> Result<()> {
    let point = from_main(move || target.absolute_frame().center());
    tap(point.x, point.y)
}

impl ViewTest for ScreenKeyboardKeptByTap {
    fn canvas() -> (u32, u32) {
        (600, 1000)
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        tap(COMPOSE.0, COMPOSE.1)?;
        wait_until("the box is edited", move || view.compose.is_editing())?;
        screen_keyboard(true, KEYBOARD_TOP)?;
        wait_until("the box is on top of the keyboard", move || {
            ScreenKeyboard::top()
                .is_some_and(|top| (view.compose.absolute_frame().max_y() - (top - MARGIN)).abs() < 0.5)
        })?;
        type_text("Hello")?;
        wait_until("the box holds Hello", move || view.compose.text() == "Hello")?;
        from_main(move || view.watch());
        checkpoint("the keyboard is up, the box is edited and holds Hello, no taps yet")?;

        tap_on(view.keeps.weak_view())?;
        wait_until("the button that keeps got its tap", move || view.keeps_taps == 1)?;
        view.kept("the button that keeps")?;
        // The keys still go to the box.
        type_text(" you")?;
        wait_until("the box holds Hello you", move || {
            view.compose.text() == "Hello you"
        })?;
        checkpoint("button keeps 1 tap, the box is still edited and holds Hello you, 0 keyboard moves")?;

        tap_on(view.keeps_view.weak_view())?;
        wait_until("the view that keeps got its tap", move || view.view_taps == 1)?;
        view.kept("the view that keeps")?;
        checkpoint("view keeps 1 tap, the box is still edited, 0 keyboard moves")?;

        // The keyboard leaves next, that move is wanted.
        from_main(move || {
            let mut view = view;
            view.watched = false;
        });
        tap_on(view.ends.weak_view())?;
        wait_until("the button that ends got its tap", move || view.ends_taps == 1)?;
        wait_until("the box is not edited any more", move || {
            !view.compose.is_editing()
        })?;
        screen_keyboard(false, KEYBOARD_TOP)?;
        wait_until("the box is back at the bottom", move || {
            (view.compose.absolute_frame().max_y() - 980.0).abs() < 0.5
        })?;
        from_main(move || {
            view.show_status();
            ensure!(
                view.keeps_taps == 1 && view.view_taps == 1 && view.ends_taps == 1,
                "taps: {} {} {}",
                view.keeps_taps,
                view.view_taps,
                view.ends_taps
            );
            ensure!(
                view.compose.text() == "Hello you",
                "the box holds {:?}",
                view.compose.text()
            );
            Ok(())
        })?;
        checkpoint("button ends 1 tap, the box is not edited, the keyboard is gone, the box is at the bottom")
    }
}
