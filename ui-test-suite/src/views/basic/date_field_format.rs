use anyhow::{Result, ensure};
use chrono::NaiveDate;
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{Color, DateField, DynamicColor, Label, Setup, TextAlignment, ViewData, ViewFrame, ViewTest, view},
    ui_test::{check_colors, checkpoint, inject_touches, system_input},
};

/// 2 date fields with the format `%b %-d`. The first holds Nov 9 and shows
/// it that way, the second is empty. A press into the first turns its text
/// into `2026-11-09`, and Enter turns it back with no change of the date. A
/// date typed into the second and a day picked in the calendar of the first
/// both show in the format.
#[view]
struct DateFieldFormatTest {
    events: Vec<String>,

    #[init]
    status:     Label,
    with_date:  DateField,
    first_name: Label,
    empty:      DateField,
    empty_name: Label,
}

impl Setup for DateFieldFormatTest {
    fn setup(self: Weak<Self>) {
        self.set_color(pair("#F7F7FA", "#1C1C1E"));
        self.status
            .set_text("at rest: Nov 9 and an empty field")
            .set_text_size(18)
            .set_alignment(TextAlignment::Left);
        self.status.set_text_color(pair("#1C1C1E", "#F2F2F4"));
        self.status.place().t(10).lr(10).h(30);

        self.with_date.place().t(80).l(20).w(320);
        self.with_date.set_format("%b %-d");
        self.with_date.set_date(NaiveDate::from_ymd_opt(2026, 11, 9).unwrap());
        name(self.first_name, "field with a date");
        self.first_name.place().t(80).l(350).size(230, 36);

        self.empty.place().t(150).l(20).w(320);
        self.empty.set_format("%b %-d");
        name(self.empty_name, "field with no date");
        self.empty_name.place().t(150).l(350).size(230, 36);

        self.with_date.changed.val(move |date| {
            let mut test = self;
            test.events.push(format!("first {date}"));
        });
        self.empty.changed.val(move |date| {
            let mut test = self;
            test.events.push(format!("second {date}"));
        });
    }
}

impl ViewTest for DateFieldFormatTest {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        settle();
        from_main(move || {
            ensure!(
                view.with_date.text() == "Nov 9",
                "the date shows in the format, it shows {}",
                view.with_date.text()
            );
            Ok(())
        })?;
        check_colors(REST)?;

        let first = from_main(move || *view.with_date.absolute_frame());
        system_input::tap(first.x() + 100.0, first.center().y)?;
        system_input::wait_until("the first field shows the typed form", move || {
            view.with_date.text() == "2026-11-09"
        })?;
        from_main(move || {
            view.status.set_text("pressed into the first field: 2026-11-09");
        });
        checkpoint("pressed into the first field, it shows 2026-11-09")?;

        system_input::press_return()?;
        system_input::wait_until("the first field shows the format again", move || {
            view.with_date.text() == "Nov 9"
        })?;
        from_main(move || {
            ensure!(
                view.events.is_empty(),
                "Enter with the same date is no change, the events are {:?}",
                view.events
            );
            view.status.set_text("Enter: Nov 9 again, no change");
            Ok(())
        })?;
        checkpoint("Enter, the first field shows Nov 9 again")?;

        let second = from_main(move || *view.empty.absolute_frame());
        system_input::tap(second.x() + 100.0, second.center().y)?;
        system_input::type_text("2026-12-01")?;
        system_input::press_return()?;
        system_input::wait_until("the typed date shows in the format", move || {
            view.empty.text() == "Dec 1"
        })?;
        from_main(move || {
            ensure!(
                view.empty.date() == NaiveDate::from_ymd_opt(2026, 12, 1),
                "the typed date is kept, it is {:?}",
                view.empty.date()
            );
            ensure!(
                view.events == ["second 2026-12-01"],
                "a typed date is 1 change, the events are {:?}",
                view.events
            );
            view.status.set_text("typed 2026-12-01 into the second: Dec 1");
            Ok(())
        })?;
        checkpoint("typed 2026-12-01 into the second field, it shows Dec 1")?;

        from_main(move || {
            view.with_date.set_open(true);
        });
        settle();
        let day = from_main(move || *view.with_date.day_button(20).unwrap().absolute_frame());
        inject_touches(format!(
            "{x} {y} b\n{x} {y} e",
            x = day.center().x,
            y = day.center().y
        ));
        settle();
        from_main(move || {
            ensure!(
                view.with_date.text() == "Nov 20",
                "a picked day shows in the format, it shows {}",
                view.with_date.text()
            );
            ensure!(
                view.events == ["second 2026-12-01", "first 2026-11-20"],
                "a picked day is 1 change, the events are {:?}",
                view.events
            );
            view.status.set_text("picked the 20th in the first: Nov 20");
            Ok(())
        })?;
        settle();
        check_colors(END)?;
        checkpoint("picked the 20th in the calendar of the first field, it shows Nov 20")?;

        Ok(())
    }
}

fn name(label: Weak<Label>, text: &str) {
    label.set_text(text).set_text_size(16).set_alignment(TextAlignment::Left);
    label.set_text_color(pair("#5F6475", "#9AA3BD"));
}

fn pair(light: &str, dark: &str) -> DynamicColor {
    DynamicColor::new(Color::hex(light), Color::hex(dark))
}

fn settle() {
    for _ in 0..3 {
        wait_for_next_frame();
    }
}

/// Nov 9 in the first field, the placeholder in the second.
const REST: &str = r"
              96   20 - #2c2c2e
              48   24 - #1c1c1e
             148   24 - #1c1c1e
             232   24 - #373739
             256   24 - #38383a
             172   28 - #1c1c1e
             200   28 - #f7f7fa
             124   80 - #d1d1d6
             180   80 - #d1d1d6
             232   80 - #d1d1d6
              20   96 - #d1d1d6
             292   96 - #565659
             308  100 - #4c4c4e
             372  100 - #777b8a
             392  100 - #f7f7fa
             428  100 - #5f6475
             460  100 - #f7f7fa
             368  164 - #5f6475
             292  168 - #565659
             372  168 - #777b8a
             392  168 - #f7f7fa
             420  168 - #f7f7fa
             428  168 - #5f6475
             448  168 - #f7f7fa
             476  168 - #747988
             168  172 - #bcbcbc
              20  176 - #d1d1d6
             592  364 - #f7f7fa
              96  388 - #f7f7fa
             300  480 - #f7f7fa
               4  592 - #f7f7fa
             592  592 - #f7f7fa
";

/// Nov 20 in the first field, Dec 1 in the second.
const END: &str = r"
             140   20 - #3b3b3d
             180   20 - #3a3a3c
              28   24 - #1c1c1e
             292   24 - #f7f7fa
              76   28 - #1c1c1e
             140   28 - #3b3b3d
             180   28 - #3a3a3c
             204   28 - #1c1c1e
             252   28 - #1d1d1f
              84   80 - #d1d1d6
             228   80 - #d1d1d6
             160   96 - #ffffff
             292   96 - #565659
             308  100 - #4c4c4e
             372  100 - #777b8a
             428  100 - #5f6475
             460  100 - #f7f7fa
              20  104 - #d1d1d6
             368  164 - #5f6475
             144  168 - #ffffff
             292  168 - #565659
             372  168 - #777b8a
             392  168 - #f7f7fa
             428  168 - #5f6475
             476  168 - #747988
             124  172 - #1d1d1f
              20  176 - #d1d1d6
             592  364 - #f7f7fa
              96  392 - #f7f7fa
             300  480 - #f7f7fa
               4  592 - #f7f7fa
             592  592 - #f7f7fa
";
