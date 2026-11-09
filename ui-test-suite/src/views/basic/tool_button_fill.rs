use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{Container, Hover, Label, Setup, ToolButton, ViewData, ViewFrame, ViewTest, view},
    ui_test::checkpoint,
};
#[cfg(any(desktop, wasm))]
use hilen::{
    ui::{BLACK, ViewSubviews, ViewTouch, WHITE},
    ui_test::inject_touches,
};

const COLUMN: f32 = 300.0;
const ROW: f32 = 44.0;
/// Where the cursor rests, at the right end of the Forum row.
#[cfg(any(desktop, wasm))]
const POINTER: (u32, u32) = (300, 182);

/// A `ToolButton` with `set_fills_width` takes the width its owner gives
/// it, a row of a menu, its icon and text at the left. Its on look and its
/// hover cover the whole row. A plain one stays as wide as its text.
#[view]
struct ToolButtonFill {
    #[init]
    title:  Label,
    column: Container,
    home:   ToolButton,
    open:   ToolButton,
    forum:  ToolButton,
    plain:  ToolButton,
    note:   Label,
}

impl Setup for ToolButtonFill {
    fn setup(self: Weak<Self>) {
        self.title.set_text("rows fill the gray column").set_text_size(20);
        self.title.place().lrt(10).h(40);

        self.column.set_color("#d0d4da");
        self.column.set_frame((20, 70, COLUMN, ROW * 3.0 + 16.0));

        let mut top = 78.0;
        for (button, text) in [
            (self.home, "Home"),
            (self.open, "Open row"),
            (self.forum, "Forum"),
        ] {
            button.set_icon("tool_tag.svg").set_text(text);
            button.set_fills_width(true);
            button.set_frame((20.0, top, COLUMN, ROW - 4.0));
            top += ROW;
        }
        self.open.set_on(true);

        self.plain.set_icon("tool_tag.svg").set_text("plain");
        self.plain.set_frame((360, 78, 200, 40));

        self.note.set_text("plain stays as wide as its text").set_text_size(16);
        self.note.set_frame((360, 130, 260, 30));

        // A pointer move draws nothing, so a black square marks where the
        // test rests the cursor. It takes the z of the engine's own touch
        // marks, set before it is added, and is not a hover view.
        #[cfg(any(desktop, wasm))]
        {
            let mut pointer = Container::new();
            pointer.set_z_position(0.1);
            pointer
                .set_color(BLACK)
                .set_border_width(3)
                .set_border_color(WHITE)
                .set_size(16, 16);
            let mut pointer = self.add_subview(pointer);
            pointer.set_center(POINTER);
        }
    }
}

impl ViewTest for ToolButtonFill {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        from_main(Hover::clear);

        let (filled, plain, fit) = from_main(move || {
            (
                [view.home.width(), view.open.width(), view.forum.width()],
                view.plain.width(),
                view.plain.fit_width(),
            )
        });
        ensure!(
            filled.iter().all(|width| (width - COLUMN).abs() < 0.5),
            "the rows are {filled:?} wide, the column is {COLUMN}"
        );
        ensure!(
            (plain - fit).abs() < 0.5,
            "the plain button is {plain} wide, its text needs {fit}"
        );
        checkpoint("3 rows as wide as the column, the open row is lit to its end")?;

        // The right end of the forum row, far past its text, is still the
        // row. Hover needs a pointer, a touch screen has none.
        #[cfg(any(desktop, wasm))]
        {
            let (x, y) = POINTER;
            inject_touches(format!("{x} {y} m"));
            let hovered = from_main(move || view.forum.is_hovered());
            ensure!(hovered, "the empty right end of the forum row took no hover");
            checkpoint("pointer at the right end of Forum, the whole row is lit")?;
        }

        from_main(Hover::clear);
        Ok(())
    }
}
