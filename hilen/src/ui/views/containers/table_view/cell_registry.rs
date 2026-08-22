use std::{any::type_name, collections::HashMap};

use crate::{
    deps::{
        netrun::Function,
        refs::{Own, Weak},
    },
    ui::{TableView, View, ViewData, ViewSubviews},
};

#[derive(Default)]
pub struct CellRegistry {
    table:            Weak<TableView>,
    pub free_cells:   HashMap<String, Vec<Weak<dyn View>>>,
    pub constructors: HashMap<&'static str, Function<(), Own<dyn View>>>,
}

impl CellRegistry {
    pub(crate) fn set_table(&mut self, table: Weak<TableView>) {
        self.table = table;
    }

    pub(crate) fn load_old_cells(&mut self, mut cells: Vec<Weak<dyn View>>) {
        for cell in cells.drain(..) {
            let entry = self.free_cells.entry(cell.label().to_string()).or_default();

            if entry.iter().any(|c| c.raw() == cell.raw()) {
                continue;
            }

            entry.push(cell);
        }
    }

    pub(crate) fn cell_for_ident(&mut self, ident: &'static str) -> Weak<dyn View> {
        let container = self.free_cells.entry(ident.to_string()).or_default();

        if let Some(cell) = container.pop() {
            cell.set_hidden(false);
            cell
        } else {
            let constructor = self
                       .constructors
                       .get(ident)
                     .unwrap_or_else(|| panic!("Constructor for cell identifier: {ident} is not registered. Use TableView::register_cell method."));

            let owned = constructor.call(());
            let weak = owned.weak();

            self.table.scroll.add_subview(owned);

            weak
        }
    }

    pub fn cell<T: View + 'static>(&mut self) -> Weak<T> {
        self.cell_for_ident(struct_name::<T>())
            .downcast::<T>()
            .expect("Failed to downcast cell")
    }

    pub fn cell_with_id<T: View + 'static>(&mut self, id: &'static str) -> Weak<T> {
        self.cell_for_ident(id).downcast::<T>().expect("Failed to downcast cell")
    }
}

pub(crate) fn struct_name<T>() -> &'static str {
    let full_name = type_name::<T>();
    full_name.split("::").last().unwrap_or(full_name)
}
