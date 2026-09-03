use anyhow::Result;

use crate::{
    deps::hreads::{from_main, wait_for_next_frame},
    gm::{
        Clock, LossyConvert,
        color::{LIGHT_GRAY, U8Color},
        flat::Point,
    },
    ui::{Button, Cursor, Keys, Setup, UIManager, View, ViewData},
    ui_test::{
        TEST_NAME,
        capture::save_shot,
        checks::check_colors_structured,
        human::{checkpoint, clean_human_mode, human_mode, show_probes},
        record::{next_check_index, print_recorded_colors, recording_colors},
    },
    window::{KeyCode, Window, request_frame},
};

/// Press a key and keep it down until `release_key`, what a player
/// reads while walking. The run resets held keys before every test.
pub fn hold_key(code: KeyCode) {
    from_main(move || Keys::set(code, true));
}

pub fn release_key(code: KeyCode) {
    from_main(move || Keys::set(code, false));
}

/// Raw mouse motion, what the system reports while the cursor is
/// captured. Dropped while the cursor is free, like the real thing.
pub fn inject_mouse_motion(delta: impl Into<Point>) {
    let delta = delta.into();
    from_main(move || Cursor::add_motion(delta));
}

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

/// Advance frame stepped time by `n` rendered frames and let the loop draw each
/// one. Only meaningful after `Clock::enter_stepped`. Each step moves the
/// virtual clock one `STEP_MS` and waits for a real render, so an animation
/// commits at the new time and its frames land on an exact count no matter how
/// fast the machine runs.
pub fn step_frames(n: u32) {
    for _ in 0..n {
        let before = from_main(|| {
            Clock::advance_frame();
            request_frame();
            Window::render_frame()
        });
        while from_main(Window::render_frame) <= before {
            wait_for_next_frame();
        }
    }
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
        let index = next_check_index(&test_name);
        if clean_human_mode() {
            checkpoint(&format!("{test_name} check {index}"))?;
        } else {
            show_probes(&probes, &test_name, index);
        }
    }

    check_colors_structured(&checks)
}
