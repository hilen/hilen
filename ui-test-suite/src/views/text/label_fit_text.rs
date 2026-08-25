use anyhow::Result;
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{GREEN, Label, Setup, TextAlignment, ViewData, ViewFrame, ViewTest, YELLOW, view},
    ui_test::check_colors,
};

#[view]
struct LabelFitText {
    #[init]
    tag:      Label,
    panel:    Label,
    centered: Label,
}

impl Setup for LabelFitText {
    fn setup(self: Weak<Self>) {
        self.tag.set_color(GREEN);
        self.tag.set_alignment(TextAlignment::Left);
        self.tag.set_text("tag").set_text_size(40);
        self.tag.place().tl(20).fit_text();

        self.panel.set_color(YELLOW);
        self.panel.set_multiline(true);
        self.panel
            .set_text("Grumpy wizards make toxic brew for the jovial queen")
            .set_text_size(40);
        self.panel.place().t(120).lr(20).fit_text_height();

        self.centered.set_color(GREEN);
        self.centered.set_text("mid").set_text_size(40);
        self.centered.place().t(400).center_x().fit_text();
    }
}

impl ViewTest for LabelFitText {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        fitted_text_colors()?;

        let (tag, panel, centered) =
            from_main(move || (*view.tag.frame(), *view.panel.frame(), *view.centered.frame()));

        assert!(
            (tag.origin.x - 20.0).abs() < f32::EPSILON && (tag.origin.y - 20.0).abs() < f32::EPSILON,
            "fit_text moved the anchored origin: {tag:?}"
        );
        assert!(
            tag.size.width < 200.0 && tag.size.height < 70.0,
            "fitted frame does not hug the text: {tag:?}"
        );
        assert!(
            (panel.size.width - 560.0).abs() < f32::EPSILON,
            "fit_text_height changed the width set by side rules: {panel:?}"
        );
        assert!(
            (centered.center().x - 300.0).abs() < 1.0,
            "fitted label is not centered: {centered:?}"
        );

        grown_text_colors(view)?;

        let (grown_tag, grown_panel, grown_centered) =
            from_main(move || (*view.tag.frame(), *view.panel.frame(), *view.centered.frame()));

        assert!(
            grown_tag.size.width > tag.size.width + 50.0,
            "fitted width did not follow longer text: {grown_tag:?}"
        );
        assert!(
            grown_panel.size.height > panel.size.height,
            "fitted height did not grow with more wrapped text: {grown_panel:?}"
        );
        assert!(
            grown_centered.size.width > centered.size.width,
            "centered fitted width did not grow: {grown_centered:?}"
        );
        assert!(
            (grown_centered.center().x - 300.0).abs() < 1.0,
            "label did not stay centered after refit: {grown_centered:?}"
        );

        Ok(())
    }
}

fn fitted_text_colors() -> Result<()> {
    check_colors(
        r"
              52   44 - #00ff00
              64   48 - #004d00
              20   64 - #00ff00
              88   64 - #00ff00
             200  120 - #ffff00
             120  140 - #7b7b00
             240  140 - #545400
             276  144 - #4c4c00
             380  144 - #5d5d00
             436  144 - #9c9c00
             440  144 - #9c9c00
             512  144 - #ffff00
              88  148 - #7a7a00
              88  152 - #7a7a00
             148  152 - #000000
             240  152 - #545400
             312  184 - #d2d200
             360  184 - #353500
              88  192 - #343400
             416  196 - #ffff00
             184  200 - #000000
             280  200 - #000000
             360  200 - #353500
             480  200 - #010100
             312  204 - #d2d200
             576  212 - #ffff00
             332  404 - #00ff00
             304  424 - #007600
             272  432 - #005600
             304  432 - #007600
               4  592 - #597c95
             592  592 - #597c95
            ",
    )
}

fn grown_text_colors(view: Weak<LabelFitText>) -> Result<()> {
    from_main(move || {
        view.tag.set_text("much longer tag");
        view.panel
            .set_text("Grumpy wizards make toxic brew for the jovial queen and jack, then brew even more");
        view.centered.set_text("wide middle");
    });

    wait_for_next_frame();

    check_colors(
        r"
              88   36 - #000000
             236   44 - #009c00
             316   48 - #00ff00
             120   52 - #009100
              20   64 - #00ff00
             164  120 - #ffff00
             576  120 - #ffff00
             332  136 - #000000
             240  140 - #545400
              64  144 - #ffff00
             440  144 - #9c9c00
              88  148 - #7a7a00
              88  152 - #7a7a00
             408  188 - #8b8b00
             476  192 - #4b4b00
             344  196 - #000000
             548  196 - #ffff00
             184  200 - #d4d400
             252  220 - #dcdc00
             124  248 - #797900
             188  248 - #9d9d00
             192  248 - #c1c100
             312  248 - #000000
             444  248 - #646400
              24  260 - #8da567
             516  260 - #8da567
             252  408 - #00ad00
             324  432 - #006f00
             196  436 - #00ff00
             404  444 - #00ff00
               4  592 - #597c95
             592  592 - #597c95
            ",
    )
}
