use std::mem::take;

use ui_proc::view;

use crate::{
    deps::refs::{Own, Weak},
    ui::{Anchor::Top, Container, Label, Setup, View, ViewData, ViewSubviews},
};

const MARGIN: f32 = 10.0;
const CAPTION_HEIGHT: f32 = 22.0;
const CAPTION_TEXT_SIZE: f32 = 16.0;

/// A grid of live views. Every tile is a real running view, not a picture, so
/// it animates and reacts to touches like it would on its own.
///
/// The grid splits the gallery between its tiles and never scrolls, so the
/// whole set stays visible at once.
#[view]
pub struct ViewGallery {
    tiles:   Vec<(String, Own<dyn View>)>,
    columns: Option<usize>,
}

impl ViewGallery {
    pub fn build() -> GalleryBuilder {
        GalleryBuilder::default()
    }
}

impl Setup for ViewGallery {
    fn setup(mut self: Weak<Self>) {
        let tiles = take(&mut self.tiles);

        let count = tiles.len();

        if count == 0 {
            return;
        }

        let columns = self.columns.unwrap_or_else(|| default_columns(count));
        let rows = count.div_ceil(columns);

        self.place().all_ver().all(MARGIN);

        let mut tiles = tiles.into_iter().enumerate();

        for _ in 0..rows {
            let row = self.add_view::<Container>();
            row.place().all_hor().all(MARGIN);

            for _ in 0..columns {
                let cell = row.add_view::<Container>();

                // An empty trailing cell keeps the last row's tiles as wide as
                // every other row's instead of letting them stretch.
                let Some((index, (caption, view))) = tiles.next() else {
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

/// Collects tiles before the gallery is built. How many rows there are depends
/// on the final count, so gathering everything first avoids rebuilding the grid
/// on every add.
#[derive(Default)]
pub struct GalleryBuilder {
    tiles:   Vec<(String, Own<dyn View>)>,
    columns: Option<usize>,
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

    pub fn make(self) -> Own<ViewGallery> {
        let mut gallery = ViewGallery::new();

        gallery.tiles = self.tiles;
        gallery.columns = self.columns;

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
