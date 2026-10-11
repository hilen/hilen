use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{
        Anchor, CellRegistry, Color, Label, ScreenKeyboard, Setup, TableData, TableView, TextAlignment,
        TextField, View, ViewCallbacks, ViewData, ViewFrame, ViewTest, view,
    },
    ui_test::{
        check_colors, checkpoint, inject_scroll, inject_touches,
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
const ROW: f32 = 50.0;
const ROWS: usize = 40;
const SCROLLED_UP: f32 = 300.0;

/// How much of the rows is under the bottom edge of the chat, 0 for a
/// chat at its end.
fn below_end(chat: &TableView) -> f32 {
    chat.content_height() + chat.content_offset() - chat.height()
}

/// Looks at the chat once a frame, after the chat and the scroll view
/// inside it got their frames and before the frame is drawn. The keyboard
/// moves over many frames, and the chat has to be at its end in each of
/// them, not only when the keyboard stands still.
#[view]
struct ChatWatch {
    chat:    Weak<TableView>,
    /// The chat has to be at its end in every frame now.
    expect:  bool,
    /// Frames that drew the chat off its end while it had to be there.
    off_end: usize,
    /// How many times the chat had another height than the frame before.
    heights: usize,
    height:  f32,
}

impl ViewCallbacks for ChatWatch {
    fn update(&mut self) {
        if self.chat.is_null() {
            return;
        }
        let height = self.chat.height();
        if height.to_bits() != self.height.to_bits() {
            self.height = height;
            self.heights += 1;
        }
        if self.expect && below_end(&self.chat).abs() > 0.5 {
            self.off_end += 1;
        }
    }
}

/// A chat above a compose box. The box is placed with `b_keyboard`, the
/// chat is anchored above it and has `set_keeps_end`. The keyboard comes
/// up, the chat gets shorter on every frame of the move and still ends
/// with its last message. A chat the reader scrolled up keeps its place.
#[view]
struct ScreenKeyboardKeepsEnd {
    // The box is laid out first, the chat reads its frame.
    #[init]
    compose: TextField,
    title:   Label,
    chat:    TableView,
    // After the chat, it reads the frames of this frame.
    watch:   ChatWatch,
    marker:  KeyboardMarker,
}

impl Setup for ScreenKeyboardKeepsEnd {
    fn setup(mut self: Weak<Self>) {
        self.title.set_text("a chat that keeps its end").set_text_size(18);
        self.title.set_alignment(TextAlignment::Left);
        self.title.place().t(17).lr(20).h(26);

        self.compose.set_text_size(24).set_placeholder("message");
        self.compose.place().lr(MARGIN).h(50).b_keyboard(MARGIN);

        self.chat.set_data_source(self).register_cell::<Label>();
        self.chat.set_keeps_end(true);
        self.chat.set_color((70, 78, 88));
        self.chat
            .place()
            .t(CHAT_TOP)
            .lr(MARGIN)
            .anchor(Anchor::Bot, self.compose, MARGIN);

        self.watch.chat = self.chat;
        // From the first frame: a chat that is filled with nothing on
        // screen yet starts at its end.
        self.watch.expect = true;
    }
}

impl TableData for ScreenKeyboardKeepsEnd {
    fn cell_height(&self, _: usize) -> f32 {
        ROW
    }

    fn number_of_cells(&self) -> usize {
        ROWS
    }

    fn setup_cell(&mut self, index: usize, registry: &mut CellRegistry) -> Weak<dyn View> {
        let cell = registry.cell::<Label>();
        let text = if index + 1 == ROWS {
            format!("message {index}, the last")
        } else {
            format!("message {index}")
        };
        cell.set_text(text).set_text_size(18);
        cell.set_color(if index.is_multiple_of(2) {
            Color::rgb(0.87, 0.91, 0.94)
        } else {
            Color::rgb(0.76, 0.82, 0.88)
        });
        cell
    }

    fn cell_selected(&mut self, _: usize) {}
}

fn keyboard_up(view: Weak<ScreenKeyboardKeepsEnd>) -> Result<()> {
    tap(COMPOSE.0, COMPOSE.1)?;
    wait_until("the box is edited", move || view.compose.is_editing())?;
    screen_keyboard(true, KEYBOARD_TOP)?;
    wait_until("the box is on top of the keyboard", move || {
        ScreenKeyboard::top()
            .is_some_and(|top| (view.compose.absolute_frame().max_y() - (top - MARGIN)).abs() < 0.5)
    })
}

fn keyboard_away(view: Weak<ScreenKeyboardKeepsEnd>) -> Result<()> {
    tap(OUTSIDE.0, OUTSIDE.1)?;
    wait_until("the box is not edited any more", move || {
        !view.compose.is_editing()
    })?;
    screen_keyboard(false, KEYBOARD_TOP)?;
    wait_until("the box is back at the bottom", move || {
        (view.compose.absolute_frame().max_y() - 980.0).abs() < 0.5
            && (view.chat.absolute_frame().max_y() - 910.0).abs() < 0.5
    })
}

fn expect_at_end(view: Weak<ScreenKeyboardKeepsEnd>, step: &'static str) -> Result<()> {
    from_main(move || {
        let below = below_end(&view.chat);
        ensure!(below.abs() < 0.5, "{step}: the chat has {below} under its edge");
        ensure!(
            view.watch.off_end == 0,
            "{step}: {} frames drew the chat off its end",
            view.watch.off_end
        );
        Ok(())
    })
}

fn expect_offset(view: Weak<ScreenKeyboardKeepsEnd>, step: &'static str, offset: f32) -> Result<()> {
    from_main(move || {
        let now = view.chat.content_offset();
        ensure!(
            now.to_bits() == offset.to_bits(),
            "{step}: the chat scrolled up moved from {offset} to {now}"
        );
        Ok(())
    })
}

impl ViewTest for ScreenKeyboardKeepsEnd {
    fn canvas() -> (u32, u32) {
        (600, 1000)
    }

    fn perform_test(mut view: Weak<Self>) -> Result<()> {
        wait_until("the chat has its rows", move || {
            view.chat.height() > 0.0 && view.chat.content_height() > view.chat.height()
        })?;
        expect_at_end(view, "at the start")?;

        let heights = from_main(move || view.watch.heights);
        keyboard_up(view)?;
        checkpoint("the keyboard is up, the chat is shorter and still ends with its last message")?;
        expect_at_end(view, "keyboard up")?;
        // The chat got shorter step by step, so the watch saw the frames
        // of the move and not only its 2 ends.
        let steps = from_main(move || view.watch.heights) - heights;
        ensure!(steps >= 3, "the chat got shorter in only {steps} steps");

        keyboard_away(view)?;
        check_colors(AT_END)?;
        expect_at_end(view, "keyboard away")?;

        from_main(move || view.watch.expect = false);
        inject_touches("300 300 m");
        inject_scroll(SCROLLED_UP);
        let scrolled = from_main(move || view.chat.content_offset());
        from_main(move || {
            let below = below_end(&view.chat);
            ensure!(
                (below - SCROLLED_UP).abs() < 0.5,
                "the wheel left {below} under the edge"
            );
            view.title.set_text("the chat is scrolled up by 300 and stays there");
            Ok(())
        })?;

        keyboard_up(view)?;
        checkpoint("the keyboard is up, the chat scrolled up kept the messages at its top")?;
        expect_offset(view, "scrolled up, keyboard up", scrolled)?;

        keyboard_away(view)?;
        check_colors(SCROLLED)?;
        expect_offset(view, "scrolled up, keyboard away", scrolled)
    }
}

const AT_END: &str = r"
              44   32 - #2f424f
             104   32 - #3d5566
             140   32 - #597c95
             152   32 - #597c95
             568   60 - #c2d1e0
             320  136 - #161718
             320  236 - #161718
             264  288 - #000000
             576  316 - #dee8f0
             308  336 - #dee8f0
              20  364 - #c2d1e0
             312  488 - #000000
             272  588 - #c2d1e0
             344  588 - #c2d1e0
             576  612 - #90979c
               4  672 - #597c95
             264  684 - #000000
             576  716 - #90979c
             344  736 - #dee8f0
             252  788 - #2b2f32
             572  816 - #dee8f0
             356  880 - #87929c
             380  884 - #a6b3bf
             224  888 - #5d646b
             332  888 - #a2afbb
             356  888 - #87929c
             328  956 - #ffffff
             260  960 - #e0e0e0
             304  960 - #ffffff
              20  976 - #ffffff
             140  976 - #ffffff
             528  992 - #597c95
";
const SCROLLED: &str = r"
              44   24 - #151d23
             300   24 - #304350
             192   28 - #24333d
             108   32 - #3e5768
             152   32 - #597c95
             264   32 - #597c95
             356   32 - #49667a
             376   32 - #597c95
             548  112 - #dee8f0
             316  136 - #000000
               4  184 - #597c95
             280  240 - #010101
             592  296 - #597c95
             340  336 - #dee8f0
              24  344 - #dee8f0
             272  388 - #c2d1e0
             576  452 - #90979c
             312  488 - #000000
             272  588 - #c2d1e0
             576  612 - #90979c
              20  664 - #c2d1e0
             252  688 - #2b2f32
             344  732 - #dee8f0
             272  836 - #161718
             576  864 - #c2d1e0
              20  932 - #ffffff
             280  956 - #bcbcbc
             328  956 - #ffffff
             344  956 - #bcbcbc
             260  960 - #e0e0e0
             304  960 - #ffffff
             488  976 - #ffffff
";
