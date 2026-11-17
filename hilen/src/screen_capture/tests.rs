use image::{Rgba, RgbaImage};

use super::ScreenPicture;
use crate::window::Display;

/// A screen of 100 by 50 points at 2 pixels a point, red with a blue square
/// of 10 points at 20 20.
fn picture() -> ScreenPicture {
    let mut image = RgbaImage::from_pixel(200, 100, Rgba([255, 0, 0, 255]));
    for x in 40..60 {
        for y in 40..60 {
            image.put_pixel(x, y, Rgba([0, 0, 255, 255]));
        }
    }
    ScreenPicture::new(
        Display {
            name:  None,
            frame: (0, 0, 100, 50).into(),
            scale: 2.0,
        },
        image,
    )
    .unwrap()
}

#[test]
fn a_part_is_cut_at_the_pixels_of_its_points() {
    let png = picture().png_of((20, 20, 10, 10).into()).unwrap();
    let part = image::load_from_memory(&png).unwrap().to_rgba8();

    assert_eq!((part.width(), part.height()), (20, 20));
    assert!(part.pixels().all(|pixel| *pixel == Rgba([0, 0, 255, 255])));
}

#[test]
fn a_part_over_the_edge_is_cut_at_the_edge() {
    let png = picture().png_of((90, 40, 50, 50).into()).unwrap();
    let part = image::load_from_memory(&png).unwrap().to_rgba8();

    assert_eq!((part.width(), part.height()), (20, 20));
}

#[test]
fn a_part_off_the_screen_is_an_error() {
    assert!(picture().png_of((120, 0, 10, 10).into()).is_err());
}
