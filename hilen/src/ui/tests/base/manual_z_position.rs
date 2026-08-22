use anyhow::{Result, ensure};

use crate::{
    deps::{hreads::from_main, refs::Weak},
    ui::{Container, Setup, UIManager, ViewFrame, ViewSubviews, ViewTest, view},
};

// Regression test. Auto z assignment used to treat z == 0.5 as "not set",
// so a manually set 0.5 was silently overwritten when the view was added.
#[view]
struct ManualZPosition {}

impl Setup for ManualZPosition {
    fn setup(self: Weak<Self>) {}
}

impl ViewTest for ManualZPosition {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        let z = from_main(move || {
            let mut container = Container::new();
            container.set_z_position(UIManager::ROOT_VIEW_Z_OFFSET);
            let added = view.add_subview(container);
            added.z_position()
        });

        ensure!(
            (z - UIManager::ROOT_VIEW_Z_OFFSET).abs() < f32::EPSILON,
            "manually set z_position {} was overwritten: {z}",
            UIManager::ROOT_VIEW_Z_OFFSET
        );

        Ok(())
    }
}
