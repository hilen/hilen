use anyhow::{Result, ensure};

use crate::{
    deps::{hreads::from_main, refs::Weak},
    ui::{Setup, TextField, UIManager, View, ViewData, ViewTest, view},
    ui_test::inject_keys,
};

/// A field must start its editing session on `focus` when the selection
/// still holds the pointer of a dead view at the same memory address. A
/// selected view that is removed stays in the selection as a dead pointer,
/// and the allocator can give its address to the next field. The selection
/// compared only the address, so that field counted as already selected,
/// subscribed to no keys and got no text. This was the rare failure of
/// `Inspect keys` in the second round of a suite run.
///
/// An allocator gives a freed address back when it likes, so the test puts
/// the dead pointer in place itself: the field's own address with a stamp
/// no live view has.
#[view]
struct StaleSelection {
    #[init]
    field: TextField,
}

impl Setup for StaleSelection {
    fn setup(self: Weak<Self>) {
        self.field.place().tl(20).size(300, 40);
    }
}

impl ViewTest for StaleSelection {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        from_main(move || {
            let mut dead = view.field.weak_view();
            dead.stamp = dead.stamp.wrapping_add(1);
            assert!(!dead.is_ok(), "the stamp of the pointer is still a live one");
            UIManager::select_dead_view(dead);

            view.field.focus();
        });
        inject_keys("abc");
        let text = from_main(move || view.field.text().to_string());
        ensure!(
            text == "abc",
            "the field at the address of a dead selected view got {text:?}"
        );
        Ok(())
    }
}
