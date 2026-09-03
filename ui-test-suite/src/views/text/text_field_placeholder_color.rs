use anyhow::Result;
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{Color, DynamicColor, Setup, TextField, Theme, ThemeMode, ViewData, ViewTest, view},
    ui_test::check_colors,
};

const FIELD_BG: DynamicColor = DynamicColor::new(Color::hex("#f0f2f5"), Color::hex("#31353c"));
const FIELD_TEXT: DynamicColor = DynamicColor::new(Color::hex("#17191d"), Color::hex("#f2f4f7"));
const HINT: DynamicColor = DynamicColor::new(Color::hex("#8a8f98"), Color::hex("#6f747d"));

/// The empty field before typing and after clearing, the hint in its own
/// muted light color.
const HINT_LIGHT_PROBES: &str = r"
             556    4 - #597c95
              72   20 - #f0f2f5
             252   20 - #f0f2f5
             316   24 - #f0f2f5
             204   32 - #8a8f98
             128   36 - #f0f2f5
             148   36 - #8a8f98
             204   36 - #8a8f98
             164   40 - #f0f2f5
             176   40 - #b6bac0
             192   40 - #f0f2f5
             204   40 - #8a8f98
             176   44 - #b6bac0
             196   44 - #f0f2f5
             204   44 - #8a8f98
             124   48 - #8a8f98
             136   48 - #8a8f98
             176   48 - #b6bac0
             192   48 - #f0f2f5
             204   48 - #8a8f98
             216   48 - #8a8f98
              20   52 - #f0f2f5
             148   52 - #8a8f98
             164   52 - #8a8f98
             200   52 - #f0f2f5
              84   60 - #f0f2f5
             272   60 - #f0f2f5
             592  240 - #597c95
               4  324 - #597c95
             288  368 - #597c95
              48  592 - #597c95
             592  592 - #597c95
";

/// Entered text in the entered-text color, not the hint color.
const TEXT_PROBES: &str = r"
             540    4 - #597c95
              84   20 - #f0f2f5
             236   20 - #f0f2f5
             316   24 - #f0f2f5
             148   32 - #626468
             152   32 - #626468
             160   40 - #f0f2f5
             176   40 - #17191d
             192   40 - #17191d
              20   44 - #f0f2f5
             164   44 - #17191d
             176   44 - #17191d
             184   44 - #f0f2f5
             188   44 - #f0f2f5
             192   44 - #1e2024
             152   48 - #f0f2f5
             176   48 - #17191d
             188   48 - #f0f2f5
             192   48 - #17191d
             148   52 - #25272b
             156   52 - #25272b
             164   52 - #f0f2f5
             180   52 - #f0f2f5
              60   60 - #f0f2f5
             104   60 - #f0f2f5
             260   60 - #f0f2f5
             592  220 - #597c95
              32  320 - #597c95
             300  388 - #597c95
               4  592 - #597c95
             224  592 - #597c95
             592  592 - #597c95
";

/// The empty field after a theme switch, the hint pair re-resolved dark.
const HINT_DARK_PROBES: &str = r"
             556    4 - #597c95
              72   20 - #31353c
             256   20 - #31353c
             316   24 - #31353c
             128   36 - #31353c
             148   36 - #6f747d
             212   36 - #6f747d
             164   40 - #31353c
             176   40 - #545961
             188   40 - #6f747d
             152   44 - #6f747d
             176   44 - #545961
             192   44 - #31353c
             196   44 - #31353c
             124   48 - #6f747d
             136   48 - #6f747d
             164   48 - #31353c
             176   48 - #545961
             192   48 - #31353c
             216   48 - #6f747d
              20   52 - #31353c
             148   52 - #6f747d
             200   52 - #31353c
              92   60 - #31353c
             272   60 - #31353c
             312   60 - #31353c
             592  240 - #597c95
               4  324 - #597c95
             288  368 - #597c95
              48  592 - #597c95
             356  592 - #597c95
             592  592 - #597c95
";

/// The placeholder holds its own muted color while the field is empty,
/// entered text swaps the label to the text color and clearing swaps it
/// back. Without `set_placeholder_color` the hint used to render in the
/// entered-text color, so a ported palette input read wrong next to the
/// browser one.
#[view]
struct TextFieldPlaceholderColor {
    #[init]
    field: TextField,
}

impl Setup for TextFieldPlaceholderColor {
    fn setup(self: Weak<Self>) {
        self.field.set_color(FIELD_BG).set_corner_radius(10);
        self.field.set_text_color(FIELD_TEXT);
        self.field.set_placeholder_color(HINT);
        self.field.set_placeholder("Search");
        self.field.set_text_size(32);
        self.field.place().tl(20).size(300, 44);
    }
}

impl ViewTest for TextFieldPlaceholderColor {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        // In human mode the OS theme may be dark. Start from a known state.
        from_main(|| {
            Theme::set_mode(ThemeMode::System);
            Theme::set_system(Theme::Light);
        });

        wait_for_next_frame();

        from_main(move || {
            assert!(view.field.is_placeholding());
        });

        check_colors(HINT_LIGHT_PROBES)?;

        from_main(move || {
            view.field.set_text("Zug");
        });

        wait_for_next_frame();

        from_main(move || {
            assert!(!view.field.is_placeholding());
        });

        check_colors(TEXT_PROBES)?;

        from_main(move || {
            view.field.clear();
        });

        wait_for_next_frame();

        from_main(move || {
            assert!(view.field.is_placeholding());
        });

        check_colors(HINT_LIGHT_PROBES)?;

        from_main(|| Theme::set_system(Theme::Dark));

        wait_for_next_frame();

        check_colors(HINT_DARK_PROBES)?;

        // Leave the default state for the tests that follow.
        from_main(|| Theme::set_system(Theme::Light));

        Ok(())
    }
}
