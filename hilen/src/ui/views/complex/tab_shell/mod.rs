//! A tab app: the page on top and a bar of tabs at the bottom, like a
//! native iOS tab app. The bar paints down to the edge of the screen, past
//! the home bar of a phone, see `UIManager::system_insets`. The page of a
//! tab is the first page of a `NavigationView`, a page pushed on it goes
//! back with its back button or a swipe and the bar stays.

mod bar;

use log::{debug, warn};

pub use self::bar::TabBar;
use crate::{
    self as hilen,
    deps::{
        refs::{Own, Weak},
        vents::Event,
    },
    gm::color::Color,
    ui::{Anchor, Container, DynamicColor, NavigationView, Setup, View, ViewData, ViewSubviews, view},
};

/// The height of the bar above the home bar, the one of a native iOS tab bar.
pub const TAB_BAR_HEIGHT: f32 = 49.0;

/// 1 entry of the bar. `icon` is an svg in `assets/images` drawn in black,
/// `#000000`, the bar tints it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tab {
    pub title: String,
    pub icon:  String,
}

impl Tab {
    pub fn new(title: impl ToString, icon: impl ToString) -> Self {
        Self {
            title: title.to_string(),
            icon:  icon.to_string(),
        }
    }
}

/// The colors of the bar, each a pair for the light and the dark theme. The
/// default is the look of the system on iOS.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TabColors {
    pub bar:      DynamicColor,
    /// The thin line on top of the bar.
    pub line:     DynamicColor,
    pub selected: DynamicColor,
    pub normal:   DynamicColor,
}

impl Default for TabColors {
    fn default() -> Self {
        Self {
            bar:      DynamicColor::new(Color::hex("#F9F9F9"), Color::hex("#161616")),
            line:     DynamicColor::new(Color::hex("#D1D1D6"), Color::hex("#38383A")),
            selected: DynamicColor::new(Color::hex("#007AFF"), Color::hex("#0A84FF")),
            normal:   DynamicColor::new(Color::hex("#8E8E93"), Color::hex("#8E8E93")),
        }
    }
}

/// Makes the page of a tab, by the index of the tab.
type PageBuilder = Box<dyn FnMut(usize) -> Own<dyn View> + Send>;

/// The page of the open tab and the bar under it. A tap on a tab makes its
/// page with the builder of `set_tabs` and drops the stack before. The bar
/// lives on, only the stack of the page is swapped.
#[view]
pub struct TabShell {
    /// The index of the tab that opened, after a tap or `select`.
    pub changed: Event<usize>,

    selected: usize,
    build:    Option<PageBuilder>,
    stack:    Weak<NavigationView>,

    #[init]
    content: Container,
    bar:     TabBar,
}

impl Setup for TabShell {
    fn setup(self: Weak<Self>) {
        self.bar.place().lr(0).b(0).h(TAB_BAR_HEIGHT);
        self.content.place().t(0).lr(0).anchor(Anchor::Bot, self.bar, 0);
        self.bar.tapped.val(move |index| self.select(index));
    }
}

impl TabShell {
    /// The tabs and the builder of their pages. No page is open after it,
    /// `select` opens the first one.
    pub fn set_tabs(
        mut self: Weak<Self>,
        tabs: Vec<Tab>,
        build: impl FnMut(usize) -> Own<dyn View> + Send + 'static,
    ) {
        self.build = Some(Box::new(build));
        self.bar.set_tabs(tabs);
    }

    pub fn set_colors(self: Weak<Self>, colors: TabColors) {
        self.bar.set_colors(colors);
    }

    /// Opens the page of a tab and marks it in the bar. A call from a view
    /// inside the open page has to wait 1 frame, `after(0.0, ...)`, that
    /// view dies in the swap.
    pub fn select(mut self: Weak<Self>, index: usize) {
        if index >= self.bar.count() {
            warn!("tab {index} was asked for, the bar has {}", self.bar.count());
            return;
        }
        let Some(mut build) = self.build.take() else {
            warn!("tab {index} was asked for before set_tabs");
            return;
        };
        debug!("tab {index} opens");
        self.selected = index;
        self.bar.mark(index);
        let page = build(index);
        self.build = Some(build);
        if self.stack.is_ok() {
            self.stack.remove_from_superview();
        }
        self.stack = self.content.add_subview(NavigationView::with_view(page));
        self.stack.place().back();
        self.changed.trigger(index);
    }

    /// The stack of the open tab. A detail page of the tab is pushed on it,
    /// the tab in the bar stays marked.
    pub fn navigation(self: Weak<Self>) -> Weak<NavigationView> {
        assert!(self.stack.is_ok(), "no tab is open, select one first");
        self.stack
    }

    /// Where the stack of the open tab sits.
    pub fn content(self: Weak<Self>) -> Weak<Container> {
        self.content
    }

    /// The bar, for a view of the app that sits on top of it.
    pub fn bar(self: Weak<Self>) -> Weak<TabBar> {
        self.bar
    }

    pub fn selected(self: Weak<Self>) -> usize {
        self.selected
    }
}
