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
             592    4 - #597c95
             156  104 - #e2b5a2
             272  108 - #bca087
             416  108 - #9a6b57
              80  112 - #010101
             212  124 - #452c1e
             196  128 - #685c3f
             360  132 - #96725c
             384  132 - #945f48
             304  136 - #281205
             244  156 - #542f1c
             168  180 - #000000
             112  184 - #010101
             348  192 - #c4a48f
             440  196 - #9c806b
             176  236 - #deb9ae
             344  236 - #826553
             476  280 - #d8aaab
              88  296 - #ddaeae
             340  300 - #cb856f
             408  324 - #2e2114
             372  344 - #ffffff
             428  344 - #ffffff
              88  388 - #9a7b64
             368  400 - #c29d8b
             328  424 - #f0d8cf
             360  424 - #755a48
             412  424 - #352415
              76  428 - #ecdfd7
             380  432 - #4e2d1a
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
             264   52 - #003aa1
             416   56 - #0038a1
              56   84 - #01359d
             444  112 - #01266e
             156  120 - #6a728e
             272  120 - #6a728f
             308  132 - #323d63
             304  172 - #041241
             420  172 - #fefefe
              88  180 - #fefefe
              88  296 - #ddaeae
             448  312 - #ac816e
             320  320 - #f4e1d9
             340  324 - #ffffff
             360  324 - #755a48
             388  324 - #bca48d
              92  328 - #a3856f
             436  356 - #836a55
             280  368 - #d3a69f
             404  368 - #c7a792
              96  376 - #a0846e
              60  384 - #e9c5bd
              84  424 - #c8a893
             328  424 - #f0d8cf
             360  424 - #755a48
             412  424 - #352415
             444  424 - #a37e66
             380  432 - #4e2d1a
              72  472 - #caa28c
             424  476 - #9c7f67
             228  592 - #597c95
             592  592 - #597c95
        ",
    )?;

    check_colors(
        r"
             264   52 - #003aa1
             416   56 - #0038a1
              56   84 - #01359d
             444  112 - #01266e
             156  120 - #6a728e
             272  120 - #6a728f
             308  132 - #323d63
             304  172 - #041241
             420  172 - #fefefe
              88  180 - #fefefe
              88  296 - #ddaeae
             448  312 - #ac816e
             320  320 - #f4e1d9
             340  324 - #ffffff
             360  324 - #755a48
             388  324 - #bca48d
              92  328 - #a3856f
             436  356 - #836a55
             280  368 - #d3a69f
             404  368 - #c7a792
              96  376 - #a0846e
              60  384 - #e9c5bd
              84  424 - #c8a893
             328  424 - #f0d8cf
             360  424 - #755a48
             412  424 - #352415
             444  424 - #a37e66
             380  432 - #4e2d1a
              72  472 - #caa28c
             424  476 - #9c7f67
             228  592 - #597c95
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
             592    4 - #597c95
             172   52 - #00389f
              56   84 - #01359d
             440   92 - #041444
             272  120 - #6a728f
             228  180 - #fefefe
             152  184 - #ffffff
             308  184 - #333e63
             444  196 - #002971
              56  212 - #00399f
             280  284 - #e6bec4
              68  312 - #4f5878
              80  312 - #04113f
             448  312 - #ac816e
             340  324 - #ffffff
             360  324 - #755a48
             408  324 - #2e2114
             348  336 - #e6c2b8
             476  360 - #9e7f6a
              52  384 - #00daff
             440  412 - #b8947e
             356  416 - #8b6d5c
             320  420 - #f4e1d9
              72  428 - #a0a5b6
              80  428 - #a0a5b6
             388  436 - #b38d70
             344  440 - #d6af9c
             436  456 - #836a55
             280  476 - #d2a19b
             476  476 - #ba9e88
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
