use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{
        Button, CellRegistry, Focus, Label, NamedKey, Setup, TableData, TableView, Touch, View, ViewData,
        ViewTest, ViewTouch, view,
    },
    ui_test::{checkpoint, inject_held_key, inject_named_key},
};

/// Longer than the hold that makes a long press, half a second.
const HOLD: f32 = 0.7;

/// Enter held on the key focus is a long press. It fires `secondary` on
/// the view under the ring and its release is no tap, like a held finger.
/// Enter down once sent the began and the ended touch in one call, so a
/// remote had no way to a long press, and a key that repeated while held
/// tapped again and again.
#[view]
struct KeyFocusLongPress {
    taps:     usize,
    holds:    usize,
    held_row: Option<usize>,
    selected: Option<usize>,

    #[init]
    title:  Label,
    button: Button,
    table:  TableView,
    state:  Label,
}

impl KeyFocusLongPress {
    fn show(self: Weak<Self>) {
        from_main(move || {
            self.state.set_text(format!(
                "taps: {}, holds: {}, held row: {:?}, selected: {:?}",
                self.taps, self.holds, self.held_row, self.selected
            ));
        });
        wait_for_next_frame();
    }
}

impl Setup for KeyFocusLongPress {
    fn setup(mut self: Weak<Self>) {
        self.title.set_text("Enter held is a long press").set_text_size(20);
        self.title.place().lrt(10).h(40);

        self.button.set_text("tap or hold").set_text_size(22);
        self.button.set_color("#cfd8e3").set_corner_radius(10);
        self.button.place().lr(60).t(60).h(60);
        self.button.on_tap(move || self.taps += 1);
        self.button.touch().secondary.sub(self, move || self.holds += 1);

        self.table.place().lr(60).t(140).b(70);
        self.table.set_data_source(self).register_cell::<Label>();
        self.table.touch().secondary.val(self, move |touch: Touch| {
            self.held_row = self.table.index_at(touch.position);
        });

        self.state.set_text_size(18);
        self.state.place().lrb(10).h(40);
    }
}

impl TableData for KeyFocusLongPress {
    fn cell_height(&self, _: usize) -> f32 {
        60.0
    }

    fn number_of_cells(&self) -> usize {
        5
    }

    fn setup_cell(&mut self, index: usize, registry: &mut CellRegistry) -> Weak<dyn View> {
        let label = registry.cell::<Label>();
        label.set_text(format!("row {}", index + 1)).set_text_size(22);
        label.set_color(if index.is_multiple_of(2) {
            "#e3e8ef"
        } else {
            "#f4f6f9"
        });
        label
    }

    fn cell_selected(&mut self, index: usize) {
        self.selected = Some(index);
    }
}

impl ViewTest for KeyFocusLongPress {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        from_main(move || Focus::set(view.button.weak_view()));
        wait_for_next_frame();

        inject_named_key(NamedKey::Enter);
        view.show();
        ensure!(
            from_main(move || (view.taps, view.holds)) == (1, 0),
            "a short Enter is not 1 tap"
        );

        inject_held_key(NamedKey::Enter, HOLD, 3);
        view.show();
        ensure!(
            from_main(move || (view.taps, view.holds)) == (1, 1),
            "a held Enter is not 1 long press and no tap"
        );
        checkpoint("1 tap and 1 hold on the button")?;

        inject_held_key(NamedKey::Enter, 0.05, 3);
        view.show();
        ensure!(
            from_main(move || (view.taps, view.holds)) == (2, 1),
            "the repeats of a held Enter tapped more than once"
        );

        inject_named_key(NamedKey::ArrowDown);
        wait_for_next_frame();
        inject_named_key(NamedKey::ArrowDown);
        wait_for_next_frame();
        ensure!(
            from_main(|| Focus::focused().tag()) == 1,
            "the ring is not on row 2 of the table"
        );

        inject_held_key(NamedKey::Enter, HOLD, 0);
        view.show();
        ensure!(
            from_main(move || (view.held_row, view.selected)) == (Some(1), None),
            "a held Enter on row 2 did not reach the table as a long press on that row"
        );
        checkpoint("the hold reached the table with row 2, nothing is selected")?;

        inject_named_key(NamedKey::Enter);
        view.show();
        ensure!(
            from_main(move || view.selected) == Some(1),
            "a short Enter did not select row 2"
        );

        Ok(())
    }
}
