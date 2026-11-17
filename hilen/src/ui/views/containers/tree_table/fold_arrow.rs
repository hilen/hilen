use crate::{
    self as hilen,
    deps::{refs::Weak, vents::Event},
    ui::{
        Setup, ToolButton, UIColor, ViewAccessibility, ViewData,
        images::{FOLDED_SVG, UNFOLDED_SVG},
        view,
    },
};

/// The arrow at the left of a tree row with rows under it: right while the
/// row is folded, down while it is open. A row with no rows under it keeps
/// the room of the arrow, so the rows of 1 level line up.
#[view]
pub struct FoldArrow {
    /// The key of the row of the arrow, on a tap.
    pub tapped: Event<u64>,

    key: u64,

    #[init]
    button: ToolButton,
}

impl Setup for FoldArrow {
    fn setup(self: Weak<Self>) {
        self.button.set_centered(true);
        self.button.place().back();
        self.button.on_tap(move || self.tapped.trigger(self.key));
    }
}

impl FoldArrow {
    pub(super) fn show(mut self: Weak<Self>, key: u64, has_rows: bool, open: bool, tint: Option<UIColor>) {
        self.key = key;
        self.set_hidden(!has_rows);
        if open {
            self.button.set_icon_data(UNFOLDED_SVG, "hilen_tree_unfolded.svg");
            self.button.set_accessibility_label("Fold");
        } else {
            self.button.set_icon_data(FOLDED_SVG, "hilen_tree_folded.svg");
            self.button.set_accessibility_label("Unfold");
        }
        if let Some(tint) = tint {
            self.button.set_tint(tint);
        }
    }
}
