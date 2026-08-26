//! The caret, the selection, the keys and the clipboard of a text field.

use web_time::{Duration, Instant};

use super::{MASK, MULTILINE_TOP_INSET, TextField, mask};
use crate::{
    deps::refs::Weak,
    gm::{
        LossyConvert, ToF32,
        color::Color,
        flat::{Point, Rect},
    },
    system::Clipboard,
    ui::{
        Container, Input, TextAlignment, UIManager, VerticalAlignment, View, ViewSubviews,
        text_field_constraint::AcceptChar,
        view::{ViewData, ViewFrame, ViewTouch},
    },
    window::{NamedKey, TextLayout},
};

const SELECTION_COLOR: Color = Color::rgba(0.2, 0.5, 1.0, 0.35);

/// Two taps this close in time and place select the word.
const DOUBLE_CLICK_INTERVAL: Duration = Duration::from_millis(400);
const DOUBLE_CLICK_DISTANCE: f32 = 6.0;

impl TextField {
    /// The text the caret indexes, empty while the placeholder shows.
    pub(super) fn entered_text(&self) -> String {
        if self.placeholding {
            String::new()
        } else {
            self.text().to_string()
        }
    }

    /// The text the label draws, the mask of a secure field.
    fn display_text(&self) -> String {
        let text = self.entered_text();
        if self.is_secure() { mask(&text) } else { text }
    }

    /// A byte index into the entered text as a byte index into the
    /// displayed text. The two differ only in a secure field, where every
    /// character becomes one mask character.
    fn display_byte(&self, byte: usize) -> usize {
        if !self.is_secure() {
            return byte;
        }
        self.entered_text()[..byte].chars().count() * MASK.len_utf8()
    }

    /// The inverse of `display_byte`.
    fn entered_byte(&self, display: usize) -> usize {
        if !self.is_secure() {
            return display;
        }
        let chars = display / MASK.len_utf8();
        let text = self.entered_text();
        text.char_indices().nth(chars).map_or(text.len(), |(index, _)| index)
    }

    /// The selected byte range, start before end, `None` when empty.
    pub(super) fn selection(&self) -> Option<(usize, usize)> {
        let anchor = self.anchor?;
        if anchor == self.caret {
            return None;
        }
        Some((anchor.min(self.caret), anchor.max(self.caret)))
    }

    fn layout(&self) -> TextLayout {
        self.label.text_layout_for(&self.display_text())
    }

    pub(super) fn on_char(self: Weak<Self>, ch: char) {
        if self.is_null() || !self.is_selected() {
            return;
        }

        if Input::command_held() {
            self.on_command(ch);
            return;
        }

        let backspace = ch as u32 == 8;

        // Enter and Escape arrive a second time as control chars. The key
        // handler owns them.
        if ch.is_control() && !backspace {
            return;
        }

        if backspace {
            self.delete_backward();
        } else {
            self.insert(&ch.to_string());
        }
    }

    fn on_command(mut self: Weak<Self>, ch: char) {
        match ch.to_ascii_lowercase() {
            'a' => {
                let len = self.entered_text().len();
                self.anchor = Some(0);
                self.caret = len;
                self.update_caret();
            }
            'c' => self.copy(),
            'x' => {
                self.copy();
                self.delete_selection();
            }
            'v' => self.paste(),
            _ => {}
        }
    }

    fn copy(self: Weak<Self>) {
        let text = self.selected_text();
        if text.is_empty() || self.is_secure() {
            return;
        }
        if let Err(err) = Clipboard::set_text(text) {
            log::error!("Failed to copy from a text field: {err}");
        }
    }

    fn paste(self: Weak<Self>) {
        #[cfg(not_wasm)]
        match Clipboard::get_text() {
            Ok(text) => self.insert(&text),
            Err(err) => log::warn!("Nothing to paste: {err}"),
        }
    }

    pub(super) fn on_key(self: Weak<Self>, key: NamedKey) {
        if self.is_null() || !self.is_selected() {
            return;
        }

        let shift = Input::modifiers().shift_key();

        match key {
            NamedKey::Enter if self.multiline => self.insert("\n"),
            NamedKey::Enter | NamedKey::Escape => UIManager::unselect_view(),
            NamedKey::ArrowLeft => {
                if let Some((start, _)) = self.selection().filter(|_| !shift) {
                    self.move_caret(start, false);
                    return;
                }
                let text = self.entered_text();
                let target = text[..self.caret].chars().last().map_or(0, |ch| self.caret - ch.len_utf8());
                self.move_caret(target, shift);
            }
            NamedKey::ArrowRight => {
                if let Some((_, end)) = self.selection().filter(|_| !shift) {
                    self.move_caret(end, false);
                    return;
                }
                let text = self.entered_text();
                let target = text[self.caret..]
                    .chars()
                    .next()
                    .map_or(self.caret, |ch| self.caret + ch.len_utf8());
                self.move_caret(target, shift);
            }
            NamedKey::Home | NamedKey::End => {
                let layout = self.layout();
                let line = &layout.lines[layout.line_of(self.display_byte(self.caret))];
                let target = if key == NamedKey::Home {
                    line.start
                } else {
                    line.end
                };
                self.move_caret(self.entered_byte(target), shift);
            }
            NamedKey::ArrowUp | NamedKey::ArrowDown => {
                let layout = self.layout();
                let (line, x) = layout.position_of(self.display_byte(self.caret));
                let target = if key == NamedKey::ArrowUp {
                    line.checked_sub(1)
                } else {
                    (line + 1 < layout.lines.len()).then_some(line + 1)
                };
                if let Some(target) = target {
                    let byte = layout.nearest_on_line(target, x);
                    self.move_caret(self.entered_byte(byte), shift);
                }
            }
            _ => {}
        }
    }

    /// Puts the caret at `byte`. With `extend` the anchor stays where the
    /// selection started, or starts at the old caret, so the selection
    /// grows. Without it any selection clears.
    fn move_caret(mut self: Weak<Self>, byte: usize, extend: bool) {
        if extend {
            if self.anchor.is_none() {
                self.anchor = Some(self.caret);
            }
        } else {
            self.anchor = None;
        }
        self.caret = byte;
        self.update_caret();
    }

    /// Replaces the selection, or inserts at the caret, with `text`.
    pub(super) fn insert(mut self: Weak<Self>, text: &str) {
        let mut current = self.entered_text();

        if let Some((start, end)) = self.selection() {
            current.replace_range(start..end, "");
            self.caret = start;
        }
        self.anchor = None;

        let mut inserted = String::new();
        for ch in text.chars() {
            let probe = format!("{current}{inserted}");
            if self.constraint.accept_char(ch, &probe) {
                inserted.push(ch);
            }
        }

        if inserted.is_empty() {
            self.update_caret();
            return;
        }

        let caret = self.caret.min(current.len());
        current.insert_str(caret, &inserted);
        self.caret = caret + inserted.len();
        self.commit(current);
    }

    fn delete_selection(mut self: Weak<Self>) -> bool {
        let Some((start, end)) = self.selection() else {
            return false;
        };
        let mut text = self.entered_text();
        text.replace_range(start..end, "");
        self.caret = start;
        self.anchor = None;
        self.commit(text);
        true
    }

    fn delete_backward(mut self: Weak<Self>) {
        if self.delete_selection() {
            return;
        }

        let mut text = self.entered_text();

        let Some(ch) = text[..self.caret].chars().last() else {
            return;
        };

        self.caret -= ch.len_utf8();
        text.remove(self.caret);
        self.commit(text);
    }

    /// Shows typed text without moving the caret, unlike `set_text`
    /// which puts the caret at the end.
    fn commit(mut self: Weak<Self>, text: String) {
        let caret = self.caret;
        self.set_text(text);
        self.caret = caret;
        self.update_caret();
    }

    /// The caret position under `position`, in the field's coordinates.
    fn byte_at(&self, position: Point) -> usize {
        if self.placeholding {
            return 0;
        }

        let layout = self.layout();
        let (top, line_starts) = self.line_origins(&layout);

        // The content is drawn shifted by the offset, which is zero or
        // negative, so a tap maps into the content by taking it back off.
        let y = position.y - self.scroll.get_scroll_content_offset();

        let line_index: usize = ((y - top) / layout.line_height)
            .floor()
            .clamp(0.0, (layout.lines.len() - 1).to_f32())
            .lossy_convert();

        let x = position.x - line_starts[line_index];
        self.entered_byte(layout.nearest_on_line(line_index, x))
    }

    pub(super) fn on_touch_began(mut self: Weak<Self>, position: Point) {
        let byte = self.byte_at(position);
        let now = Instant::now();

        let double = self.last_tap.is_some_and(|(at, where_)| {
            now.duration_since(at) < DOUBLE_CLICK_INTERVAL
                && (where_ - position).length() < DOUBLE_CLICK_DISTANCE
        });
        self.last_tap = Some((now, position));

        if double {
            self.select_word_at(byte);
            return;
        }

        if Input::modifiers().shift_key() {
            self.move_caret(byte, true);
            return;
        }

        self.caret = byte;
        self.anchor = Some(byte);
        self.update_caret();
    }

    pub(super) fn on_touch_moved(mut self: Weak<Self>, position: Point) {
        if self.anchor.is_none() {
            return;
        }
        self.caret = self.byte_at(position);
        self.update_caret();
    }

    pub(super) fn on_touch_ended(mut self: Weak<Self>) {
        if self.anchor == Some(self.caret) {
            self.anchor = None;
            self.update_caret();
        }
    }

    /// Selects the run of letters and digits around `byte`.
    fn select_word_at(mut self: Weak<Self>, byte: usize) {
        let text = self.entered_text();
        let is_word = |ch: char| ch.is_alphanumeric();

        let start = text[..byte]
            .char_indices()
            .rev()
            .take_while(|(_, ch)| is_word(*ch))
            .last()
            .map_or(byte, |(index, _)| index);
        let end = text[byte..]
            .char_indices()
            .find(|(_, ch)| !is_word(*ch))
            .map_or(text.len(), |(index, _)| byte + index);

        self.anchor = Some(start);
        self.caret = end;
        self.update_caret();
    }

    /// The y of the first line's top and the x every line starts at, in
    /// the scroll content's coordinates, mirroring how the drawer places
    /// text.
    fn line_origins(&self, layout: &TextLayout) -> (f32, Vec<f32>) {
        let frame = self.label.frame();
        let inset = self.label.text_inset();

        let top = match self.label.vertical_alignment {
            VerticalAlignment::Top => frame.y(),
            VerticalAlignment::Center => frame.y() + frame.height() / 2.0 - layout.total_height() / 2.0,
        };

        let starts = layout
            .lines
            .iter()
            .map(|line| match self.label.alignment {
                TextAlignment::Left => frame.x() + inset,
                TextAlignment::Center => frame.x() + (frame.width() - line.width) / 2.0,
                TextAlignment::Right => frame.max_x() - inset - line.width,
            })
            .collect();

        (top, starts)
    }

    /// Sizes the label to the field, or in multiline mode to its lines
    /// when they need more, so the scroll content grows with the text.
    pub(super) fn update_layout(mut self: Weak<Self>) {
        let width = self.width();
        let height = self.height();

        if self.multiline {
            self.label
                .set_frame((0.0, MULTILINE_TOP_INSET, width, height - MULTILINE_TOP_INSET));
            let content = self.layout().total_height();
            let label_height = content.max(height - MULTILINE_TOP_INSET);
            self.label.set_frame((0.0, MULTILINE_TOP_INSET, width, label_height));
            self.scroll.set_content_height(label_height + MULTILINE_TOP_INSET);
        } else {
            self.label.set_frame((0.0, 0.0, width, height));
            self.scroll.set_content_height(height);
        }
    }

    pub(super) fn update_caret(mut self: Weak<Self>) {
        self.update_layout();

        if !self.is_editing {
            self.caret_view.set_hidden(true);
            self.clear_selection_views();
            return;
        }

        let layout = self.layout();
        let (top, line_starts) = self.line_origins(&layout);
        let (line, x) = layout.position_of(self.display_byte(self.caret));

        let height = layout.ascent - layout.descent;
        let y = top + line.to_f32() * layout.line_height;

        self.caret_view.set_hidden(false);
        self.caret_view.set_frame((line_starts[line] + x, y, 1.0, height));

        self.update_selection_views(&layout, top, &line_starts);

        // Keep the caret line in view, like a text area follows typing.
        let field_height = self.height();
        let visible_top = -self.scroll.get_scroll_content_offset();
        let visible_bottom = visible_top + field_height;
        let line_bottom = y + layout.line_height;

        if line_bottom > visible_bottom {
            self.scroll.set_content_offset(field_height - line_bottom);
        } else if y < visible_top {
            self.scroll.set_content_offset(-y);
        }
    }

    fn clear_selection_views(mut self: Weak<Self>) {
        for mut view in self.selection_views.drain(..) {
            view.remove_from_superview();
        }
    }

    /// One translucent rectangle per line of the selection, behind the
    /// glyphs.
    fn update_selection_views(mut self: Weak<Self>, layout: &TextLayout, top: f32, line_starts: &[f32]) {
        self.clear_selection_views();

        let Some((start, end)) = self.selection() else {
            return;
        };
        let (start, end) = (self.display_byte(start), self.display_byte(end));

        let label_z = self.label.z_position();

        for (index, line) in layout.lines.iter().enumerate() {
            if line.end < start || line.start > end {
                continue;
            }

            let from = layout.x_on_line(index, start.max(line.start));
            let to = layout.x_on_line(index, end.min(line.end));

            if to <= from {
                continue;
            }

            let rect = Rect::new(
                line_starts[index] + from,
                top + index.to_f32() * layout.line_height,
                to - from,
                layout.line_height,
            );

            let view = self.scroll.add_view::<Container>();
            view.set_color(SELECTION_COLOR);
            view.set_frame(rect);
            // Added after the label, so it would draw over the glyphs.
            // Push it just behind the label, still in front of the field.
            view.__base_view().z_position = label_z + UIManager::additional_z_offset();
            self.selection_views.push(view);
        }
    }
}
