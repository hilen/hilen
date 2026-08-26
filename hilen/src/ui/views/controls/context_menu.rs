use crate::{
    self as hilen,
    deps::refs::{Own, Weak, main_lock::MainLock},
    gm::{
        color::{BLACK, CLEAR, Color, WHITE},
        flat::{Point, Rect},
    },
    ui::{
        Container, DynamicColor, Label, Setup, Shadow, TextAlignment, TouchStack, UIColor, UIManager,
        ViewData, ViewFrame, ViewSubviews, ViewTouch, view,
    },
    window::NamedKey,
};

const BACKGROUND: DynamicColor = DynamicColor::new(WHITE, Color::rgb(0.173, 0.173, 0.18));
const BORDER: DynamicColor = DynamicColor::new(Color::rgb(0.82, 0.82, 0.84), Color::rgb(0.33, 0.33, 0.35));
const TEXT: DynamicColor = DynamicColor::new(BLACK, WHITE);
const DISABLED_TEXT: DynamicColor =
    DynamicColor::new(Color::rgb(0.557, 0.557, 0.576), Color::rgb(0.443, 0.443, 0.467));
const DANGER_TEXT: DynamicColor =
    DynamicColor::new(Color::rgb(1.0, 0.231, 0.188), Color::rgb(1.0, 0.271, 0.227));
const HIGHLIGHT: DynamicColor = DynamicColor::new(Color::rgb(0.93, 0.93, 0.94), Color::rgb(0.27, 0.27, 0.29));

const ITEM_HEIGHT: f32 = 28.0;
const SEPARATOR_HEIGHT: f32 = 9.0;
const TEXT_SIZE: f32 = 14.0;
const BADGE_TEXT_SIZE: f32 = 11.0;
const BADGE_GAP: f32 = 6.0;
const PADDING: f32 = 4.0;
const SIDE_PADDING: f32 = 8.0;
const MIN_WIDTH: f32 = 160.0;
const CORNER_RADIUS: f32 = 8.0;

/// A row of a [`ContextMenu`]. Build with [`MenuItem::new`] or
/// [`MenuItem::separator`], then chain `disabled`, `danger` and `badge`.
pub struct MenuItem {
    title:   String,
    action:  Option<Box<dyn FnMut() + Send>>,
    enabled: bool,
    danger:  bool,
    badges:  Vec<(String, UIColor)>,
}

impl MenuItem {
    pub fn new(title: impl ToString, action: impl FnMut() + Send + 'static) -> Self {
        Self {
            title:   title.to_string(),
            action:  Some(Box::new(action)),
            enabled: true,
            danger:  false,
            badges:  vec![],
        }
    }

    /// A thin line between groups of items. Never selectable.
    pub fn separator() -> Self {
        Self {
            title:   String::new(),
            action:  None,
            enabled: false,
            danger:  false,
            badges:  vec![],
        }
    }

    /// A short colored text at the right edge of the row, for status like
    /// a dirty marker or an ahead count. Chain once per badge, they show
    /// in call order left to right.
    pub fn badge(mut self, text: impl ToString, color: impl Into<UIColor>) -> Self {
        self.badges.push((text.to_string(), color.into()));
        self
    }

    /// Grayed out, a tap on it does nothing and the menu stays open.
    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }

    /// Red text for a destructive action.
    pub fn danger(mut self) -> Self {
        self.danger = true;
        self
    }

    fn is_separator(&self) -> bool {
        self.action.is_none()
    }
}

#[view]
pub struct MenuItemView {
    action:  Option<Box<dyn FnMut() + Send>>,
    enabled: bool,
    badges:  Vec<Weak<Label>>,

    #[init]
    label: Label,
}

impl MenuItemView {
    pub fn title(&self) -> &str {
        self.label.text()
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// The badge labels in visual order, left to right.
    pub fn badges(&self) -> &[Weak<Label>] {
        &self.badges
    }

    /// Points the badges take at the right edge of the row, including the
    /// gap between the title and the first badge. Zero without badges.
    fn badges_width(&self) -> f32 {
        self.badges
            .iter()
            .map(|badge| badge.content_size().width.ceil() + BADGE_GAP)
            .sum()
    }

    fn fill(mut self: Weak<Self>, item: MenuItem) {
        self.enabled = item.enabled;
        self.action = item.action;
        self.label.set_text(&item.title);

        let mut offset = SIDE_PADDING;
        for (text, color) in item.badges.iter().rev() {
            let badge = self.add_view::<Label>();
            badge.set_text(text).set_text_size(BADGE_TEXT_SIZE).set_text_color(*color);
            let width = badge.content_size().width.ceil();
            badge.place().r(offset).center_y().size(width, ITEM_HEIGHT);
            offset += width + BADGE_GAP;
            self.badges.push(badge);
        }
        self.badges.reverse();

        let color = if !item.enabled {
            DISABLED_TEXT
        } else if item.danger {
            DANGER_TEXT
        } else {
            TEXT
        };

        self.label.set_text_color(color);
    }

    fn tapped(mut self: Weak<Self>) {
        if !self.enabled {
            return;
        }

        // The menu is gone before the action runs, so an action that opens
        // another menu or a dialog never fights the one it came from.
        let Some(mut action) = self.action.take() else {
            return;
        };

        ContextMenu::dismiss_open();
        action();
    }
}

impl Setup for MenuItemView {
    fn setup(self: Weak<Self>) {
        self.set_color(CLEAR).set_corner_radius(CORNER_RADIUS - PADDING);

        self.label
            .set_color(CLEAR)
            .set_alignment(TextAlignment::Left)
            .set_text_size(TEXT_SIZE);
        self.label.place().tb(0).l(SIDE_PADDING).r(SIDE_PADDING);

        self.enable_touch();
        self.enable_hover();

        self.touch().hovered.val(self, move |hovered| {
            if self.enabled {
                let color: UIColor = if hovered { HIGHLIGHT.into() } else { CLEAR.into() };
                self.set_color(color);
            }
        });

        self.touch().up_inside.sub(self, move || self.tapped());
    }
}

static OPEN: MainLock<Weak<ContextMenu>> = MainLock::new();

/// A floating list of actions anchored at a point, usually the cursor.
/// One column, separators, disabled items, a danger tint, no submenus.
/// Dismissed by a tap outside, Escape, or picking an item. Only one is
/// open at a time, opening another closes the first.
///
/// Open it from a view's `secondary` touch event, which is a right click
/// on desktop and in the browser and a long press on a touch screen.
#[view]
pub struct ContextMenu {
    items: Vec<Weak<MenuItemView>>,
}

impl ContextMenu {
    /// Opens at the last known cursor or touch position.
    pub fn show_at_cursor(items: Vec<MenuItem>) -> Weak<Self> {
        Self::show(items, UIManager::cursor_position())
    }

    /// `at` is an absolute screen position in points. The menu opens with
    /// its top left corner there and slides back inside the screen when
    /// it would not fit.
    pub fn show(items: Vec<MenuItem>, at: Point) -> Weak<Self> {
        Self::dismiss_open();

        let mut backdrop = Container::new();
        backdrop.set_color(CLEAR);
        backdrop.set_z_position(UIManager::MENU_Z_OFFSET);

        let mut menu = Own::new(Self::default());
        menu.set_z_position(UIManager::MENU_Z_OFFSET - UIManager::subview_z_offset());
        let weak = menu.weak();

        let mut backdrop = UIManager::root_view().add_subview_to_root(backdrop);
        backdrop.set_label("Context menu backdrop");
        backdrop.place().back();

        // The layer goes up before anything registers a touch, a view
        // joins the top layer that holds it at registration time.
        TouchStack::push_layer(backdrop);
        backdrop.enable_touch();
        backdrop.touch().began.sub(Self::dismiss_open);

        backdrop.add_subview(menu);
        weak.fill(items, at);

        UIManager::keymap().add(weak, NamedKey::Escape, Self::dismiss_open);

        *OPEN.get_mut() = weak;

        weak
    }

    /// The menu on screen, null when none is open.
    pub fn open() -> Weak<Self> {
        *OPEN
    }

    pub fn dismiss_open() {
        let menu = *OPEN;

        if menu.is_null() {
            return;
        }

        *OPEN.get_mut() = Weak::default();
        menu.dismiss();
    }

    pub fn items(&self) -> &[Weak<MenuItemView>] {
        &self.items
    }

    fn dismiss(self: Weak<Self>) {
        let mut backdrop = *self.superview();
        TouchStack::pop_layer(backdrop);
        backdrop.remove_from_superview();
    }

    fn fill(mut self: Weak<Self>, items: Vec<MenuItem>, at: Point) {
        let mut y = PADDING;
        let mut width: f32 = MIN_WIDTH;

        for item in items {
            if item.is_separator() {
                let line = self.add_view::<Container>();
                line.set_color(BORDER);
                line.place().t(y + (SEPARATOR_HEIGHT / 2.0).floor()).lr(SIDE_PADDING).h(1);
                y += SEPARATOR_HEIGHT;
                continue;
            }

            let view = self.add_view::<MenuItemView>();
            view.fill(item);
            view.place().t(y).lr(PADDING).h(ITEM_HEIGHT);
            width = width
                .max(view.label.content_size().width + view.badges_width() + (SIDE_PADDING + PADDING) * 2.0);
            y += ITEM_HEIGHT;

            self.items.push(view);
        }

        let height = y + PADDING;
        let width = width.ceil();

        self.place().custom(move |rect: &mut Rect| {
            let screen = UIManager::root_view().frame().size;
            let x = at.x.min(screen.width - width).max(0.0);
            let y = at.y.min(screen.height - height).max(0.0);
            *rect = Rect::new(x.round(), y.round(), width, height);
        });
    }
}

impl Setup for ContextMenu {
    fn setup(self: Weak<Self>) {
        self.set_color(BACKGROUND)
            .set_border_color(BORDER)
            .set_border_width(1)
            .set_corner_radius(CORNER_RADIUS)
            .set_shadow(Shadow::default());

        // The menu itself swallows touches, so a tap on the padding does
        // not fall through to the backdrop and dismiss.
        self.enable_touch();
    }
}
