use ui_proc::view;

use crate::{
    deps::{
        refs::{Weak, weak_from_ref},
        vents::Event,
    },
    gm::{
        ToF32,
        color::{BLACK, CLEAR, Color, GRAY, LIGHTER_GRAY, WHITE},
    },
    ui::{
        Label, Setup, TextAlignment, TextFieldConstraint, ToLabel, UIEvents, UIManager,
        text_field_constraint::AcceptChar,
        view::{ViewData, ViewFrame, ViewTouch},
    },
    window::NamedKey,
};

#[view]
pub struct TextField {
    pub(crate) constraint: Option<TextFieldConstraint>,

    placeholder:      String,
    text_color:       Color,
    selected_color:   Color,
    background_color: Color,
    placeholding:     bool,
    is_editing:       bool,

    pub changed: Event<String>,

    pub editing_ended: Event<String>,

    #[init]
    label: Label,
}

impl Setup for TextField {
    fn setup(mut self: Weak<Self>) {
        self.text_color = BLACK;
        self.selected_color = GRAY;
        self.placeholding = true;
        self.label.place().back();
        self.label.set_text_color(LIGHTER_GRAY);
        self.label.set_color(CLEAR);
        self.set_color(WHITE);

        self.enable_touch();
    }

    fn on_selection_changed(mut self: Weak<Self>, selected: bool) {
        self.is_editing = selected;

        if selected {
            UIEvents::keyboard_key().val(self, |key| {
                if matches!(key, NamedKey::Enter) {
                    UIManager::unselect_view();
                }
            });

            UIEvents::keyboard_input().val(self, move |key| {
                if self.is_null() {
                    return;
                }

                let mut text = self.label.text().to_string();

                let backspace = key as u32 == 8;

                if self.is_selected() {
                    if backspace {
                        if self.placeholding {
                            return;
                        }
                        text.pop();
                    } else if self.constraint.accept_char(key, &text) {
                        if self.placeholding {
                            text = String::default();
                            self.placeholding = false;
                        }
                        text.push(key);
                    }
                }

                self.set_text(text);
                self.changed.trigger(self.label.text().to_string());
            });
            UIManager::open_keyboard(self.absolute_frame());
        } else {
            if let Some(string) = UIManager::close_keyboard() {
                self.set_text(string);
            }
            UIEvents::keyboard_input().unsubscribe(self);
            UIEvents::keyboard_key().unsubscribe(self);

            self.editing_ended.trigger(self.label.text().to_string());
        }

        let color = if selected {
            self.background_color = *self.color();
            self.selected_color
        } else {
            self.background_color
        };

        self.set_color(color);
    }
}

impl TextField {
    pub fn set_alignment(&mut self, alignment: TextAlignment) -> &mut Self {
        self.label.set_alignment(alignment);
        self
    }

    pub fn text(&self) -> &str {
        self.label.text()
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

        self.changed.trigger(text);
        self
    }

    pub(crate) fn is_editing(&self) -> bool {
        self.is_editing
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

    pub fn set_selected_color(&self, color: impl Into<Color>) -> &Self {
        let color = color.into();
        weak_from_ref(self).selected_color = color;
        self
    }

    pub fn set_text_color(&self, color: impl Into<Color>) -> &Self {
        let color = color.into();
        weak_from_ref(self).text_color = color;
        self.label.set_text_color(color);
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
