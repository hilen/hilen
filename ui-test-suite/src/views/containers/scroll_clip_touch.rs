use anyhow::{Result, ensure};
use hilen::{
    refs::Weak,
    ui::{
        Button, Color, Container, Label, ScrollView, Setup, Shadow, TextAlignment, ViewData, ViewSubviews,
        ViewTest, WHITE, view,
    },
    ui_test::{check_colors, inject_touches},
};

/// A `ScrollView` with a button straddling its bottom edge, half in
/// view and half clipped. The clipped half is not drawn, so a tap on
/// it must not reach the button, while a tap on the visible half must.
#[view]
struct ScrollClipTouch {
    taps: u32,

    #[init]
    scroll: ScrollView,
    result: Label,
}

impl Setup for ScrollClipTouch {
    fn setup(mut self: Weak<Self>) {
        self.scroll
            .set_color(Color::rgb(0.92, 0.94, 0.97))
            .set_corner_radius(16)
            .set_border_width(1)
            .set_border_color(Color::rgb(0.80, 0.84, 0.90));
        self.scroll.place().tl(20).size(300, 200);

        // The button spans 160..240 in the content, so the scroll's
        // bottom edge at 200 cuts it in half.
        let button = self.scroll.add_view::<Button>();
        button
            .set_text("Tap me")
            .set_text_color(WHITE)
            .set_text_size(20)
            .set_corner_radius(14)
            .set_shadow(Shadow::default());
        button.set_gradient(Color::rgb(0.13, 0.80, 0.98), Color::rgb(0.55, 0.36, 0.96));
        button.place().l(60).t(160).size(180, 80);
        button.on_tap(move || {
            self.taps += 1;
            self.result.set_text(format!("Taps: {}", self.taps));
        });

        // Enough content under the button so the scroll has something
        // to clip against, not just the button itself.
        let filler = self.scroll.add_view::<Container>();
        filler.place().t(300).l(0).size(1, 1);

        self.result
            .set_text("Taps: 0")
            .set_text_size(20)
            .set_alignment(TextAlignment::Left);
        self.result.place().t(300).l(20).size(200, 40);
    }
}

impl ViewTest for ScrollClipTouch {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        // Ten taps across the clipped half, 20 points below the edge.
        for x in (80..=220).step_by(15) {
            inject_touches(format!("{x} 240 b\n{x} 240 e"));
        }
        ensure!(view.taps == 0, "the clipped half of the button took a tap");

        // Ten taps inside the scroll view hugging the button, which sits
        // at 80..260 by 180..260 on screen. A row 20 points above it and
        // two on each visible flank.
        for x in (90..=240).step_by(30) {
            inject_touches(format!("{x} 160 b\n{x} 160 e"));
        }
        for (x, y) in [(60, 190), (60, 210), (280, 190), (280, 210)] {
            inject_touches(format!("{x} {y} b\n{x} {y} e"));
        }
        ensure!(view.taps == 0, "empty scroll space took a tap");

        // Ten taps across the visible half, 20 points above the edge.
        for x in (80..=220).step_by(15) {
            inject_touches(format!("{x} 200 b\n{x} 200 e"));
        }
        ensure!(
            view.taps == 10,
            "the visible half took {} taps, not 10",
            view.taps
        );

        check_colors(
            r"
              36   20 - #ccd6e5
             228   20 - #ccd6e5
             308   24 - #ebf0f7
             316   68 - #999ca1
             172   72 - #ebf0f7
             316  116 - #999ca1
              20  120 - #ccd6e6
             104  124 - #ebf0f7
             116  180 - #22cbfa
             196  180 - #22cbfa
             256  188 - #2dc0f9
             260  200 - #b4b8bd
              80  204 - #42aaf8
             140  212 - #91c3fb
             180  212 - #4d9ef8
             144  216 - #5299f8
             216  216 - #5299f8
             260  216 - #b4b8bd
             592  272 - #597c95
              44  312 - #374d5c
              96  316 - #49657a
             104  316 - #597c95
              48  320 - #597c95
              56  320 - #445e72
              60  320 - #000000
              64  320 - #597c95
              96  320 - #49657a
              60  324 - #000000
              96  324 - #49657a
              60  328 - #0c1014
             272  592 - #597c95
             592  592 - #597c95
            ",
        )?;

        Ok(())
    }
}
