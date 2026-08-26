use crate::{
    deps::refs::Own,
    ui::{ToLabel, TooltipContent, View, ViewTouch},
};

pub trait ViewTooltip {
    /// A short text shown next to the cursor after it rests on the view.
    /// Turns hover on for the view. On a touch screen a long press shows
    /// it, unless the view hangs a `secondary` action on the hold.
    fn set_tooltip(&self, text: impl ToLabel) -> &Self;

    /// A tooltip built from a view, for content richer than one line.
    /// `make` runs on every show, the view is dropped on hide, so it can
    /// read whatever is current. Size the view in `make`, the tooltip
    /// layer takes its frame as is.
    fn set_tooltip_view(&self, make: impl Fn() -> Own<dyn View> + Send + Sync + 'static) -> &Self;

    /// The tooltip text, empty for none or for a view tooltip.
    fn tooltip_text(&self) -> &str;
}

impl<T: ?Sized + View> ViewTooltip for T {
    fn set_tooltip(&self, text: impl ToLabel) -> &Self {
        self.__base_view().tooltip = Some(TooltipContent::Text(text.to_label()));
        self.enable_hover();
        self
    }

    fn set_tooltip_view(&self, make: impl Fn() -> Own<dyn View> + Send + Sync + 'static) -> &Self {
        self.__base_view().tooltip = Some(TooltipContent::View(Box::new(make)));
        self.enable_hover();
        self
    }

    fn tooltip_text(&self) -> &str {
        match &self.__base_view().tooltip {
            Some(TooltipContent::Text(text)) => text,
            _ => "",
        }
    }
}
