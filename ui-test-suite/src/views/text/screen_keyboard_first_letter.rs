use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{
        Anchor, Label, ScreenKeyboard, ScreenKeyboardMove, Setup, TextAlignment, TextField, UIEvents,
        UIManager, ViewData, ViewFrame, ViewTest, view,
    },
    ui_test::{
        checkpoint,
        system_input::{screen_keyboard, tap, type_text, wait_until},
    },
};

use super::keyboard_marker::KeyboardMarker;

const COMPOSE: (f32, f32) = (300.0, 955.0);
/// In the chat, above every keyboard.
const OUTSIDE: (f32, f32) = (300.0, 300.0);
const KEYBOARD_TOP: f32 = 814.0;
const MARGIN: f32 = 20.0;
const CHAT_TOP: f32 = 100.0;

/// The frames the box is watched for after each change of the text, 1.5 s
/// of the clock of a test run. A phone sent its last word about the
/// keyboard 40 ms after the letter, and a move it starts takes 0.4 s.
const WATCHED_FRAMES: usize = 60;
/// The letter is erased and typed again this many times after the first
/// one, 5 changes of the text in all.
const ERASES: usize = 2;
const CHANGES: usize = 1 + ERASES * 2;

/// The compose box of a chat, a text area placed with `b_keyboard`, and the
/// first letter typed into it. The keyboard stays where it is for a letter,
/// so the box must not move and the keyboard may not start a move away from
/// its place.
///
/// An iPhone lays the engine view out again when a text area gets its first
/// letter and when it loses its last one, and the engine hears a resize to
/// the size it has. That resize once opened the system field again. The
/// keyboard left for that time and everything on it jumped.
#[view]
struct ScreenKeyboardFirstLetter {
    /// Where the keyboard stood when the watch began.
    stands:  Option<f32>,
    /// The moves away from that place that started since.
    moves:   Vec<ScreenKeyboardMove>,
    /// How far the bottom of the box was from its place, at most.
    moved:   f32,
    /// How many times the text of the box changed.
    changes: usize,
    watched: bool,

    // The box is laid out first, the chat reads its frame.
    #[init]
    compose: TextField,
    title:   Label,
    status:  Label,
    chat:    Label,
    marker:  KeyboardMarker,
}

impl Setup for ScreenKeyboardFirstLetter {
    fn setup(mut self: Weak<Self>) {
        let line = |label: Weak<Label>, y: f32| {
            label.set_text_size(18);
            label.set_alignment(TextAlignment::Left);
            label.place().t(y).lr(20).h(26);
        };

        line(self.title, 47.0);
        self.title.set_text("the first letter in a compose box");
        line(self.status, 73.0);

        self.compose.set_text_size(24).set_multiline(true);
        self.compose.set_alignment(TextAlignment::Left);
        self.compose.set_placeholder("message");
        self.compose.place().lr(MARGIN).h(50).b_keyboard(MARGIN);

        self.chat.set_text("the chat ends above the box").set_text_size(18);
        self.chat.set_color((230, 236, 240));
        self.chat
            .place()
            .t(CHAT_TOP)
            .lr(MARGIN)
            .anchor(Anchor::Bot, self.compose, MARGIN);

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

impl ScreenKeyboardFirstLetter {
    fn show_status(self: Weak<Self>) {
        let text = if self.changes == 0 {
            "no letter typed yet".to_string()
        } else {
            format!(
                "text changes: {}, keyboard moves: {}, the box moved by {:.0}",
                self.changes,
                self.moves.len(),
                self.moved
            )
        };
        self.status.set_text(text);
    }

    fn watch(mut self: Weak<Self>) {
        self.stands = ScreenKeyboard::top();
        self.moves.clear();
        self.moved = 0.0;
        self.watched = true;
        self.show_status();
    }

    fn box_bottom(self: Weak<Self>) -> f32 {
        self.compose.absolute_frame().max_y()
    }

    fn type_letter(self: Weak<Self>) -> Result<()> {
        // A capital, the key an empty field shows. The tap is then the
        // letter itself and not Shift.
        type_text("A")?;
        wait_until("the box holds the letter", move || self.compose.text() == "A")
    }

    /// The system input helper has no delete key, so the engine erases.
    fn erase(self: Weak<Self>) {
        from_main(move || {
            self.compose.set_text("");
        });
    }

    /// The resize an iPhone sends at such a change of the text, then the
    /// watch of the box. The simulator of the test lane sends no such
    /// layout, so the test sends what `AppRunner::resize` does with it.
    fn resize_and_watch(self: Weak<Self>, place: f32, holds: &str) -> Result<()> {
        from_main(move || {
            let mut view = self;
            view.changes += 1;
            view.show_status();
            UIManager::set_scale(UIManager::display_scale());
        });

        for _ in 0..WATCHED_FRAMES {
            from_main(move || {
                let mut view = self;
                let off = (view.box_bottom() - place).abs();
                if off > view.moved {
                    view.moved = off;
                    view.show_status();
                }
            });
            wait_for_next_frame();
        }

        let change = from_main(move || self.changes);
        checkpoint(&format!(
            "text change {change} of {CHANGES}, {holds}, 0 keyboard moves, the box moved by 0"
        ))
    }
}

impl ViewTest for ScreenKeyboardFirstLetter {
    fn canvas() -> (u32, u32) {
        (600, 1000)
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        tap(COMPOSE.0, COMPOSE.1)?;
        wait_until("the box is edited", move || view.compose.is_editing())?;
        screen_keyboard(true, KEYBOARD_TOP)?;
        wait_until("the box is on top of the keyboard", move || {
            ScreenKeyboard::top().is_some_and(|top| (view.box_bottom() - (top - MARGIN)).abs() < 0.5)
        })?;
        checkpoint("the keyboard is up, the box rides on it, no letter typed yet")?;

        let place = from_main(move || {
            view.watch();
            view.box_bottom()
        });

        view.type_letter()?;
        view.resize_and_watch(place, "the box holds A")?;
        for _ in 0..ERASES {
            view.erase();
            view.resize_and_watch(place, "the box is empty and shows its placeholder")?;
            view.type_letter()?;
            view.resize_and_watch(place, "the box holds A")?;
        }

        from_main(move || {
            ensure!(
                view.moves.is_empty(),
                "the keyboard stayed and the letter moved it away: {:?}",
                view.moves
            );
            ensure!(
                view.moved < 0.5,
                "the box moved by {} at the first letter",
                view.moved
            );
            // The keyboard leaves next, that move is not of the letter.
            let mut view = view;
            view.watched = false;
            Ok(())
        })?;

        tap(OUTSIDE.0, OUTSIDE.1)?;
        wait_until("the box is not edited any more", move || {
            !view.compose.is_editing()
        })?;
        screen_keyboard(false, KEYBOARD_TOP)?;
        wait_until("the box is back at the bottom", move || {
            (view.box_bottom() - 980.0).abs() < 0.5
        })?;
        checkpoint("the keyboard is gone, the box is back at the bottom and holds A")
    }
}
