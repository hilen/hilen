use anyhow::Result;
use test_engine::{
    dispatch::from_main,
    refs::Weak,
    ui::{
        Anchor::{Top, X},
        CellRegistry, Container, LIGHT_BLUE, Label, Setup, TableData, TableView, View, ViewData,
        ViewSubviews, ViewTest, WHITE, view,
    },
    ui_test::helpers::check_colors,
};

#[view]
struct LabelImage {
    resizing_image: bool,

    #[init]
    label:      Label,
    table_view: TableView,
    container:  Container,
}

impl Setup for LabelImage {
    fn setup(self: Weak<Self>) {
        self.label.set_text("ßšėčыў").set_text_size(110).set_image("cat.png");
        self.label.place().tl(50).w(400).h(200);

        self.table_view.set_data_source(self).register_cell::<Label>();
        self.table_view
            .place()
            .same([X], self.label)
            .anchor(Top, self.label, 40)
            .w(50)
            .h(200);
        self.table_view.set_color(LIGHT_BLUE);

        self.container.place().t(280).l(280).size(200, 200).all_ver();
        self.container.set_color(LIGHT_BLUE);

        self.container
            .add_view::<Label>()
            .set_text("test 1")
            .set_text_size(50)
            .set_text_color(WHITE)
            .set_image("cat.png");
        self.container
            .add_view::<Label>()
            .set_text("test 2")
            .set_text_size(50)
            .set_text_color(WHITE)
            .set_image("cat.png");
    }
}

impl TableData for LabelImage {
    fn cell_height(&self, _: usize) -> f32 {
        50.0
    }

    fn number_of_cells(&self) -> usize {
        4
    }

    fn setup_cell(&mut self, index: usize, registry: &mut CellRegistry) -> Weak<dyn View> {
        let mut cell = registry.cell::<Label>();
        cell.set_text(index);
        cell.set_text_size(50);
        cell.set_text_color(WHITE);
        if self.resizing_image {
            cell.set_resizing_image("button");
        } else {
            cell.set_image("cat.png");
        }
        cell
    }
}

fn check_initial_images() -> Result<()> {
    check_colors(
        r"
               4    4 - #597c95
             156   52 - #ecc5ca
             316   52 - #deb6b7
             448   52 - #d8aaac
              84  108 - #000000
             220  108 - #000000
             152  136 - #e9cdc2
             392  136 - #010000
             268  140 - #bca186
             288  172 - #010000
              76  176 - #000000
             384  192 - #a3846f
             448  220 - #b59580
             192  244 - #d9b3a6
             356  244 - #a7846e
              92  300 - #d39d9d
             392  312 - #ffffff
             476  312 - #cc9a99
              56  320 - #f4d9d0
              96  328 - #997d67
             336  332 - #fffffe
             460  360 - #9e826c
              96  364 - #c99796
             416  388 - #e1b3b5
              56  420 - #f4d9d0
              96  428 - #997d67
             368  428 - #ffffff
             324  436 - #ffffff
             472  456 - #9b7f6a
              84  468 - #ffffff
             224  592 - #597c95
             592  592 - #597c95
        ",
    )
}

fn check_label_resizing_image(mut view: Weak<LabelImage>) -> Result<()> {
    from_main(move || {
        view.label.set_resizing_image("button");
        view.label.set_text_color(WHITE);
    });

    check_colors(
        r"
               4    4 - #597c95
             592    4 - #597c95
             312   52 - #003ba1
             332   52 - #0039a1
             216   60 - #041342
              60   92 - #051340
             152  104 - #ffffff
             232  132 - #ffffff
             424  136 - #fefefe
             332  148 - #fefefe
             112  172 - #ffffff
             256  204 - #041342
             400  296 - #deb0b2
             472  308 - #cf9999
              56  320 - #f4d9d0
              96  328 - #997d67
             336  328 - #fffffe
             428  348 - #c6a592
             472  356 - #9b7f6a
              96  364 - #c99796
             284  396 - #ebc4c9
             388  396 - #e1b3b5
              92  400 - #d39d9d
             476  404 - #d19f9e
              56  420 - #f4d9d0
              96  428 - #997d67
             448  432 - #c39693
              96  464 - #c99796
             440  472 - #a68870
             312  476 - #e5c3ba
             376  476 - #d2ac99
             592  592 - #597c95
        ",
    )?;

    check_colors(
        r"
               4    4 - #597c95
             592    4 - #597c95
             312   52 - #003ba1
             332   52 - #0039a1
             216   60 - #041342
              60   92 - #051340
             152  104 - #ffffff
             232  132 - #ffffff
             424  136 - #fefefe
             332  148 - #fefefe
             112  172 - #ffffff
             256  204 - #041342
             400  296 - #deb0b2
             472  308 - #cf9999
              56  320 - #f4d9d0
              96  328 - #997d67
             336  328 - #fffffe
             428  348 - #c6a592
             472  356 - #9b7f6a
              96  364 - #c99796
             284  396 - #ebc4c9
             388  396 - #e1b3b5
              92  400 - #d39d9d
             476  404 - #d19f9e
              56  420 - #f4d9d0
              96  428 - #997d67
             448  432 - #c39693
              96  464 - #c99796
             440  472 - #a68870
             312  476 - #e5c3ba
             376  476 - #d2ac99
             592  592 - #597c95
        ",
    )
}

fn check_cells_resizing_image(mut view: Weak<LabelImage>) -> Result<()> {
    from_main(move || {
        view.resizing_image = true;
        view.table_view.reload_data();
    });

    check_colors(
        r"
               4    4 - #597c95
             592    4 - #597c95
             312   52 - #003ba1
             332   52 - #0039a1
             188   60 - #051444
              84   72 - #04123f
             400  108 - #ffffff
             200  132 - #ffffff
             316  132 - #ffffff
             112  172 - #ffffff
             264  196 - #041241
             392  200 - #ffffff
              96  292 - #00daff
             400  296 - #deb0b2
             472  308 - #cf9999
             336  328 - #fffffe
              72  344 - #051241
             428  348 - #c6a592
             472  356 - #9b7f6a
              96  388 - #00daff
              52  396 - #00daff
             284  396 - #ebc4c9
             388  396 - #e1b3b5
             476  404 - #d19f9e
             448  432 - #c39693
              96  436 - #00daff
              52  440 - #00daff
             440  472 - #a68870
             312  476 - #e5c3ba
             376  476 - #d2ac99
              52  488 - #00daff
             592  592 - #597c95
        ",
    )
}

impl ViewTest for LabelImage {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        check_initial_images()?;
        check_label_resizing_image(view)?;
        check_cells_resizing_image(view)?;

        Ok(())
    }
}
