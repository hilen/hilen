use crate::{
    deps::refs::Weak,
    ui::{CellRegistry, View},
};

pub trait TableData {
    fn cell_height(&self, index: usize) -> f32;
    fn number_of_cells(&self) -> usize;
    fn cell_selected(&mut self, index: usize);
    fn setup_cell(&mut self, index: usize, registry: &mut CellRegistry) -> Weak<dyn View>;
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
}
