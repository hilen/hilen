use std::{
    cell::{Ref, RefMut},
    fmt::{Debug, Formatter},
    sync::Arc,
};

use parking_lot::Mutex;

use crate::{
    deps::refs::{Rglica, ToRglica},
    gm::{
        RefCell,
        flat::{Rect, Size},
    },
    ui::{View, WeakView, layout::layout_rule::LayoutRule},
};

#[derive(Clone)]
pub struct Placer {
    pub(crate) rules:            RefCell<Vec<LayoutRule>>,
    pub(crate) all_tiling_rules: RefCell<Vec<LayoutRule>>,

    // Since `Placer` is owned by `View` this should be OK. I hope.
    pub(crate) view:      Rglica<dyn View>,
    pub(crate) s_content: Rglica<Size>,

    pub(crate) all_margin: RefCell<f32>,

    pub(crate) has: RefCell<Size<bool>>,

    pub(crate) fit_text: RefCell<Size<bool>>,

    #[allow(clippy::type_complexity)]
    pub(crate) custom: RefCell<Option<Arc<Mutex<dyn FnMut(&mut Rect) + Send>>>>,
}

impl Placer {
    pub(crate) fn empty() -> Self {
        Self {
            rules:            RefCell::new(vec![]),
            all_tiling_rules: RefCell::new(vec![]),
            view:             Rglica::default(),
            s_content:        Rglica::default(),
            all_margin:       RefCell::new(0.0),
            has:              RefCell::new(Size::default()),
            fit_text:         RefCell::new(Size::default()),
            custom:           RefCell::new(None),
        }
    }

    pub fn view(&self) -> WeakView {
        self.view.weak_view()
    }

    pub fn is_empty(&self) -> bool {
        self.rules.borrow().is_empty() && self.all_tiling_rules.borrow().is_empty()
    }

    pub(crate) fn is_ok(&self) -> bool {
        self.view.is_ok()
    }

    pub(crate) fn init(&mut self, view: WeakView) {
        let s_content = view.__base_view().superview.content_size();
        self.view = unsafe { view.to_rglica() };
        self.s_content = s_content.to_rglica();
    }

    pub fn clear(&self) -> &Self {
        self.rules.borrow_mut().clear();
        self.all_tiling_rules.borrow_mut().clear();
        *self.has.borrow_mut() = Size::default();
        *self.fit_text.borrow_mut() = Size::default();
        self
    }

    pub(crate) fn get_rules(&self) -> Ref<'_, Vec<LayoutRule>> {
        self.rules.borrow()
    }

    pub fn edit_rule(&self, index: usize) -> RefMut<'_, LayoutRule> {
        RefMut::map(self.rules.borrow_mut(), |rules| &mut rules[index])
    }

    pub fn get_tiling_rules(&self) -> Ref<'_, Vec<LayoutRule>> {
        self.all_tiling_rules.borrow()
    }

    pub(super) fn rules(&self) -> RefMut<'_, Vec<LayoutRule>> {
        self.rules.borrow_mut()
    }

    pub(super) fn all_tiling_rules(&self) -> RefMut<'_, Vec<LayoutRule>> {
        self.all_tiling_rules.borrow_mut()
    }

    pub(super) fn has(&self) -> RefMut<'_, Size<bool>> {
        self.has.borrow_mut()
    }
}

impl Debug for Placer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.rules.borrow().fmt(f)
    }
}

impl PartialEq for Placer {
    fn eq(&self, other: &Self) -> bool {
        self.rules == other.rules
            && self.all_tiling_rules == other.all_tiling_rules
            && self.all_margin == other.all_margin
            && self.has == other.has
            && self.fit_text == other.fit_text
    }
}
