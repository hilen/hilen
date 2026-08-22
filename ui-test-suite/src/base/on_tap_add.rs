use anyhow::Result;
use hilen::{
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
             376    4 - #597c95
              24   24 - #ffffff
              32   32 - #000000
             584  108 - #00ff00
             188  116 - #000000
             196  116 - #ffffff
             192  120 - #ffffff
             200  124 - #000000
             336  136 - #ffffff
             196  160 - #000000
               4  192 - #ffffff
             208  192 - #ffffff
             468  196 - #00ae00
             456  204 - #000000
             468  204 - #00ae00
             480  204 - #00ff00
             480  208 - #00b400
             484  208 - #00b400
             516  208 - #00ff00
             468  212 - #00ae00
             480  212 - #00ff00
             388  304 - #00ff00
             580  304 - #00ff00
             184  432 - #2196f3
             592  440 - #597c95
             444  444 - #00ffff
              64  476 - #56aef4
             132  476 - #56aef4
             160  564 - #2196f3
             392  584 - #00ffff
             544  584 - #00ffff
               4  592 - #597c95
        ",
        )?;

        Ok(())
    }
}
