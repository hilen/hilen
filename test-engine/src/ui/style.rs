use std::{any::type_name, collections::HashMap, mem::take, ops::DerefMut, panic::Location};

use refs::{Weak, main_lock::MainLock};

use crate::ui::{Button, Label, NumberView, View};

pub(crate) type GlobalStyles = HashMap<&'static str, Vec<Style>>;

static GLOBAL_STYLES: MainLock<GlobalStyles> = MainLock::new();

static ALLOWED_TYPES: &[&str] = &[
    type_name::<Button>(),
    type_name::<Label>(),
    type_name::<NumberView>(),
];

#[derive(Debug, Clone, Copy)]
pub struct Style {
    action:   fn(&mut dyn View),
    location: &'static Location<'static>,
}

/// Function pointers are not stable identity. The compiler can merge two
/// identical bodies into one function or duplicate one body across codegen
/// units, so styles compare by their declaration site instead.
impl PartialEq for Style {
    fn eq(&self, other: &Self) -> bool {
        self.location == other.location
    }
}

impl Eq for Style {}

impl Style {
    #[track_caller]
    pub const fn new(action: fn(&mut dyn View)) -> Self {
        Self {
            action,
            location: Location::caller(),
        }
    }

    pub(crate) fn apply(self, view: &mut dyn View) {
        (self.action)(view);
    }

    pub(crate) fn apply_global<T: View>(view: Weak<T>) {
        Self::check_allowed::<T>();

        if view.__base_view().ignore_global_style {
            return;
        }

        for style in Self::get_global_for::<T>() {
            style.apply(view.weak_view().deref_mut());
        }
    }

    fn get_global_for<T: View>() -> &'static [Style] {
        if let Some(styles) = GLOBAL_STYLES.get(type_name::<T>()) {
            styles
        } else {
            &[]
        }
    }

    pub fn apply_globally<T: View>(&self) {
        Self::check_allowed::<T>();
        let styles = GLOBAL_STYLES.get_mut().entry(type_name::<T>()).or_default();

        assert!(
            !styles.contains(self),
            "{} already has this global style",
            type_name::<T>()
        );

        styles.push(*self);
    }

    pub(crate) fn take_globals() -> GlobalStyles {
        take(GLOBAL_STYLES.get_mut())
    }

    pub(crate) fn restore_globals(styles: GlobalStyles) {
        *GLOBAL_STYLES.get_mut() = styles;
    }

    pub fn reset_global<T: View>(&self) {
        Self::check_allowed::<T>();
        let styles = GLOBAL_STYLES.get_mut().entry(type_name::<T>()).or_default();
        styles.clear();
    }

    fn check_allowed<T: View>() {
        assert!(
            ALLOWED_TYPES.contains(&type_name::<T>()),
            "Global style for {} is not allowed. Allowed types: {ALLOWED_TYPES:?}",
            type_name::<T>()
        );
    }
}

#[cfg(test)]
mod test {
    use hreads::set_current_thread_as_main;

    use crate::ui::{Button, Label, Style, Switch};

    // Identical empty bodies on purpose. The compiler may merge them into one
    // function, and the styles must still compare as three distinct values
    // because identity comes from the declaration site.
    const STYLE: Style = Style::new(|_v| {});
    const STYLE2: Style = Style::new(|_v| {});
    const STYLE3: Style = Style::new(|_v| {});

    #[test]
    fn valid_global_style_type() {
        set_current_thread_as_main();
        STYLE.apply_globally::<Button>();
        STYLE2.apply_globally::<Button>();
        STYLE3.apply_globally::<Label>();

        assert_eq!(Style::get_global_for::<Button>(), &[STYLE, STYLE2]);
        assert_eq!(Style::get_global_for::<Label>(), &[STYLE3]);
    }

    #[test]
    #[should_panic = "is not allowed"]
    fn invalid_global_style_type() {
        STYLE.apply_globally::<Switch>();
    }
}
