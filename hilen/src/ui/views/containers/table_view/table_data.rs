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
}
