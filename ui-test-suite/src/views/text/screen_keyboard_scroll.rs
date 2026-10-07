use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{
        Label, ScreenKeyboard, ScrollView, Setup, TextAlignment, TextField, ViewData, ViewFrame,
        ViewSubviews, ViewTest, view,
    },
    ui_test::{
        check_colors, checkpoint,
        system_input::{screen_keyboard, tap, wait_until},
    },
};

use super::keyboard_marker::KeyboardMarker;

const FIELD: (f32, f32) = (300.0, 925.0);
/// The title above the scroll view, above every keyboard and under the
/// status bar of a phone, which takes a tap on itself.
const OUTSIDE: (f32, f32) = (300.0, 83.0);
const KEYBOARD_TOP: f32 = 814.0;
const ROWS: usize = 30;

/// A field low in a long form that scrolls. The keyboard would cover it, so
/// the form scrolls the field into view. The screen itself stays where it
/// is.
#[view]
struct ScreenKeyboardScroll {
    field: Weak<TextField>,

    #[init]
    title:  Label,
    scroll: ScrollView,
    marker: KeyboardMarker,
}

impl Setup for ScreenKeyboardScroll {
    fn setup(mut self: Weak<Self>) {
        self.title.set_text("a form that scrolls, tap here to end").set_text_size(18);
        self.title.set_alignment(TextAlignment::Left);
        self.title.place().t(70).lr(20).h(26);

        self.scroll.place().t(120).lr(0).b(0);

        for row in 0..ROWS {
            let y = row.to_f32_lossy() * 60.0;
            // Abs 900 to 950, under the keyboard.
            if row == 13 {
                let field = self.scroll.add_view::<TextField>();
                field.set_text_size(24).set_placeholder("tap to type");
                field.place().t(y).lr(20).h(50);
                self.field = field;
                continue;
            }
            let label = self.scroll.add_view::<Label>();
            label.set_text(format!("row {row}")).set_text_size(18);
            label.set_alignment(TextAlignment::Left);
            label.place().t(y).lr(20).h(50);
        }
    }
}

trait ToF32Lossy {
    fn to_f32_lossy(self) -> f32;
}

impl ToF32Lossy for usize {
    fn to_f32_lossy(self) -> f32 {
        u16::try_from(self).map_or(0.0, f32::from)
    }
}

impl ViewTest for ScreenKeyboardScroll {
    fn canvas() -> (u32, u32) {
        (600, 1000)
    }

    fn perform_test(mut view: Weak<Self>) -> Result<()> {
        tap(FIELD.0, FIELD.1)?;
        wait_until("the field is edited", move || view.field.is_editing())?;
        screen_keyboard(true, KEYBOARD_TOP)?;
        wait_until("the field is above the keyboard", move || {
            ScreenKeyboard::top().is_some_and(|top| view.field.absolute_frame().max_y() < top)
        })?;
        checkpoint("the keyboard is up, the form scrolled, the title did not move")?;
        let scrolled = from_main(move || {
            let top = ScreenKeyboard::top().unwrap_or_default();
            let bottom = view.field.absolute_frame().max_y();
            ensure!(
                (top - bottom - 8.0).abs() < 1.0,
                "the field ends at {bottom}, the keyboard starts at {top}"
            );
            let shift = ScreenKeyboard::shift();
            ensure!(
                shift == 0.0,
                "the screen moved up by {shift}, the form can scroll"
            );
            let offset = view.scroll.get_scroll_content_offset();
            ensure!(
                (offset + (950.0 + 8.0 - top)).abs() < 1.0,
                "the form scrolled to {offset} for a keyboard at {top}"
            );
            Ok(offset)
        })?;

        tap(OUTSIDE.0, OUTSIDE.1)?;
        wait_until("the field is not edited any more", move || {
            !view.field.is_editing()
        })?;
        screen_keyboard(false, KEYBOARD_TOP)?;
        from_main(move || {
            let offset = view.scroll.get_scroll_content_offset();
            ensure!(
                (offset - scrolled).abs() < 0.5,
                "the form scrolled from {scrolled} to {offset} when the keyboard left"
            );
            // Each keyboard has its own height, the look is checked from
            // the top of the form.
            view.scroll.set_content_offset(0);
            Ok(())
        })?;

        check_colors(FINAL)
    }
}

const FINAL: &str = r"
              96   84 - #283844
             160   84 - #597c95
             204   84 - #384e5d
             228   84 - #000001
             296   84 - #466175
             304   84 - #597c95
              40   88 - #000000
              76  144 - #597c95
             592  188 - #597c95
              76  208 - #23313b
              76  384 - #2d3e4b
             592  496 - #597c95
              72  504 - #000000
             340  632 - #597c95
              76  740 - #23313b
              76  800 - #23313b
              76  860 - #23313b
             576  900 - #ffffff
             156  912 - #ffffff
             288  924 - #e5e5e5
             316  924 - #ffffff
             332  924 - #bcbcbc
             288  928 - #e5e5e5
             300  928 - #ffffff
             336  928 - #ffffff
             248  932 - #bcbcbc
             272  932 - #bcbcbc
             324  932 - #bcbcbc
             332  936 - #bcbcbc
             460  948 - #ffffff
              88  980 - #456073
              84  988 - #456073
";
