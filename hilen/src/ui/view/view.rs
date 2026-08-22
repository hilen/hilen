use crate::{
    deps::refs::{AsAny, Weak},
    ui::{CellCallbacks, ViewBase, ViewCallbacks, view::setup::__ViewInternalSetup},
};

#[allow(clippy::mut_from_ref)]
pub trait View: ViewCallbacks + __ViewInternalSetup + AsAny {
    fn __init_views(&mut self);
    fn __base_view(&self) -> &mut ViewBase;
    fn weak_view(&self) -> WeakView;
    fn as_cell(&mut self) -> &mut dyn CellCallbacks;
}

pub type WeakView = Weak<dyn View>;
