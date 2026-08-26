use std::sync::atomic::{AtomicU64, Ordering};

use crate::{
    deps::{
        hreads::after,
        refs::{Own, main_lock::MainLock},
    },
    gm::{
        color::{BLACK, Color, WHITE},
        flat::{Point, Rect},
    },
    ui::{DynamicColor, Label, Setup, Shadow, UIManager, View, ViewData, ViewFrame, ViewSubviews, WeakView},
};

const BACKGROUND: DynamicColor = DynamicColor::new(Color::rgb(0.2, 0.2, 0.22), Color::rgb(0.85, 0.85, 0.87));
const TEXT: DynamicColor = DynamicColor::new(WHITE, BLACK);

const TEXT_SIZE: f32 = 13.0;
const HEIGHT: f32 = 24.0;
const SIDE_PADDING: f32 = 8.0;
const CORNER_RADIUS: f32 = 4.0;

/// Below and to the right of the cursor, so the pointer does not cover
/// the first letters.
const CURSOR_OFFSET: Point = Point::new(12.0, 18.0);

/// What a view shows as its tooltip, set through `ViewTooltip`.
pub enum TooltipContent {
    Text(String),
    View(Box<dyn Fn() -> Own<dyn View> + Send + Sync>),
}

#[derive(Default)]
struct State {
    /// The view the cursor rests on, waiting for the delay to pass.
    pending: WeakView,
    /// The floating view on screen.
    shown:   WeakView,
}

static STATE: MainLock<State> = MainLock::new();
static GENERATION: AtomicU64 = AtomicU64::new(0);

/// The one floating tooltip. Shows after the cursor rests on a view with
/// a tooltip for `DELAY`, hides when the cursor leaves it or anything is
/// pressed. Never takes touches, so it hides nothing under it.
pub struct Tooltip;

impl Tooltip {
    pub(crate) const DELAY: f32 = 0.5;

    /// The floating view on screen, null while none is shown.
    pub fn shown() -> WeakView {
        STATE.shown
    }

    pub(crate) fn hover_changed(view: WeakView) {
        Self::hide();

        if view.is_null() || view.__base_view().tooltip.is_none() {
            return;
        }

        STATE.get_mut().pending = view;
        let generation = GENERATION.load(Ordering::Relaxed);

        after(Self::DELAY, move || Self::show_pending(generation));
    }

    fn show_pending(generation: u64) {
        if GENERATION.load(Ordering::Relaxed) != generation {
            return;
        }

        let pending = STATE.pending;

        if pending.is_null() || pending.is_hidden_in_tree() {
            return;
        }

        Self::show(pending, UIManager::cursor_position());
    }

    /// `at` is an absolute screen position in points, the floating view
    /// opens a little below and right of it and slides back inside the
    /// screen when it would not fit.
    pub(crate) fn show(source: WeakView, at: Point) {
        Self::hide();

        let Some(content) = &source.__base_view().tooltip else {
            return;
        };

        let (mut view, size): (Own<dyn View>, _) = match content {
            TooltipContent::Text(text) => {
                let label = Label::new();
                label.set_text(text).set_text_size(TEXT_SIZE).set_text_color(TEXT);
                label
                    .set_color(BACKGROUND)
                    .set_corner_radius(CORNER_RADIUS)
                    .set_shadow(Shadow::default());
                let width = (label.content_size().width + SIDE_PADDING * 2.0).ceil();
                (label, (width, HEIGHT))
            }
            TooltipContent::View(make) => {
                let view = make();
                let size = view.size();
                (view, (size.width, size.height))
            }
        };

        view.set_z_position(UIManager::MENU_Z_OFFSET - UIManager::subview_z_offset() * 2.0);
        let mut view = UIManager::root_view().add_subview_to_root(view);
        view.set_label("Tooltip");

        let (width, height) = size;
        let at = at + CURSOR_OFFSET;

        view.place().custom(move |rect: &mut Rect| {
            let screen = UIManager::root_view().frame().size;
            let x = at.x.min(screen.width - width).max(0.0);
            let y = at.y.min(screen.height - height).max(0.0);
            *rect = Rect::new(x.round(), y.round(), width, height);
        });

        STATE.get_mut().shown = view;
    }

    pub fn hide() {
        GENERATION.fetch_add(1, Ordering::Relaxed);

        let state = STATE.get_mut();
        state.pending = WeakView::default();

        let mut shown = state.shown;
        state.shown = WeakView::default();

        if shown.is_ok() {
            shown.remove_from_superview();
        }
    }
}
