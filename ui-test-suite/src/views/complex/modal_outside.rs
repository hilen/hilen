use anyhow::{Result, ensure};
use hilen::{
    OnceEvent,
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{
        BLACK, CellRegistry, Label, ModalView, Setup, Size, TableData, TableView, UIColor, View, ViewData,
        ViewFrame, ViewTest, WHITE, view,
    },
    ui_test::{checkpoint, get_str_state, inject_touches, state::append_state},
};

/// A panel whose cancel is `None`, like a picker that closes on a press
/// outside it.
#[view]
struct OutsidePanel {
    event: OnceEvent<Option<String>>,

    #[init]
    title: Label,
}

impl Setup for OutsidePanel {
    fn setup(self: Weak<Self>) {
        self.set_color(WHITE).set_corner_radius(12);
        self.title.set_text("a press outside closes it").set_text_size(18);
        self.title.place().lrt(10).h(30);
    }
}

impl ModalView<(), Option<String>> for OutsidePanel {
    fn modal_event(&self) -> &OnceEvent<Option<String>> {
        &self.event
    }

    fn modal_size() -> Size {
        (320, 200).into()
    }

    fn modal_scrim_color() -> UIColor {
        BLACK.with_alpha(0.3).into()
    }

    fn modal_cancel(self: Weak<Self>) -> Option<Option<String>> {
        Some(None)
    }
}

/// A form with no cancel result, it must be answered, a press outside does
/// nothing.
#[view]
struct AnswerForm {
    event: OnceEvent<()>,

    #[init]
    title: Label,
}

impl Setup for AnswerForm {
    fn setup(self: Weak<Self>) {
        self.set_color(WHITE).set_corner_radius(12);
        self.title.set_text("this one stays").set_text_size(18);
        self.title.place().lrt(10).h(30);
    }
}

impl ModalView for AnswerForm {
    fn modal_event(&self) -> &OnceEvent<()> {
        &self.event
    }

    fn modal_size() -> Size {
        (320, 200).into()
    }

    fn modal_scrim_color() -> UIColor {
        BLACK.with_alpha(0.3).into()
    }
}

/// A panel that is a list, like a picker of rows. A table takes its taps
/// at low priority, the scrim behind the modal took them once: on
/// 2026-11-11 the machine list of the new session popup of banda took no
/// tap after the scrim heard presses outside.
#[view]
struct RowPanel {
    event: OnceEvent<Option<String>>,

    #[init]
    table: TableView,
}

impl Setup for RowPanel {
    fn setup(self: Weak<Self>) {
        self.set_color(WHITE).set_corner_radius(12);
        self.table.place().t(10).b(10).lr(10);
        self.table.set_data_source(self).register_cell::<Label>();
    }
}

impl TableData for RowPanel {
    fn number_of_cells(&self) -> usize {
        3
    }

    fn cell_height(&self, _: usize) -> f32 {
        50.0
    }

    fn setup_cell(&mut self, index: usize, registry: &mut CellRegistry) -> Weak<dyn View> {
        let cell = registry.cell::<Label>();
        cell.set_text(format!("row {index}"));
        cell
    }

    fn cell_selected(&mut self, index: usize) {
        append_state(format!("cell_selected: {index}\n"));
    }
}

impl ModalView<(), Option<String>> for RowPanel {
    fn modal_event(&self) -> &OnceEvent<Option<String>> {
        &self.event
    }

    fn modal_size() -> Size {
        (320, 200).into()
    }

    fn modal_scrim_color() -> UIColor {
        BLACK.with_alpha(0.3).into()
    }

    fn modal_cancel(self: Weak<Self>) -> Option<Option<String>> {
        Some(None)
    }
}

/// A press on the scrim outside a modal closes it with its cancel result,
/// the same as Escape. A press on the modal itself and a modal with no
/// cancel result keep it open.
#[view]
struct ModalOutside {
    #[init]
    page: Label,
}

impl Setup for ModalOutside {
    fn setup(self: Weak<Self>) {
        self.page.set_text("the page behind the modal").set_text_size(20);
        self.page.set_color(WHITE);
        self.page.place().back();
    }
}

fn tap(x: f32, y: f32) {
    inject_touches(format!("{x} {y} m\n{x} {y} b\n{x} {y} e"));
    wait_for_next_frame();
    wait_for_next_frame();
}

impl ViewTest for ModalOutside {
    fn perform_test(_: Weak<Self>) -> Result<()> {
        let panel = from_main(OutsidePanel::prepare_modally);
        wait_for_next_frame();
        let frame = from_main(move || *panel.absolute_frame());

        // The empty part of the panel, under its title.
        tap(frame.x() + frame.width() / 2.0, frame.y() + frame.height() - 20.0);
        ensure!(from_main(move || panel.is_ok()), "a press on the panel closed it");
        checkpoint("a press on the panel keeps it")?;

        tap(frame.x() - 30.0, frame.y() + 10.0);
        ensure!(
            from_main(move || panel.is_null()),
            "a press outside left the panel open"
        );
        checkpoint("a press outside closed the panel")?;

        let form = from_main(AnswerForm::prepare_modally);
        wait_for_next_frame();
        let frame = from_main(move || *form.absolute_frame());
        tap(frame.x() - 30.0, frame.y() + 10.0);
        ensure!(
            from_main(move || form.is_ok()),
            "a press outside closed a modal with no cancel result"
        );
        checkpoint("a modal with no cancel result stays")?;
        from_main(move || form.hide_modal(()));
        wait_for_next_frame();

        let rows = from_main(RowPanel::prepare_modally);
        wait_for_next_frame();
        let frame = from_main(move || *rows.absolute_frame());
        // The middle of the second row, the rows start 10 points in.
        tap(frame.x() + frame.width() / 2.0, frame.y() + 10.0 + 75.0);
        ensure!(
            get_str_state().contains("cell_selected: 1"),
            "a tap on a row of a table in a modal selected nothing, state: {}",
            get_str_state()
        );
        ensure!(from_main(move || rows.is_ok()), "a tap on a row closed the modal");
        checkpoint("a row of a table in a modal takes its tap")?;
        tap(frame.x() - 30.0, frame.y() + 10.0);
        ensure!(
            from_main(move || rows.is_null()),
            "a press outside left the list open"
        );

        Ok(())
    }
}
