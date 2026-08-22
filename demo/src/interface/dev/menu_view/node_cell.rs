use hilen::{
    gm::LossyConvert,
    refs::Weak,
    ui::{Anchor::Left, Button, Label, TextAlignment, UIImages, ViewData, view},
};

use crate::interface::dev::{MenuEntry, Node};

#[view]
pub struct NodeCell {
    #[init]
    pub button: Button,
    pub label:  Label,
}

impl NodeCell {
    pub fn set_node(self: Weak<Self>, node: &Node<MenuEntry>) {
        self.button.set_image(if node.open {
            UIImages::down()
        } else {
            UIImages::right()
        });

        self.label.set_text(node.value.label);

        self.button
            .place()
            .clear()
            .l(5.0 + node.depth.lossy_convert() * 28.0)
            .center_y()
            .size(if node.is_leaf() { 0 } else { 20 }, 20);
        self.label
            .set_alignment(TextAlignment::Left)
            .place()
            .clear()
            .anchor(Left, self.button, 5)
            .trb(2.5);
    }
}
