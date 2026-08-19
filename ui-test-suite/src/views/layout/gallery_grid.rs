use test_engine::{
    dispatch::from_main,
    refs::Weak,
    ui::{Container, Rect, Setup, ViewData, ViewFrame, ViewGallery, ViewSubviews, ViewTest, view},
};

const EPSILON: f32 = 0.01;

#[view]
struct GalleryGrid {
    gallery: Weak<ViewGallery>,
}

impl Setup for GalleryGrid {
    fn setup(mut self: Weak<Self>) {
        let gallery = ViewGallery::build()
            .columns(3)
            .tile::<Container>("one")
            .tile::<Container>("two")
            .tile::<Container>("three")
            .tile::<Container>("four")
            .tile::<Container>("five")
            .make();

        self.gallery = self.add_subview(gallery);
        self.gallery.place().back();
    }
}

impl ViewTest for GalleryGrid {
    fn perform_test(view: Weak<Self>) -> anyhow::Result<()> {
        let rows = cell_frames(view);

        assert_eq!(rows.len(), 2, "five tiles in three columns need two rows");

        for row in &rows {
            assert_eq!(row.len(), 3, "every row keeps a cell per column");
        }

        let first = rows[0][0];

        for row in &rows {
            for cell in row {
                assert!(
                    (cell.width() - first.width()).abs() < EPSILON,
                    "cells must share one width, got {} and {}",
                    cell.width(),
                    first.width()
                );
                assert!(
                    (cell.height() - first.height()).abs() < EPSILON,
                    "cells must share one height, got {} and {}",
                    cell.height(),
                    first.height()
                );
            }
        }

        // The last row holds two tiles and one empty cell. Without that spacer
        // the two would stretch and stop lining up with the row above.
        for row in &rows {
            for pair in row.windows(2) {
                assert!(
                    pair[1].origin.x > pair[0].origin.x,
                    "cells in a row must run left to right without overlapping"
                );
            }
        }

        for (top, bottom) in rows[0].iter().zip(&rows[1]) {
            assert!(
                (top.origin.x - bottom.origin.x).abs() < EPSILON,
                "columns must line up across rows"
            );
        }

        Ok(())
    }
}

fn cell_frames(view: Weak<GalleryGrid>) -> Vec<Vec<Rect>> {
    from_main(move || {
        view.gallery
            .subviews()
            .iter()
            .map(|row| row.subviews().iter().map(|cell| *cell.frame()).collect())
            .collect()
    })
}
