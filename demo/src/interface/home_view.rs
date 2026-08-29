use hilen::{
    level::LevelManager,
    refs::{Own, Weak, manage::DataManager},
    system::Router,
    ui::{
        Button, Container, Font, Label, Setup, Shadow, TextAlignment, UIManager, ViewData, ViewFrame,
        ViewSubviews, WHITE, view,
    },
};
use parking_lot::Mutex;

use crate::interface::{
    page::Page,
    palette::{ACCENT, BG, BORDER, SURFACE, TEXT},
    sidebar::{SIDEBAR_WIDTH, Sidebar},
};

// The open home shell, so a card or sidebar tap can swap its page
// without walking the view tree.
static CURRENT: Mutex<Option<Weak<HomeView>>> = Mutex::new(None);

const TOP_BAR_HEIGHT: f32 = 56.0;
// Below this width the sidebar folds into a top bar with a menu button.
const WIDE: f32 = 700.0;

/// The home shell. A sidebar on the left and the open page on the
/// right. On a narrow screen the sidebar hides behind a menu button in
/// a top bar and slides over the page when opened.
#[view]
pub struct HomeView {
    page:      Page,
    menu_open: bool,

    #[init]
    sidebar: Sidebar,
    top_bar: TopBar,
    content: Container,
}

/// The narrow screen header, a menu button and the open page title.
#[view]
struct TopBar {
    #[init]
    menu:  Button,
    title: Label,
}

impl Setup for TopBar {
    fn setup(self: Weak<Self>) {
        self.set_color(SURFACE).set_shadow(Shadow::default());

        self.menu
            .set_text("Menu")
            .set_text_size(14)
            .set_color(ACCENT)
            .set_text_color(WHITE)
            .set_corner_radius(10);
        self.menu.place().l(12).center_y().size(70, 32);

        self.title
            .set_text_color(TEXT)
            .set_text_size(20)
            .set_font(Font::get("RussoOne-Regular.ttf"))
            .set_alignment(TextAlignment::Left);
        self.title.place().l(96).r(12).center_y().h(30);
    }
}

impl Setup for HomeView {
    fn setup(mut self: Weak<Self>) {
        UIManager::set_clear_color(BG);
        CURRENT.lock().replace(self);

        self.top_bar.menu.on_tap(move || {
            self.menu_open = !self.menu_open;
            self.arrange();
        });

        // The sidebar draws over the page when it slides out on a phone.
        self.sidebar.bump_z_position(0.000_1);

        let page = Router::current_path().map_or(Page::Landing, |path| Page::from_path(&path));
        self.show(page);

        Router::on_pop().val(self, move |path| self.show(Page::from_path(&path)));

        self.size_changed().sub(move || self.arrange());
        self.arrange();
    }
}

impl HomeView {
    /// Open a page from anywhere. An in place page swaps the content and
    /// updates the URL, a full screen page replaces the root view.
    pub fn open(page: Page) {
        if page.is_in_place() {
            let home = CURRENT.lock().filter(Weak::is_ok);
            if let Some(home) = home {
                if home.page != page {
                    Router::push(page.path());
                }
                home.show(page);
            } else {
                Router::push(page.path());
                UIManager::set_view(HomeView::new());
            }
        } else {
            page.open_full_screen();
        }
    }

    fn show(mut self: Weak<Self>, page: Page) {
        self.page = page;
        self.menu_open = false;
        // The landing runs the chamber level, every other page has none.
        if page != Page::Landing {
            LevelManager::stop_level();
        }
        self.content.remove_all_subviews();
        let view: Own<dyn hilen::ui::View> = page.make_view();
        self.content.add_subview(view).place().back();
        self.top_bar.title.set_text(page.title());
        self.sidebar.mark(page);
        self.arrange();
    }

    fn arrange(self: Weak<Self>) {
        let wide = self.width() >= WIDE;

        self.top_bar.set_hidden(wide);
        self.sidebar.set_hidden(!wide && !self.menu_open);

        if wide {
            self.sidebar.place().clear().t(0).lb(0).w(SIDEBAR_WIDTH);
            self.content.place().clear().trb(0).l(SIDEBAR_WIDTH);
        } else {
            self.top_bar.place().clear().t(0).lr(0).h(TOP_BAR_HEIGHT);
            self.sidebar.place().clear().t(TOP_BAR_HEIGHT).lb(0).w(SIDEBAR_WIDTH);
            self.content.place().clear().t(TOP_BAR_HEIGHT).lrb(0);
        }
        self.sidebar.set_border_color(BORDER);
    }
}
