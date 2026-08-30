use crate::{
    deps::refs::Own,
    inspect::protocol::ui::ViewRepr,
    ui::{Button, Label, TextField, View, ViewData, ViewFrame, ViewSubviews, WeakView},
};

pub trait ViewToInspect {
    fn view_to_inspect(&self) -> Own<ViewRepr>;
}

impl<T: View + ?Sized> ViewToInspect for T {
    fn view_to_inspect(&self) -> Own<ViewRepr> {
        Own::new(ViewRepr {
            label:          self.label().to_string(),
            id:             weak_to_id(self.weak_view()),
            frame:          *self.frame(),
            content_offset: self.content_offset(),
            color:          *self.color(),
            text:           text_of(self.weak_view()),
            hidden:         self.is_hidden(),
            placer:         self.placer_copy(),
            subviews:       self
                .subviews()
                .iter()
                .filter(|v| !v.is_system())
                .map(|v| v.view_to_inspect())
                .collect(),
        })
    }
}

pub(crate) fn text_of(view: WeakView) -> Option<String> {
    if let Some(label) = view.downcast::<Label>() {
        return Some(label.text().to_string());
    }
    if let Some(button) = view.downcast::<Button>() {
        return Some(button.text().to_string());
    }
    if let Some(field) = view.downcast::<TextField>() {
        return Some(field.text().to_string());
    }
    None
}

// Public for the inspect tests, which address a view the way the CLI
// does, by its wire id.
pub fn weak_to_id(weak_view: WeakView) -> String {
    let raw = weak_view.raw();
    format!(
        "{}:{}",
        hex::encode(raw.addr().to_le_bytes()),
        hex::encode(raw.stamp().to_le_bytes())
    )
}
