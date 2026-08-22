use anyhow::Result;
use hilen::{
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
              52   52 - #ebc8ce
             268   52 - #e0b8b9
             180   76 - #b18071
             412  108 - #9b6b57
             356  120 - #936b57
             204  124 - #4a3520
             196  128 - #5a4f35
             296  136 - #291608
             300  136 - #271306
             240  152 - #7f452b
              72  176 - #010101
             148  180 - #000000
             288  180 - #010101
             364  192 - #7c604a
             384  208 - #a1826d
             216  248 - #cc9d8b
             456  280 - #dcaeb0
             592  288 - #597c95
              92  300 - #d39d9d
              56  320 - #f4d9d0
             380  324 - #ffffff
             328  332 - #e6c2ac
             424  376 - #977961
              76  380 - #ffffff
             328  424 - #efd6cd
             404  428 - #4d391d
              68  436 - #d8b2a5
             372  440 - #ce9f8e
             416  468 - #b1917c
             280  476 - #dda1a3
              72  480 - #ffffff
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
             592    4 - #597c95
             172   52 - #00389f
             348   56 - #070825
              56   84 - #01359d
             440   92 - #041444
             156  120 - #6a728e
             272  120 - #6a728f
             228  180 - #fefefe
             152  184 - #ffffff
             308  184 - #333e63
             444  196 - #002971
              56  216 - #013698
             280  284 - #edc8cf
             456  312 - #a36856
              56  320 - #f4d9d0
             340  324 - #ffffff
             404  328 - #4d391d
             436  356 - #826656
              96  360 - #cb9998
             280  372 - #e0a4a6
              60  400 - #e4bcbd
             476  400 - #ce9f9f
             384  420 - #aa8b6c
              76  428 - #ebdfd6
             404  428 - #4d391d
             328  432 - #e6c2ac
              96  436 - #b5977f
             344  436 - #e1bcb2
             416  468 - #b1917c
             476  476 - #b69a84
              64  480 - #dbbca8
             220  592 - #597c95
        ",
    )?;

    check_colors(
        r"
             592    4 - #597c95
             172   52 - #00389f
             348   56 - #070825
              56   84 - #01359d
             440   92 - #041444
             156  120 - #6a728e
             272  120 - #6a728f
             228  180 - #fefefe
             152  184 - #ffffff
             308  184 - #333e63
             444  196 - #002971
              56  216 - #013698
             280  284 - #edc8cf
             456  312 - #a36856
              56  320 - #f4d9d0
             340  324 - #ffffff
             404  328 - #4d391d
             436  356 - #826656
              96  360 - #cb9998
             280  372 - #e0a4a6
              60  400 - #e4bcbd
             476  400 - #ce9f9f
             384  420 - #aa8b6c
              76  428 - #ebdfd6
             404  428 - #4d391d
             328  432 - #e6c2ac
              96  436 - #b5977f
             344  436 - #e1bcb2
             416  468 - #b1917c
             476  476 - #b69a84
              64  480 - #dbbca8
             220  592 - #597c95
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
             592    4 - #597c95
             172   52 - #00389f
              56   84 - #01359d
             440   92 - #041444
             272  120 - #6a728f
             232  180 - #fefefe
             156  184 - #ffffff
             308  184 - #333e63
             444  196 - #002971
              56  212 - #00399f
             280  284 - #edc8cf
              76  292 - #050f31
             456  312 - #a36856
              68  316 - #4e5777
             340  324 - #ffffff
             404  328 - #4d391d
             360  340 - #dfb6af
             424  376 - #977961
              52  380 - #00daff
             476  400 - #ce9f9f
             408  408 - #c2a691
             328  420 - #f2dfd8
              72  428 - #a0a5b6
              80  428 - #a0a5b6
              80  432 - #1a2750
             388  432 - #af8d71
             344  440 - #d4a998
             360  440 - #dfb6af
             416  468 - #b1917c
             476  476 - #b69a84
              96  484 - #00daff
              52  488 - #00daff
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
