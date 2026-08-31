use anyhow::Result;

use crate::{
    deps::hreads::{from_main, wait_for_next_frame},
    gm::{
        LossyConvert,
        color::{LIGHT_GRAY, U8Color},
        flat::Point,
    },
    ui::{Button, Setup, UIManager, View, ViewData},
    ui_test::{
        TEST_NAME,
        capture::save_shot,
        checks::check_colors_structured,
        human::{human_mode, show_probes},
        record::{next_check_index, print_recorded_colors, recording_colors},
    },
};

#[allow(dead_code)]
pub(crate) fn add_action(action: impl FnMut() + Send + 'static) {
    let button = UIManager::root_view()
        .add_subview_to_root(Button::new())
        .downcast::<Button>()
        .unwrap();
    button.place().size(100, 100).bl(0);
    button.set_color(LIGHT_GRAY);
    button.on_tap(action);
    button.__base_view().view_label = "Debug Action Button".into();
}

pub fn check_colors(data: &str) -> Result<()> {
    save_shot("check")?;

    if recording_colors() {
        return print_recorded_colors();
    }

    let checks: Vec<_> = data
        .split('\n')
        .filter_map(|line| {
            let parts: Vec<_> = line.split('-').collect();

            if parts.len() != 2 {
                return None;
            }

            let pos = parts[0];
            let color = parts[1];

            let pos: Vec<_> = pos.split(' ').filter(|a| !a.is_empty()).collect();
            let color = color.trim();

            let pos: Point = Point::new(pos[0].parse().unwrap(), pos[1].parse().unwrap());

            Some((pos, U8Color::parse_hex(color)))
        })
        .collect();

    if human_mode() {
        from_main(UIManager::clear_touch_marks);
        wait_for_next_frame();

        let probes: Vec<((u32, u32), U8Color)> = checks
            .iter()
            .map(|(pos, color)| ((pos.x.lossy_convert(), pos.y.lossy_convert()), *color))
            .collect();

        let test_name = TEST_NAME.lock().clone();
        show_probes(&probes, &test_name, next_check_index(&test_name));
    }

    check_colors_structured(&checks)
}
