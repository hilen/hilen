use hilen::{
    refs::Own,
    ui::{Setup, UIManager, View},
};

use crate::interface::{
    dev::MenuView,
    landing::Landing,
    noise_view::NoiseView,
    root_layout_view::RootLayoutView,
    scenes::{EffectsScene, ScrollTables, TextFonts, WidgetGallery},
};

/// Every screen the sidebar can open. In place pages render inside the
/// home shell next to the sidebar, full screen pages take the whole
/// window because they draw a level behind the UI.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    #[default]
    Landing,
    Widgets,
    Effects,
    Fonts,
    Scrolling,
    Noise,
    Layout,
    Dev,
}

impl Page {
    pub const ALL: [Page; 8] = [
        Page::Landing,
        Page::Effects,
        Page::Noise,
        Page::Widgets,
        Page::Fonts,
        Page::Scrolling,
        Page::Layout,
        Page::Dev,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Page::Landing => "Home",
            Page::Widgets => "Views",
            Page::Effects => "Effects",
            Page::Fonts => "Fonts",
            Page::Scrolling => "Scrolling",
            Page::Noise => "Noise",
            Page::Layout => "Layout",
            Page::Dev => "Dev",
        }
    }

    /// Lucide line icons, <https://lucide.dev>, ISC licensed.
    pub fn icon(self) -> &'static str {
        match self {
            Page::Landing => "nav_home.svg",
            Page::Widgets => "nav_views.svg",
            Page::Effects => "nav_effects.svg",
            Page::Fonts => "nav_fonts.svg",
            Page::Scrolling => "nav_scrolling.svg",
            Page::Noise => "nav_noise.svg",
            Page::Layout => "nav_layout.svg",
            Page::Dev => "nav_dev.svg",
        }
    }

    /// The browser path of an in place page. Full screen pages have no
    /// path, so the URL keeps the page the user came from and Back
    /// restores it.
    pub fn path(self) -> &'static str {
        match self {
            Page::Widgets => "views",
            Page::Effects => "effects",
            Page::Fonts => "fonts",
            Page::Scrolling => "scrolling",
            Page::Dev => "dev",
            _ => "",
        }
    }

    pub fn from_path(path: &str) -> Page {
        Page::ALL
            .into_iter()
            .find(|page| page.is_in_place() && page.path() == path.trim_matches('/'))
            .unwrap_or(Page::Landing)
    }

    pub fn is_in_place(self) -> bool {
        matches!(
            self,
            Page::Landing | Page::Widgets | Page::Effects | Page::Fonts | Page::Scrolling | Page::Dev
        )
    }

    /// The view of an in place page, to add into the home content area.
    pub fn make_view(self) -> Own<dyn View> {
        match self {
            Page::Widgets => WidgetGallery::new(),
            Page::Effects => EffectsScene::new(),
            Page::Fonts => TextFonts::new(),
            Page::Scrolling => ScrollTables::new(),
            Page::Dev => MenuView::new(),
            _ => Landing::new(),
        }
    }

    /// Replace the root view with a full screen page.
    pub fn open_full_screen(self) {
        match self {
            Page::Noise => {
                UIManager::set_view(NoiseView::new());
            }
            Page::Layout => {
                UIManager::set_view(RootLayoutView::new());
            }
            _ => {}
        }
    }
}
