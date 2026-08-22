use ui_proc::view;

use crate::{
    deps::refs::Weak,
    gm::color::GRAY,
    ui::{Button, Container, Setup, ViewData, ViewSubviews},
};

const A: &[char] = &['q', 'w', 'e', 'r', 't', 'y', 'u', 'i', 'o', 'p'];
const B: &[char] = &['a', 's', 'd', 'f', 'g', 'h', 'j', 'k', 'l'];
const C: &[char] = &['z', 'x', 'c', 'v', 'b', 'n', 'm'];

const LAYOUT: &[&[char]] = &[A, B, C];

#[view]
pub struct KeyboardView {}

impl Setup for KeyboardView {
    fn setup(self: Weak<Self>) {
        self.place().all_ver();

        for (i, row) in LAYOUT.iter().enumerate() {
            let container = self.add_view::<Container>();

            if i == 1 {
                container.place().lr(20);
            }
            if i == 2 {
                container.place().lr(80);
            }

            container.place().all_hor();
            for button in *row {
                container.add_view::<Button>().set_text(button).outline(GRAY);
            }
        }
    }
}
