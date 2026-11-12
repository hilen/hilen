use anyhow::{Result, ensure};
use chrono::NaiveDate;
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::{Own, Weak},
    ui::{
        Color, ColorPicker, DateField, NumberField, Setup, SortHeader, SortOrder, TimeField, ViewData,
        ViewFrame, ViewTest, view,
    },
    ui_test::{check_colors, checkpoint, inject_touches, system_input},
};

/// The form views: a number with a unit and steps, a time, a date with its
/// calendar, a color and a header of sortable columns. Taps and typing
/// fire `changed`, the setters called by code do not.
#[view]
struct FormFields {
    events: Own<Vec<String>>,

    #[init]
    number: NumberField,
    time:   TimeField,
    header: SortHeader,
    color:  ColorPicker,
    date:   DateField,
}

impl Setup for FormFields {
    fn setup(self: Weak<Self>) {
        self.set_color(Color::hex("#F7F7FA"));
        self.number.place().t(10).l(10).w(220);
        self.number
            .set_unit("EUR/kWh")
            .set_step(0.05)
            .set_range(0.0, 1.0)
            .set_value(0.25);
        self.time.place().t(10).l(250).w(100);
        self.header.place().t(60).lr(10);
        self.header.set_columns([("Name", 2.0), ("Zone", 1.0), ("Health", 1.0)]);
        self.color.place().t(110).l(10).w(360);
        self.color.set_presets([
            Color::hex("#FF6B6B"),
            Color::hex("#FFD93D"),
            Color::hex("#6BCB77"),
        ]);
        self.color.set_color_value(Color::hex("#4D96FF"));
        self.date.place().t(290).l(10).w(320);
        self.date.set_range(
            NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2026, 12, 31).unwrap(),
        );
        self.date.set_date(NaiveDate::from_ymd_opt(2026, 3, 9).unwrap());

        let mut events = self.events.weak();
        self.number.changed.val(move |value| events.push(format!("number {value:.2}")));
        self.time.changed.val(move |time| events.push(format!("time {time}")));
        self.header
            .changed
            .val(move |(column, order)| events.push(format!("sort {column} {order:?}")));
        self.color
            .changed
            .val(move |color| events.push(format!("color {}", color.as_hex())));
        self.date.changed.val(move |date| events.push(format!("date {date}")));
    }
}

impl ViewTest for FormFields {
    fn canvas() -> (u32, u32) {
        (600, 700)
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        settle();
        ensure!(from_main(move || view.events.is_empty()), "setters fire nothing");
        check_colors(SET_BY_CODE)?;
        checkpoint("0.25 EUR/kWh, an empty time, no sorted column, a blue color, 2026-03-09")?;

        let number = from_main(move || *view.number.absolute_frame());
        tap(number.max_x() - 16.0, number.center().y);
        let header = from_main(move || *view.header.absolute_frame());
        tap(header.x() + header.width() * 0.6, header.center().y);
        tap(header.x() + header.width() * 0.6, header.center().y);
        let color = from_main(move || *view.color.absolute_frame());
        tap(color.x() + 14.0 + 36.0 * 2.0, color.y() + 4.0 * 32.0 + 8.0 + 14.0);

        let time = from_main(move || *view.time.absolute_frame());
        system_input::tap(time.center().x, time.center().y)?;
        system_input::type_text("0730")?;
        system_input::press_return()?;
        settle();

        let date = from_main(move || *view.date.absolute_frame());
        tap(date.max_x() - 30.0, date.y() + 18.0);
        from_main(move || {
            ensure!(view.date.is_open(), "the calendar opened");
            Ok(())
        })?;
        check_colors(CALENDAR)?;
        checkpoint("the calendar of March 2026 is open under the date field")?;
        let day = from_main(move || *view.date.day_button(20).unwrap().absolute_frame());
        tap(day.center().x, day.center().y);

        from_main(move || {
            ensure!(
                *view.events
                    == [
                        "number 0.30",
                        "sort 1 Up",
                        "sort 1 Down",
                        "color #6bcb77",
                        "time 07:30:00",
                        "date 2026-03-20",
                    ],
                "events {:?}",
                *view.events
            );
            ensure!(!view.date.is_open(), "a pick closes the calendar");
            ensure!(view.header.sorted() == Some((1, SortOrder::Down)));
            Ok(())
        })?;
        check_colors(DONE)?;
        checkpoint("0.30, 07:30, Zone sorted down, the green preset, 2026-03-20")?;
        Ok(())
    }
}

fn tap(x: f32, y: f32) {
    let (x, y) = (x.round(), y.round());
    inject_touches(format!("{x} {y} b\n{x} {y} e"));
    settle();
}

fn settle() {
    for _ in 0..3 {
        wait_for_next_frame();
    }
}

const SET_BY_CODE: &str = r"
             132   24 - #b1b1b3
             164   24 - #78787c
             276   24 - #bcbcbc
             324   28 - #e8e8e8
             100   32 - #1c1c1e
              28   72 - #464648
             456   76 - #3e3e41
             460   76 - #cccccf
             484   76 - #cfcfd1
              16  112 - #4d96ff
             172  124 - #0a84ff
             204  144 - #dbdbdd
             284  156 - #bdbdc1
             352  156 - #d1d1d6
             116  160 - #f7f7fa
             224  160 - #0a84ff
              52  164 - #4d96ff
             252  164 - #e0e0e3
              88  208 - #d1d1d6
             360  208 - #dddddf
              44  224 - #939394
              56  248 - #ffd93d
              32  252 - #ff6b6b
             104  252 - #6bcb77
              16  268 - #ff6b6b
              64  268 - #ffd93d
              88  268 - #6bcb77
             156  308 - #ffffff
             304  308 - #f7f7fa
             112  312 - #1c1c1e
               4  692 - #f7f7fa
             592  692 - #f7f7fa
            ";
const CALENDAR: &str = r"
             140   24 - #85858a
             348   72 - #0b6dff
              28   76 - #464648
             460   76 - #cccccf
             484   76 - #cfcfd1
             180  124 - #0a84ff
             352  124 - #d1d1d6
              96  136 - #6bcb77
              12  156 - #6bcb77
             148  192 - #0a84ff
             228  192 - #086fd6
             308  192 - #d1d1d6
              60  220 - #29292b
              56  248 - #ffd93d
              32  252 - #ff6b6b
              16  268 - #ff6b6b
              64  268 - #ffd93d
             288  308 - #7a7a7c
              96  312 - #1d1d1f
             592  348 - #f7f7fa
              52  352 - #b3b3b4
             132  352 - #575758
             304  380 - #ffffff
              32  384 - #cdcdcf
             212  384 - #c5c5c7
             164  480 - #7b7b7c
             208  484 - #656566
              28  520 - #656566
             216  548 - #dcdcdd
             120  556 - #ffffff
             300  556 - #ffffff
             592  692 - #f7f7fa
            ";
const DONE: &str = r"
             132   24 - #b1b1b3
             164   24 - #78787c
             104   28 - #1d1d1f
             284   28 - #ffffff
             136   32 - #8c8c90
             352   72 - #0b6dff
              28   76 - #464648
             460   76 - #cccccf
             484   76 - #cfcfd1
             180  124 - #0a84ff
             248  124 - #bbbbbf
             340  124 - #d1d1d6
             216  132 - #e2e2e4
              96  140 - #6bcb77
              12  156 - #6bcb77
             328  156 - #bcbcc1
             296  164 - #e0e0e3
             152  192 - #0a84ff
             228  192 - #086fd6
             352  192 - #d1d1d6
              60  224 - #29292b
              56  248 - #ffd93d
              32  252 - #ff6b6b
              16  268 - #ff6b6b
              64  268 - #ffd93d
             104  308 - #ffffff
             288  308 - #7a7a7c
             304  308 - #f7f7fa
             316  308 - #565658
             156  312 - #1c1c1e
               4  692 - #f7f7fa
             592  692 - #f7f7fa
            ";
