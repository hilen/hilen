use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{
        Color, DynamicColor, Label, SegmentedControl, SegmentedStyle, Setup, TextAlignment, Theme, ThemeMode,
        ViewData, ViewFrame, ViewTest, view,
    },
    ui_test::{check_colors, checkpoint, inject_touches},
};

/// A segmented control under a global `SegmentedStyle` of an app: its own
/// track, picked segment and picked text, in the light and the dark theme.
#[view]
struct SegmentedStyleTest {
    #[init]
    status:   Label,
    caption:  Label,
    segments: SegmentedControl,
}

impl Setup for SegmentedStyleTest {
    fn setup(self: Weak<Self>) {
        self.set_color(pair("#F4F6FC", "#0E1222"));
        self.status
            .set_text("Quarters picked by code")
            .set_text_size(18)
            .set_alignment(TextAlignment::Left);
        self.status.set_text_color(pair("#1C1C1E", "#F2F2F4"));
        self.status.place().t(10).lr(10).h(30);

        self.caption
            .set_text("segmented control with the style of the app")
            .set_text_size(14)
            .set_alignment(TextAlignment::Left);
        self.caption.set_text_color(pair("#55596A", "#9AA0B4"));
        self.caption.place().t(60).lr(20).h(24);

        self.segments.set_texts(["Weeks", "Months", "Quarters"]);
        self.segments.set_picked(2);
        self.segments.place().t(100).l(20).w(360);
    }
}

impl ViewTest for SegmentedStyleTest {
    fn before_start() {
        from_main(|| {
            SegmentedStyle {
                track:       pair("#DCE3F5", "#1B2340").into(),
                picked:      pair("#3D5AFE", "#5B73FF").into(),
                picked_text: pair("#FFFFFF", "#FFFFFF").into(),
            }
            .apply_globally();
        });
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        settle();
        check_colors(STYLED)?;
        checkpoint("the track, the picked Quarters and its text in the colors of the style")?;

        let segment = from_main(move || *view.segments.frame());
        let (x, y) = (
            (segment.origin.x + 60.0).round(),
            (segment.origin.y + 16.0).round(),
        );
        inject_touches(format!("{x} {y} b\n{x} {y} e"));
        settle();
        from_main(move || {
            ensure!(
                view.segments.picked_text() == "Weeks",
                "{}",
                view.segments.picked_text()
            );
            view.status.set_text("Weeks picked by a tap");
            Ok(())
        })?;
        settle();
        check_colors(TAPPED)?;
        checkpoint("Weeks picked by a tap, in the picked color of the style")?;

        from_main(|| Theme::set_mode(ThemeMode::Dark));
        settle();
        check_colors(DARK)?;
        checkpoint("the same in the dark theme, with the dark colors of the style")?;
        from_main(|| Theme::set_mode(ThemeMode::System));

        Ok(())
    }
}

fn pair(light: &str, dark: &str) -> DynamicColor {
    DynamicColor::new(Color::hex(light), Color::hex(dark))
}

fn settle() {
    for _ in 0..3 {
        wait_for_next_frame();
    }
}

const STYLED: &str = r"
              36   24 - #1d1d1f
             112   24 - #1c1c1e
             204   24 - #f4f6fc
              60   28 - #242426
             104   68 - #757988
             176   68 - #55596a
             192   68 - #616575
             264   68 - #55596a
              56   72 - #55596a
              64   72 - #55596a
             168   72 - #565a6b
             188   72 - #797c8b
             252   72 - #6d7180
             300   72 - #f4f6fc
             276  104 - #3d5afe
             352  104 - #3d5afe
             376  112 - #3d5afe
              88  116 - #1c1c1e
             296  116 - #3d5afe
              84  120 - #dce3f5
              88  120 - #1c1c1e
             192  120 - #dce3f5
             324  120 - #ffffff
             332  120 - #3d5afe
             336  120 - #b7c2ff
             264  124 - #3d5afe
             360  132 - #3d5afe
             592  304 - #f4f6fc
               4  348 - #f4f6fc
             300  448 - #f4f6fc
               4  592 - #f4f6fc
             592  592 - #f4f6fc
";

const TAPPED: &str = r"
             588    4 - #f4f6fc
              56   24 - #f4f6fc
             180   24 - #1c1c1e
              88   28 - #f4f6fc
             180   28 - #1c1c1e
             104   68 - #757988
             176   68 - #55596a
             192   68 - #616575
             264   68 - #55596a
              56   72 - #55596a
              64   72 - #55596a
             136   72 - #f4f6fc
             168   72 - #565a6b
             188   72 - #797c8b
             252   72 - #6d7180
             300   72 - #f4f6fc
              48  104 - #3d5afe
             112  104 - #3d5afe
              76  112 - #3d5afe
             324  116 - #1c1c1e
              76  120 - #3d5afe
              84  120 - #3d5afe
             192  120 - #dce3f5
             324  120 - #1c1c1e
             332  120 - #dce3f5
              24  124 - #3d5afe
             136  124 - #3d5afe
             108  132 - #3d5afe
             592  284 - #f4f6fc
             340  416 - #f4f6fc
             116  592 - #f4f6fc
             592  592 - #f4f6fc
";

const DARK: &str = r"
             588    4 - #0e1222
              56   24 - #0e1222
             180   24 - #f2f2f4
              88   28 - #0e1222
             104   68 - #7d8396
             176   68 - #9aa0b4
             192   68 - #9095a9
             264   68 - #9aa0b4
              56   72 - #9aa0b4
              64   72 - #9aa0b4
             168   72 - #999fb3
             188   72 - #7b8093
             252   72 - #858a9e
             300   72 - #0e1222
              48  104 - #5b73ff
             116  104 - #5b73ff
              76  112 - #5b73ff
              88  116 - #ffffff
             324  116 - #f2f2f4
              76  120 - #5b73ff
              84  120 - #5b73ff
              88  120 - #ffffff
             192  120 - #1b2340
             324  120 - #f2f2f4
             332  120 - #1b2340
              24  124 - #5b73ff
             136  124 - #5b73ff
              52  132 - #5b73ff
             592  284 - #0e1222
             340  416 - #0e1222
             116  592 - #0e1222
             592  592 - #0e1222
";
