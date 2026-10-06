//! The caret, the selection, the keys and the clipboard of a text field.

#[cfg(not_wasm)]
use std::thread;

#[cfg(not_wasm)]
use anyhow::Error;
use plat::Platform;
use zeroize::Zeroizing;

use super::{MASK, MULTILINE_TOP_INSET, MultilineEnter, SYSTEM_EDITS, TextField, mask};
#[cfg(not_wasm)]
use crate::deps::hreads::on_main;
use crate::{
    deps::refs::Weak,
    gm::{
        LossyConvert, ToF32,
        flat::{Point, Rect},
    },
    system::Clipboard,
    ui::{
        Container, Input, TextAlignment, TouchStack, UIManager, VerticalAlignment, View, ViewSubviews,
        WeakView,
        selection_drawer::SELECTION_COLOR,
        text_selection::word_range,
        view::{ViewData, ViewFrame, ViewTouch},
    },
    window::{NamedKey, TextLayout},
    wipe::joined,
};

// The entered text, the one the caret indexes, is read through `text`,
// which borrows it. Nothing here copies it to read it, a copy of a secure
// field would be freed with the password still in it. An edit builds the
// new text with `joined`, in one buffer that wipes itself.
impl TextField {
    /// A byte index into the entered text as a byte index into the
    /// displayed text. The two differ only in a secure field, where every
    /// character becomes one mask character.
    fn display_byte(&self, byte: usize) -> usize {
        if !self.is_secure() {
            return byte;
        }
        self.text()[..byte].chars().count() * MASK.len_utf8()
    }

    /// The inverse of `display_byte`.
    fn entered_byte(&self, display: usize) -> usize {
        if !self.is_secure() {
            return display;
        }
        let chars = display / MASK.len_utf8();
        let text = self.text();
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

    /// The layout of the displayed text, which the label holds: the
    /// entered text, or its mask in a secure field. While the field is
    /// empty the label holds the placeholder and nothing is displayed.
    fn layout(&self) -> TextLayout {
        let displayed = if self.placeholding { "" } else { self.label.text() };
        self.label.text_layout_for(displayed)
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
            self.insert(ch.encode_utf8(&mut [0; 4]));
        }
    }

    fn on_command(mut self: Weak<Self>, ch: char) {
        match ch.to_ascii_lowercase() {
            'a' => {
                let len = self.text().len();
                self.anchor = Some(0);
                self.caret = len;
                self.update_caret();
            }
            'c' => self.copy(),
            'x' => {
                self.copy();
                self.delete_selection();
            }
            #[cfg(not_wasm)]
            'v' => self.paste(),
            _ => {}
        }
    }

    fn copy(self: Weak<Self>) {
        // A field that hides its text copies nothing.
        if self.is_secure() {
            return;
        }
        let Some((start, end)) = self.selection() else {
            return;
        };
        let selected = &self.text()[start..end];

        // A field that shows its secret copies it the way a secret is
        // copied, kept on this device and out of clipboard managers.
        let copied = if self.holds_secret() {
            Clipboard::set_secret(selected)
        } else {
            Clipboard::set_text(selected)
        };
        if let Err(err) = copied {
            log::error!("Failed to copy from a text field: {err}");
        }
    }

    // A page cannot read the clipboard synchronously, so the browser has
    // no paste shortcut.
    #[cfg(not_wasm)]
    fn paste(self: Weak<Self>) {
        match Clipboard::get_text() {
            // Wiped after the insert, a paste into a secure field is the
            // usual way a password gets there.
            Ok(text) => self.insert(&Zeroizing::new(text)),
            Err(no_text) => self.paste_image(no_text),
        }
    }

    /// The clipboard holds no text. A picture in it goes to the owner
    /// through `image_pasted`. Making its png file takes up to half a
    /// second for a big screenshot, so it runs off the main thread and
    /// the event fires some frames later.
    #[cfg(not_wasm)]
    fn paste_image(self: Weak<Self>, no_text: Error) {
        thread::spawn(move || {
            let image = Clipboard::get_image();
            on_main(move || match image {
                // The field can be gone by now.
                Ok(Some(image)) if self.is_ok() => {
                    log::debug!("Pasted a picture of {} by {} pixels", image.width, image.height);
                    self.image_pasted.trigger(image);
                }
                Ok(Some(_)) => log::debug!("A pasted picture came after its text field was gone"),
                Ok(None) => log::warn!("Nothing to paste: {no_text}"),
                Err(err) => log::warn!("Failed to read a picture to paste: {err}"),
            });
        });
    }

    pub(super) fn on_key(self: Weak<Self>, key: NamedKey) {
        if self.is_null() || !self.is_selected() {
            return;
        }

        let shift = Input::modifiers().shift_key();

        match key {
            // Focus moves even in a multiline field, the way a browser
            // text area treats Tab, so it sits above the Enter case and
            // never inserts a tab character.
            NamedKey::Tab => self.select_next_field(shift),
            // A chat box sends on a plain Enter and stays in edit for
            // the next message. A phone keyboard has no Shift to ask for
            // a new line with, so there Enter stays the new line.
            NamedKey::Enter
                if self.multiline
                    && self.multiline_enter == MultilineEnter::Submit
                    && !shift
                    && !Platform::MOBILE =>
            {
                let text = self.text().to_string();
                self.submitted.trigger(text);
            }
            NamedKey::Enter if self.multiline => self.insert("\n"),
            NamedKey::Enter => {
                let text = if self.holds_secret() {
                    mask(self.text())
                } else {
                    self.text().to_string()
                };
                UIManager::unselect_view();
                self.submitted.trigger(text);
            }
            NamedKey::Escape => UIManager::unselect_view(),
            NamedKey::ArrowLeft => {
                if let Some((start, _)) = self.selection().filter(|_| !shift) {
                    self.move_caret(start, false);
                    return;
                }
                let caret = self.caret;
                let target = self.text()[..caret].chars().last().map_or(0, |ch| caret - ch.len_utf8());
                self.move_caret(target, shift);
            }
            NamedKey::ArrowRight => {
                if let Some((_, end)) = self.selection().filter(|_| !shift) {
                    self.move_caret(end, false);
                    return;
                }
                let caret = self.caret;
                let target = self.text()[caret..].chars().next().map_or(caret, |ch| caret + ch.len_utf8());
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

    /// Tab order is view tree order, the order fields were added, which
    /// matches how a form lays them out. The walk is scoped to the top
    /// touch layer, so a modal cycles only its own fields and Tab never
    /// reaches a field under the scrim.
    fn select_next_field(self: Weak<Self>, backward: bool) {
        fn collect(view: WeakView, fields: &mut Vec<Weak<TextField>>) {
            if view.is_null() || view.is_hidden() {
                return;
            }
            if let Some(field) = view.downcast_view::<TextField>() {
                fields.push(field);
                return;
            }
            for sub in view.subviews() {
                collect(sub.weak_view(), fields);
            }
        }

        let mut fields = Vec::new();
        collect(TouchStack::top_layer_root(), &mut fields);

        let this = self.weak_view().raw();
        let Some(current) = fields.iter().position(|field| field.weak_view().raw() == this) else {
            return;
        };

        let next = if backward {
            (current + fields.len() - 1) % fields.len()
        } else {
            (current + 1) % fields.len()
        };

        // Selecting the already selected field is a no-op in
        // `set_selected`, so a lone field keeps its editing session.
        fields[next].focus();
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
        let selection = self.selection();
        if let Some((start, _)) = selection {
            self.caret = start;
        }
        self.anchor = None;

        // Only a constraint needs a copy, the chars it accepts.
        let accepted = self.constraint.as_ref().map(|constraint| constraint.filter(text));
        let inserted = accepted.as_ref().map_or(text, |accepted| accepted.as_str());

        if inserted.is_empty() {
            self.update_caret();
            return;
        }

        let current = self.text();
        let (start, end) = selection.unwrap_or_else(|| {
            let caret = self.caret.min(current.len());
            (caret, caret)
        });
        let edited = joined(&[&current[..start], inserted, &current[end..]]);

        self.caret = start + inserted.len();
        self.commit(edited);
    }

    fn delete_selection(mut self: Weak<Self>) -> bool {
        let Some((start, end)) = self.selection() else {
            return false;
        };
        let current = self.text();
        let edited = joined(&[&current[..start], &current[end..]]);

        self.caret = start;
        self.anchor = None;
        self.commit(edited);
        true
    }

    fn delete_backward(mut self: Weak<Self>) {
        if self.delete_selection() {
            return;
        }

        let caret = self.caret;
        let current = self.text();

        let Some(ch) = current[..caret].chars().last() else {
            return;
        };

        let start = caret - ch.len_utf8();
        let edited = joined(&[&current[..start], &current[caret..]]);

        self.caret = start;
        self.commit(edited);
    }

    /// Shows typed text without moving the caret, unlike `set_text`
    /// which puts the caret at the end.
    fn commit(mut self: Weak<Self>, text: Zeroizing<String>) {
        let caret = self.caret;
        self.replace_text(text);
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

        // A double click selects the word, and so does every fast click
        // after it.
        if self.clicks.tap_in_row(position) {
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
        let word = word_range(self.text(), byte);

        self.anchor = Some(word.start);
        self.caret = word.end;
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

        if !self.is_editing || SYSTEM_EDITS {
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
