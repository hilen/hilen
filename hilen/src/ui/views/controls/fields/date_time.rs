use chrono::{Datelike, NaiveDate, NaiveTime};

use super::super::floating::{layer_root, move_under};
use crate::{
    self as hilen,
    deps::{
        hreads::on_main_later,
        refs::{Weak, weak_from_ref},
        vents::Event,
    },
    gm::{
        ToF32,
        color::CLEAR,
        flat::{Point, Size},
    },
    ui::{
        Anchor, DateFieldStyle, Label, Setup, Shadow, TextAlignment, TextField, ToolButton, TouchStack,
        UIEvents, UIManager, View, ViewCallbacks, ViewData, ViewFrame, ViewSubviews, view, view::depth_order,
    },
};

const HEIGHT: f32 = 36.0;
const DAY: f32 = 36.0;
const CALENDAR: f32 = 40.0 + 24.0 + 6.0 * DAY;
const MIN_CALENDAR: f32 = 300.0;
/// The space between the field and its open calendar.
const GAP: f32 = 4.0;

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
        self.set_border_color(style().border);
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
                self.set_border_color(style().bad_border);
            }
        }
    }
}

impl Setup for TimeField {
    fn setup(self: Weak<Self>) {
        self.set_color(style().fill)
            .set_corner_radius(8)
            .set_border_width(1)
            .set_border_color(style().border);
        self.place().h(HEIGHT);
        self.field
            .set_alignment(TextAlignment::Center)
            .set_text_size(15)
            .set_text_color(style().text)
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

/// A date typed as `YYYY-MM-DD`, with a button that opens a month calendar.
/// The calendar floats over the page under the field, or over it when the
/// window has more room there, like the open list of a drop down, so the
/// field keeps its height. It is at least 300 points wide, and a press
/// outside closes it. A typed date is kept when the field loses the focus or
/// on Enter, a text that is no date gets a red border. `set_range` keeps the
/// date between 2 days. `changed` fires only for the user, a typed date or a
/// tap on a day.
#[view]
pub struct DateField {
    date:        Option<NaiveDate>,
    range:       Option<(NaiveDate, NaiveDate)>,
    shown:       Option<NaiveDate>,
    open:        bool,
    days:        Vec<Weak<ToolButton>>,
    previous:    Weak<ToolButton>,
    month:       Weak<Label>,
    next:        Weak<ToolButton>,
    /// The date after a change by the user.
    pub changed: Event<NaiveDate>,

    #[init]
    field:    TextField,
    button:   ToolButton,
    calendar: CalendarPanel,
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
        self.set_border_color(style().border);
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
        if open {
            this.float_calendar();
        } else {
            this.calendar.set_hidden(true);
            depth_order::set_floats(&*this.calendar, false);
            move_under(this.calendar.weak_view(), this.weak_view());
        }
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

    /// The calendar leaves the field for the end of its layer, so no view
    /// that clips its subviews cuts it off, and floats in front of all of
    /// that layer, as the open list of a drop down does.
    fn float_calendar(self: Weak<Self>) {
        let mut calendar = self.calendar;
        move_under(calendar.weak_view(), layer_root(self.weak_view()));
        depth_order::set_floats(&*calendar, true);
        let frame = *self.absolute_frame();
        let window = UIManager::root_view().height();
        let room_below = window - frame.max_y() - GAP;
        let room_above = frame.y() - GAP;
        let below = CALENDAR <= room_below || room_below >= room_above;
        let y = if below {
            self.height() + GAP
        } else {
            -(CALENDAR + GAP)
        };
        calendar.offset = Point::new(0.0, y);
        calendar.size = (self.width().max(MIN_CALENDAR), CALENDAR).into();
        calendar.follow_field();
        calendar.set_hidden(false);
        // Drawn in front is not enough, a touch goes to the view that
        // registered last.
        TouchStack::raise_subtree(calendar.weak_view());
    }

    /// A press anywhere but on the field or on the open calendar closes it.
    fn close_on_touch_outside(self: Weak<Self>, at: Point) {
        if !self.open || self.absolute_frame().contains(at) || self.calendar.absolute_frame().contains(at) {
            return;
        }
        self.set_open(false);
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
                self.set_border_color(style().bad_border);
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
        let width = (self.calendar.width() / 7.0).floor();
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
        field_box
            .set_alignment(TextAlignment::Center)
            .set_text_size(15)
            .set_text_color(style().text)
            .set_placeholder("YYYY-MM-DD");
        field_box
            .set_color(style().fill)
            .set_corner_radius(8)
            .set_border_width(1)
            .set_border_color(style().border);
        field_box.place().t(0).l(0).h(HEIGHT).anchor(Anchor::Right, self.button, 4);
        field_box.editing_ended.val(move |text| self.typed(&text));
        field_box.submitted.val(move |text| self.typed(&text));

        self.button.set_text("Calendar").place().t(2).r(0);
        tint_button(self.button);
        self.button.on_tap(move || {
            self.set_open(!self.open);
        });

        let mut calendar = self.calendar;
        calendar.owner = self;
        calendar
            .set_color(style().fill)
            .set_corner_radius(8)
            .set_border_width(1)
            .set_border_color(style().border)
            .set_shadow(Shadow::default());
        calendar.set_hidden(true);

        self.previous = calendar.add_view::<ToolButton>();
        self.previous.set_text("Previous").place().t(4).l(4);
        self.next = calendar.add_view::<ToolButton>();
        self.next.set_text("Next").place().t(4).r(4);
        tint_button(self.previous);
        tint_button(self.next);
        self.month = calendar.add_view::<Label>();
        self.month
            .set_alignment(TextAlignment::Center)
            .set_text_size(15)
            .set_text_color(style().text)
            .set_color(CLEAR);
        self.month.place().t(0).h(40).lr(90);
        self.previous.on_tap(move || self.step_month(-1));
        self.next.on_tap(move || self.step_month(1));

        for (index, name) in ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"].into_iter().enumerate() {
            let label = calendar.add_view::<Label>();
            label
                .set_alignment(TextAlignment::Center)
                .set_text(name)
                .set_text_size(12)
                .set_text_color(style().faint)
                .set_color(CLEAR);
            label
                .place()
                .t(40)
                .h(24)
                .relative_x(index.to_f32() / 7.0)
                .relative_width(calendar, 1.0 / 7.0);
        }
        for _ in 0..42 {
            let day = calendar.add_view::<ToolButton>();
            day.set_fills_width(true).set_centered(true);
            tint_button(day);
            day.set_on_color(style().picked)
                .set_on_hover_color(style().picked)
                .set_on_tint(style().picked_text);
            day.on_tap(move || {
                if let (Some(shown), Ok(number)) = (self.shown, day.text().parse::<u32>())
                    && let Some(date) = shown.with_day(number)
                {
                    self.user_set(date);
                }
            });
            self.days.push(day);
        }
        calendar.size_changed().sub(move || self.show_month());
        UIEvents::touch_began().val(self, move |touch| self.close_on_touch_outside(touch.position));
        self.set_open(false);
    }
}

impl Drop for DateField {
    /// An open calendar sits in the layer root, out of this field, a closed
    /// one is freed with it. The open one hides now and leaves the tree at
    /// the next turn of the main loop, as the open list of a drop down does.
    fn drop(&mut self) {
        let mut calendar = self.calendar;
        if !self.open || calendar.is_null() {
            return;
        }
        calendar.set_hidden(true);
        on_main_later(move || {
            if calendar.is_ok() {
                calendar.remove_from_superview();
            }
        });
    }
}

/// The open calendar of a [`DateField`]. It lives at the end of the layer of
/// its field while it shows and follows the field there, so it moves with a
/// scroll of the page under it.
#[view]
struct CalendarPanel {
    owner:  Weak<DateField>,
    /// Where the calendar sits from the top left of the field, and its size.
    offset: Point,
    size:   Size,
}

impl Setup for CalendarPanel {
    fn setup(self: Weak<Self>) {}
}

impl ViewCallbacks for CalendarPanel {
    /// Only an open calendar is updated, a hidden view is not. A field that
    /// left the screen, scrolled out or hidden with its page, takes its
    /// calendar with it.
    fn update(&mut self) {
        let owner = self.owner;
        if owner.is_null() {
            return;
        }
        if owner.open && owner.is_visible_on_screen() {
            weak_from_ref(self).follow_field();
        } else if owner.open {
            owner.set_open(false);
        }
    }
}

impl CalendarPanel {
    /// Puts the calendar under or over the field, in the points of the view
    /// it sits in.
    fn follow_field(self: Weak<Self>) {
        let parent = *self.superview();
        if parent.is_null() || self.owner.is_null() {
            return;
        }
        let origin = self.owner.absolute_frame().origin - parent.absolute_frame().origin + self.offset;
        self.set_frame((origin.x, origin.y, self.size.width, self.size.height));
    }
}

/// The text and hover colors of the style on a button of the field.
fn tint_button(button: Weak<ToolButton>) {
    button.set_tint(style().button_text).set_hover_color(style().hover);
}

fn days_in_month(first: NaiveDate) -> u32 {
    let next = if first.month() == 12 {
        NaiveDate::from_ymd_opt(first.year() + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(first.year(), first.month() + 1, 1)
    };
    next.map_or(31, |next| u32::try_from((next - first).num_days()).unwrap_or(31))
}

/// The style the fields read when they are built and when they change.
fn style() -> DateFieldStyle {
    DateFieldStyle::current()
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
