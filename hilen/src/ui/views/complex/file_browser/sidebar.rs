use super::look::{
    DIM_TEXT, FAINT_TEXT, HOVER, Icon, PANEL, PICKED_PLACE, RADIUS, SMALL_TEXT_SIZE, TEXT, TEXT_SIZE,
};
use crate::{
    self as hilen,
    deps::{refs::Weak, vents::Event},
    filesystem::{FilePath, FilePlace},
    gm::{LossyConvert, color::CLEAR, flat::Point},
    ui::{
        ImageView, Label, Setup, TextAlignment, UIColor, ViewCallbacks, ViewData, ViewFrame, ViewSubviews,
        ViewTouch, view,
    },
};

pub(super) const SIDEBAR_WIDTH: f32 = 164.0;

const ROW_HEIGHT: f32 = 28.0;
const TITLE_HEIGHT: f32 = 28.0;
const INSET: f32 = 8.0;
const ICON: f32 = 16.0;

/// A row of the sidebar. A tap opens its folder.
#[view]
pub(super) struct PlaceRow {
    place:  FilePlace,
    picked: bool,

    pub tapped: Event<FilePath>,

    #[init]
    icon:  ImageView,
    title: Label,
}

impl PlaceRow {
    fn fill(mut self: Weak<Self>, place: FilePlace, picked: bool) {
        self.title.set_text(&place.title);
        self.place = place;
        self.picked = picked;
        self.refresh();
    }

    pub(super) fn title(&self) -> &str {
        self.title.text()
    }

    fn refresh(&self) {
        let background: UIColor = if self.picked {
            PICKED_PLACE.into()
        } else if self.is_hovered() {
            HOVER.into()
        } else {
            CLEAR.into()
        };
        self.set_color(background);
        self.icon.set_image(Icon::of_place(self.place.kind).image(DIM_TEXT.resolve()));
    }
}

impl ViewCallbacks for PlaceRow {
    fn theme_changed(&mut self) {
        self.refresh();
    }
}

impl Setup for PlaceRow {
    fn setup(self: Weak<Self>) {
        self.set_corner_radius(RADIUS - 1.0);

        self.icon.place().l(INSET).center_y().size(ICON, ICON);

        self.title
            .set_color(CLEAR)
            .set_text_size(TEXT_SIZE)
            .set_text_color(TEXT)
            .set_alignment(TextAlignment::Left)
            .set_ellipsize(true);
        // A label keeps its own inset before the text.
        self.title.place().tb(0).l(INSET + ICON + 2.0).r(INSET);

        self.enable_touch();
        self.enable_hover();
        self.touch().hovered.sub(self, move || self.refresh());
        self.touch()
            .up_inside
            .sub(self, move || self.tapped.trigger(self.place.path.clone()));
    }
}

/// The places of a file browser: the home folder, the roots or drives of
/// the source, and the places the app added.
#[view]
pub(super) struct Sidebar {
    places:  Vec<FilePlace>,
    current: FilePath,
    rows:    Vec<Weak<PlaceRow>>,

    pub picked: Event<FilePath>,

    #[init]
    title: Label,
}

impl Sidebar {
    pub(super) fn set_places(mut self: Weak<Self>, places: Vec<FilePlace>) {
        self.places = places;

        for mut row in self.rows.drain(..) {
            row.remove_from_superview();
        }
        for index in 0..self.places.len() {
            let row = self.add_view::<PlaceRow>();
            row.tapped.val(move |path| self.picked.trigger(path));
            let position: f32 = index.lossy_convert();
            row.place()
                .t(TITLE_HEIGHT + ROW_HEIGHT * position)
                .lr(INSET - 2.0)
                .h(ROW_HEIGHT);
            self.rows.push(row);
        }
        self.refresh();
    }

    /// The open folder, the row of its place shows as picked.
    pub(super) fn set_current(mut self: Weak<Self>, path: FilePath) {
        self.current = path;
        self.refresh();
    }

    pub(super) fn rows(&self) -> &[Weak<PlaceRow>] {
        &self.rows
    }

    /// The middle of the row with this title in the points of the window,
    /// for a test that taps it. `None` while the sidebar is hidden.
    pub(super) fn row_center(&self, title: &str) -> Option<Point> {
        let row = self.rows.iter().find(|row| row.title() == title && !row.is_hidden_in_tree())?;
        Some(row.absolute_frame().center())
    }

    fn refresh(&self) {
        for (row, place) in self.rows.iter().zip(&self.places) {
            row.fill(place.clone(), place.path == self.current);
        }
    }
}

impl Setup for Sidebar {
    fn setup(self: Weak<Self>) {
        self.set_color(PANEL);

        self.title
            .set_text("Places")
            .set_color(CLEAR)
            .set_text_size(SMALL_TEXT_SIZE)
            .set_text_color(FAINT_TEXT)
            .set_alignment(TextAlignment::Left);
        self.title.place().t(0).l(INSET + 6.0).r(INSET).h(TITLE_HEIGHT);
    }
}
