use std::ops::DerefMut;

use crate::{
    deps::{
        refs::{Own, Weak, weak_from_ref},
        vents::Event,
    },
    gm::{ToF32, color::Color, flat::CornerRadii},
    ui::{
        Gradient, NavigationView, Shadow, Style, UIAnimation, UIColor, UIManager, View, WeakView,
        layout::Placer,
    },
};

pub trait ViewData {
    fn tag(&self) -> usize;
    fn set_tag(&mut self, tag: usize) -> &mut Self;

    fn view_label(&self) -> &str;

    fn is_system(&self) -> bool;

    fn content_offset(&self) -> f32;

    fn color(&self) -> &Color;
    fn ui_color(&self) -> UIColor;
    fn set_color(&self, color: impl Into<UIColor>) -> &Self;

    fn gradient(&self) -> Option<Gradient>;
    /// The plain top to bottom ramp. `apply_gradient` takes an angle or a
    /// radial shape.
    fn set_gradient(&self, start: impl Into<Color>, end: impl Into<Color>) -> &Self;
    fn apply_gradient(&self, gradient: Gradient) -> &Self;

    fn border_color(&self) -> &Color;
    fn set_border_color(&self, color: impl Into<UIColor>) -> &Self;

    fn border_width(&self) -> f32;
    fn set_border_width(&self, width: impl ToF32) -> &Self;

    fn corner_radii(&self) -> CornerRadii;
    fn set_corner_radius(&self, radius: impl ToF32) -> &Self;
    fn set_corner_radii(&self, radii: CornerRadii) -> &Self;

    fn shadow(&self) -> Option<Shadow>;
    fn set_shadow(&self, shadow: impl Into<Option<Shadow>>) -> &Self;

    fn is_hidden(&self) -> bool;
    fn is_hidden_in_tree(&self) -> bool;
    fn set_hidden(&self, is_hidden: bool) -> &Self;

    fn place(&self) -> &Placer;
    fn placer_copy(&self) -> Placer;

    fn navigation_view(&self) -> Weak<NavigationView>;
    fn set_navigation_view(&mut self, nav: Weak<NavigationView>) -> &mut Self;

    fn label(&self) -> &str;
    fn set_label(&mut self, label: impl ToString) -> &mut Self;

    fn dont_hide(&self) -> bool;

    fn position_changed(&self) -> &Event;
    fn size_changed(&self) -> &Event;

    fn apply_style(&self, style: Style) -> &Self;

    fn steal_appearance(&self, other: WeakView) -> &Self;

    fn add_animation(&self, anim: UIAnimation);

    fn weak(&self) -> Weak<Self>;
}

impl<T: ?Sized + View> ViewData for T {
    fn tag(&self) -> usize {
        self.__base_view().tag
    }

    fn set_tag(&mut self, tag: usize) -> &mut Self {
        self.__base_view().tag = tag;
        self
    }

    fn view_label(&self) -> &str {
        &self.__base_view().view_label
    }

    fn is_system(&self) -> bool {
        self.__base_view().is_system
    }

    fn content_offset(&self) -> f32 {
        self.__base_view().__content_offset
    }

    fn color(&self) -> &Color {
        &self.__base_view().color
    }

    /// The color as it was set, keeping the theme pair when there is one.
    /// `color()` only returns what the pair resolved to, so restoring a
    /// color through it would flatten a dynamic color into a plain one and
    /// the view would stop following theme switches.
    fn ui_color(&self) -> UIColor {
        let base = self.__base_view();
        match base.dynamic_color {
            Some(dynamic) => UIColor::Dynamic(dynamic),
            None => UIColor::Plain(base.color),
        }
    }

    fn set_color(&self, color: impl Into<UIColor>) -> &Self {
        let base = self.__base_view();
        match color.into() {
            UIColor::Plain(color) => {
                base.color = color;
                base.dynamic_color = None;
            }
            UIColor::Dynamic(color) => {
                base.color = color.resolve();
                base.dynamic_color = Some(color);
            }
        }
        base.gradient = None;
        self
    }

    fn gradient(&self) -> Option<Gradient> {
        self.__base_view().gradient
    }

    fn set_gradient(&self, start: impl Into<Color>, end: impl Into<Color>) -> &Self {
        self.apply_gradient(Gradient::vertical(start, end))
    }

    fn apply_gradient(&self, gradient: Gradient) -> &Self {
        let base = self.__base_view();
        base.color = gradient.start;
        base.dynamic_color = None;
        base.gradient = Some(gradient);
        self
    }

    fn border_color(&self) -> &Color {
        &self.__base_view().border_color
    }

    fn set_border_color(&self, color: impl Into<UIColor>) -> &Self {
        let base = self.__base_view();
        match color.into() {
            UIColor::Plain(color) => {
                base.border_color = color;
                base.dynamic_border_color = None;
            }
            UIColor::Dynamic(color) => {
                base.border_color = color.resolve();
                base.dynamic_border_color = Some(color);
            }
        }
        self
    }

    fn corner_radii(&self) -> CornerRadii {
        self.__base_view().corner_radii
    }

    fn set_corner_radius(&self, radius: impl ToF32) -> &Self {
        self.__base_view().corner_radii = CornerRadii::all(radius);
        self
    }

    fn set_corner_radii(&self, radii: CornerRadii) -> &Self {
        self.__base_view().corner_radii = radii;
        self
    }

    fn shadow(&self) -> Option<Shadow> {
        self.__base_view().shadow
    }

    fn set_shadow(&self, shadow: impl Into<Option<Shadow>>) -> &Self {
        self.__base_view().shadow = shadow.into();
        self
    }

    fn is_hidden(&self) -> bool {
        self.__base_view().is_hidden
    }

    fn is_hidden_in_tree(&self) -> bool {
        if self.__base_view().is_hidden {
            return true;
        }

        let mut superview = self.__base_view().superview;

        while superview.is_ok() {
            if superview.__base_view().is_hidden {
                return true;
            }
            superview = superview.__base_view().superview;
        }

        false
    }

    fn set_hidden(&self, is_hidden: bool) -> &Self {
        self.weak_view().__base_view().is_hidden = is_hidden;
        self
    }

    fn place(&self) -> &Placer {
        let placer = &self.__base_view().placer;
        assert!(
            placer.is_ok(),
            "Invalid placer. Most likely this view was not initialized properly."
        );
        placer
    }

    fn placer_copy(&self) -> Placer {
        let placer = &self.__base_view().placer;

        if placer.is_ok() {
            placer.clone()
        } else {
            Placer::empty()
        }
    }

    fn navigation_view(&self) -> Weak<NavigationView> {
        self.__base_view().navigation_view
    }

    fn set_navigation_view(&mut self, nav: Weak<NavigationView>) -> &mut Self {
        self.__base_view().navigation_view = nav;
        self
    }

    fn label(&self) -> &str {
        &self.__base_view().view_label
    }

    fn set_label(&mut self, label: impl ToString) -> &mut Self {
        self.__base_view().view_label = label.to_string();
        self
    }

    fn dont_hide(&self) -> bool {
        self.__base_view().dont_hide_off_screen
    }

    fn position_changed(&self) -> &Event {
        &self.__base_view().position_changed
    }

    fn size_changed(&self) -> &Event {
        &self.__base_view().size_changed
    }

    fn apply_style(&self, style: Style) -> &Self {
        style.apply(self.weak_view().deref_mut());
        self
    }

    fn border_width(&self) -> f32 {
        self.__base_view().border_width
    }

    fn set_border_width(&self, width: impl ToF32) -> &Self {
        self.__base_view().border_width = width.to_f32();
        self
    }

    fn steal_appearance(&self, other: WeakView) -> &Self {
        let this = self.weak_view();
        this.set_color(*other.color());
        this.set_border_color(*other.border_color());
        this.__base_view().dynamic_color = other.__base_view().dynamic_color;
        this.__base_view().dynamic_border_color = other.__base_view().dynamic_border_color;
        this.set_border_width(other.border_width());
        this.set_corner_radii(other.corner_radii());
        this.set_shadow(other.shadow());
        self
    }

    fn add_animation(&self, mut anim: UIAnimation) {
        anim.view = self.weak_view();
        UIManager::add_animation(anim);
    }

    fn weak(&self) -> Weak<Self> {
        weak_from_ref(self)
    }
}

pub trait AfterSetup {
    fn after_setup(self: Own<Self>, action: impl FnOnce(Weak<Self>) + Send + 'static) -> Own<Self>;
}

impl<T: ?Sized + View + 'static> AfterSetup for T {
    fn after_setup(self: Own<Self>, action: impl FnOnce(Weak<Self>) + Send + 'static) -> Own<Self> {
        let weak = self.weak();
        self.__base_view().events.setup.sub(move || {
            action(weak);
        });
        self
    }
}
