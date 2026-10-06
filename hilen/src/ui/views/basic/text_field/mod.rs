mod editing;
#[cfg(all(ios, not(tvos)))]
mod system_field;

use std::mem::take;

use ui_proc::view;
use zeroize::Zeroizing;

#[cfg(all(ios, not(tvos)))]
use crate::ui::view::ViewCallbacks;
use crate::{
    deps::{
        refs::{Weak, weak_from_ref},
        vents::Event,
    },
    gm::{
        ToF32,
        color::{BLACK, CLEAR, GRAY, LIGHTER_GRAY, WHITE},
    },
    system::ClipboardImage,
    ui::{
        Container, Label, ScrollView, Setup, TextAlignment, TextFieldConstraint, ToLabel, UIColor, UIEvents,
        UIManager, VerticalAlignment, ViewSubviews,
        view::{DoubleTap, View, ViewData, ViewFrame, ViewTouch},
    },
    window::Font,
};

/// Space above the first line of a multiline field, so the text does not
/// touch the top edge the way a text area does not.
const MULTILINE_TOP_INSET: f32 = 8.0;

/// On an iPhone a system text field covers the field while it is edited and
/// draws the text, the caret and the selection itself, see `system_field`.
pub(super) const SYSTEM_EDITS: bool = cfg!(all(ios, not(tvos)));

/// What a secure field draws for every character.
pub(super) const MASK: char = '\u{2022}';

pub(super) fn mask(text: &str) -> String {
    std::iter::repeat_n(MASK, text.chars().count()).collect()
}

/// What a plain Enter does in a multiline field.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(super) enum MultilineEnter {
    #[default]
    NewLine,
    /// Fires `submitted` and keeps the field in edit, a chat box.
    Submit,
}

#[view]
pub struct TextField {
    pub(crate) constraint: Option<TextFieldConstraint>,

    placeholder:       String,
    text_color:        UIColor,
    selected_color:    UIColor,
    background_color:  UIColor,
    placeholder_color: Option<UIColor>,
    placeholding:      bool,
    is_editing:        bool,
    multiline:         bool,
    /// What a plain Enter does in a multiline field.
    multiline_enter:   MultilineEnter,

    /// The real text of a secure field, `None` for a plain one. The label
    /// only ever shows one mask character per character of it. Every
    /// change puts a new buffer here, and the old one writes zeros over
    /// itself before its memory is freed, so does the last one when the
    /// field is dropped.
    secret: Option<Zeroizing<String>>,

    /// Byte index into the text where typing inserts.
    caret: usize,

    /// The other end of the selection, the caret being the moving end.
    /// `None` or equal to the caret means nothing is selected.
    anchor: Option<usize>,

    /// The click before, to tell a double click.
    clicks: DoubleTap,

    pub changed: Event<String>,

    pub editing_ended: Event<String>,

    /// Enter in a single line field. It fires after `editing_ended`, with
    /// the same text, so a form confirms on it the way its button does.
    /// A multiline field fires it only with `set_submit_on_enter`, Enter
    /// is a new line there otherwise.
    pub submitted: Event<String>,

    /// A paste that found a picture in the clipboard and no text, with
    /// that picture as a png file. The field itself takes nothing from
    /// it, the owner keeps the picture. It fires a moment after the keys,
    /// the png file is made on another thread. Only the paste shortcut
    /// of a desktop fires it, see `Clipboard::get_image` for the rest.
    pub image_pasted: Event<ClipboardImage>,

    /// All of these live inside the scroll content so a multiline field
    /// scrolls its lines, the selection and the caret together. A single
    /// line field never scrolls, its content is exactly the field.
    label:           Weak<Label>,
    caret_view:      Weak<Container>,
    selection_views: Vec<Weak<Container>>,

    #[init]
    scroll: ScrollView,
}

impl Setup for TextField {
    fn setup(mut self: Weak<Self>) {
        self.text_color = BLACK.into();
        self.selected_color = GRAY.into();
        self.placeholding = true;

        self.scroll.place().back();
        // A drag inside a text field selects text, only the wheel scrolls.
        self.scroll.disable_drag();
        self.label = self.scroll.add_view::<Label>();
        self.caret_view = self.scroll.add_view::<Container>();

        self.label.set_text_color(LIGHTER_GRAY);
        self.label.set_color(CLEAR);
        self.set_color(WHITE);

        self.caret_view.set_color(BLACK).set_hidden(true);

        self.enable_touch();
        self.touch().began.val(move |touch| self.on_touch_began(touch.position));
        self.touch().moved.val(move |touch| self.on_touch_moved(touch.position));
        self.touch().all.val(move |touch| {
            if touch.is_ended() {
                self.on_touch_ended();
            }
        });
        // The caret and the selection are laid out in points from the frame
        // and from the text inset, and the inset is a fixed count of screen
        // pixels. So both move when the frame or the UI scale changes.
        self.size_changed().sub(move || self.update_caret());
        UIManager::on_scale_changed(self, move |_| {
            self.update_caret();
            // The system field counts in pixels, so its text size and its
            // insets are other numbers at the new scale.
            #[cfg(all(ios, not(tvos)))]
            if self.is_editing {
                system_field::begin(self);
            }
        });
        self.update_layout();
    }

    fn on_selection_changed(mut self: Weak<Self>, selected: bool) {
        // Closing the system field accepts a pending autocorrection, which
        // reports one last change. The field takes it only while it still
        // counts as edited.
        #[cfg(all(ios, not(tvos)))]
        if !selected {
            system_field::end();
        }

        self.is_editing = selected;

        if selected {
            UIEvents::keyboard_key().val(self, move |key| self.on_key(key));
            UIEvents::keyboard_input().val(self, move |key| self.on_char(key));
            #[cfg(all(ios, not(tvos)))]
            system_field::begin(self);
        } else {
            UIEvents::keyboard_input().unsubscribe(self);
            UIEvents::keyboard_key().unsubscribe(self);

            self.anchor = None;
            let ended_with = if self.holds_secret() {
                mask(self.text())
            } else {
                self.label.text().to_string()
            };
            self.editing_ended.trigger(ended_with);
        }

        let color = if selected {
            // Keep the original theme pair, not the resolved plain color,
            // so the field keeps following theme switches after editing.
            self.background_color = self.ui_color();
            self.selected_color
        } else {
            self.background_color
        };

        self.set_color(color);
        self.update_caret();
        self.show_or_hide_label();
    }
}

#[cfg(all(ios, not(tvos)))]
impl ViewCallbacks for TextField {
    fn update(&mut self) {
        if self.is_editing {
            system_field::follow(self);
        }
    }
}

impl TextField {
    pub fn set_alignment(&mut self, alignment: TextAlignment) -> &mut Self {
        self.label.set_alignment(alignment);
        self
    }

    /// The label font, so a field renders mono or any loaded face. The
    /// caret follows it, positions come from the same label layout.
    pub fn set_font(&self, font: Weak<Font>) -> &Self {
        self.label.set_font(font);
        self
    }

    /// A text area. Enter inserts a new line instead of ending editing,
    /// Escape or a tap outside ends it, lines wrap at the field width,
    /// start at the top and scroll when they do not fit.
    pub fn set_multiline(&self, multiline: bool) -> &Self {
        weak_from_ref(self).multiline = multiline;
        self.label.set_multiline(multiline);
        self.label.set_vertical_alignment(if multiline {
            VerticalAlignment::Top
        } else {
            VerticalAlignment::Center
        });
        weak_from_ref(self).update_layout();
        self
    }

    /// The Enter of a chat box. In a multiline field a plain Enter fires
    /// `submitted` with the text and the field stays in edit, the owner
    /// clears it and the user types on. Shift and Enter is the new line.
    /// A phone keyboard has no Shift, so there Return stays a new line
    /// and a button sends. A single line field ignores this, its Enter
    /// always submits.
    pub fn set_submit_on_enter(&self, submit: bool) -> &Self {
        weak_from_ref(self).multiline_enter = if submit {
            MultilineEnter::Submit
        } else {
            MultilineEnter::NewLine
        };
        self
    }

    /// What the user entered. The placeholder is a hint drawn by the label,
    /// never text, so an empty field reads as empty.
    ///
    /// The text is borrowed from the field, nothing is copied. A copy the
    /// caller makes of a secret is the caller's to wipe.
    pub fn text(&self) -> &str {
        if self.placeholding {
            return "";
        }
        match &self.secret {
            Some(secret) => secret.as_str(),
            None => self.label.text(),
        }
    }

    /// A password field. The entered text stays readable through `text`,
    /// the label shows one bullet per character and copy is disabled.
    ///
    /// From the first `set_secure(true)` on, the field holds a secret for
    /// good, also while `set_secure(false)` shows the text. Such a field:
    ///
    /// - writes zeros over its text before the memory is freed, on every change
    ///   and when the field is dropped, and makes no copy on the way that is
    ///   freed as it is,
    /// - never puts the text into `changed` and `editing_ended`, which hand a
    ///   `String` to the subscriber. Both carry one bullet per character, the
    ///   real text is read with `text`,
    /// - copies shown text with `Clipboard::set_secret`.
    ///
    /// What stays outside of this: a `String` given to `set_text` by
    /// value is the copy of the caller, single typed characters pass
    /// through the window system and the event queue as they are, on iOS
    /// the system text field that owns the keyboard keeps its own copy,
    /// and a shown text leaves glyphs behind like every drawn text, see
    /// `Label::set_secret`.
    pub fn set_secure(&self, secure: bool) -> &Self {
        if secure {
            self.label.set_secret(true);
        }
        let text = Zeroizing::new(self.text().to_owned());
        weak_from_ref(self).secret = secure.then(Zeroizing::default);
        self.replace_text(text)
    }

    pub fn is_secure(&self) -> bool {
        self.secret.is_some()
    }

    /// The field was secure at some point, see `set_secure`.
    pub(super) fn holds_secret(&self) -> bool {
        self.label.is_secret()
    }

    /// Height of the scrollable content, the lines plus the inset in
    /// multiline mode.
    pub fn content_height(&self) -> f32 {
        self.scroll.content_height()
    }

    /// Zero at the top, negative once scrolled down.
    pub fn scroll_offset(&self) -> f32 {
        self.scroll.get_scroll_content_offset()
    }

    /// Whether the caret line is inside the visible part of the field.
    pub fn scrolled_to_caret(&self) -> bool {
        let caret = self.caret_view.frame();
        let top = -self.scroll.get_scroll_content_offset();
        caret.y() >= top - 0.5 && caret.max_y() <= top + self.height() + 0.5
    }

    /// The selected text, empty when nothing is selected. A copy the
    /// caller owns, also of a secure field.
    pub fn selected_text(&self) -> String {
        match self.selection() {
            Some((start, end)) => self.text()[start..end].to_string(),
            None => String::new(),
        }
    }

    /// The field is empty and its label draws the placeholder.
    pub fn is_placeholding(&self) -> bool {
        self.placeholding
    }

    pub fn set_text(&self, text: impl ToLabel) -> &Self {
        self.replace_text(Zeroizing::new(text.to_label()))
    }

    /// Every change of the text ends here. The text comes in a buffer
    /// that wipes itself, so a part that is not kept, the chars a
    /// constraint drops or the whole text of a field that shows it, never
    /// stays behind in freed memory.
    pub(super) fn replace_text(&self, text: Zeroizing<String>) -> &Self {
        let mut this = weak_from_ref(self);

        let mut text = match &self.constraint {
            Some(constraint) => constraint.filter(&text),
            None => text,
        };
        let len = text.len();

        // A field that holds a secret never hands it to an event.
        let masked = self.holds_secret().then(|| mask(&text));

        if text.is_empty() && !self.placeholder.is_empty() {
            this.placeholding = true;
            self.label.set_text(self.placeholder.clone());
            self.label.set_text_color(self.placeholder_color.unwrap_or(LIGHTER_GRAY.into()));
        } else {
            this.placeholding = false;
            match &masked {
                Some(masked) if self.is_secure() => self.label.set_text(masked),
                _ => self.label.set_text(text.as_str()),
            };
            self.label.set_text_color(self.text_color);
        }

        let changed_to = masked.unwrap_or_else(|| take(&mut *text));

        if self.is_secure() {
            this.secret = Some(text);
        }

        this.caret = len;
        this.anchor = None;
        this.update_caret();
        self.show_or_hide_label();

        // A constraint that dropped a typed character, or a `set_text` call,
        // changed the text under the system field.
        #[cfg(all(ios, not(tvos)))]
        if self.is_editing {
            system_field::set_text(self.text());
        }

        self.changed.trigger(changed_to);
        self
    }

    /// The system field draws the entered text while it edits, so the
    /// label then shows only the placeholder of an empty field.
    fn show_or_hide_label(&self) {
        self.label.set_hidden(SYSTEM_EDITS && self.is_editing && !self.placeholding);
    }

    /// The user changed the text in the system field.
    #[cfg(all(ios, not(tvos)))]
    pub(super) fn system_text_changed(&self, text: Zeroizing<String>) {
        if self.text() != text.as_str() {
            self.replace_text(text);
        }
    }

    /// An editing session is open, the field has the keys.
    pub fn is_editing(&self) -> bool {
        self.is_editing
    }

    /// Programmatic focus, the same editing session a tap starts. The
    /// caret lands at the end of the entered text.
    pub fn focus(&self) {
        weak_from_ref(self).caret = self.text().len();
        weak_from_ref(self).anchor = None;
        UIManager::set_selected(self.weak_view(), true);
    }

    pub fn clear(&self) -> &Self {
        self.set_text("")
    }

    pub fn is_empty(&self) -> bool {
        self.text().is_empty()
    }

    pub fn float_only(&mut self) -> &mut Self {
        self.constraint = TextFieldConstraint::Float.into();
        self
    }

    pub fn integer_only(&self) -> &Self {
        weak_from_ref(self).constraint = TextFieldConstraint::Integer.into();
        self
    }

    pub fn set_selected_color(&self, color: impl Into<UIColor>) -> &Self {
        weak_from_ref(self).selected_color = color.into();
        self
    }

    pub fn set_text_color(&self, color: impl Into<UIColor>) -> &Self {
        let color = color.into();
        weak_from_ref(self).text_color = color;
        // With no explicit placeholder color the visible hint follows the
        // text color, the way it always did. With one it keeps its own.
        if !self.placeholding || self.placeholder_color.is_none() {
            self.label.set_text_color(color);
        }
        self.caret_view.set_color(color);
        self
    }

    /// Color of the hint text while the field is empty. Entering the first
    /// character swaps the label to the text color, clearing the field
    /// swaps it back. Theme pairs re-resolve on a switch like every other
    /// color.
    pub fn set_placeholder_color(&self, color: impl Into<UIColor>) -> &Self {
        let color = color.into();
        weak_from_ref(self).placeholder_color = Some(color);
        if self.placeholding {
            self.label.set_text_color(color);
        }
        self
    }

    pub fn set_text_size(&self, size: impl ToF32) -> &Self {
        self.label.set_text_size(size);
        self
    }

    pub fn set_placeholder(&self, placeholder: impl ToLabel) -> &Self {
        weak_from_ref(self).placeholder = placeholder.to_label();
        if self.placeholding {
            self.label.set_text(self.placeholder.clone());
            self.label.set_text_color(self.placeholder_color.unwrap_or(GRAY.into()));
        }
        self
    }
}
