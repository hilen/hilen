use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{
        CellRegistry, Color, Container, DropDown, DropDownData, Label, Setup, TextAlignment, ViewData,
        ViewFrame, ViewSubviews, ViewTest, ViewTouch, WeakView, view,
    },
    ui_test::{checkpoint, inject_touches},
};

const ROW: f32 = 44.0;
const FIELD: Color = Color::rgb(0.90, 0.90, 0.92);
const FIELD_HOVER: Color = Color::rgb(0.95, 0.75, 0.55);
const POINTER: Color = Color::rgb(0.85, 0.10, 0.10);

/// A view that lights up while the pointer hovers it, like a text field.
#[view]
struct Field {
    #[init]
    name: Label,
}

impl Setup for Field {
    fn setup(self: Weak<Self>) {
        self.set_color(FIELD);
        self.name.set_text_size(16).set_alignment(TextAlignment::Left);
        self.name.set_text("field under the list, lit while hovered");
        self.name.place().l(12).r(8).b(8).h(24);
        self.enable_hover();
        self.touch().hovered.val(self, move |hovered| {
            self.set_color(if hovered { FIELD_HOVER } else { FIELD });
        });
    }
}

/// A row that takes no hover, like a cell an app draws for its own list.
#[view]
struct PlainCell {
    #[init]
    name: Label,
}

impl Setup for PlainCell {
    fn setup(self: Weak<Self>) {
        self.name.set_alignment(TextAlignment::Left).set_text_size(16);
        self.name.place().l(12).r(8).tb(0);
    }
}

#[view]
struct Fruits {
    fruits: Vec<&'static str>,

    #[init]
    drop: DropDown,
}

impl Setup for Fruits {
    fn setup(mut self: Weak<Self>) {
        self.fruits = vec!["Apple", "Pear", "Plum"];
        self.drop.set_color(Color::rgb(1.0, 1.0, 1.0));
        self.drop.place().back();
        self.drop.register_cell::<PlainCell>().set_data_source(self);
    }
}

impl DropDownData for Fruits {
    fn number_of_rows(&self) -> usize {
        self.fruits.len()
    }

    fn row_height(&self, _: usize) -> f32 {
        ROW
    }

    fn setup_cell(&mut self, index: usize, registry: &mut CellRegistry) -> Option<WeakView> {
        let cell = registry.cell::<PlainCell>();
        cell.name.set_text(self.fruits[index]);
        Some(cell)
    }

    fn row_selected(&mut self, _: usize) {}
}

/// An open drop down whose rows take no hover, over a field that does. The
/// pointer on a row of the list does not light the field under it, the
/// pointer on the field beside the list still does.
#[view]
struct DropDownHoverUnder {
    pointer: Weak<Container>,

    #[init]
    title:  Label,
    field:  Field,
    fruits: Fruits,
}

impl Setup for DropDownHoverUnder {
    fn setup(mut self: Weak<Self>) {
        self.title.set_text_size(18).set_alignment(TextAlignment::Left);
        self.title.set_text("a list with rows that take no hover, over a field");
        self.title.place().t(30).lr(20).h(26);
        // The field is added before the box, so the box and its list draw
        // in front of it.
        self.field.place().t(150).l(80).size(400, 200);
        self.fruits.place().t(70).l(80).size(240, ROW);
        // The engine's own touch marks take this z, set before the mark is
        // added. It is no hover view, so it never takes the hover itself.
        let mut pointer = Container::new();
        pointer.set_z_position(0.1);
        pointer.set_color(POINTER).set_corner_radius(5);
        pointer.set_frame((560, 560, 10, 10));
        self.pointer = self.add_subview(pointer);
    }
}

/// Moves the pointer and the red mark that shows where it is.
fn point(view: Weak<DropDownHoverUnder>, x: f32, y: f32) {
    inject_touches(format!("{x} {y} m"));
    from_main(move || {
        view.pointer.set_frame((x - 5.0, y - 5.0, 10.0, 10.0));
    });
    wait_for_next_frame();
}

impl ViewTest for DropDownHoverUnder {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        // The pointer can stay over this place from the test before.
        point(view, 560.0, 560.0);

        point(view, 200.0, 200.0);
        from_main(move || {
            ensure!(
                view.field.is_hovered(),
                "the field took no hover with the list closed"
            );
            Ok(())
        })?;
        checkpoint("list closed, the pointer on the field, the field is lit")?;

        // Open the list. It opens 6 under the box at y 120 and covers the
        // field from y 150, x 80 to 320. The pointer at 200 200 is on Pear.
        point(view, 560.0, 560.0);
        inject_touches("150 92 b\n150 92 e");
        wait_for_next_frame();
        point(view, 200.0, 200.0);
        from_main(move || {
            ensure!(
                !view.field.is_hovered(),
                "the field under the open list took the hover of the pointer on a row"
            );
            Ok(())
        })?;
        checkpoint("list open, the pointer on a row over the field, the field stays dark")?;

        point(view, 420.0, 300.0);
        from_main(move || {
            ensure!(
                view.field.is_hovered(),
                "the field beside the open list took no hover"
            );
            Ok(())
        })?;
        checkpoint("list open, the pointer on the field beside it, the field is lit")?;

        inject_touches("560 560 b\n560 560 e");
        Ok(())
    }
}
