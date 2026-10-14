use std::ops::Deref;

use crate::{
    self as hilen,
    deps::refs::{Own, Weak, main_lock::MainLock},
    gm::{
        color::{BLACK, CLEAR, Color, WHITE},
        flat::{Point, Rect},
    },
    ui::{
        BlurTail, BlurView, Container, DrawingView, DynamicColor, FillRule, Font, ImageView, Input, Label,
        Setup, Shadow, StrokeStyle, TailEdge, TextAlignment, Touch, TouchStack, UIColor, UIManager,
        VectorPath, View, ViewCallbacks, ViewData, ViewFrame, ViewSubviews, ViewTouch, view,
    },
    window::{
        NamedKey,
        image::{Tinted, ToImage, tint_svg},
    },
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
const ICON_SIZE: f32 = 16.0;
const ICON_GAP: f32 = 8.0;
const ANCHOR_GAP: f32 = 4.0;

const CHECK_ICON: &[u8] = include_bytes!("../../images/check.svg");

/// How a [`ContextMenu`] looks. [`MenuStyle::default`] is the plain menu
/// that follows the system theme, the one the `ContextMenu::show` calls
/// open. A menu with its own look, like one over a video, takes the
/// default, changes the fields it needs and opens through the style:
///
/// ```ignore
/// let style = MenuStyle { blur: 16.0, ..MenuStyle::default() };
/// style.show_above(items, button, MenuAlign::Right);
/// ```
#[derive(Clone, Copy)]
pub struct MenuStyle {
    /// The fill of the menu. With a `blur` it tints the blurred picture,
    /// so a half clear color lets what is under the menu show through.
    pub background:    DynamicColor,
    /// Blurs what is under the menu by this radius in points. Zero draws
    /// no blur.
    pub blur:          f32,
    pub border:        DynamicColor,
    /// Zero draws no border.
    pub border_width:  f32,
    /// The line of [`MenuItem::separator`].
    pub separator:     DynamicColor,
    pub text:          DynamicColor,
    pub disabled_text: DynamicColor,
    pub danger_text:   DynamicColor,
    /// The row under the pointer.
    pub highlight:     DynamicColor,
    pub corner_radius: f32,
    pub item_height:   f32,
    pub text_size:     f32,
    /// The font of the titles, the default font when `None`.
    pub font:          Option<Weak<Font>>,
    pub shadow:        bool,
    /// The height of an arrow on the edge of the menu that faces the
    /// anchor of a `show_above` or `show_below` call, with its tip at the
    /// middle of the anchor. It is twice as wide as it is tall. Zero draws
    /// none. A menu with an arrow and a blur draws no shadow.
    pub arrow:         f32,
}

impl Default for MenuStyle {
    fn default() -> Self {
        Self {
            background:    BACKGROUND,
            blur:          0.0,
            border:        BORDER,
            border_width:  1.0,
            separator:     BORDER,
            text:          TEXT,
            disabled_text: DISABLED_TEXT,
            danger_text:   DANGER_TEXT,
            highlight:     HIGHLIGHT,
            corner_radius: CORNER_RADIUS,
            item_height:   ITEM_HEIGHT,
            text_size:     TEXT_SIZE,
            font:          None,
            shadow:        true,
            arrow:         0.0,
        }
    }
}

impl MenuStyle {
    /// [`ContextMenu::show_at_cursor`] with this look.
    pub fn show_at_cursor(self, items: Vec<MenuItem>) -> Weak<ContextMenu> {
        self.show(items, UIManager::cursor_position())
    }

    /// [`ContextMenu::show`] with this look.
    pub fn show(self, items: Vec<MenuItem>, at: Point) -> Weak<ContextMenu> {
        ContextMenu::present(items, Placement::At(at), &self)
    }

    /// [`ContextMenu::show_below`] with this look.
    pub fn show_below(
        self,
        items: Vec<MenuItem>,
        anchor: impl Deref<Target = impl View + ?Sized>,
        align: MenuAlign,
    ) -> Weak<ContextMenu> {
        ContextMenu::present(items, Placement::Below(*anchor.absolute_frame(), align), &self)
    }

    /// [`Self::show_below`] at a frame in screen points instead of a view.
    pub fn show_below_frame(self, items: Vec<MenuItem>, anchor: Rect, align: MenuAlign) -> Weak<ContextMenu> {
        ContextMenu::present(items, Placement::Below(anchor, align), &self)
    }

    /// [`Self::show_above`] at a frame in screen points instead of a view,
    /// for an anchor that is no single view, like the middle of a button
    /// together with the top of a bar over it.
    pub fn show_above_frame(self, items: Vec<MenuItem>, anchor: Rect, align: MenuAlign) -> Weak<ContextMenu> {
        ContextMenu::present(items, Placement::Above(anchor, align), &self)
    }

    /// [`ContextMenu::show_above`] with this look.
    pub fn show_above(
        self,
        items: Vec<MenuItem>,
        anchor: impl Deref<Target = impl View + ?Sized>,
        align: MenuAlign,
    ) -> Weak<ContextMenu> {
        ContextMenu::present(items, Placement::Above(*anchor.absolute_frame(), align), &self)
    }
}

/// A row of a [`ContextMenu`]. Build with [`MenuItem::new`] or
/// [`MenuItem::separator`], then chain `icon`, `checked`, `disabled`,
/// `danger` and `badge`.
pub struct MenuItem {
    title:   String,
    action:  Option<Box<dyn FnMut() + Send>>,
    enabled: bool,
    danger:  bool,
    badges:  Vec<(String, UIColor)>,
    icon:    Option<String>,
    checked: Option<bool>,
}

impl MenuItem {
    pub fn new(title: impl ToString, action: impl FnMut() + Send + 'static) -> Self {
        Self {
            title:   title.to_string(),
            action:  Some(Box::new(action)),
            enabled: true,
            danger:  false,
            badges:  vec![],
            icon:    None,
            checked: None,
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
            icon:    None,
            checked: None,
        }
    }

    /// An icon in front of the title, the name of an SVG asset drawn in
    /// black, the way [`Tinted`] expects. It takes the color of the
    /// title, gray when disabled and red on a `danger` item. Once one
    /// item has an icon, every title in the menu moves right to line up.
    pub fn icon(mut self, name: impl ToString) -> Self {
        self.icon = Some(name.to_string());
        self
    }

    /// Marks the item as one choice of several. A checked item draws a
    /// check in front of its title, an unchecked one leaves the place
    /// empty, so the titles of all choices line up.
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = Some(checked);
        self
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

/// Which leading places the rows of one menu reserve, so every title
/// starts at the same x.
#[derive(Clone, Copy)]
struct Leading {
    check: bool,
    icon:  bool,
}

impl Leading {
    fn of(items: &[MenuItem]) -> Self {
        Self {
            check: items.iter().any(|item| item.checked.is_some()),
            icon:  items.iter().any(|item| item.icon.is_some()),
        }
    }

    /// Where the title starts inside a row.
    fn title_x(self) -> f32 {
        let places = u8::from(self.check) + u8::from(self.icon);
        SIDE_PADDING + f32::from(places) * (ICON_SIZE + ICON_GAP)
    }
}

#[view]
pub struct MenuItemView {
    action:    Option<Box<dyn FnMut() + Send>>,
    enabled:   bool,
    badges:    Vec<Weak<Label>>,
    danger:    bool,
    icon_name: Option<String>,
    check:     Weak<ImageView>,
    icon:      Weak<ImageView>,
    style:     MenuStyle,

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

    /// The check of a checked item, null otherwise.
    pub fn check(&self) -> Weak<ImageView> {
        self.check
    }

    /// The leading icon, null when the item has none.
    pub fn icon(&self) -> Weak<ImageView> {
        self.icon
    }

    /// Points the badges take at the right edge of the row, including the
    /// gap between the title and the first badge. Zero without badges.
    fn badges_width(&self) -> f32 {
        self.badges
            .iter()
            .map(|badge| badge.content_size().width.ceil() + BADGE_GAP)
            .sum()
    }

    fn fill(mut self: Weak<Self>, item: MenuItem, leading: Leading, style: &MenuStyle) {
        self.style = *style;
        self.set_corner_radius(style.corner_radius - PADDING);
        self.label.set_text_size(style.text_size);
        if let Some(font) = style.font {
            self.label.set_font(font);
        }

        self.enabled = item.enabled;
        self.action = item.action;
        self.label.set_text(&item.title);
        self.label.place().tb(0).l(leading.title_x()).r(SIDE_PADDING);

        let mut x = SIDE_PADDING;
        if leading.check {
            if item.checked == Some(true) {
                self.check = self.leading_image(x);
            }
            x += ICON_SIZE + ICON_GAP;
        }
        if item.icon.is_some() {
            self.icon = self.leading_image(x);
        }
        self.icon_name = item.icon;

        let mut offset = SIDE_PADDING;
        for (text, color) in item.badges.iter().rev() {
            let badge = self.add_view::<Label>();
            badge.set_text(text).set_text_size(BADGE_TEXT_SIZE).set_text_color(*color);
            let width = badge.content_size().width.ceil();
            badge.place().r(offset).center_y().size(width, style.item_height);
            offset += width + BADGE_GAP;
            self.badges.push(badge);
        }
        self.badges.reverse();

        self.danger = item.danger;
        self.label.set_text_color(self.text_color());
        self.tint_images();
    }

    fn text_color(&self) -> DynamicColor {
        if !self.enabled {
            self.style.disabled_text
        } else if self.danger {
            self.style.danger_text
        } else {
            self.style.text
        }
    }

    fn leading_image(self: Weak<Self>, x: f32) -> Weak<ImageView> {
        let image = self.add_view::<ImageView>();
        image.place().l(x).center_y().size(ICON_SIZE, ICON_SIZE);
        image
    }

    /// The icons are SVGs with the color written in, so a theme change
    /// has to tint them again, a label follows the theme by itself.
    fn tint_images(&self) {
        let color = self.text_color().resolve();

        if self.check.is_ok() {
            self.check.set_image(tint_svg(CHECK_ICON, "menu_check.svg", color));
        }

        if let Some(name) = &self.icon_name {
            self.icon.set_image(
                Tinted {
                    tint: color,
                    name: name.clone(),
                }
                .to_image(),
            );
        }
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

impl ViewCallbacks for MenuItemView {
    fn theme_changed(&mut self) {
        self.tint_images();
    }
}

impl Setup for MenuItemView {
    fn setup(self: Weak<Self>) {
        self.set_color(CLEAR);

        self.label.set_color(CLEAR).set_alignment(TextAlignment::Left);

        self.enable_touch();
        self.enable_hover();

        self.touch().hovered.val(self, move |hovered| {
            if self.enabled {
                let color: UIColor = if hovered {
                    self.style.highlight.into()
                } else {
                    CLEAR.into()
                };
                self.set_color(color);
            }
        });

        self.touch().up_inside.sub(self, move || self.tapped());
    }
}

static OPEN: MainLock<Weak<ContextMenu>> = MainLock::new();

/// Which edge of the anchor view a menu from [`ContextMenu::show_below`]
/// or [`ContextMenu::show_above`] lines up with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuAlign {
    Left,
    Right,
    /// The middle of the menu over the middle of the anchor, the place
    /// for a menu with an arrow.
    Center,
}

impl MenuAlign {
    /// The left edge of a menu `width` wide lined up with `anchor`.
    fn x(self, anchor: Rect, width: f32) -> f32 {
        match self {
            Self::Left => anchor.x(),
            Self::Right => anchor.max_x() - width,
            Self::Center => anchor.x() + (anchor.width() - width) / 2.0,
        }
    }
}

#[derive(Clone, Copy)]
enum Placement {
    At(Point),
    Below(Rect, MenuAlign),
    Above(Rect, MenuAlign),
}

/// A floating list of actions anchored at a point, usually the cursor,
/// or under or over a view, like a button's dropdown. One column, separators,
/// icons, checks, disabled items, a danger tint, no submenus.
/// Dismissed by a tap outside, Escape, or picking an item. Only one is
/// open at a time, opening another closes the first.
///
/// Open it from a view's `secondary` touch event, which is a right click
/// on desktop and in the browser and a long press on a touch screen.
#[view]
pub struct ContextMenu {
    items:      Vec<Weak<MenuItemView>>,
    style:      MenuStyle,
    /// The edge with the arrow, `None` for a menu without one.
    arrow_edge: Option<TailEdge>,
    /// The panel and its arrow with a blur, null without a blur.
    blur:       Weak<BlurView>,
    /// The arrow of a menu without a blur, null otherwise.
    arrow:      Weak<DrawingView>,
}

impl ContextMenu {
    /// Opens at the last known cursor or touch position.
    pub fn show_at_cursor(items: Vec<MenuItem>) -> Weak<Self> {
        MenuStyle::default().show_at_cursor(items)
    }

    /// `at` is an absolute screen position in points. The menu opens with
    /// its top left corner there and slides back inside the screen when
    /// it would not fit.
    pub fn show(items: Vec<MenuItem>, at: Point) -> Weak<Self> {
        MenuStyle::default().show(items, at)
    }

    /// Opens just under `anchor`, with the `align` edge of the menu on the
    /// same edge of the anchor, so a menu from a button at the right of a
    /// toolbar grows to the left and covers nothing beside the button.
    /// Slides back inside the screen when it would not fit.
    pub fn show_below(
        items: Vec<MenuItem>,
        anchor: impl Deref<Target = impl View + ?Sized>,
        align: MenuAlign,
    ) -> Weak<Self> {
        MenuStyle::default().show_below(items, anchor, align)
    }

    /// Opens just over `anchor`, the bottom edge of the menu a gap above
    /// the top edge of the anchor, for a button in a bar at the bottom of
    /// the window, where a menu under the button would slide up over it.
    /// `align` works like in [`Self::show_below`]. Slides back inside the
    /// screen when it would not fit.
    pub fn show_above(
        items: Vec<MenuItem>,
        anchor: impl Deref<Target = impl View + ?Sized>,
        align: MenuAlign,
    ) -> Weak<Self> {
        MenuStyle::default().show_above(items, anchor, align)
    }

    fn present(items: Vec<MenuItem>, placement: Placement, style: &MenuStyle) -> Weak<Self> {
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
        // A right press outside closes the menu and goes on to the view
        // under it, which opens its own menu with the same press.
        backdrop.touch().secondary.val(backdrop, move |mut touch: Touch| {
            // The closed backdrop lives until the frame ends and its
            // touch would move to the layer below. Out first, or the
            // press comes back to this handler while it still runs.
            backdrop.disable_touch();
            Self::dismiss_open();
            touch.position = UIManager::cursor_position();
            Input::offer_to_views(&mut touch);
        });

        backdrop.add_subview(menu);
        weak.fill(items, placement, style);

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

    fn fill(mut self: Weak<Self>, items: Vec<MenuItem>, placement: Placement, style: &MenuStyle) {
        self.style = *style;
        self.arrow_edge = match placement {
            _ if style.arrow <= 0.0 => None,
            Placement::At(_) => None,
            Placement::Below(..) => Some(TailEdge::Top),
            Placement::Above(..) => Some(TailEdge::Bottom),
        };
        let arrow = if self.arrow_edge.is_some() {
            style.arrow
        } else {
            0.0
        };
        // The rows start under an arrow on the top edge.
        let top = if self.arrow_edge == Some(TailEdge::Top) {
            arrow
        } else {
            0.0
        };

        // The fill goes in first, so the rows draw over it.
        if self.arrow_edge.is_some() {
            self.paint_with_arrow(top, arrow);
        } else {
            self.paint();
        }

        let leading = Leading::of(&items);
        let mut y = top + PADDING;
        let mut width: f32 = MIN_WIDTH;

        for item in items {
            if item.is_separator() {
                let line = self.add_view::<Container>();
                line.set_color(style.separator);
                line.place().t(y + (SEPARATOR_HEIGHT / 2.0).floor()).lr(SIDE_PADDING).h(1);
                y += SEPARATOR_HEIGHT;
                continue;
            }

            let view = self.add_view::<MenuItemView>();
            view.fill(item, leading, style);
            view.place().t(y).lr(PADDING).h(style.item_height);
            width = width.max(
                leading.title_x()
                    + view.label.content_size().width
                    + view.badges_width()
                    + SIDE_PADDING
                    + PADDING * 2.0,
            );
            y += style.item_height;

            self.items.push(view);
        }

        let height = y + PADDING + arrow - top;
        let width = width.ceil();

        self.place().custom(move |rect: &mut Rect| {
            let screen = UIManager::root_view().frame().size;
            let anchor_middle = match placement {
                Placement::At(at) => at.x,
                Placement::Below(anchor, _) | Placement::Above(anchor, _) => anchor.center().x,
            };
            let at = match placement {
                Placement::At(at) => at,
                Placement::Below(anchor, align) => {
                    Point::new(align.x(anchor, width), anchor.max_y() + ANCHOR_GAP)
                }
                Placement::Above(anchor, align) => {
                    Point::new(align.x(anchor, width), anchor.y() - ANCHOR_GAP - height)
                }
            };
            let x = at.x.min(screen.width - width).max(0.0).round();
            let y = at.y.min(screen.height - height).max(0.0).round();
            *rect = Rect::new(x, y, width, height);
            self.point_arrow(anchor_middle - x, width, height);
        });
    }

    /// A menu without an arrow is one rounded view, with a blur behind
    /// its rows when the style has one.
    fn paint(self: Weak<Self>) {
        let style = self.style;
        self.set_border_color(style.border)
            .set_border_width(style.border_width)
            .set_corner_radius(style.corner_radius);
        if style.shadow {
            self.set_shadow(Shadow::default());
        }

        if style.blur > 0.0 {
            self.set_color(CLEAR);
            let mut blur = self.add_view::<BlurView>();
            blur.set_blur_radius(style.blur);
            blur.set_color(style.background).set_corner_radius(style.corner_radius);
            blur.place().back();
        } else {
            self.set_color(style.background);
        }
    }

    /// The menu itself stays clear, its frame holds the panel and the
    /// arrow. With a blur both are one `BlurView` with a tail, so they
    /// share 1 blur and 1 outline. Without one the panel is a rounded view
    /// and the arrow a triangle over its edge.
    fn paint_with_arrow(mut self: Weak<Self>, top: f32, arrow: f32) {
        let style = self.style;
        self.set_color(CLEAR);

        if style.blur > 0.0 {
            let mut blur = self.add_view::<BlurView>();
            blur.set_blur_radius(style.blur);
            blur.set_color(style.background)
                .set_border_color(style.border)
                .set_border_width(style.border_width)
                .set_corner_radius(style.corner_radius);
            blur.place().back();
            self.blur = blur;
            return;
        }

        let panel = self.add_view::<Container>();
        panel
            .set_color(style.background)
            .set_border_color(style.border)
            .set_border_width(style.border_width)
            .set_corner_radius(style.corner_radius);
        if style.shadow {
            panel.set_shadow(Shadow::default());
        }
        panel.place().lr(0).t(top).b(arrow - top);

        self.arrow = self.add_view::<DrawingView>();
        self.paint_arrow();
    }

    /// The triangle of a menu without a blur, in its own frame. Its base
    /// goes as deep as the border into the panel, so it covers the border
    /// line there and the panel and the arrow read as one shape.
    fn paint_arrow(&self) {
        let (Some(edge), true) = (self.arrow_edge, self.arrow.is_ok()) else {
            return;
        };
        let style = self.style;
        let half = style.arrow;
        let height = style.arrow + style.border_width;
        let (base, tip) = match edge {
            TailEdge::Bottom => (0.0, height),
            TailEdge::Top => (height, 0.0),
        };

        let mut drawing = self.arrow;
        drawing.remove_all_paths();
        let shape = VectorPath::polygon([(0.0, base), (half * 2.0, base), (half, tip)]);
        drawing.add_fill(&shape, style.background.resolve(), FillRule::NonZero);
        if style.border_width > 0.0 {
            let sides = VectorPath::polyline([(0.0, base), (half, tip), (half * 2.0, base)]);
            drawing.add_stroke(
                &sides,
                style.border.resolve(),
                StrokeStyle::width(style.border_width),
            );
        }
    }

    /// Moves the tip of the arrow to `middle`, from the left edge of the
    /// menu, kept clear of the rounded corners.
    fn point_arrow(self: Weak<Self>, middle: f32, width: f32, height: f32) {
        let Some(edge) = self.arrow_edge else {
            return;
        };
        let style = self.style;
        let half = style.arrow;
        let inset = style.corner_radius + half;
        let middle = if width > inset * 2.0 {
            middle.clamp(inset, width - inset)
        } else {
            width / 2.0
        };

        if self.blur.is_ok() {
            let mut blur = self.blur;
            blur.set_tail(Some(BlurTail {
                edge,
                x: middle,
                width: half * 2.0,
                height: style.arrow,
            }));
        }

        if self.arrow.is_ok() {
            let depth = style.arrow + style.border_width;
            let y = match edge {
                TailEdge::Bottom => height - style.arrow - style.border_width,
                TailEdge::Top => 0.0,
            };
            self.arrow.set_frame((middle - half, y, half * 2.0, depth));
        }
    }
}

impl ViewCallbacks for ContextMenu {
    /// The triangle holds resolved colors, a rounded view follows the
    /// theme by itself.
    fn theme_changed(&mut self) {
        self.paint_arrow();
    }
}

impl Setup for ContextMenu {
    fn setup(self: Weak<Self>) {
        // The menu itself swallows touches, so a tap on the padding does
        // not fall through to the backdrop and dismiss.
        self.enable_touch();
    }
}
