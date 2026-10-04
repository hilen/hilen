use anyhow::{Result, bail};

use crate::{
    AppRunner,
    deps::hreads::from_main,
    gm::{color::U8Color, flat::Point},
    ui::{HighlightView, Setup, UIManager, ViewFrame, ViewSubviews},
    ui_test::{TEST_NAME, failure_report, push_failure},
    window::Screenshot,
};

/// How far a pixel may be from its recorded color, summed over the channels.
const MAX_DIFF: i16 = 45;

pub(super) fn check_pixel_color(screenshot: &Screenshot, pos: Point, color: U8Color) -> Result<()> {
    let pixel: U8Color = screenshot.get_pixel(pos);

    let diff = pixel.diff_u8(color);

    let max_diff = MAX_DIFF;

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

/// The check of a shots run. Such a run exists to save every checked state,
/// so a wrong point must not end the test at its first check. Every wrong
/// point of the check goes into one failure and the test carries on, with no
/// highlight marker left in the frames that follow.
pub(super) fn check_colors_keep_going(data: &[(Point, U8Color)]) -> Result<()> {
    let screenshot = AppRunner::take_screenshot()?;

    let wrong: Vec<String> = data
        .iter()
        .filter_map(|(pos, color)| {
            let pixel: U8Color = screenshot.get_pixel(*pos);
            (pixel.diff_u8(*color) > MAX_DIFF).then(|| {
                format!(
                    "{:>4} {:>4} - {} -> {}",
                    pos.x,
                    pos.y,
                    color.as_hex(),
                    pixel.as_hex()
                )
            })
        })
        .collect();

    if !wrong.is_empty() {
        let test_name = TEST_NAME.lock().clone();
        push_failure(
            &test_name,
            format!("{} wrong points in one check:\n{}", wrong.len(), wrong.join("\n")),
        );
    }

    Ok(())
}
