use anyhow::Result;
use chrono::{NaiveDate, NaiveTime};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{
        Color, DateField, DateFieldStyle, DynamicColor, Label, Setup, TextAlignment, Theme, ThemeMode,
        TimeField, ViewData, ViewTest, view,
    },
    ui_test::{check_colors, checkpoint},
};

/// A date field with its calendar open and a time field, under a global
/// `DateFieldStyle` of an app, in the light and the dark theme.
#[view]
struct DateFieldStyleTest {
    #[init]
    status: Label,
    date:   DateField,
    time:   TimeField,
}

impl Setup for DateFieldStyleTest {
    fn setup(self: Weak<Self>) {
        self.set_color(pair("#FAF7F2", "#16120D"));
        self.status
            .set_text("date and time fields with the style of the app")
            .set_text_size(18)
            .set_alignment(TextAlignment::Left);
        self.status.set_text_color(pair("#3B2A14", "#F2E6D6"));
        self.status.place().t(10).lr(10).h(30);
        self.date.place().t(60).l(20).w(300);
        self.date.set_date(NaiveDate::from_ymd_opt(2026, 11, 18).unwrap());
        self.time.place().t(60).l(340).w(100);
        self.time.set_time(NaiveTime::from_hms_opt(9, 30, 0).unwrap());
    }
}

impl ViewTest for DateFieldStyleTest {
    fn before_start() {
        from_main(|| {
            DateFieldStyle {
                fill:        pair("#FFF4E6", "#2B1D0E").into(),
                border:      pair("#F08C00", "#FFA94D").into(),
                bad_border:  pair("#E03131", "#FF6B6B").into(),
                text:        pair("#5C2B00", "#FFE8CC").into(),
                faint:       pair("#B08850", "#9C7A4E").into(),
                button_text: pair("#5C2B00", "#FFE8CC").into(),
                hover:       pair("#FFE8CC", "#3D2914").into(),
                picked:      pair("#F08C00", "#FFA94D").into(),
                picked_text: pair("#FFFFFF", "#2B1D0E").into(),
            }
            .apply_globally();
        });
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        from_main(move || {
            view.date.set_open(true);
        });
        settle();
        check_colors(LIGHT)?;
        checkpoint("the fields and the open calendar in the light colors of the style")?;

        from_main(|| Theme::set_mode(ThemeMode::Dark));
        settle();
        check_colors(DARK)?;
        checkpoint("the same in the dark colors of the style")?;
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

const LIGHT: &str = r"
             172   20 - #42311c
              48   24 - #3b2a14
             356   24 - #3b2a14
             240   28 - #5e503d
             112   60 - #f08c00
             428   60 - #f08c00
             272   76 - #866141
             376   76 - #fff4e6
             272   80 - #866141
              20  120 - #f08c00
             176  120 - #a07f60
             132  152 - #ccae85
             300  152 - #c9aa7f
             320  160 - #bfbdb9
              40  220 - #7d542f
              84  252 - #5c2b00
             296  252 - #fff4e6
              16  272 - #d6d3cf
             212  288 - #fff4e6
             592  300 - #faf7f2
             320  312 - #bfbdb9
             128  324 - #fff4e6
              36  328 - #7d542f
             248  328 - #7d542f
             140  380 - #a8a6a3
             252  380 - #a8a6a3
             312  380 - #a8a6a3
              24  384 - #ccc9c5
              84  388 - #e3e1dc
             196  388 - #e3e1dc
             372  592 - #faf7f2
             592  592 - #faf7f2
";

const DARK: &str = r"
             172   20 - #eadfcf
              48   24 - #f2e6d6
             104   24 - #16120d
             360   24 - #f2e6d6
             272   28 - #16120d
             220   60 - #ffa94d
             428   60 - #ffa94d
             160   72 - #2b1d0e
             112   76 - #2b1d0e
             340   76 - #ffa94d
             288   80 - #ccb9a2
              20  120 - #ffa94d
             176  120 - #a7947d
             592  124 - #16120d
             132  152 - #745937
             300  152 - #795d3a
              40  220 - #d4bfa5
             256  248 - #ffe8cc
             296  252 - #2b1d0e
             172  256 - #d4bfa5
             104  280 - #ffa94d
             212  288 - #2b1d0e
             288  292 - #d4bfa5
             592  300 - #16120d
              88  324 - #ffe8cc
             168  324 - #ffe8cc
              36  328 - #d4bfa5
             248  328 - #d4bfa5
             428  432 - #16120d
               4  592 - #16120d
             296  592 - #16120d
             592  592 - #16120d
";
