use anyhow::Result;
use hilen::{
    refs::Weak,
    ui::{Setup, Switch, ViewData, ViewTest, view},
    ui_test::{state::set_state, test_combinations},
};

#[view]
struct SwitchTestView {
    #[init]
    switch: Switch,
}

impl Setup for SwitchTestView {
    fn setup(self: Weak<Self>) {
        self.switch.place().size(100, 50).center();

        self.switch.selected.val(|on| {
            set_state(on);
        });
    }
}

impl ViewTest for SwitchTestView {
    fn canvas() -> (u32, u32) {
        (200, 100)
    }

    fn perform_test(_view: Weak<Self>) -> Result<()> {
        test_combinations([
            (
                r"
                174.58594    49.171875    b
                114.09766    45.835938    e
                98.78125     10.671875    b
                95.50391     49.164063    e
                18.003906    50.20703     b
                66.41797     48.191406    e
                119.44531    86.00391     b
                118.953125   47.95703     e
                ",
                false,
            ),
            (
                r"
                56.40625     35.191406    b
                56.40625     35.1875      e
                141.73047    37.035156    b
                141.66406    37.035156    e
                140.44531    69.25        b
                140.3789     69.25        e
                56.01172     69.88672     b
                56.04297     69.88672     e
                100.87109    50.507813    b
                100.80469    50.507813    e
                ",
                true,
            ),
            (
                r"
                98.99219     54.15625     b
                98.99219     54.15625     e
                98.99219     54.15625     b
                98.99219     54.15625     e
                ",
                true,
            ),
            (
                r"
                98.99219     54.15625     b
                98.99219     54.15625     e
                ",
                false,
            ),
        ])?;

        Ok(())
    }
}
