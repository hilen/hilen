mod editing;

use ui_proc::view;
use web_time::Instant;

use crate::{
    deps::{
        refs::{Weak, weak_from_ref},
        vents::Event,
    },
    gm::{
        ToF32,
        color::{BLACK, CLEAR, GRAY, LIGHTER_GRAY, WHITE},
        flat::Point,
    },
    ui::{
        Container, Label, ScrollView, Setup, TextAlignment, TextFieldConstraint, ToLabel, UIColor, UIEvents,
        UIManager, VerticalAlignment, ViewSubviews,
        view::{View, ViewData, ViewFrame, ViewTouch},
    },
};

/// Space above the first line of a multiline field, so the text does not
/// touch the top edge the way a text area does not.
const MULTILINE_TOP_INSET: f32 = 8.0;

#[view]
pub struct TextField {
    pub(crate) constraint: Option<TextFieldConstraint>,

    placeholder:      String,
    text_color:       UIColor,
    selected_color:   UIColor,
    background_color: UIColor,
    placeholding:     bool,
    is_editing:       bool,
    multiline:        bool,

    /// Byte index into the text where typing inserts.
    caret: usize,

    /// The other end of the selection, the caret being the moving end.
    /// `None` or equal to the caret means nothing is selected.
    anchor: Option<usize>,

    /// The last tap, to tell a double click.
    last_tap: Option<(Instant, Point)>,

    pub changed: Event<String>,

    pub editing_ended: Event<String>,

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
        self.size_changed().sub(move || self.update_layout());
        self.update_layout();
    }

    fn on_selection_changed(mut self: Weak<Self>, selected: bool) {
        self.is_editing = selected;

        if selected {
            UIEvents::keyboard_key().val(self, move |key| self.on_key(key));
            UIEvents::keyboard_input().val(self, move |key| self.on_char(key));
            UIManager::open_keyboard(self.absolute_frame());
        } else {
            if let Some(string) = UIManager::close_keyboard() {
                self.set_text(string);
            }
            UIEvents::keyboard_input().unsubscribe(self);
            UIEvents::keyboard_key().unsubscribe(self);

            self.anchor = None;
            self.editing_ended.trigger(self.label.text().to_string());
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
    }
}

impl TextField {
    pub fn set_alignment(&mut self, alignment: TextAlignment) -> &mut Self {
        self.label.set_alignment(alignment);
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

    pub fn text(&self) -> &str {
        self.label.text()
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

    /// The selected text, empty when nothing is selected.
    pub fn selected_text(&self) -> String {
        match self.selection() {
            Some((start, end)) => self.entered_text()[start..end].to_string(),
            None => String::new(),
        }
    }

    /// While empty the field shows its placeholder and `text` returns
    /// that placeholder, so a reader needs this to tell entered text
    /// from the hint.
    pub fn is_placeholding(&self) -> bool {
        self.placeholding
    }

    pub fn set_text(&self, text: impl ToLabel) -> &Self {
        let text = self.filter_constraint(text);

        if text.is_empty() && !self.placeholder.is_empty() {
            weak_from_ref(self).placeholding = true;
            self.label.set_text(self.placeholder.clone());
            self.label.set_text_color(LIGHTER_GRAY);
        } else {
            weak_from_ref(self).placeholding = false;
            self.label.set_text(&text);
            self.label.set_text_color(self.text_color);
        }

        weak_from_ref(self).caret = text.len();
        weak_from_ref(self).anchor = None;
        weak_from_ref(self).update_caret();

        self.changed.trigger(text);
        self
    }

    pub(crate) fn is_editing(&self) -> bool {
        self.is_editing
    }

    /// Programmatic focus, the same editing session a tap starts. The
    /// caret lands at the end of the entered text.
    pub fn focus(&self) {
        weak_from_ref(self).caret = self.entered_text().len();
        weak_from_ref(self).anchor = None;
        UIManager::set_selected(self.weak_view(), true);
    }

    pub fn clear(&self) -> &Self {
        self.set_text("")
    }

    pub fn is_empty(&self) -> bool {
        self.label.text().is_empty()
    }

    fn filter_constraint(&self, text: impl ToLabel) -> String {
        match &self.constraint {
            Some(constraint) => constraint.filter(text),
            None => text.to_label(),
        }
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
        self.label.set_text_color(color);
        self.caret_view.set_color(color);
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
            self.label.set_text_color(GRAY);
        }
        self
    }
}
