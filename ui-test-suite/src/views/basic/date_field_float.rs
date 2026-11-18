use anyhow::{Result, ensure};
use chrono::NaiveDate;
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{
        Color, Container, DateField, DynamicColor, Label, Setup, TextAlignment, ViewData, ViewFrame,
        ViewSubviews, ViewTest, view,
    },
    ui_test::{check_colors, checkpoint, inject_touches},
};

/// A date field narrower than its calendar, over a box that was added after
/// it. The open calendar floats in front of the box, is at least 300 points
/// wide, and the field keeps its height. A press outside closes it.
#[view]
struct DateFieldFloatTest {
    #[init]
    status: Label,
    date:   DateField,
    below:  Container,
}

impl Setup for DateFieldFloatTest {
    fn setup(self: Weak<Self>) {
        self.set_color(pair("#F7F7FA", "#1C1C1E"));
        self.status
            .set_text("calendar open over the green box")
            .set_text_size(18)
            .set_alignment(TextAlignment::Left);
        self.status.set_text_color(pair("#1C1C1E", "#F2F2F4"));
        self.status.place().t(10).lr(10).h(30);
        self.date.place().t(60).l(20).w(200);
        self.date.set_date(NaiveDate::from_ymd_opt(2026, 11, 18).unwrap());

        self.below.set_color(pair("#2F9E44", "#2B8A3E")).set_corner_radius(8);
        self.below.place().t(110).l(20).size(420, 120);
        let caption = self.below.add_view::<Label>();
        caption
            .set_text("view below the field")
            .set_text_size(16)
            .set_alignment(TextAlignment::Left)
            .set_text_color(pair("#FFFFFF", "#FFFFFF"))
            .set_color(pair("#2F9E44", "#2B8A3E"));
        caption.place().l(240).t(10).size(170, 24);
    }
}

impl ViewTest for DateFieldFloatTest {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        settle();
        from_main(move || {
            view.date.set_open(true);
        });
        settle();
        from_main(move || {
            ensure!(view.date.is_open(), "the calendar opened");
            ensure!(
                (view.date.height() - 36.0).abs() < 0.5,
                "the field keeps its height, it is {}",
                view.date.height()
            );
            let day = view.date.day_button(18).expect("the 18th shows");
            ensure!(
                day.width() >= 42.0,
                "a day of a 300 point calendar is 42 wide, it is {}",
                day.width()
            );
            Ok(())
        })?;
        check_colors(OPEN)?;
        checkpoint("the calendar, 300 wide, in front of the green box, the field still 36 tall")?;

        inject_touches("520 520 b\n520 520 e");
        settle();
        from_main(move || {
            ensure!(!view.date.is_open(), "a press outside closed the calendar");
            view.status.set_text("closed by a press outside");
            Ok(())
        })?;
        settle();
        check_colors(CLOSED)?;
        checkpoint("closed by a press outside, the green box shows again")?;
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

const OPEN: &str = r"
             264   20 - #242426
             148   24 - #f7f7fa
              52   76 - #1c1c1e
             248  100 - #d1d1d6
             320  108 - #c0c0c2
             324  108 - #dcdcdf
             176  120 - #7a7a7c
             360  132 - #d1ead6
             328  140 - #2e9942
             436  148 - #2f9e44
             132  152 - #b6b6b9
             300  152 - #b1b1b5
             320  188 - #247934
             384  188 - #2f9e44
              16  204 - #d3d3d6
             328  228 - #2e9942
             432  228 - #2f9e44
              84  252 - #1c1c1e
             296  252 - #ffffff
             172  256 - #4a4a4c
             320  276 - #bdbdbf
             320  312 - #bdbdbf
             212  320 - #ffffff
              36  328 - #4a4a4c
             120  328 - #4a4a4c
             108  380 - #a6a6a8
             252  380 - #a6a6a8
             312  380 - #a6a6a8
             196  388 - #e1e1e3
               4  592 - #f7f7fa
             372  592 - #f7f7fa
             592  592 - #f7f7fa
";

const CLOSED: &str = r"
             592    4 - #f7f7fa
              76   20 - #3a3a3c
              76   24 - #1c1c1e
             160   24 - #f7f7fa
             192   24 - #1c1c1e
             224   24 - #f7f7fa
              44   28 - #f7f7fa
             108   28 - #1c1c1e
             132   28 - #3b3b3d
             192   28 - #1c1c1e
              32   60 - #d1d1d6
              68   60 - #d1d1d6
             112   60 - #d1d1d6
             100   72 - #ffffff
              52   76 - #1c1c1e
              84   76 - #474748
             172   76 - #565659
              84   80 - #474748
             172   80 - #565659
             188   80 - #4c4c4e
              20   84 - #d1d1d6
             316  132 - #2f9e44
             360  132 - #d1ead6
             404  132 - #ffffff
              20  188 - #2f9e44
             112  228 - #2f9e44
             236  228 - #2f9e44
             432  228 - #2f9e44
             592  324 - #f7f7fa
             300  524 - #f7f7fa
               4  592 - #f7f7fa
             592  592 - #f7f7fa
";
