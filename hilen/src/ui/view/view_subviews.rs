use std::{any::type_name, ops::DerefMut};

use crate::{
    deps::refs::{Own, Weak, weak_from_ref},
    gm::{LossyConvert, color::Color},
    ui::{Container, DELETED_VIEWS, UIManager, View, ViewData, ViewFrame, WeakView, view::setup::Setup},
};

pub trait ViewSubviews {
    fn __manually_set_superview(&mut self, superview: WeakView);
    fn superview(&self) -> &WeakView;
    fn subviews(&self) -> &[Own<dyn View>];
    fn subviews_mut(&mut self) -> &mut Vec<Own<dyn View>>;
    fn subviews_weak(&mut self) -> Vec<WeakView>;
    fn remove_from_superview(&mut self);
    fn take_from_superview(&mut self) -> Own<dyn View>;
    fn remove_all_subviews(&self);

    fn add_view<V: 'static + View + Default>(&self) -> Weak<V>;
    fn add_subview<V: ?Sized + View + 'static>(&self, view: Own<V>) -> Weak<V>;

    fn __add_view_internal<V: 'static + View + Default>(&self) -> Weak<V>;
    fn __add_subview_internal<V: ?Sized + View + 'static>(&self, view: Own<V>, is_root: bool) -> Weak<V>;

    fn apply_if<V: View + 'static>(&mut self, action: impl FnMut(Weak<V>) + Clone + 'static);

    fn apply_to<V: View + 'static>(&mut self, action: impl FnMut(&mut V) + Clone + 'static);

    fn apply_to_all_subviews(&mut self, action: impl FnMut(&mut dyn View) + Clone + 'static);

    fn get_subview<V: 'static + View + Default>(&mut self) -> Weak<V>;
    fn dump_subviews(&self) -> Vec<String>;

    fn downcast_view<V: 'static + View>(&self) -> Option<Weak<V>>;

    fn outline(&self, color: Color) -> Weak<Self>;

    fn find_superview<V: View + 'static>(&self) -> Weak<V>;

    fn draw_on_top(&mut self);
}

pub trait __ViewIntoUnsizedOwn {
    unsafe fn __into_unsized_own<V: ?Sized + View + 'static>(own: Own<V>) -> Own<dyn View>;
}

impl<T: ?Sized + View + 'static> __ViewIntoUnsizedOwn for T {
    default unsafe fn __into_unsized_own<V: ?Sized + View + 'static>(own: Own<V>) -> Own<dyn View> {
        assert!(!own.sized());
        assert_eq!(size_of::<Own<V>>(), size_of::<Own<dyn View>>());

        let unsz = unsafe { std::mem::transmute_copy(&own) };
        std::mem::forget(own);

        unsz
    }
}

impl<T: ?Sized + View> ViewSubviews for T {
    fn __manually_set_superview(&mut self, superview: WeakView) {
        self.__base_view().superview = superview;
        let weak = self.weak_view();
        self.__base_view().placer.init(weak);
    }

    fn superview(&self) -> &WeakView {
        &self.__base_view().superview
    }

    fn subviews(&self) -> &[Own<dyn View>] {
        &self.__base_view().subviews
    }

    fn subviews_mut(&mut self) -> &mut Vec<Own<dyn View>> {
        &mut self.__base_view().subviews
    }

    fn subviews_weak(&mut self) -> Vec<WeakView> {
        self.__base_view().subviews.iter().map(Own::weak).collect()
    }

    fn remove_from_superview(&mut self) {
        let removed = self.take_from_superview();
        DELETED_VIEWS.lock().push(removed);
    }

    fn take_from_superview(&mut self) -> Own<dyn View> {
        let this_addr = self.weak_view().raw();
        let super_subs = &mut self.__base_view().superview.__base_view().subviews;

        let index = super_subs.iter().position(|a| a.raw() == this_addr).unwrap();

        super_subs.remove(index)
    }

    default fn remove_all_subviews(&self) {
        DELETED_VIEWS.lock().append(&mut self.__base_view().subviews);
    }

    fn add_view<V: 'static + View + Default>(&self) -> Weak<V> {
        let view = V::new();
        let result = view.weak();
        self.add_subview(view);
        result
    }

    default fn add_subview<V: ?Sized + View + 'static>(&self, view: Own<V>) -> Weak<V> {
        self.__add_subview_internal(view, false)
    }

    fn __add_view_internal<V: 'static + View + Default>(&self) -> Weak<V> {
        let view = V::new();
        let result = view.weak();
        self.__add_subview_internal(view, false);
        result
    }

    fn __add_subview_internal<V: ?Sized + View + 'static>(&self, view: Own<V>, is_root: bool) -> Weak<V> {
        assert!(
            is_root || self.superview().is_ok(),
            "Adding subview to view without superview is not allowed"
        );

        let mut weak = view.weak();

        let mut view: Own<dyn View> = unsafe { V::__into_unsized_own(view) };

        // This view was already added, and is readded again
        // This should be used only internally for types like TableView
        if view.superview().raw() == self.weak().raw() {
            self.__base_view().subviews.push(view);
            self.__base_view().events.setup.trigger(());
            return weak;
        }

        view.__internal_before_setup();

        if view.__base_view().navigation_view.is_null() {
            view.__base_view().navigation_view = self.__base_view().navigation_view;
        }

        if !weak.__base_view().z_position_custom {
            weak.__base_view().z_position = self.z_position()
                - UIManager::subview_z_offset()
                - UIManager::additional_z_offset() * self.__base_view().subviews.len().lossy_convert();
        }

        self.__base_view().subviews.push(view);
        weak.__manually_set_superview(self.weak_view());
        weak.__init_views();
        weak.__internal_setup();
        weak
    }

    fn apply_if<V: View + 'static>(&mut self, mut action: impl FnMut(Weak<V>) + Clone + 'static) {
        if let Some(view_type) = self.downcast_view::<V>() {
            action(view_type);
        }
    }

    fn apply_to<V: View + 'static>(&mut self, mut action: impl FnMut(&mut V) + Clone + 'static) {
        for view in &mut self.__base_view().subviews {
            if let Some(mut view_type) = view.downcast_view::<V>() {
                action(&mut view_type);
            }
        }
    }

    fn apply_to_all_subviews(&mut self, mut action: impl FnMut(&mut dyn View) + Clone + 'static) {
        action(self.weak_view().deref_mut());
        for view in &mut self.__base_view().subviews {
            view.apply_to_all_subviews(action.clone());
        }
    }

    fn get_subview<V: 'static + View + Default>(&mut self) -> Weak<V> {
        for sub in self.subviews() {
            if let Some(view) = sub.downcast_weak::<V>() {
                return view;
            }
        }

        panic!("View of type: {} not found", type_name::<V>());
    }

    fn dump_subviews(&self) -> Vec<String> {
        self.subviews().iter().map(|v| v.label().to_string()).collect()
    }

    fn downcast_view<V: 'static + View>(&self) -> Option<Weak<V>> {
        self.weak_view().downcast::<V>()
    }

    fn outline(&self, color: Color) -> Weak<Self> {
        const WIDTH: f32 = 2.0;

        self.add_view::<Container>().set_color(color).place().lrt(0).h(WIDTH);
        self.add_view::<Container>().set_color(color).place().lrb(0).h(WIDTH);
        self.add_view::<Container>().set_color(color).place().t(0).l(0).b(0).w(WIDTH);
        self.add_view::<Container>().set_color(color).place().t(0).r(0).b(0).w(WIDTH);

        weak_from_ref(self)
    }

    fn find_superview<V: View + 'static>(&self) -> Weak<V> {
        let mut superview = self.__base_view().superview;

        while superview.is_ok() {
            if let Some(view) = superview.downcast_view::<V>() {
                return view;
            }
            superview = superview.__base_view().superview;
        }

        panic!("This view doesn't have `{}` in superview chain", type_name::<V>());
    }

    fn draw_on_top(&mut self) {
        let neighbours = self.superview().subviews();
        let last_z_pos = neighbours.last().unwrap().z_position();
        self.__base_view().z_position = last_z_pos - UIManager::subview_z_offset() * 2.0;
    }
}
