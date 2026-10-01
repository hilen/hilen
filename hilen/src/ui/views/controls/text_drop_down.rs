use crate::{
    self as hilen,
    deps::{refs::Weak, vents::Event},
    gm::{
        ToF32,
        color::{CLEAR, Color, WHITE},
    },
    ui::{DropDown, DropDownData, Setup, UIColor, View, ViewData, view},
};

/// A drop down of plain texts. It owns the texts, shows one per row and
/// reports the picked one. For rows with anything more than a text write
/// an own view around `DropDown`, see `DropDownData`.
///
/// Its own color, border and corners are the look of the box.
#[view]
pub struct TextDropDown {
    values:  Vec<String>,
    changed: Event<String>,

    #[init]
    drop: DropDown,
}

impl TextDropDown {
    pub fn on_changed(&self, action: impl FnMut(String) + Send + 'static) {
        self.changed.val(action);
    }

    pub fn try_get_value(&self) -> Option<&str> {
        self.values.get(self.drop.selected_index()).map(String::as_str)
    }

    pub fn value(&self) -> &str {
        self.try_get_value().expect("the drop down has no values")
    }

    pub fn values(&self) -> &[String] {
        &self.values
    }

    /// The text shown while the drop down is collapsed.
    pub fn text(&self) -> &str {
        self.drop.text()
    }

    pub fn is_opened(&self) -> bool {
        self.drop.is_opened()
    }

    /// Replaces the texts. The pick falls back to the first one.
    pub fn set_values<T: ToString>(&mut self, values: impl IntoIterator<Item = T>) {
        self.values = values.into_iter().map(|value| value.to_string()).collect();
        self.drop.select(0);
        self.drop.reload();
    }

    /// Points the drop down at `value` and updates the collapsed text.
    /// The list stays closed and `changed` does not fire, so restoring a
    /// selection is never mistaken for a user pick. Returns false and
    /// changes nothing when the value is not among the current ones.
    pub fn set_value(&mut self, value: &str) -> bool {
        let Some(index) = self.values.iter().position(|existing| existing == value) else {
            return false;
        };
        self.drop.select(index)
    }

    /// Text color of the collapsed label and the rows.
    pub fn set_text_color(&mut self, color: impl Into<UIColor>) -> &mut Self {
        self.drop.set_text_color(color);
        self
    }

    pub fn set_text_size(&mut self, size: impl ToF32) -> &mut Self {
        self.drop.set_text_size(size);
        self
    }

    /// The color of the picked row, the check mark, the hover wash and
    /// the border while hovered or open.
    pub fn set_accent_color(&mut self, color: impl Into<Color>) -> &mut Self {
        self.drop.set_accent_color(color);
        self
    }
}

impl DropDownData for TextDropDown {
    fn number_of_rows(&self) -> usize {
        self.values.len()
    }

    fn title(&self, index: usize) -> String {
        self.values[index].clone()
    }

    fn row_selected(&mut self, index: usize) {
        self.changed.trigger(self.values[index].clone());
    }
}

impl Setup for TextDropDown {
    fn setup(mut self: Weak<Self>) {
        self.set_color(WHITE);

        self.drop.set_color(CLEAR);
        self.drop.place().back();
        let look = self.weak_view();
        self.drop.set_look_source(look);
        self.drop.set_data_source(self);
    }
}
