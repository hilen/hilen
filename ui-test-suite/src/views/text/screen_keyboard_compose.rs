use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{
        Anchor, Label, ScreenKeyboard, Setup, TextAlignment, TextField, ViewData, ViewFrame, ViewTest, view,
    },
    ui_test::{
        check_colors, checkpoint,
        system_input::{screen_keyboard, tap, wait_until},
    },
};

use super::keyboard_marker::KeyboardMarker;

const COMPOSE: (f32, f32) = (300.0, 955.0);
/// In the chat, above every keyboard.
const OUTSIDE: (f32, f32) = (300.0, 300.0);
const KEYBOARD_TOP: f32 = 814.0;
const MARGIN: f32 = 20.0;
const CHAT_TOP: f32 = 60.0;

/// The compose box of a chat, placed with `b_keyboard`. It rides on top of
/// the keyboard and the chat above it gets shorter. The screen itself does
/// not move, the box is in view already.
#[view]
struct ScreenKeyboardCompose {
    // The box is laid out first, the chat reads its frame.
    #[init]
    compose: TextField,
    title:   Label,
    chat:    Label,
    marker:  KeyboardMarker,
}

impl Setup for ScreenKeyboardCompose {
    fn setup(self: Weak<Self>) {
        self.title.set_text("a chat with a compose box").set_text_size(18);
        self.title.set_alignment(TextAlignment::Left);
        self.title.place().t(17).lr(20).h(26);

        self.compose.set_text_size(24).set_placeholder("message");
        self.compose.place().lr(MARGIN).h(50).b_keyboard(MARGIN);

        self.chat.set_text("the chat ends above the box").set_text_size(18);
        self.chat.set_color((230, 236, 240));
        self.chat
            .place()
            .t(CHAT_TOP)
            .lr(MARGIN)
            .anchor(Anchor::Bot, self.compose, MARGIN);
    }
}

impl ViewTest for ScreenKeyboardCompose {
    fn canvas() -> (u32, u32) {
        (600, 1000)
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        from_main(move || {
            let bottom = view.compose.absolute_frame().max_y();
            ensure!(
                (bottom - 980.0).abs() < 0.5,
                "with no keyboard the box ends at {bottom}"
            );
            Ok(())
        })?;

        tap(COMPOSE.0, COMPOSE.1)?;
        wait_until("the box is edited", move || view.compose.is_editing())?;
        screen_keyboard(true, KEYBOARD_TOP)?;
        wait_until("the box is on top of the keyboard", move || {
            ScreenKeyboard::top()
                .is_some_and(|top| (view.compose.absolute_frame().max_y() - (top - MARGIN)).abs() < 0.5)
        })?;
        checkpoint("the keyboard is up, the box rides on it, the chat is shorter")?;
        from_main(move || {
            let top = ScreenKeyboard::top().unwrap_or_default();
            let shift = ScreenKeyboard::shift();
            ensure!(shift == 0.0, "the screen moved up by {shift}, the box is in view");
            let chat = *view.chat.absolute_frame();
            ensure!(
                (chat.y() - CHAT_TOP).abs() < 0.5,
                "the chat starts at {}",
                chat.y()
            );
            let chat_bottom = top - MARGIN - 50.0 - MARGIN;
            ensure!(
                (chat.max_y() - chat_bottom).abs() < 1.0,
                "the chat ends at {}, not at {chat_bottom}",
                chat.max_y()
            );
            Ok(())
        })?;

        tap(OUTSIDE.0, OUTSIDE.1)?;
        wait_until("the box is not edited any more", move || {
            !view.compose.is_editing()
        })?;
        screen_keyboard(false, KEYBOARD_TOP)?;
        wait_until("the box is back at the bottom", move || {
            (view.compose.absolute_frame().max_y() - 980.0).abs() < 0.5
                && (view.chat.absolute_frame().max_y() - 910.0).abs() < 0.5
        })?;

        check_colors(FINAL)
    }
}

const FINAL: &str = r"
             116   24 - #394f5f
              56   32 - #597c95
             136   32 - #384e5d
             156   32 - #597c95
             176   32 - #304350
             212   32 - #597c95
             236   32 - #597c95
             576   60 - #e6ecf0
               4  296 - #597c95
             576  316 - #e6ecf0
             284  480 - #000000
             384  480 - #353738
             196  484 - #000000
             248  484 - #8f9295
             356  484 - #525456
             200  488 - #ccd1d5
             208  488 - #e6ecf0
             268  488 - #010101
             308  488 - #000000
             344  488 - #e6ecf0
             592  736 - #597c95
              20  752 - #e6ecf0
             268  956 - #d3d3d3
             280  956 - #bcbcbc
             316  956 - #bcbcbc
             320  956 - #c3c3c3
             332  956 - #cecece
             344  956 - #bcbcbc
             260  960 - #e0e0e0
             304  960 - #ffffff
             312  960 - #bcbcbc
             592  992 - #597c95
";
