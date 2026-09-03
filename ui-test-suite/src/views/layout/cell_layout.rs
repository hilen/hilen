use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{
        Anchor::{Right, Top},
        Button, Container, GREEN, Label, Setup, TURQUOISE, UIImages, ViewData, ViewSubviews, ViewTest, WHITE,
        YELLOW, view,
    },
    ui_test::check_colors,
};

#[view]
struct CellLayout {
    title: Weak<Container>,
    table: Weak<Container>,

    #[init]
    delete: Button,
    label:  Label,

    container: Container,
}

impl Setup for CellLayout {
    fn setup(mut self: Weak<Self>) {
        self.delete.place().t(100).l(400).size(50, 50);
        self.delete.set_image(UIImages::delete());

        self.label
            .set_color(WHITE)
            .place()
            .l(50)
            .t(100)
            .h(200)
            .anchor(Right, self.delete, 10);

        self.container.set_color(TURQUOISE);
        self.container.place().t(400).l(10).size(200, 160);

        self.title = self.container.add_view();
        self.table = self.container.add_view();

        self.title.place().lrt(10).h(50);
        self.title.set_color(GREEN);

        self.table.place().anchor(Top, self.title, 10).lrb(10);
        self.table.set_color(YELLOW);
    }
}

impl ViewTest for CellLayout {
    fn perform_test(view: Weak<Self>) -> anyhow::Result<()> {
        check_initial_layout()?;
        check_reapplied_placement(view)?;

        Ok(())
    }
}

fn check_initial_layout() -> anyhow::Result<()> {
    check_colors(
        r"
             172    4 - #597c95
             408  100 - #ffffff
             408  108 - #1f618c
             420  108 - #1f618c
             436  108 - #1f406c
             428  112 - #1f406d
             440  112 - #1f406d
             408  116 - #52c6ff
             432  120 - #4787fc
             420  124 - #45aee5
             440  124 - #4788fc
             444  124 - #1e3f6b
             256  128 - #ffffff
             424  132 - #20618d
             432  132 - #254b83
             420  136 - #45aee5
             444  140 - #ffffff
             408  144 - #ffffff
             428  144 - #4889ff
             420  148 - #50c5ff
             432  148 - #4889ff
             436  148 - #4889ff
              52  228 - #ffffff
             260  280 - #ffffff
              12  408 - #00ffff
             176  412 - #00ff00
             112  416 - #00ff00
             196  472 - #ffff00
              16  500 - #00ffff
             108  544 - #ffff00
               4  592 - #597c95
             592  592 - #597c95
        ",
    )?;

    check_colors(
        r"
             172    4 - #597c95
             408  100 - #ffffff
             408  108 - #1f618c
             420  108 - #1f618c
             436  108 - #1f406c
             428  112 - #1f406d
             440  112 - #1f406d
             408  116 - #52c6ff
             432  120 - #4787fc
             420  124 - #45aee5
             440  124 - #4788fc
             444  124 - #1e3f6b
             256  128 - #ffffff
             424  132 - #20618d
             432  132 - #254b83
             420  136 - #45aee5
             444  140 - #ffffff
             408  144 - #ffffff
             428  144 - #4889ff
             420  148 - #50c5ff
             432  148 - #4889ff
             436  148 - #4889ff
              52  228 - #ffffff
             260  280 - #ffffff
              12  408 - #00ffff
             176  412 - #00ff00
             112  416 - #00ff00
             196  472 - #ffff00
              16  500 - #00ffff
             108  544 - #ffff00
               4  592 - #597c95
             592  592 - #597c95
        ",
    )?;

    Ok(())
}

fn check_reapplied_placement(view: Weak<CellLayout>) -> anyhow::Result<()> {
    from_main(move || {
        view.title.place().clear().h(50).lrt(10);

        view.table.place().clear().lrb(10).anchor(Top, view.title, 10);
    });

    check_colors(
        r"
             172    4 - #597c95
             408  100 - #ffffff
             408  108 - #1f618c
             420  108 - #1f618c
             436  108 - #1f406c
             428  112 - #1f406d
             440  112 - #1f406d
             408  116 - #52c6ff
             432  120 - #4787fc
             420  124 - #45aee5
             440  124 - #4788fc
             444  124 - #1e3f6b
             256  128 - #ffffff
             424  132 - #20618d
             432  132 - #254b83
             420  136 - #45aee5
             444  140 - #ffffff
             408  144 - #ffffff
             428  144 - #4889ff
             420  148 - #50c5ff
             432  148 - #4889ff
             436  148 - #4889ff
              52  228 - #ffffff
             260  280 - #ffffff
              12  408 - #00ffff
             176  412 - #00ff00
             112  416 - #00ff00
             196  472 - #ffff00
              16  500 - #00ffff
             108  544 - #ffff00
               4  592 - #597c95
             592  592 - #597c95
        ",
    )?;

    Ok(())
}
