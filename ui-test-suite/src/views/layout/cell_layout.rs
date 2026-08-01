use test_engine::{
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
             424  104 - #ffffff
             404  108 - #1e608b
             416  108 - #1e608b
             436  108 - #1e3f6b
             428  112 - #1e3f6b
             436  112 - #1e3f6b
             444  112 - #1e3f6b
             404  120 - #ffffff
             416  120 - #50c5ff
             432  120 - #4889ff
             256  124 - #ffffff
             404  124 - #1e608b
             444  124 - #1e3f6b
             428  128 - #4889ff
             428  140 - #4889ff
             444  140 - #ffffff
             412  144 - #50c5ff
             432  144 - #4889ff
             432  148 - #4889ff
              52  228 - #ffffff
             252  272 - #ffffff
              24  412 - #00ff00
             120  412 - #00ff00
             184  412 - #00ff00
             196  472 - #ffff00
             108  484 - #ffff00
              36  504 - #ffff00
             116  556 - #00ffff
             208  556 - #00ffff
               4  592 - #597c95
             592  592 - #597c95
        ",
    )?;

    check_colors(
        r"
             172    4 - #597c95
             424  104 - #ffffff
             404  108 - #1e608b
             416  108 - #1e608b
             436  108 - #1e3f6b
             428  112 - #1e3f6b
             436  112 - #1e3f6b
             444  112 - #1e3f6b
             404  120 - #ffffff
             416  120 - #50c5ff
             432  120 - #4889ff
             256  124 - #ffffff
             404  124 - #1e608b
             444  124 - #1e3f6b
             428  128 - #4889ff
             428  140 - #4889ff
             444  140 - #ffffff
             412  144 - #50c5ff
             432  144 - #4889ff
             432  148 - #4889ff
              52  228 - #ffffff
             252  272 - #ffffff
              24  412 - #00ff00
             120  412 - #00ff00
             184  412 - #00ff00
             196  472 - #ffff00
             108  484 - #ffff00
              36  504 - #ffff00
             116  556 - #00ffff
             208  556 - #00ffff
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
             424  104 - #ffffff
             404  108 - #1e608b
             416  108 - #1e608b
             436  108 - #1e3f6b
             428  112 - #1e3f6b
             436  112 - #1e3f6b
             444  112 - #1e3f6b
             404  120 - #ffffff
             416  120 - #50c5ff
             432  120 - #4889ff
             256  124 - #ffffff
             404  124 - #1e608b
             444  124 - #1e3f6b
             428  128 - #4889ff
             428  140 - #4889ff
             444  140 - #ffffff
             412  144 - #50c5ff
             432  144 - #4889ff
             432  148 - #4889ff
              52  228 - #ffffff
             252  272 - #ffffff
              24  412 - #00ff00
             120  412 - #00ff00
             184  412 - #00ff00
             196  472 - #ffff00
             108  484 - #ffff00
              36  504 - #ffff00
             116  556 - #00ffff
             208  556 - #00ffff
               4  592 - #597c95
             592  592 - #597c95
        ",
    )?;

    Ok(())
}
