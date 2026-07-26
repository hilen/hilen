use anyhow::Result;
use test_engine::{
    refs::Weak,
    ui::{
        Button, CellRegistry, Container, GREEN, ImageView, Label, Setup, TURQUOISE, TableData, TableView,
        View, ViewData, ViewSubviews, ViewTest, WHITE,
        ui_test::{helpers::check_colors, inject_touches},
        view,
    },
};

#[view]
struct SomeView {
    #[init]
    table:  TableView,
    label:  Label,
    image:  ImageView,
    square: Container,
}

impl Setup for SomeView {
    fn setup(self: Weak<Self>) {
        self.table.set_data_source(self).register_cell::<Label>().place().size(400, 400);
        self.label.set_text("Hello").set_color(GREEN).place().size(200, 200).tr(10);
        self.image.set_image("plus.png").place().size(200, 200).bl(10);
        self.square.set_color(TURQUOISE).place().size(200, 200).br(10);
    }
}

impl TableData for SomeView {
    fn number_of_cells(&self) -> usize {
        2
    }

    fn cell_height(&self, _: usize) -> f32 {
        50.0
    }

    fn setup_cell(&mut self, index: usize, registry: &mut CellRegistry) -> Weak<dyn View> {
        registry.cell::<Label>().set_color(WHITE).set_text(format!("{index}")).weak()
    }
}

#[view]
struct AddOnTap {
    #[init]
    button: Button,
}

impl Setup for AddOnTap {
    fn setup(self: Weak<Self>) {
        self.button.set_text("A").place().size(50, 50);
        self.button.on_tap(move || {
            let view = self.add_view::<SomeView>();
            view.place().size(600, 500).br(5);
        });
    }
}

impl ViewTest for AddOnTap {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        assert_eq!(view.dump_subviews(), vec!["AddOnTap.button: Button".to_string()]);

        inject_touches(
            "
            25   25   b
            25   25   e
        ",
        );

        assert_eq!(
            view.dump_subviews(),
            vec!["AddOnTap.button: Button".to_string(), "SomeView".to_string()]
        );

        check_colors(
            r"
               4    4 - #ffffff
             292    4 - #597c95
             592    4 - #597c95
              80   96 - #ffffff
             396  108 - #00ff00
             576  112 - #00ff00
             192  116 - #ffffff
             196  116 - #ffffff
             192  120 - #ffffff
             196  120 - #ffffff
              16  192 - #ffffff
             120  192 - #ffffff
             300  192 - #ffffff
             512  200 - #00ff00
             512  204 - #00ff00
             480  208 - #00ff00
             512  208 - #00ff00
             388  300 - #00ff00
             484  300 - #00ff00
             580  300 - #00ff00
             516  388 - #00ffff
             160  404 - #2196f3
             592  436 - #597c95
             444  440 - #00ffff
              16  444 - #2196f3
             296  468 - #597c95
              72  524 - #2196f3
             472  528 - #00ffff
             160  564 - #2196f3
             396  580 - #00ffff
             548  580 - #00ffff
               4  592 - #597c95
        ",
        )?;

        Ok(())
    }
}
