//! A sidebar and a page that follow the width of the shell. Wide, the
//! sidebar stands left of the page. Narrow, like a phone, it hides: a bar
//! over the page holds a menu button, and a tap on it slides the sidebar in
//! over the page with a scrim behind it. A tap on the scrim, Escape, the menu
//! button and `close` slide it out again. The shell picks its form from its
//! own width, not from the window, so it also works in a part of a window.

mod bar;
#[cfg(feature = "ui-tests")]
mod drawer_shell_test;

use log::{debug, warn};

pub use self::bar::DrawerBar;
use crate::{
    self as hilen,
    deps::{refs::Weak, vents::Event},
    gm::{Animation, color::Color},
    ui::{
        Container, DynamicColor, NamedKey, ScrimView, Setup, TouchStack, UIAnimation, UIColor, UIManager,
        View, ViewData, ViewFrame, ViewTouch, view,
    },
};

/// The height of the bar over the page of a narrow shell.
pub const DRAWER_BAR_HEIGHT: f32 = 44.0;

/// A strip of the page stays in sight right of an open drawer, and a tap
/// there closes it, as on a phone.
const PAGE_EDGE: f32 = 56.0;

const SLIDE_SECONDS: f32 = 0.2;

/// The colors of the shell, each a pair for the light and the dark theme.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DrawerColors {
    /// The bar over the page of a narrow shell.
    pub bar:   DynamicColor,
    /// The thin line under the bar.
    pub line:  DynamicColor,
    /// The menu icon and the title in the bar.
    pub text:  DynamicColor,
    /// The page under an open drawer.
    pub scrim: DynamicColor,
}

impl Default for DrawerColors {
    fn default() -> Self {
        Self {
            bar:   DynamicColor::new(Color::hex("#F9F9F9"), Color::hex("#161616")),
            line:  DynamicColor::new(Color::hex("#D1D1D6"), Color::hex("#38383A")),
            text:  DynamicColor::new(Color::hex("#1C1C1E"), Color::hex("#F2F2F7")),
            scrim: DynamicColor::new(Color::hex("#00000059"), Color::hex("#00000080")),
        }
    }
}

/// The scrim and the panel of the sidebar. While the drawer is open it is
/// the root of a touch layer, so the page under the scrim takes no touch.
#[view]
pub struct DrawerLayer {
    #[init]
    scrim: ScrimView,
    panel: Container,
}

impl Setup for DrawerLayer {
    fn setup(self: Weak<Self>) {}
}

/// A sidebar next to a page on a wide shell, in a drawer on a narrow one.
/// The app puts its sidebar into `side` and its page into `content`, both
/// filled with `place().back()`. Both live on through every change of the
/// width.
#[view]
pub struct DrawerShell {
    /// True when the sidebar slid in over the page, false when it slid out.
    pub opened:         Event<bool>,
    /// True when the shell turned narrow, false when it turned wide. The
    /// first layout fires it too.
    pub narrow_changed: Event<bool>,

    side_width:   f32,
    narrow_below: f32,
    narrow:       Option<bool>,
    open:         bool,
    /// The touch layer of the open drawer is on the stack.
    layered:      bool,
    /// How far the drawer is in, 0 out of sight and 1 open.
    shown:        f32,
    /// Counts the slides, a slide that a newer one replaced does nothing.
    slide:        u32,

    #[init]
    content: Container,
    bar:     DrawerBar,
    layer:   DrawerLayer,
}

impl Setup for DrawerShell {
    fn setup(mut self: Weak<Self>) {
        self.side_width = 260.0;
        self.narrow_below = 700.0;
        self.layer.scrim.set_color(DrawerColors::default().scrim);

        self.bar.menu.on_tap(move || self.toggle());

        let layer = self.layer;
        layer.scrim.touch().began.val(move |_| {
            if !layer.panel.absolute_frame().contains(UIManager::cursor_position()) {
                self.close();
            }
        });
        // Bound on the layer, which holds the top touch layer while the
        // drawer is open, so the Escape of a page under it stays silent.
        UIManager::keymap().add(self.layer, NamedKey::Escape, move || {
            if self.open && !UIManager::text_editing() {
                self.close();
            }
        });

        self.size_changed().sub(move || self.relayout());
    }
}

impl DrawerShell {
    /// Where the app puts its sidebar.
    pub fn side(self: Weak<Self>) -> Weak<Container> {
        self.layer.panel
    }

    /// Where the app puts its page, right of the sidebar or under the bar.
    pub fn content(self: Weak<Self>) -> Weak<Container> {
        self.content
    }

    /// The bar over the page of a narrow shell, for a view of the app that
    /// sits in it.
    pub fn bar(self: Weak<Self>) -> Weak<DrawerBar> {
        self.bar
    }

    /// The text in the bar right of the menu button, like the name of the
    /// open page.
    pub fn set_title(self: Weak<Self>, title: &str) {
        self.bar.set_title(title);
    }

    /// An icon left of the title in the bar, like the icon of the open
    /// project: the name of an SVG in `assets/images` drawn in black, shown
    /// in `tint`. A `DynamicColor` follows the theme.
    pub fn set_title_icon(self: Weak<Self>, name: &str, tint: impl Into<UIColor>) {
        self.bar.set_title_icon(Some((name, tint.into())));
    }

    /// The title stands alone again, right of the menu button.
    pub fn clear_title_icon(self: Weak<Self>) {
        self.bar.set_title_icon(None);
    }

    pub fn set_colors(self: Weak<Self>, colors: DrawerColors) {
        self.bar.paint(colors);
        self.layer.scrim.set_color(colors.scrim);
    }

    /// The width of the sidebar, 260 by default. An open drawer leaves a
    /// strip of the page in sight on a shell narrower than that.
    pub fn set_side_width(mut self: Weak<Self>, width: f32) {
        self.side_width = width;
        self.relayout();
    }

    /// The shell is narrow below this width, 700 by default.
    pub fn set_narrow_below(mut self: Weak<Self>, width: f32) {
        self.narrow_below = width;
        self.relayout();
    }

    pub fn is_narrow(self: Weak<Self>) -> bool {
        self.narrow == Some(true)
    }

    /// The drawer is open, or slides in. Always false on a wide shell.
    pub fn is_open(self: Weak<Self>) -> bool {
        self.open
    }

    /// Slides the sidebar in over the page. Nothing on a wide shell, the
    /// sidebar is in sight there.
    pub fn open(mut self: Weak<Self>) {
        if !self.is_narrow() || self.open {
            return;
        }
        debug!("the drawer opens");
        self.open = true;
        self.layer.set_hidden(false);
        if !self.layered {
            TouchStack::push_layer(self.layer.weak_view());
            // After the push, a view joins the top layer at registration.
            TouchStack::enable_for_backdrop(self.layer.scrim.weak_view());
            self.layered = true;
        }
        self.slide_to(1.0);
        self.opened.trigger(true);
    }

    /// Slides the sidebar out. A modal over the open drawer holds the top
    /// touch layer, the drawer stays open under it.
    pub fn close(mut self: Weak<Self>) {
        if !self.open {
            return;
        }
        if !self.leave_layer() {
            return;
        }
        debug!("the drawer closes");
        self.open = false;
        self.slide_to(0.0);
        self.opened.trigger(false);
    }

    pub fn toggle(self: Weak<Self>) {
        if self.open {
            self.close();
        } else {
            self.open();
        }
    }

    /// Gives the touches back to the page. False when another layer, like a
    /// modal, sits over the drawer.
    fn leave_layer(mut self: Weak<Self>) -> bool {
        if !self.layered {
            return true;
        }
        if TouchStack::top_layer_root().raw() != self.layer.weak_view().raw() {
            warn!("the drawer stays open, a layer over it holds the touches");
            return false;
        }
        TouchStack::disable_for(self.layer.scrim.weak_view());
        TouchStack::pop_layer(self.layer.weak_view());
        self.layered = false;
        true
    }

    fn slide_to(mut self: Weak<Self>, to: f32) {
        self.slide = self.slide.wrapping_add(1);
        let slide = self.slide;
        let animation = UIAnimation::new(move |_, shown| {
            let mut shell = self;
            if shell.slide == slide {
                shell.shown = shown;
                shell.place_drawer();
            }
        })
        .animation(Animation::new(self.shown, to, SLIDE_SECONDS));
        animation.on_finish.sub(move || {
            let mut shell = self;
            if shell.slide != slide {
                return;
            }
            shell.shown = to;
            shell.place_drawer();
            if !shell.open && shell.is_narrow() {
                shell.layer.set_hidden(true);
            }
        });
        self.layer.add_animation(animation);
    }

    fn panel_width(self: Weak<Self>) -> f32 {
        self.side_width.min(self.width() - PAGE_EDGE).max(0.0)
    }

    /// The panel and the scrim of a narrow shell at the place of `shown`.
    fn place_drawer(self: Weak<Self>) {
        if !self.is_narrow() {
            return;
        }
        let width = self.panel_width();
        self.layer
            .panel
            .set_frame((-width * (1.0 - self.shown), 0, width, self.height()));
        self.layer.scrim.set_opacity(self.shown);
    }

    fn relayout(self: Weak<Self>) {
        let (width, height) = (self.width(), self.height());
        if width <= 0.0 {
            return;
        }
        let narrow = width < self.narrow_below;
        if self.narrow != Some(narrow) {
            self.turn(narrow);
        }

        if narrow {
            self.bar.set_hidden(false);
            self.bar.set_frame((0, 0, width, DRAWER_BAR_HEIGHT));
            self.content
                .set_frame((0, DRAWER_BAR_HEIGHT, width, (height - DRAWER_BAR_HEIGHT).max(0.0)));
            self.layer.set_frame((0, 0, width, height));
            self.layer.scrim.set_hidden(false);
            self.layer.scrim.set_frame((0, 0, width, height));
            self.place_drawer();
        } else {
            let side = self.side_width.min(width);
            self.bar.set_hidden(true);
            self.layer.set_hidden(false);
            self.layer.set_frame((0, 0, side, height));
            self.layer.scrim.set_hidden(true);
            self.layer.panel.set_frame((0, 0, side, height));
            self.content.set_frame((side, 0, width - side, height));
        }
    }

    /// The shell changed its form. A wide shell has no drawer, an open one
    /// gives up its layer at once.
    fn turn(mut self: Weak<Self>, narrow: bool) {
        debug!(
            "the drawer shell turns {}",
            if narrow { "narrow" } else { "wide" }
        );
        self.narrow = Some(narrow);
        self.slide = self.slide.wrapping_add(1);
        if narrow {
            self.shown = 0.0;
            self.layer.set_hidden(true);
        } else if self.open && self.leave_layer() {
            self.open = false;
            self.shown = 0.0;
            self.opened.trigger(false);
        }
        self.narrow_changed.trigger(narrow);
    }
}
