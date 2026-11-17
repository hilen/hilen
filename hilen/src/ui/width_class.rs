#[cfg(any(feature = "gpu", test))]
use std::sync::atomic::{AtomicU8, Ordering};

/// How much room the window has across, the size classes Android publishes:
/// narrow is a phone or a narrow iPad window, medium an iPad in a split or a
/// small desktop window, wide an iPad in full screen or a desktop window. A
/// screen picks its layout from this, never from the platform.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WidthClass {
    Narrow,
    Medium,
    Wide,
}

impl WidthClass {
    /// The first width in points that is medium.
    pub const MEDIUM_FROM: f32 = 600.0;
    /// The first width in points that is wide.
    pub const WIDE_FROM: f32 = 840.0;

    pub fn of(width: f32) -> Self {
        if width < Self::MEDIUM_FROM {
            Self::Narrow
        } else if width < Self::WIDE_FROM {
            Self::Medium
        } else {
            Self::Wide
        }
    }

    pub fn is_narrow(self) -> bool {
        self == Self::Narrow
    }

    #[cfg(any(feature = "gpu", test))]
    const fn code(self) -> u8 {
        match self {
            Self::Narrow => 1,
            Self::Medium => 2,
            Self::Wide => 3,
        }
    }
}

/// The class the main window had at its last resize, 0 before the first.
/// Only a build with a window resizes.
#[cfg(any(feature = "gpu", test))]
static LAST: AtomicU8 = AtomicU8::new(0);

/// The new class when a resize of the main window to `width` changed it.
#[cfg(any(feature = "gpu", test))]
pub(crate) fn changed_to(width: f32) -> Option<WidthClass> {
    let class = WidthClass::of(width);
    let old = LAST.swap(class.code(), Ordering::Relaxed);
    (old != 0 && old != class.code()).then_some(class)
}

#[cfg(test)]
mod test {
    use super::{LAST, WidthClass, changed_to};

    #[test]
    fn the_borders_follow_the_android_size_classes() {
        assert_eq!(WidthClass::of(375.0), WidthClass::Narrow);
        assert_eq!(WidthClass::of(599.9), WidthClass::Narrow);
        assert_eq!(WidthClass::of(600.0), WidthClass::Medium);
        assert_eq!(WidthClass::of(839.9), WidthClass::Medium);
        assert_eq!(WidthClass::of(840.0), WidthClass::Wide);
        assert_eq!(WidthClass::of(1600.0), WidthClass::Wide);
    }

    #[test]
    fn only_a_new_class_is_a_change() {
        LAST.store(0, std::sync::atomic::Ordering::Relaxed);
        // The first resize sets the class, there is nothing it changed from.
        assert_eq!(changed_to(1280.0), None);
        assert_eq!(changed_to(900.0), None);
        assert_eq!(changed_to(390.0), Some(WidthClass::Narrow));
        assert_eq!(changed_to(430.0), None);
        assert_eq!(changed_to(700.0), Some(WidthClass::Medium));
        assert_eq!(changed_to(1024.0), Some(WidthClass::Wide));
    }
}
