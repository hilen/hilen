use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    refs::{Weak, manage::DataManager},
    ui::{Font, Label, Setup, TextAlignment, TextField, UIManager, ViewData, ViewTest, view},
    ui_test::{
        checkpoint,
        system_input::{ScreenInk, screen_ink, system_request, tap, wait_until},
    },
};

const OUTSIDE: (f32, f32) = (300.0, 540.0);

/// Half a point of an iPhone screen at the scale of the test.
const PLACE_TOLERANCE: f32 = 1.0;
/// Per color channel. The 2 rasterizers smooth the edge of a glyph in their
/// own way, the middle of a thick stroke is what both paint in full.
const COLOR_TOLERANCE: u8 = 24;

/// The system text field that takes over while a field is edited must draw
/// the text where the engine drew it, in the same font and color, so the
/// user does not see the switch. The text is measured on a capture of the
/// real screen, the only picture that holds both.
#[view]
struct ScreenKeyboardLook {
    #[init]
    plain_title:   Label,
    plain:         TextField,
    styled_title:  Label,
    styled:        TextField,
    outside_title: Label,
}

/// The frame of each field, `x y width height`, with a tap point right of
/// its text.
const PLAIN: (f32, f32, f32, f32) = (20.0, 90.0, 560.0, 60.0);
const STYLED: (f32, f32, f32, f32) = (20.0, 200.0, 560.0, 60.0);

impl Setup for ScreenKeyboardLook {
    fn setup(mut self: Weak<Self>) {
        let title = |label: Weak<Label>, text: &str, y: f32| {
            label.set_text(text).set_text_size(18);
            label.set_alignment(TextAlignment::Left);
            label.place().t(y).lr(20).h(26);
        };

        title(self.plain_title, "default font, centered, black", 60.0);
        self.plain.set_text_size(32);
        self.plain.set_text("Hpqd");
        self.plain.place().t(PLAIN.1).l(PLAIN.0).size(PLAIN.2, PLAIN.3);

        title(self.styled_title, "mono font, left, pink", 170.0);
        self.styled.set_font(Font::get("DroidSansMono.ttf"));
        self.styled.set_text_size(36);
        self.styled.set_text_color((194, 24, 91));
        self.styled.set_text("Hpqd");
        self.styled.place().t(STYLED.1).l(STYLED.0).size(STYLED.2, STYLED.3);
        self.styled.set_alignment(TextAlignment::Left);

        title(self.outside_title, "a tap down here ends the editing", 527.0);
    }
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() <= PLACE_TOLERANCE
}

fn same_look(what: &str, engine: ScreenInk, system: ScreenInk) -> Result<()> {
    ensure!(
        close(engine.left, system.left)
            && close(engine.top, system.top)
            && close(engine.bottom, system.bottom),
        "{what}: the text moved, engine {engine:?}, system {system:?}"
    );
    let same_color = engine
        .color
        .iter()
        .zip(system.color)
        .all(|(engine, system)| engine.abs_diff(system) <= COLOR_TOLERANCE);
    ensure!(
        same_color,
        "{what}: the color changed, engine {engine:?}, system {system:?}"
    );
    Ok(())
}

/// Compares the text of one field before, while and after it is edited.
fn check_field(
    what: &'static str,
    field: Weak<TextField>,
    (x, y, width, height): (f32, f32, f32, f32),
) -> Result<()> {
    // Inside the field, so the first pixel is its background.
    let (x, y, width, height) = (x + 4.0, y + 4.0, width - 8.0, height - 8.0);
    let whole = screen_ink(x, y, width, height)?;

    // The caret of the system field stands right after the text and would
    // count as ink. Both pictures are measured up to just before the end
    // of the text, which cuts the last glyph the same way in both.
    let width = whole.right - 6.0;
    let engine = screen_ink(x, y, width, height)?;

    tap(x + 530.0, y + height / 2.0)?;
    wait_until("the field is edited", move || field.is_editing())?;
    system_request("keyboard 1")?;
    checkpoint(&format!("{what}: edited, the text looks like before the tap"))?;
    let system = screen_ink(x, y, width, height)?;
    same_look(what, engine, system)?;

    tap(OUTSIDE.0, OUTSIDE.1)?;
    wait_until("the field is not edited any more", move || !field.is_editing())?;
    let after = screen_ink(x, y, width, height)?;
    same_look(what, engine, after)
}

fn set_scale(scale: f32) -> Result<()> {
    from_main(move || UIManager::override_scale(scale));
    wait_until("the UI scale changed", move || {
        UIManager::scale().to_bits() == scale.to_bits()
    })
}

/// The UI scale changes while a field is edited. The system field counts
/// in pixels, so it has to take the new text size and the new place, else
/// its text and its caret stay where the old scale had them.
fn check_scale_change(field: Weak<TextField>, (x, y, _, height): (f32, f32, f32, f32)) -> Result<()> {
    // At scale 2 the field is wider than the screen, the text starts at
    // its left end and half of the canvas width holds all of it.
    let (x, y, width, height) = (x + 4.0, y + 4.0, 280.0, height - 8.0);

    set_scale(2.0)?;
    let whole = screen_ink(x, y, width, height)?;
    let width = whole.right - 6.0;
    let engine = screen_ink(x, y, width, height)?;
    set_scale(1.0)?;

    tap(x + 530.0, y + height / 2.0)?;
    wait_until("the field is edited", move || field.is_editing())?;
    system_request("keyboard 1")?;

    set_scale(2.0)?;
    checkpoint("mono font at scale 2: edited, twice as big, the text where the engine draws it")?;
    let system = screen_ink(x, y, width, height)?;
    let moved = same_look("mono font, the scale changed while edited", engine, system);
    set_scale(1.0)?;
    moved?;

    tap(OUTSIDE.0, OUTSIDE.1)?;
    wait_until("the field is not edited any more", move || !field.is_editing())
}

impl ViewTest for ScreenKeyboardLook {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        check_field("default font", view.plain, PLAIN)?;
        check_field("mono font", view.styled, STYLED)?;
        check_scale_change(view.styled, STYLED)
    }
}
