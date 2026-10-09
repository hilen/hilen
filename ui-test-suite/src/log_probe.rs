//! What the `LogView` tests share: the style of a fixture and the way a
//! test finds a line on the screen.

use hilen::{
    refs::Weak,
    ui::{
        BLACK, Label, LogStyle, LogView, TextAlignment, VerticalAlignment, View, ViewData, ViewFrame,
        ViewSubviews, WeakView,
    },
};

/// Text big enough for a probe to land on a glyph.
pub(crate) fn test_style() -> LogStyle {
    LogStyle {
        text_size: 16.0,
        ..LogStyle::DEFAULT
    }
}

/// The label that says what the test did, under or beside the log.
pub(crate) fn status_style(label: Weak<Label>) {
    label
        .set_multiline(true)
        .set_text_size(14)
        .set_text_color(BLACK)
        .set_alignment(TextAlignment::Left)
        .set_vertical_alignment(VerticalAlignment::Top);
}

/// The label of the log whose text has `part` in it, when a part of it
/// is inside the frame of the log. A row that was set up beyond the edge
/// is not on screen. Call it on the main thread.
pub(crate) fn line_on_screen(log: Weak<LogView>, part: &str) -> Option<Weak<Label>> {
    let frame = *log.absolute_frame();
    let label = find(log.weak_view(), part)?;
    let own = label.absolute_frame();
    (own.max_y() > frame.y() && own.y() < frame.max_y()).then_some(label)
}

/// Like `line_on_screen`, for a line the test knows is there.
pub(crate) fn line(log: Weak<LogView>, part: &str) -> Weak<Label> {
    line_on_screen(log, part).unwrap_or_else(|| panic!("the log shows no `{part}`"))
}

fn find(view: WeakView, part: &str) -> Option<Weak<Label>> {
    if view.is_null() || view.is_hidden() {
        return None;
    }
    if let Some(label) = view.downcast_view::<Label>()
        && label.text().contains(part)
    {
        return Some(label);
    }
    view.subviews().iter().find_map(|sub| find(sub.weak(), part))
}

/// Every label of the log whose text has `part` in it and that is at
/// least partly inside the frame of the log. Call it on the main thread.
pub(crate) fn lines_on_screen(log: Weak<LogView>, part: &str) -> Vec<Weak<Label>> {
    let frame = *log.absolute_frame();
    let mut found = Vec::new();
    collect(log.weak_view(), part, &mut found);
    found.retain(|label| {
        let own = label.absolute_frame();
        own.max_y() > frame.y() && own.y() < frame.max_y()
    });
    found
}

fn collect(view: WeakView, part: &str, found: &mut Vec<Weak<Label>>) {
    if view.is_null() || view.is_hidden() {
        return;
    }
    if let Some(label) = view.downcast_view::<Label>() {
        if label.text().contains(part) {
            found.push(label);
        }
        return;
    }
    for sub in view.subviews() {
        collect(sub.weak(), part, found);
    }
}
