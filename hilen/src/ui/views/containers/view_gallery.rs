use std::mem::take;

use ui_proc::view;

use crate::{
    deps::refs::{Own, Weak},
    ui::{
        Anchor::{Left, Right, Top},
        Button, Container, Label, Setup, View, ViewData, ViewSubviews,
    },
};

const MARGIN: f32 = 10.0;
const CAPTION_HEIGHT: f32 = 22.0;
const CAPTION_TEXT_SIZE: f32 = 16.0;
const FOOTER_HEIGHT: f32 = 36.0;
const PAGER_BUTTON_WIDTH: f32 = 56.0;

/// A tile with its number in the whole set: index, caption, view.
type NumberedTile = (usize, String, Own<dyn View>);

/// A grid of live views. Every tile is a real running view, not a picture, so
/// it animates and reacts to touches like it would on its own.
///
/// The grid splits the gallery between its tiles and never scrolls, so the
/// whole set stays visible at once. With `page_size` the tiles split into
/// pages instead, switched with the pager under the grid, so a set too big
/// for one screen stays readable. Tile numbers run through the whole set,
/// not per page, so "tile 7" means the same view on every page.
#[view]
pub struct ViewGallery {
    tiles:      Vec<(String, Own<dyn View>)>,
    columns:    Option<usize>,
    page_size:  Option<usize>,
    page:       usize,
    pages:      Vec<Weak<Container>>,
    page_label: Weak<Label>,
    prev:       Weak<Button>,
    next:       Weak<Button>,
}

impl ViewGallery {
    pub fn build() -> GalleryBuilder {
        GalleryBuilder::default()
    }

    pub fn page(&self) -> usize {
        self.page
    }

    pub fn show_page(mut self: Weak<Self>, page: usize) {
        if self.pages.is_empty() {
            return;
        }
        let page = page.min(self.pages.len() - 1);
        self.page = page;
        for (index, container) in self.pages.iter().enumerate() {
            container.set_hidden(index != page);
        }
        let total = self.pages.len();
        self.page_label.set_text(format!("{} / {total}", page + 1));
        self.prev.set_enabled(page > 0);
        self.next.set_enabled(page + 1 < total);
    }

    fn build_grid<T: View + 'static>(target: Weak<T>, tiles: Vec<NumberedTile>, columns: usize) {
        let rows = tiles.len().div_ceil(columns);
        let mut tiles = tiles.into_iter();

        for _ in 0..rows {
            let row = target.add_view::<Container>();
            row.place().all_hor().all(MARGIN);

            for _ in 0..columns {
                let cell = row.add_view::<Container>();

                // An empty trailing cell keeps the last row's tiles as wide as
                // every other row's instead of letting them stretch.
                let Some((index, caption, view)) = tiles.next() else {
                    continue;
                };

                // The number is what a person points at. Without it they have
                // to read and repeat a name to say which tile they mean.
                let label = cell.add_view::<Label>();
                label
                    .set_text(format!("{}. {caption}", index + 1))
                    .set_text_size(CAPTION_TEXT_SIZE);
                label.place().lrt(0).h(CAPTION_HEIGHT);

                let view = cell.add_subview(view);
                view.place().lrb(0).anchor(Top, label, 0);
            }
        }
    }
}

impl Setup for ViewGallery {
    fn setup(mut self: Weak<Self>) {
        let tiles = take(&mut self.tiles);

        let count = tiles.len();

        if count == 0 {
            return;
        }

        let page_size = self.page_size.unwrap_or(count).max(1);

        if count <= page_size {
            let columns = self.columns.unwrap_or_else(|| default_columns(count));
            self.place().all_ver().all(MARGIN);
            let indexed = tiles
                .into_iter()
                .enumerate()
                .map(|(index, (caption, view))| (index, caption, view))
                .collect();
            Self::build_grid(self, indexed, columns);
            return;
        }

        let columns = self.columns.unwrap_or_else(|| default_columns(page_size));

        let mut chunks: Vec<Vec<NumberedTile>> = vec![];
        for (index, (caption, view)) in tiles.into_iter().enumerate() {
            if index % page_size == 0 {
                chunks.push(vec![]);
            }
            chunks.last_mut().unwrap().push((index, caption, view));
        }

        for chunk in chunks {
            let container = self.add_view::<Container>();
            container.place().lrt(0).b(FOOTER_HEIGHT);
            container.place().all_ver().all(MARGIN);
            Self::build_grid(container, chunk, columns);
            self.pages.push(container);
        }

        let label = self.add_view::<Label>();
        label.set_text_size(CAPTION_TEXT_SIZE);
        label.place().b(4).center_x().size(90, FOOTER_HEIGHT - 8.0);
        self.page_label = label;

        let prev = self.add_view::<Button>();
        prev.set_text("<");
        prev.place()
            .b(4)
            .size(PAGER_BUTTON_WIDTH, FOOTER_HEIGHT - 8.0)
            .anchor(Right, label, MARGIN);
        prev.on_tap(move || self.show_page(self.page.saturating_sub(1)));
        self.prev = prev;

        let next = self.add_view::<Button>();
        next.set_text(">");
        next.place()
            .b(4)
            .size(PAGER_BUTTON_WIDTH, FOOTER_HEIGHT - 8.0)
            .anchor(Left, label, MARGIN);
        next.on_tap(move || self.show_page(self.page + 1));
        self.next = next;

        self.show_page(0);
    }
}

/// Collects tiles before the gallery is built. How many rows there are depends
/// on the final count, so gathering everything first avoids rebuilding the grid
/// on every add.
#[derive(Default)]
pub struct GalleryBuilder {
    tiles:     Vec<(String, Own<dyn View>)>,
    columns:   Option<usize>,
    page_size: Option<usize>,
}

impl GalleryBuilder {
    pub fn tile<V: View + Default + 'static>(mut self, caption: impl ToString) -> Self {
        self.tiles.push((caption.to_string(), V::new()));
        self
    }

    pub fn tile_view(mut self, caption: impl ToString, view: Own<dyn View>) -> Self {
        self.tiles.push((caption.to_string(), view));
        self
    }

    pub fn columns(mut self, columns: usize) -> Self {
        self.columns = Some(columns);
        self
    }

    /// Tiles per page. Without it every tile shares one screen. With it the
    /// gallery adds a pager and shows `page_size` tiles at a time.
    pub fn page_size(mut self, page_size: usize) -> Self {
        self.page_size = Some(page_size);
        self
    }

    pub fn make(self) -> Own<ViewGallery> {
        let mut gallery = ViewGallery::new();

        gallery.tiles = self.tiles;
        gallery.columns = self.columns;
        gallery.page_size = self.page_size;

        gallery
    }
}

/// The squarest grid that holds the count, so a gallery reads the same whether
/// it has four tiles or twenty.
fn default_columns(count: usize) -> usize {
    let mut columns = 1;
    while columns * columns < count {
        columns += 1;
    }
    columns
}
