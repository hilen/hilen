use crate::{
    deps::{
        hreads::{from_main, on_main},
        refs::{Own, Weak},
        vents::OnceEvent,
    },
    gm::{LossyConvert, color::CLEAR, flat::Size},
    ui::{
        BlurView, NamedKey, ScrimView, Setup, TouchStack, UIColor, UIManager, View, ViewData, ViewFrame,
        ViewTouch, view::ViewSubviews,
    },
};

pub trait ModalView<In = (), Out: 'static + Send = ()>: 'static + View + Default {
    fn show_modally(view: Self) -> Weak<Self> {
        let mut view = Own::new(view);
        // Stacked modals must not share one z position, or the top one
        // interleaves with the modal under it in the depth test.
        let depth: f32 = TouchStack::overlay_count().lossy_convert();
        let z = UIManager::MODAL_Z_OFFSET - UIManager::MODAL_LAYER_Z_STEP * depth;
        view.set_z_position(z);
        let size = Self::modal_size();
        let weak = view.weak();

        // The scrim owns the modal, so hiding removes both at once.
        // It sits one subview step behind the modal and in front of
        // everything else. It is the root of the touch layer, so it
        // blocks touches under it and hears a press outside the modal,
        // like the backdrop of a context menu. With a blur it is a
        // BlurView, which blurs the whole scene and tints it with the
        // scrim color.
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
        // The layer goes up before the modal registers its touches, a view
        // joins the top layer that holds it at registration time.
        TouchStack::push_layer(scrim);
        // Every view of the modal takes a press first, a table too, which
        // takes its taps at low priority. A press that reaches the scrim
        // outside the modal closes it the way Escape does, a modal with no
        // cancel result stays.
        TouchStack::enable_for_backdrop(scrim);
        scrim.touch().began.val(move |_| {
            let outside = !weak.absolute_frame().contains(UIManager::cursor_position());
            if outside
                && TouchStack::top_layer_root().raw() == scrim.raw()
                && let Some(result) = weak.modal_cancel()
            {
                weak.hide_modal(result);
            }
        });
        scrim.add_subview(view);

        place_modal(weak, size);

        UIManager::keymap().add(weak, NamedKey::Escape, move || {
            // Only the topmost modal, a menu or a modal over it takes the
            // key first, and a text field ends its editing first.
            if TouchStack::top_layer_root().raw() != weak.superview().raw() || UIManager::text_editing() {
                return;
            }
            if let Some(result) = weak.modal_cancel() {
                weak.hide_modal(result);
            }
        });

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
            TouchStack::pop_layer(scrim);
            scrim.remove_from_superview();
            self.modal_event().trigger(result);
        });
    }

    fn modal_event(&self) -> &OnceEvent<Out>;

    /// The result Escape and a press outside the modal end it with, the
    /// same as its cancel button gives. `None`, the default, leaves both
    /// off.
    fn modal_cancel(self: Weak<Self>) -> Option<Out> {
        None
    }

    fn modal_size() -> Size;

    /// Gives the open modal another size, it stays centered. For a form
    /// whose fields show and hide, `modal_size` is only the size it opens
    /// with.
    fn set_modal_size(self: Weak<Self>, size: impl Into<Size>) {
        place_modal(self, size.into());
    }

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

/// The room a modal keeps to the edges of a window smaller than it.
const MODAL_MARGIN: f32 = 12.0;

/// The size a modal that asks for `size` gets in the window now: that size,
/// or the window minus a margin where the window is smaller, like a desktop
/// form on a phone. A modal that asks for the whole window, like the bug
/// report, keeps it. A modal that measures text while it builds, like the
/// height of a wrapped paragraph, measures it at this width.
pub fn fitted_modal_size(size: impl Into<Size>) -> Size {
    let size = size.into();
    let root = UIManager::root_view();
    Size::new(
        fit_side(size.width, root.width()),
        fit_side(size.height, root.height()),
    )
}

fn fit_side(asked: f32, room: f32) -> f32 {
    if (asked - room).abs() < 0.5 {
        return room;
    }
    asked.min(room - 2.0 * MODAL_MARGIN).max(0.0)
}

/// The modal in the middle of the window at its fitted size, see
/// `fitted_modal_size`. The modal lays out its own content to the width it
/// gets.
fn place_modal<T: View>(view: Weak<T>, size: Size) {
    view.place().clear().custom(move |frame| {
        let root = UIManager::root_view();
        let fitted = fitted_modal_size(size);
        frame.origin.x = (root.width() - fitted.width) / 2.0;
        frame.origin.y = (root.height() - fitted.height) / 2.0;
        frame.size = fitted;
    });
}
