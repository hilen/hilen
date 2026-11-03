use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{
        CLEAR, CellRegistry, Color, DropDown, DropDownData, Label, Setup, TextAlignment, ViewData, ViewTest,
        ViewTouch, WeakView, view,
    },
    ui_test::{check_colors, inject_touches},
};

const ROW: f32 = 44.0;
const BOX: Color = Color::rgb(0.98, 0.98, 1.0);
const ACCENT: Color = Color::rgb(0.85, 0.35, 0.10);
const ROW_HOVER: Color = Color::rgb(0.90, 0.95, 1.0);

const IDLE: &str = r"
              44   44 - #2f424f
              56   44 - #597c95
              80   44 - #597c95
             108   44 - #597c95
             132   44 - #10171b
             160   44 - #080b0d
             188   44 - #090c0f
             204   44 - #597c95
             240   44 - #000000
             256   44 - #263540
             284   44 - #25343f
             304   44 - #090c0f
             348   44 - #000000
             368   44 - #10171b
             400   44 - #304351
             228   48 - #010101
             252   48 - #000000
             328   48 - #010101
              80   84 - #ccd6e6
             224   88 - #fafaff
              80   92 - #ccd6e6
             132   92 - #fafaff
             144   92 - #7d7d80
              80   96 - #ccd6e6
              80  100 - #ccd6e6
             316  104 - #fafaff
             192  112 - #fafaff
             256  112 - #fafaff
             592  284 - #597c95
             300  440 - #597c95
               4  592 - #597c95
             592  592 - #597c95
";
const BOX_HOVERED: &str = r"
              44   44 - #2f424f
              56   44 - #597c95
              80   44 - #597c95
             108   44 - #597c95
             132   44 - #10171b
             160   44 - #080b0d
             188   44 - #090c0f
             204   44 - #597c95
             240   44 - #000000
             256   44 - #263540
             284   44 - #25343f
             348   44 - #000000
             368   44 - #10171b
             400   44 - #304351
             228   48 - #010101
             252   48 - #000000
             328   48 - #010101
             284   84 - #f7eded
              80   88 - #d9591a
              80   92 - #d9591a
             132   92 - #f7eded
             144   92 - #7c7777
              80   96 - #d9591a
             128   96 - #7a7575
              80  100 - #d9591a
             200  104 - #f7eded
             316  104 - #f7eded
             256  112 - #f7eded
             592  284 - #597c95
             300  440 - #597c95
               4  592 - #597c95
             592  592 - #597c95
";
const ROW_HOVERED: &str = r"
              44   44 - #2f424f
             104   44 - #597c95
             132   44 - #10171b
             160   44 - #080b0d
             184   44 - #000000
             212   44 - #090c0f
             296   44 - #000001
             328   44 - #597c95
             376   44 - #597c95
             400   44 - #304351
             252   48 - #000000
              80   84 - #d9591a
             132   92 - #fafaff
              80   96 - #d9591a
              80  100 - #d9591a
             188  112 - #f9f9fe
             248  120 - #ccd6e6
             320  140 - #445f72
             116  148 - #78787a
             136  148 - #fafaff
              80  168 - #ccd6e6
             116  188 - #e6f2ff
             216  188 - #e6f2ff
             140  192 - #45494d
              80  208 - #ccd6e6
             316  216 - #fafaff
             136  236 - #000000
             232  264 - #476377
             288  264 - #476377
             300  552 - #597c95
               4  592 - #597c95
             592  592 - #597c95
";

/// A row that paints its own hover, like a cell of an app does for the rows of
/// the open list.
#[view]
struct HoverCell {
    #[init]
    name: Label,
}

impl Setup for HoverCell {
    fn setup(self: Weak<Self>) {
        self.name.set_alignment(TextAlignment::Left).set_text_size(16);
        self.name.place().l(12).r(8).tb(0);
        self.enable_hover();
        self.touch().hovered.val(self, move |hovered| {
            self.set_color(if hovered { ROW_HOVER } else { CLEAR });
        });
    }
}

#[view]
struct Fruits {
    fruits:   Vec<&'static str>,
    box_cell: Weak<HoverCell>,

    #[init]
    drop: DropDown,
}

impl Setup for Fruits {
    fn setup(mut self: Weak<Self>) {
        self.fruits = vec!["Apple", "Pear", "Plum"];
        self.drop
            .set_color(BOX)
            .set_corner_radius(12)
            .set_border_width(1)
            .set_border_color(Color::rgb(0.80, 0.84, 0.90));
        self.drop.set_accent_color(ACCENT);
        self.drop.place().back();
        self.drop.register_cell::<HoverCell>().set_data_source(self);
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
        let cell = registry.cell::<HoverCell>();
        cell.name.set_text(self.fruits[index]);
        Some(cell)
    }

    fn setup_box_cell(&mut self, index: usize, registry: &mut CellRegistry) -> Option<WeakView> {
        let cell = registry.cell::<HoverCell>();
        cell.name.set_text(self.fruits[index]);
        self.box_cell = cell;
        Some(cell)
    }

    fn row_selected(&mut self, _: usize) {}
}

/// The closed box of a drop down with cells that paint their own hover. A
/// pointer over the box lights the whole box with the accent, its corners and
/// the arrow too, and the cell in the box stays unlit. The rows of the open
/// list still light themselves.
#[view]
struct DropDownBoxHover {
    #[init]
    title:  Label,
    fruits: Fruits,
}

impl Setup for DropDownBoxHover {
    fn setup(self: Weak<Self>) {
        self.title.set_text_size(18).set_alignment(TextAlignment::Left);
        self.title.set_text("a drop down whose cells paint their own hover");
        self.title.place().t(30).lr(20).h(26);
        self.fruits.place().t(70).l(80).size(240, ROW);
    }
}

impl ViewTest for DropDownBoxHover {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        // The pointer can stay over this place from the test before.
        inject_touches("500 500 m");
        wait_for_next_frame();
        check_colors(IDLE)?;

        // Over the box, on the text of its cell.
        inject_touches("150 92 m");
        wait_for_next_frame();
        from_main(move || {
            ensure!(view.fruits.drop.is_hovered(), "the box took no hover");
            ensure!(
                !view.fruits.box_cell.is_hovered(),
                "the cell of the box took the hover"
            );
            ensure!(*view.fruits.drop.color() != BOX, "the box did not light up");
            Ok(())
        })?;
        check_colors(BOX_HOVERED)?;

        inject_touches("500 500 m");
        wait_for_next_frame();
        from_main(move || {
            ensure!(*view.fruits.drop.color() == BOX, "the box kept its hover fill");
            Ok(())
        })?;

        // Open the list, then hover its second row. The panel opens 6 under the
        // box at y 120, its rows start 4 lower and are 44 tall, Pear is
        // 168..212.
        inject_touches("150 92 b\n150 92 e");
        inject_touches("150 190 m");
        wait_for_next_frame();
        check_colors(ROW_HOVERED)?;

        inject_touches("500 500 b\n500 500 e");
        Ok(())
    }
}
