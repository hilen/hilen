//! A panel tied to the view that opened it. On a medium or wide window it
//! floats under or over that view with an arrow at it, on a narrow one it is
//! a sheet that comes up from the bottom. See `docs/popover.md`.

use std::ops::Deref;

use crate::{
    deps::{
        refs::{Own, Weak},
        vents::Event,
    },
    gm::{
        Animation,
        color::{CLEAR, Color, WHITE},
        flat::Rect,
    },
    ui::{
        Container, CornerRadii, DrawingView, DynamicColor, FillRule, NamedKey, Setup, Shadow, StrokeStyle,
        TailEdge, Touch, TouchStack, UIAnimation, UIColor, UIEvents, UIManager, VectorPath, View,
        ViewCallbacks, ViewData, ViewFocus, ViewFrame, ViewSubviews, ViewTouch, WeakView, view,
    },
    window::slots::WindowLock,
};

const BACKGROUND: DynamicColor = DynamicColor::new(WHITE, Color::rgb(0.173, 0.173, 0.18));
const BORDER: DynamicColor = DynamicColor::new(Color::rgb(0.82, 0.82, 0.84), Color::rgb(0.33, 0.33, 0.35));
const SCRIM: DynamicColor =
    DynamicColor::new(Color::rgba(0.0, 0.0, 0.0, 0.2), Color::rgba(0.0, 0.0, 0.0, 0.4));
const HANDLE: DynamicColor = DynamicColor::new(Color::rgb(0.78, 0.78, 0.8), Color::rgb(0.4, 0.4, 0.42));

/// Between the anchor and the tip of the arrow.
const ANCHOR_GAP: f32 = 4.0;
/// The room at the top of a sheet with the grab bar in its middle.
const HANDLE_AREA: f32 = 20.0;
const HANDLE_SIZE: (f32, f32) = (36.0, 5.0);
/// A sheet never covers the top of the window, so it reads as a panel over
/// the page and not as a new page.
const SHEET_TOP_ROOM: f32 = 40.0;
const SLIDE_SECONDS: f32 = 0.25;
/// A drag down past this part of its height closes a sheet, a shorter one
/// puts it back.
const CLOSE_PART: f32 = 1.0 / 3.0;

/// How a [`Popover`] looks. [`PopoverStyle::default`] follows the system
/// theme. An app with its own look changes the fields it needs and opens
/// through the style:
///
/// ```ignore
/// let style = PopoverStyle { background: colors::PANEL, ..PopoverStyle::default() };
/// style.show(content, anchor);
/// ```
#[derive(Clone, Copy)]
pub struct PopoverStyle {
    pub background:    DynamicColor,
    pub border:        DynamicColor,
    /// Zero draws no border.
    pub border_width:  f32,
    pub corner_radius: f32,
    /// The height of the arrow at the anchor, twice as wide as it is tall.
    /// Zero draws none. A sheet has no arrow.
    pub arrow:         f32,
    pub shadow:        bool,
    /// Behind a sheet. A popover dims nothing.
    pub scrim:         DynamicColor,
    /// The grab bar at the top of a sheet.
    pub handle:        DynamicColor,
}

impl Default for PopoverStyle {
    fn default() -> Self {
        Self {
            background:    BACKGROUND,
            border:        BORDER,
            border_width:  1.0,
            corner_radius: 10.0,
            arrow:         8.0,
            shadow:        true,
            scrim:         SCRIM,
            handle:        HANDLE,
        }
    }
}

impl PopoverStyle {
    /// [`Popover::show`] with this look.
    pub fn show(
        self,
        content: Own<dyn View>,
        anchor: impl Deref<Target = impl View + ?Sized>,
    ) -> Weak<Popover> {
        Popover::present(content, *anchor.absolute_frame(), self)
    }
}

static OPEN: WindowLock<Weak<Popover>> = WindowLock::new();

/// A panel with any view in it, tied to the view that opened it. On a
/// medium or wide window it opens under the anchor with an arrow at its
/// middle, or over it when there is no room below. On a narrow window it is
/// a sheet over a scrim that slides up from the bottom, as wide as the
/// window, and a drag down closes it.
///
/// The content comes sized: a popover keeps the size of the view, a sheet
/// keeps its height and gives it the width of the window, so lay its
/// children out with `lr()` and anchors. A press outside, Escape or
/// [`Popover::dismiss_open`] closes it. Only 1 is open at a time.
#[view]
pub struct Popover {
    /// It closed, by any way out.
    pub closed: Event,

    style:      PopoverStyle,
    sheet:      bool,
    /// The edge of a popover with the arrow, `None` for a sheet or no arrow.
    arrow_edge: Option<TailEdge>,
    arrow:      Weak<DrawingView>,
    content:    WeakView,
    /// The full height of a sheet.
    height:     f32,
    /// How far a sheet is below its place, by the slide or a drag.
    lowered:    f32,
    /// The window y of the finger and `lowered` when a drag began.
    drag_from:  Option<(f32, f32)>,
}

impl Popover {
    /// Opens `content` tied to `anchor`, a popover on a medium or wide
    /// window and a sheet on a narrow one.
    pub fn show(content: Own<dyn View>, anchor: impl Deref<Target = impl View + ?Sized>) -> Weak<Self> {
        PopoverStyle::default().show(content, anchor)
    }

    /// The popover or sheet on screen, null when none is open.
    pub fn open() -> Weak<Self> {
        **OPEN
    }

    pub fn dismiss_open() {
        let popover = **OPEN;
        if popover.is_null() {
            return;
        }
        *OPEN.get_mut() = Weak::default();
        popover.dismiss();
    }

    /// Opens as a sheet from the bottom.
    pub fn is_sheet(&self) -> bool {
        self.sheet
    }

    /// The view the popover holds.
    pub fn content(&self) -> WeakView {
        self.content
    }

    fn present(content: Own<dyn View>, anchor: Rect, style: PopoverStyle) -> Weak<Self> {
        Self::dismiss_open();
        let sheet = UIManager::width_class().is_narrow();

        let backdrop = Container::new();
        let fill: UIColor = if sheet { style.scrim.into() } else { CLEAR.into() };
        backdrop.set_color(fill);
        backdrop.set_z_position(UIManager::POPOVER_Z_OFFSET);

        let popover = Own::new(Self::default());
        popover.set_z_position(UIManager::POPOVER_Z_OFFSET - UIManager::subview_z_offset());
        let weak = popover.weak();

        let mut backdrop = UIManager::root_view().add_subview_to_root(backdrop);
        backdrop.set_label("Popover backdrop");
        backdrop.place().back();

        // The layer goes up before anything registers a touch, a view
        // joins the top layer that holds it at registration time.
        TouchStack::push_layer(backdrop);
        backdrop.enable_touch();
        backdrop.touch().began.sub(Self::dismiss_open);

        backdrop.add_subview(popover);
        if sheet {
            weak.fill_sheet(content, style);
        } else {
            weak.fill_popover(content, anchor, style);
        }

        UIManager::keymap().add(weak, NamedKey::Escape, Self::dismiss_open);
        // A sheet and a popover lay out differently, the other kind of
        // window gets neither.
        UIEvents::width_class_changed().val(weak, |_| Self::dismiss_open());

        *OPEN.get_mut() = weak;
        log::debug!("popover opened, sheet {sheet}");
        weak
    }

    fn dismiss(self: Weak<Self>) {
        let mut backdrop = *self.superview();
        TouchStack::pop_layer(backdrop);
        self.closed.trigger(());
        backdrop.remove_from_superview();
    }

    fn fill_popover(mut self: Weak<Self>, content: Own<dyn View>, anchor: Rect, style: PopoverStyle) {
        self.style = style;
        let size = content.size();
        let (width, height) = (size.width.ceil(), size.height.ceil());
        let arrow = style.arrow.max(0.0);
        let total = height + arrow;

        let screen = UIManager::root_view().frame().size;
        let below = anchor.max_y() + ANCHOR_GAP + total <= screen.height;
        self.arrow_edge = match (arrow > 0.0, below) {
            (false, _) => None,
            (true, true) => Some(TailEdge::Top),
            (true, false) => Some(TailEdge::Bottom),
        };
        // The panel starts under an arrow on the top edge.
        let top = if self.arrow_edge == Some(TailEdge::Top) {
            arrow
        } else {
            0.0
        };

        self.set_color(CLEAR);
        let panel = self.add_view::<Container>();
        panel
            .set_color(style.background)
            .set_border_color(style.border)
            .set_border_width(style.border_width)
            .set_corner_radius(style.corner_radius);
        if style.shadow {
            panel.set_shadow(Shadow::default());
        }
        panel.place().lr(0).t(top).h(height);
        self.content = panel.add_subview(content);
        self.content.place().back();

        if self.arrow_edge.is_some() {
            self.arrow = self.add_view::<DrawingView>();
            self.paint_arrow();
        }

        self.place().custom(move |rect: &mut Rect| {
            let screen = UIManager::root_view().frame().size;
            let middle = anchor.center().x;
            let x = (middle - width / 2.0).min(screen.width - width).max(0.0).round();
            let y = if below {
                anchor.max_y() + ANCHOR_GAP
            } else {
                anchor.y() - ANCHOR_GAP - total
            };
            let y = y.min(screen.height - total).max(0.0).round();
            *rect = Rect::new(x, y, width, total);
            self.point_arrow(middle - x, width, total);
        });
    }

    fn fill_sheet(mut self: Weak<Self>, content: Own<dyn View>, style: PopoverStyle) {
        self.style = style;
        self.sheet = true;
        let bottom = UIManager::system_insets().bottom;
        let screen = UIManager::root_view().frame().size;
        self.height =
            (content.size().height.ceil() + HANDLE_AREA + bottom).min(screen.height - SHEET_TOP_ROOM);
        self.lowered = self.height;

        self.set_color(CLEAR);
        // The bottom of a sheet is the bottom of the window, only its top
        // corners are round.
        let panel = self.add_view::<Container>();
        panel
            .set_color(style.background)
            .set_border_color(style.border)
            .set_border_width(style.border_width)
            .set_corner_radii(CornerRadii::top(style.corner_radius));
        if style.shadow {
            panel.set_shadow(Shadow::default());
        }
        panel.place().back();

        let handle = panel.add_view::<Container>();
        handle.set_color(style.handle).set_corner_radius(HANDLE_SIZE.1 / 2.0);
        handle
            .place()
            .center_x()
            .t((HANDLE_AREA - HANDLE_SIZE.1) / 2.0)
            .size(HANDLE_SIZE.0, HANDLE_SIZE.1);

        self.content = panel.add_subview(content);
        self.content.place().clear().t(HANDLE_AREA).lr(0).b(bottom);

        self.place().custom(move |rect: &mut Rect| *rect = self.sheet_frame());

        // A press on the sheet that no control takes is the drag.
        self.touch().all.val(move |touch| self.dragged(touch));
        self.slide_to(0.0, None);
    }

    /// Moves a sheet to `lowered` points under its place, then runs `done`.
    fn slide_to(self: Weak<Self>, lowered: f32, done: Option<fn(Weak<Self>)>) {
        let distance = (self.lowered - lowered).abs();
        let seconds = if self.height > 0.0 {
            SLIDE_SECONDS * distance / self.height
        } else {
            0.0
        };
        let animation = UIAnimation::new(move |_, value| self.lower(value)).animation(Animation::new(
            self.lowered,
            lowered,
            seconds.max(0.01),
        ));
        if let Some(done) = done {
            animation.on_finish.sub(move || done(self));
        }
        self.add_animation(animation);
    }

    fn dragged(mut self: Weak<Self>, touch: Touch) {
        let y = self.absolute_frame().y() + touch.position.y;
        if touch.is_began() {
            self.drag_from = Some((y, self.lowered));
            return;
        }
        let Some((start, lowered)) = self.drag_from else {
            return;
        };
        let moved = (lowered + y - start).max(0.0);
        if touch.is_ended() {
            self.drag_from = None;
            if moved > self.height * CLOSE_PART {
                log::debug!("sheet dragged down {moved} of {}, closes", self.height);
                self.slide_to(self.height, Some(|_| Self::dismiss_open()));
            } else {
                self.slide_to(0.0, None);
            }
            return;
        }
        self.lower(moved);
    }

    /// Puts a sheet `lowered` points under its place at once, the slide and
    /// the drag move it on every frame.
    fn lower(mut self: Weak<Self>, lowered: f32) {
        self.lowered = lowered;
        self.set_frame(self.sheet_frame());
    }

    fn sheet_frame(&self) -> Rect {
        let screen = UIManager::root_view().frame().size;
        let y = screen.height - self.height + self.lowered;
        Rect::new(0.0, y.round(), screen.width, self.height)
    }

    /// The triangle of a popover, in its own frame. Its base goes as deep
    /// as the border into the panel, so it covers the border line there
    /// and the panel and the arrow read as one shape.
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

        let drawing = self.arrow;
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
    /// popover, kept clear of the rounded corners.
    fn point_arrow(self: Weak<Self>, middle: f32, width: f32, height: f32) {
        let (Some(edge), true) = (self.arrow_edge, self.arrow.is_ok()) else {
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
        let depth = style.arrow + style.border_width;
        let y = match edge {
            TailEdge::Bottom => height - style.arrow - style.border_width,
            TailEdge::Top => 0.0,
        };
        self.arrow.set_frame((middle - half, y, half * 2.0, depth));
    }
}

impl ViewCallbacks for Popover {
    /// The triangle holds resolved colors, a rounded view follows the theme
    /// by itself.
    fn theme_changed(&mut self) {
        self.paint_arrow();
    }
}

impl Setup for Popover {
    fn setup(self: Weak<Self>) {
        // The popover swallows touches, so a press on its panel does not
        // fall through to the backdrop and close it.
        self.enable_touch();
        self.set_key_focus(false);
    }
}
