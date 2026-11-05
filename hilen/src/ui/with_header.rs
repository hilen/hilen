use log::trace;
use plat::Platform;

use crate::{
    deps::refs::Weak,
    ui::{View, ViewData, WeakView, layout::Anchor, view::depth_order},
};

pub trait WithHeader: View {
    fn header(&self) -> WeakView;
    fn main_view(&self) -> WeakView;
    fn header_size(&self) -> f32;
    fn header_margin(&self) -> f32;
    fn layout_header(&self) {
        let header = self.header();

        if header.is_null() {
            trace!("No header");
            return;
        }

        depth_order::set_raise(&*header, 1);

        if Platform::IOS {
            header.place().lr(0).t(40).h(self.header_size());
        } else {
            header.place().lrt(0).h(self.header_size());
        }

        if !self.main_view().is_null() {
            self.main_view()
                .place()
                .anchor(Anchor::Top, header, self.header_margin())
                .lrb(0);
        }
    }
}

impl<T: View> WithHeader for T {
    default fn header(&self) -> WeakView {
        Weak::default()
    }

    default fn main_view(&self) -> WeakView {
        Weak::default()
    }

    default fn header_size(&self) -> f32 {
        50.0
    }

    default fn header_margin(&self) -> f32 {
        0.0
    }
}
