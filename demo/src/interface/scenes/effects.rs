use hilen::{
    refs::Weak,
    ui::{
        BlurView, ColorMeter, Container, CornerRadii, ImageView, Label, ScrollView, Setup, Shadow,
        TextAlignment, ViewData, ViewSubviews, ViewTouch, WHITE, view,
    },
};

use crate::interface::{
    palette::{ACCENT, ACCENT_END, ACCENT_START, BORDER, SURFACE},
    scenes::{HEADER_HEIGHT, add_title},
};

/// One tile per visual effect the engine gives every view: shadow,
/// gradient, per corner rounding, frosted blur, hover and the live
/// color meter eyedropper. The tiles scroll, so every one stays
/// reachable on small screens.
#[view]
pub struct EffectsScene {
    grid: Weak<Container>,

    #[init]
    scroll: ScrollView,
}

impl Setup for EffectsScene {
    fn setup(mut self: Weak<Self>) {
        self.scroll.place().t(HEADER_HEIGHT).lrb(0);
        self.grid = self.scroll.add_view::<Container>();
        self.grid.place().t(4).lr(28).all(16).all_wrap();

        self.add_effects();

        add_title(self, "Effects", "What every view gets for free.");
    }
}

impl EffectsScene {
    fn tile(self: Weak<Self>, caption: &str) -> Weak<Container> {
        let tile = self.grid.add_view::<Container>();
        tile.set_color(SURFACE)
            .set_corner_radius(16)
            .set_border_width(1)
            .set_border_color(BORDER);
        tile.place().size(168, 150);

        let cap = tile.add_view::<Label>();
        cap.set_text(caption.to_uppercase())
            .set_text_color(ACCENT)
            .set_text_size(10)
            .set_letter_spacing(1.2)
            .set_alignment(TextAlignment::Center);
        cap.place().t(8).lr(6).h(16);

        tile
    }

    fn add_effects(self: Weak<Self>) {
        let shadow = self.tile("Shadow").add_view::<Container>();
        shadow.set_color(WHITE).set_corner_radius(12).set_shadow(Shadow::default());
        shadow.place().t(42).center_x().size(104, 68);

        let gradient = self.tile("Gradient").add_view::<Container>();
        gradient.set_gradient(ACCENT_START, ACCENT_END).set_corner_radius(12);
        gradient.place().t(42).center_x().size(104, 68);

        let corners = self.tile("Corner Radii").add_view::<Container>();
        corners.set_color(ACCENT).set_corner_radii(CornerRadii::top(30));
        corners.place().t(42).center_x().size(104, 68);

        let photo = self.tile("Frosted Blur");
        let img = photo.add_view::<ImageView>();
        img.set_image("cat.png");
        img.place().t(34).lrb(10);
        let mut blur = photo.add_view::<BlurView>();
        blur.set_blur_radius(6).set_color(WHITE.with_alpha(0.1)).set_corner_radius(10);
        blur.place().t(34).rb(10).w(74);

        let hover = self.tile("Hover me").add_view::<Container>();
        hover.set_color(ACCENT_START).set_corner_radius(12);
        hover.place().t(42).center_x().size(104, 68);
        hover.enable_hover();
        hover.touch().hovered.val(hover, move |hovered| {
            hover.set_color(if hovered { ACCENT_END } else { ACCENT_START });
        });

        let meter = self.tile("Eyedropper").add_view::<ColorMeter>();
        meter.set_corner_radius(10).set_border_width(1).set_border_color(BORDER);
        meter.place().t(40).center_x().size(80, 80);
    }
}
