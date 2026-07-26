use anyhow::Result;
use test_engine::{
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
              24   24 - #00ff00
              44   40 - #00ff00
              64   40 - #000000
              56   44 - #00ff00
              80   44 - #00ff00
              24   56 - #00ff00
             360  124 - #ffff00
             576  124 - #ffff00
             116  132 - #000000
             260  132 - #000000
             292  136 - #000000
             488  136 - #ffff00
             428  140 - #ffff00
             528  140 - #000000
              24  144 - #ffff00
              80  144 - #010100
             216  148 - #010100
             316  180 - #000000
             128  184 - #ffff00
             284  184 - #ffff00
             352  188 - #ffff00
             424  196 - #010100
             576  196 - #ffff00
             592  360 - #597c95
               4  368 - #597c95
             272  404 - #00ff00
             316  404 - #00ff00
             320  424 - #00ff00
             288  436 - #00ff00
               4  592 - #597c95
             404  592 - #597c95
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
             220   24 - #00ff00
             100   36 - #00ff00
             168   40 - #00ff00
             272   44 - #000000
              24   56 - #00ff00
             384  124 - #ffff00
              64  136 - #ffff00
             292  136 - #000000
             528  140 - #000000
             116  148 - #000000
             228  148 - #010100
             164  176 - #ffff00
             424  176 - #ffff00
             516  180 - #010100
             552  184 - #010100
             232  220 - #000000
             472  220 - #000000
             104  224 - #ffff00
             308  228 - #ffff00
              24  236 - #ffff00
             392  236 - #ffff00
             576  236 - #ffff00
             592  380 - #597c95
             240  404 - #00ff00
             292  404 - #00ff00
               4  412 - #597c95
             396  416 - #00ff00
             344  420 - #00ff00
             196  436 - #00ff00
               4  592 - #597c95
             288  592 - #597c95
             592  592 - #597c95
            ",
    )
}
