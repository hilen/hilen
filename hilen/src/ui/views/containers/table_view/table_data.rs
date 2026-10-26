use crate::{
    deps::refs::Weak,
    ui::{CellRegistry, View},
};

pub trait TableData {
    fn cell_height(&self, index: usize) -> f32;
    fn number_of_cells(&self) -> usize;
    fn cell_selected(&mut self, index: usize);
    fn setup_cell(&mut self, index: usize, registry: &mut CellRegistry) -> Weak<dyn View>;
    /// A sticky row is a section header: it pins to the top of the table
    /// viewport while its section scrolls and the next sticky row pushes
    /// it away. Read only when `set_sticky_rows(true)` is on.
    fn is_sticky(&self, _index: usize) -> bool {
        false
    }
    /// The user dragged row `from` to place `to`, counted in the rows
    /// without it, so `to` is its index in the new order. The data moves
    /// it, the table reloads after this call. Only a table with
    /// `set_row_moves(true)` calls it.
    fn move_row(&mut self, _from: usize, _to: usize) {}
}

#[allow(unused_variables)]
impl<T: View + 'static> TableData for T {
    default fn cell_height(&self, _index: usize) -> f32 {
        50.0
    }

    default fn number_of_cells(&self) -> usize {
        0
    }

    default fn cell_selected(&mut self, index: usize) {}

    default fn setup_cell(&mut self, index: usize, registry: &mut CellRegistry) -> Weak<dyn View> {
        unimplemented!("TableData::setup_cell must be overloaded")
    }

    default fn is_sticky(&self, index: usize) -> bool {
        false
    }

    default fn move_row(&mut self, from: usize, to: usize) {}
}
