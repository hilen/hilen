use std::sync::atomic::{AtomicU64, Ordering};

use crate::{
    deps::{
        hreads::after,
        refs::{Own, main_lock::MainLock},
    },
    gm::{
        color::{BLACK, Color, WHITE},
        flat::Rect,
    },
    ui::{DynamicColor, Label, Setup, Shadow, UIManager, View, ViewData, ViewFrame, ViewSubviews, WeakView},
};

const BACKGROUND: DynamicColor = DynamicColor::new(Color::rgb(0.2, 0.2, 0.22), Color::rgb(0.85, 0.85, 0.87));
const TEXT: DynamicColor = DynamicColor::new(WHITE, BLACK);

const TEXT_SIZE: f32 = 13.0;
const HEIGHT: f32 = 24.0;
const SIDE_PADDING: f32 = 8.0;
const CORNER_RADIUS: f32 = 4.0;
/// Between the note and the thing it is about.
const GAP: f32 = 6.0;

#[derive(Default)]
struct State {
    shown: WeakView,
}

static STATE: MainLock<State> = MainLock::new();
static GENERATION: AtomicU64 = AtomicU64::new(0);

/// A short note next to the thing it is about, like `Copied` over a
/// copied text. It goes away by itself after `SECONDS`, a new note takes
/// the place of the one on screen. Never takes touches, so it hides
/// nothing under it.
pub struct Toast;

impl Toast {
    pub const SECONDS: f32 = 1.5;

    /// Shows `text` centered above `over`, a rect in the points of the
    /// screen, or under it when there is no room above. It slides back
    /// inside the screen when it would not fit.
    pub fn show(text: &str, over: Rect) {
        Self::hide();

        let label = Label::new();
        label.set_text(text).set_text_size(TEXT_SIZE).set_text_color(TEXT);
        label
            .set_color(BACKGROUND)
            .set_corner_radius(CORNER_RADIUS)
            .set_shadow(Shadow::default());
        let width = (label.content_size().width + SIDE_PADDING * 2.0).ceil();

        let mut view: Own<dyn View> = label;
        view.set_z_position(UIManager::MENU_Z_OFFSET - UIManager::subview_z_offset() * 2.0);
        let mut view = UIManager::root_view().add_subview_to_root(view);
        view.set_label("Toast");

        view.place().custom(move |rect: &mut Rect| {
            let screen = UIManager::root_view().frame().size;
            let x = over.origin.x + (over.size.width - width) / 2.0;
            let above = over.origin.y - GAP - HEIGHT;
            let y = if above >= 0.0 {
                above
            } else {
                over.origin.y + over.size.height + GAP
            };
            let x = x.min(screen.width - width).max(0.0);
            let y = y.min(screen.height - HEIGHT).max(0.0);
            *rect = Rect::new(x.round(), y.round(), width, HEIGHT);
        });

        STATE.get_mut().shown = view;
        let generation = GENERATION.load(Ordering::Relaxed);
        after(Self::SECONDS, move || {
            if GENERATION.load(Ordering::Relaxed) == generation {
                Self::hide();
            }
        });
    }

    /// The note on screen, null while none is shown.
    pub fn shown() -> WeakView {
        STATE.shown
    }

    pub fn hide() {
        GENERATION.fetch_add(1, Ordering::Relaxed);

        let state = STATE.get_mut();
        let mut shown = state.shown;
        state.shown = WeakView::default();

        if shown.is_ok() {
            shown.remove_from_superview();
        }
    }
}
