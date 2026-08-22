use anyhow::{Result, bail};

use crate::{
    AppRunner,
    deps::hreads::from_main,
    gm::{color::U8Color, flat::Point},
    ui::{HighlightView, Setup, UIManager, ViewFrame, ViewSubviews},
    ui_test::{TEST_NAME, failure_report},
    window::Screenshot,
};

pub(super) fn check_pixel_color(screenshot: &Screenshot, pos: Point, color: U8Color) -> Result<()> {
    let pixel: U8Color = screenshot.get_pixel(pos);

    let diff = pixel.diff_u8(color);

    let max_diff = 45;

    if diff > max_diff {
        from_main(move || {
            let mut high = HighlightView::new();
            high.set_z_position(0.1);

            UIManager::root_view()
                .add_subview_to_root(high)
                .downcast_view::<HighlightView>()
                .unwrap()
                .set(pos, color.into(), pixel.into());
        });

        let test_name = TEST_NAME.lock().clone();

        bail!(
            r"
        Test: {test_name} has failed.
        Color diff is too big: {diff}. Max: {max_diff}. Position: {pos:?}.
        Expected: {}, got: {}.
        {:>4} {:>4} - {} -> {}
        {}",
            color.as_hex(),
            pixel.as_hex(),
            pos.x,
            pos.y,
            color.as_hex(),
            pixel.as_hex(),
            failure_report()?,
        )
    }

    Ok(())
}

pub(super) fn check_colors_structured(data: &[(Point, U8Color)]) -> Result<()> {
    let screenshot = AppRunner::take_screenshot()?;

    for (pos, color) in data {
        check_pixel_color(&screenshot, *pos, *color)?;
    }

    Ok(())
}
