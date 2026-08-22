use ui_proc::view;

use crate::{
    deps::refs::Weak,
    inspect::{
        protocol::{UIRequest, ui::ViewRepr},
        views::LayoutRuleCell,
    },
    ui::{CellRegistry, Setup, TableData, TableView, UIEvent, View, ViewData},
};

#[view(crate = crate)]
pub struct PlacerView {
    pub rule_changed: UIEvent<UIRequest>,

    view: Weak<ViewRepr>,

    #[init]
    table: TableView,
}

impl Setup for PlacerView {
    fn setup(self: Weak<Self>) {
        self.place().all_ver();
        self.table.set_data_source(self).register_cell::<LayoutRuleCell>();
    }
}

impl PlacerView {
    pub fn set_view(mut self: Weak<Self>, view: Weak<ViewRepr>) {
        self.view = view;
        self.table.reload_data();
    }
}

impl TableData for PlacerView {
    fn cell_height(&self, _: usize) -> f32 {
        50.0
    }

    fn number_of_cells(&self) -> usize {
        let Some(view) = self.view.get() else {
            return 0;
        };
        view.placer.get_rules().len()
    }

    fn setup_cell(&mut self, index: usize, registry: &mut CellRegistry) -> Weak<dyn View> {
        let cell = registry.cell::<LayoutRuleCell>();

        if !self.view.is_ok() {
            return cell;
        }

        cell.set_data(self.view, index);
        let this = self.weak();
        cell.rule_edited.val(this, move |request| {
            this.rule_changed.trigger(request);
        });

        cell
    }
}
