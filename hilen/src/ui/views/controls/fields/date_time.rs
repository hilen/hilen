use chrono::{Datelike, NaiveDate, NaiveTime};

use crate::{
    self as hilen,
    deps::{
        refs::{Weak, weak_from_ref},
        vents::Event,
    },
    gm::{
        ToF32,
        color::{CLEAR, Color},
    },
    ui::{
        Anchor, Container, DynamicColor, Label, Setup, TextField, ToolButton, ViewData, ViewFrame,
        ViewSubviews, view,
    },
};

const BORDER: DynamicColor = DynamicColor::new(Color::hex("#D1D1D6"), Color::hex("#48484E"));
const BAD_BORDER: DynamicColor = DynamicColor::new(Color::hex("#FF3B30"), Color::hex("#FF453A"));
const FILL: DynamicColor = DynamicColor::new(Color::hex("#FFFFFF"), Color::hex("#1C1C1E"));
const TEXT: DynamicColor = DynamicColor::new(Color::hex("#1C1C1E"), Color::hex("#F2F2F4"));
const FAINT: DynamicColor = DynamicColor::new(Color::hex("#8E8E93"), Color::hex("#8E8E93"));
const HEIGHT: f32 = 36.0;
const DAY: f32 = 36.0;
const CALENDAR: f32 = 40.0 + 24.0 + 6.0 * DAY;

/// A time of day typed as `HH:MM`, 24 hours. A typed time is kept when the
/// field loses the focus or on Enter, a text that is no time gets a red
/// border and keeps the last good time. `changed` fires only for the user.
#[view]
pub struct TimeField {
    time:        Option<NaiveTime>,
    /// The time after a change by the user.
    pub changed: Event<NaiveTime>,

    #[init]
    field: TextField,
}

impl TimeField {
    pub fn time(&self) -> Option<NaiveTime> {
        self.time
    }

    /// Shows a time with no `changed`.
    pub fn set_time(&self, time: NaiveTime) -> &Self {
        weak_from_ref(self).time = Some(time);
        self.field.set_text(time.format("%H:%M").to_string());
        self.set_border_color(BORDER);
        self
    }

    fn typed(self: Weak<Self>, text: &str) {
        match parse_time(text) {
            Some(time) => {
                let changed = self.time != Some(time);
                self.set_time(time);
                if changed {
                    self.changed.trigger(time);
                }
            }
            None => {
                self.set_border_color(BAD_BORDER);
            }
        }
    }
}

impl Setup for TimeField {
    fn setup(self: Weak<Self>) {
        self.set_color(FILL)
            .set_corner_radius(8)
            .set_border_width(1)
            .set_border_color(BORDER);
        self.place().h(HEIGHT);
        self.field
            .set_text_size(15)
            .set_text_color(TEXT)
            .set_color(CLEAR)
            .set_placeholder("HH:MM");
        self.field.place().back();
        self.field.editing_ended.val(move |text| self.typed(&text));
        self.field.submitted.val(move |text| self.typed(&text));
    }
}

/// `7:05`, `07:05` and `0705` are 7:05.
pub(crate) fn parse_time(text: &str) -> Option<NaiveTime> {
    let text = text.trim();
    let (hours, minutes) = match text.split_once(':') {
        Some(parts) => parts,
        None if text.len() == 4 => text.split_at(2),
        None => return None,
    };
    NaiveTime::from_hms_opt(hours.trim().parse().ok()?, minutes.trim().parse().ok()?, 0)
}

/// A date typed as `YYYY-MM-DD`, with a button that opens a month calendar
/// under the field. The calendar makes the view taller, so in a stack the
/// views under it move down. A typed date is kept when the field loses the
/// focus or on Enter, a text that is no date gets a red border. `set_range`
/// keeps the date between 2 days. `changed` fires only for the user, a
/// typed date or a tap on a day.
#[view]
pub struct DateField {
    date:        Option<NaiveDate>,
    range:       Option<(NaiveDate, NaiveDate)>,
    shown:       Option<NaiveDate>,
    open:        bool,
    days:        Vec<Weak<ToolButton>>,
    /// The date after a change by the user.
    pub changed: Event<NaiveDate>,

    #[init]
    field:    TextField,
    button:   ToolButton,
    calendar: Container,
    previous: ToolButton,
    month:    Label,
    next:     ToolButton,
}

impl DateField {
    pub fn date(&self) -> Option<NaiveDate> {
        self.date
    }

    /// Shows a date with no `changed`.
    pub fn set_date(&self, date: NaiveDate) -> &Self {
        let date = self.clamp(date);
        let mut this = weak_from_ref(self);
        this.date = Some(date);
        this.shown = Some(date);
        self.field.set_text(date.format("%Y-%m-%d").to_string());
        self.set_border_color(BORDER);
        self.show_month();
        self
    }

    /// The first and the last day the user can pick.
    pub fn set_range(&self, first: NaiveDate, last: NaiveDate) -> &Self {
        weak_from_ref(self).range = Some((first.min(last), first.max(last)));
        if let Some(date) = self.date {
            self.set_date(date);
        }
        self.show_month();
        self
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Opens or closes the calendar.
    pub fn set_open(&self, open: bool) -> &Self {
        let mut this = weak_from_ref(self);
        this.open = open;
        if this.shown.is_none() {
            this.shown = Some(this.date.unwrap_or_else(|| chrono::Local::now().date_naive()));
        }
        self.calendar.set_hidden(!open);
        self.previous.set_hidden(!open);
        self.next.set_hidden(!open);
        self.month.set_hidden(!open);
        self.place().replace_h(if open { HEIGHT + 4.0 + CALENDAR } else { HEIGHT });
        self.show_month();
        self
    }

    /// The button of a day in the open calendar, for a test.
    pub fn day_button(&self, day: u32) -> Option<Weak<ToolButton>> {
        self.days
            .iter()
            .copied()
            .find(|button| !button.is_hidden() && button.text() == day.to_string())
    }

    fn clamp(&self, date: NaiveDate) -> NaiveDate {
        match self.range {
            Some((first, last)) => date.clamp(first, last),
            None => date,
        }
    }

    fn user_set(self: Weak<Self>, date: NaiveDate) {
        let date = self.clamp(date);
        let changed = self.date != Some(date);
        self.set_date(date);
        self.set_open(false);
        if changed {
            self.changed.trigger(date);
        }
    }

    fn typed(self: Weak<Self>, text: &str) {
        match NaiveDate::parse_from_str(text.trim(), "%Y-%m-%d") {
            Ok(date) => self.user_set(date),
            Err(_) => {
                self.set_border_color(BAD_BORDER);
            }
        }
    }

    fn step_month(mut self: Weak<Self>, months: i32) {
        let Some(shown) = self.shown else {
            return;
        };
        let total = shown.year() * 12 + shown.month0().cast_signed() + months;
        self.shown = NaiveDate::from_ymd_opt(total.div_euclid(12), total.rem_euclid(12) as u32 + 1, 1);
        self.show_month();
    }

    fn show_month(&self) {
        let Some(shown) = self.shown else {
            return;
        };
        let first = NaiveDate::from_ymd_opt(shown.year(), shown.month(), 1).unwrap();
        self.month.set_text(first.format("%B %Y").to_string());
        let lead = first.weekday().num_days_from_monday() as usize;
        let length = days_in_month(first);
        let width = (self.width() / 7.0).floor();
        for (slot, button) in self.days.iter().enumerate() {
            let day = i64::try_from(slot).unwrap_or(0) - i64::try_from(lead).unwrap_or(0) + 1;
            let in_month = day >= 1 && day <= i64::from(length);
            button.set_hidden(!in_month);
            if !in_month {
                continue;
            }
            let date = first.with_day(u32::try_from(day).unwrap_or(1)).unwrap();
            let allowed = self.clamp(date) == date;
            button
                .set_text(day.to_string())
                .set_enabled(allowed)
                .set_on(self.date == Some(date));
            let (column, row) = (slot % 7, slot / 7);
            button.set_frame((column.to_f32() * width, 64.0 + row.to_f32() * DAY, width, DAY));
        }
    }
}

impl Setup for DateField {
    fn setup(mut self: Weak<Self>) {
        self.set_color(CLEAR);
        self.place().h(HEIGHT);

        let field_box = self.field;
        field_box.set_text_size(15).set_text_color(TEXT).set_placeholder("YYYY-MM-DD");
        field_box
            .set_color(FILL)
            .set_corner_radius(8)
            .set_border_width(1)
            .set_border_color(BORDER);
        field_box.place().t(0).l(0).h(HEIGHT).anchor(Anchor::Right, self.button, 4);
        field_box.editing_ended.val(move |text| self.typed(&text));
        field_box.submitted.val(move |text| self.typed(&text));

        self.button.set_text("Calendar").place().t(2).r(0);
        self.button.on_tap(move || {
            self.set_open(!self.open);
        });

        self.calendar
            .set_color(FILL)
            .set_corner_radius(8)
            .set_border_width(1)
            .set_border_color(BORDER);
        self.calendar.place().t(HEIGHT + 4.0).lr(0).h(CALENDAR);
        self.calendar.set_hidden(true);

        self.previous.set_text("Previous").place().t(HEIGHT + 8.0).l(4);
        self.next.set_text("Next").place().t(HEIGHT + 8.0).r(4);
        self.month.set_text_size(15).set_text_color(TEXT).set_color(CLEAR);
        self.month.place().t(HEIGHT + 4.0).h(40).lr(90);
        self.previous.on_tap(move || self.step_month(-1));
        self.next.on_tap(move || self.step_month(1));

        for (index, name) in ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"].into_iter().enumerate() {
            let label = self.calendar.add_view::<Label>();
            label.set_text(name).set_text_size(12).set_text_color(FAINT).set_color(CLEAR);
            label
                .place()
                .t(40)
                .h(24)
                .relative_x(index.to_f32() / 7.0)
                .relative_width(self.calendar, 1.0 / 7.0);
        }
        for _ in 0..42 {
            let day = self.calendar.add_view::<ToolButton>();
            day.set_fills_width(true).set_centered(true);
            day.on_tap(move || {
                if let (Some(shown), Ok(number)) = (self.shown, day.text().parse::<u32>())
                    && let Some(date) = shown.with_day(number)
                {
                    self.user_set(date);
                }
            });
            self.days.push(day);
        }
        self.calendar.size_changed().sub(move || self.show_month());
        self.set_open(false);
    }
}

fn days_in_month(first: NaiveDate) -> u32 {
    let next = if first.month() == 12 {
        NaiveDate::from_ymd_opt(first.year() + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(first.year(), first.month() + 1, 1)
    };
    next.map_or(31, |next| u32::try_from((next - first).num_days()).unwrap_or(31))
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, NaiveTime};

    use super::{days_in_month, parse_time};

    #[test]
    fn times_parse_in_3_spellings() {
        let seven = NaiveTime::from_hms_opt(7, 5, 0);
        assert_eq!(parse_time("7:05"), seven);
        assert_eq!(parse_time(" 07:05 "), seven);
        assert_eq!(parse_time("0705"), seven);
        assert_eq!(parse_time("25:00"), None);
        assert_eq!(parse_time("abc"), None);
    }

    #[test]
    fn month_lengths() {
        let first = |y, m| NaiveDate::from_ymd_opt(y, m, 1).unwrap();
        assert_eq!(days_in_month(first(2026, 2)), 28);
        assert_eq!(days_in_month(first(2028, 2)), 29);
        assert_eq!(days_in_month(first(2026, 12)), 31);
    }
}
