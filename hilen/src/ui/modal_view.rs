use crate::{
    deps::{
        hreads::{from_main, on_main},
        refs::{Own, Weak},
        vents::OnceEvent,
    },
    gm::{LossyConvert, color::CLEAR, flat::Size},
    ui::{
        BlurView, ScrimView, Setup, TouchStack, UIColor, UIManager, View, ViewData, ViewFrame,
        view::ViewSubviews,
    },
};

pub trait ModalView<In = (), Out: 'static = ()>: 'static + View + Default {
    fn show_modally(view: Self) -> Weak<Self> {
        let mut view = Own::new(view);
        // Stacked modals must not share one z position, or the top one
        // interleaves with the modal under it in the depth test.
        let depth: f32 = TouchStack::overlay_count().lossy_convert();
        let z = UIManager::MODAL_Z_OFFSET - UIManager::MODAL_LAYER_Z_STEP * depth;
        view.set_z_position(z);
        let size = Self::modal_size();
        let weak = view.weak();
        TouchStack::push_layer(weak.weak_view());

        // The scrim owns the modal, so hiding removes both at once.
        // It sits one subview step behind the modal and in front of
        // everything else. It is not a touch layer, the modal already
        // blocks touches under it. With a blur it is a BlurView, which
        // blurs the whole scene and tints it with the scrim color.
        let mut scrim: Own<dyn View> = if Self::modal_blur() > 0.0 {
            let mut blur = BlurView::new();
            blur.set_blur_radius(Self::modal_blur());
            blur
        } else {
            ScrimView::new()
        };
        scrim.set_z_position(z + UIManager::subview_z_offset());
        let scrim = UIManager::root_view().add_subview_to_root(scrim);
        scrim.set_color(Self::modal_scrim_color());
        scrim.place().back();
        scrim.add_subview(view);

        weak.place().size(size.width, size.height).center();
        weak
    }

    fn prepare_modally() -> Weak<Self> {
        Self::show_modally(Self::default())
    }

    fn prepare_modally_with_input(input: In) -> Weak<Self> {
        let view = Self::prepare_modally();
        view.setup_input(input);
        view
    }

    fn show_modally_with_input(input: In, callback: impl FnOnce(Out) + 'static + Send)
    where
        In: 'static + Send,
        Out: Send, {
        on_main(move || {
            let weak = Self::prepare_modally_with_input(input);
            weak.modal_event().val(callback);
        });
    }

    #[allow(async_fn_in_trait)]
    async fn show_modally_async(input: In) -> Out
    where
        In: 'static + Send,
        Out: Send, {
        from_main(|| Self::prepare_modally_with_input(input).modal_event().receiver().recv().unwrap())
    }

    fn hide_modal(self: Weak<Self>, result: Out)
    where Out: Send {
        on_main(move || {
            let mut scrim = *self.superview();
            scrim.remove_from_superview();
            TouchStack::pop_layer(self.weak_view());
            self.modal_event().trigger(result);
        });
    }

    fn modal_event(&self) -> &OnceEvent<Out>;

    fn modal_size() -> Size;

    /// The color of the fullscreen backdrop behind the modal.
    /// Transparent by default, override to dim the background.
    fn modal_scrim_color() -> UIColor {
        CLEAR.into()
    }

    /// The blur radius of the backdrop behind the modal. Zero by
    /// default, override to blur the background. Combines with
    /// `modal_scrim_color`, which tints the blur.
    fn modal_blur() -> f32 {
        0.0
    }

    fn setup_input(self: Weak<Self>, _: In) {}
}
