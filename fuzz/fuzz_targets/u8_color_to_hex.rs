#![no_main]

use colors_transform::Rgb;
use hilen::gm::color::{Color, U8Color};
use libfuzzer_sys::fuzz_target;
use palette::{LinSrgb, Srgb};

fuzz_target!(|color: U8Color| {
    let my_hex = color.as_hex();

    let rgb = Rgb::from(f32::from(color.r), f32::from(color.g), f32::from(color.b));

    assert_eq!(my_hex, rgb.to_css_hex_string());

    let srgb = Srgb::new(
        f32::from(color.r) / 255.0,
        f32::from(color.g) / 255.0,
        f32::from(color.b) / 255.0,
    );

    let srgb: LinSrgb = srgb.into_linear();
    let my_srgb: Color = color.into();

    assert!(colors_equal(&my_srgb, &srgb));
});

fn approx_equal(a: f32, b: f32, tol: f32) -> bool {
    (a - b).abs() < tol
}

fn colors_equal(c1: &Color, c2: &LinSrgb) -> bool {
    let tol = 0.000_000_5;
    approx_equal(c1.r, c2.red, tol) && approx_equal(c1.g, c2.green, tol) && approx_equal(c1.b, c2.blue, tol)
}
