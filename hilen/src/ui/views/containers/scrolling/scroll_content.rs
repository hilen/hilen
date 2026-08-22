use crate::{
    gm::flat::Size,
    ui::{View, ViewCallbacks, view},
    {self as hilen},
};

#[view]
pub(crate) struct ScrollContent {
    pub(super) content_size: Size,
}

impl ScrollContent {
    pub(super) fn content_offset_mut(&mut self) -> &mut f32 {
        &mut self.__base_view().__content_offset
    }
}

impl ViewCallbacks for ScrollContent {
    fn content_size(&self) -> &Size {
        &self.content_size
    }
}
