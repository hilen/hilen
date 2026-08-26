mod capture;
mod checks;
mod collect;
pub mod helpers;
mod human;
mod record;
mod report;
pub mod state;
mod suite;
mod ui_test;
/// The web driver carries its own stuck handling, and a page cannot spawn
/// the plain watchdog thread.
#[cfg(not_wasm)]
mod watchdog;

use std::{
    fmt::Display,
    ops::Deref,
    sync::{Arc, mpsc::channel},
    thread::sleep,
    time::Duration,
};

use anyhow::{Result, bail};
pub use capture::{capture_requested_screenshot, capture_screenshot, enable_screenshot_capture};
pub use collect::{TestFailure, any_failed, clear_failures, push_failure, run_test, take_failures};
pub use helpers::*;
pub use human::{enable_human_mode, human_checkpoint, human_mode};
pub(crate) use human::{hold_for_human, human_pause, human_pause_key, human_pause_quick};
use log::{error, warn};
use parking_lot::Mutex;
pub use record::{enable_color_recording, recording_colors, set_record_probe_count};
pub(crate) use record::{reset_record_probe_count, set_record_canvas};
pub use report::failure_report;
use serde::de::DeserializeOwned;
pub use state::*;
pub use suite::{TestRunReport, present_test, run_all_tests, run_test_map};

pub use self::ui_test::*;
use crate::{
    AppRunner,
    deps::{
        hreads::{from_main, is_main_thread, on_main, wait_for_next_frame},
        refs::Own,
    },
    gm::{LossyConvert, ToF32, drop_on_main, flat::Point},
    ui::{
        Input, LongPress, ModifiersState, NamedKey, Tooltip, Touch, U8Color, UIEvents, UIManager,
        input::TouchEvent,
    },
    window::{MouseButton, Window},
};

pub fn test_combinations<const A: usize, Val>(comb: [(&'static str, Val); A]) -> Result<()>
where Val: Display + PartialEq + DeserializeOwned + Default + Send + 'static {
    for comb in comb {
        clear_state();

        let touches = Touch::vec_from_str(comb.0);

        for touch in touches {
            from_main(move || {
                inject_touch(touch);
            });

            if touch.is_moved() {
                human_pause_quick();
            } else {
                human_pause();
            }
        }

        if get_state::<Val>() != comb.1 {
            error!(
                "Failed state for: {} Expected: {} got: {}",
                comb.0,
                comb.1,
                get_state::<Val>()
            );
            bail!("UI test failed")
        }
    }
    Ok(())
}

fn inject_touch(touch: impl Into<Touch> + Send + Copy + 'static) {
    Input::process_touch_event(touch.into());
}

#[allow(dead_code)]
pub fn inject_scroll(scroll: impl ToF32) {
    from_main(move || {
        Input::on_scroll((0, scroll).into());
    });
    human_pause();
}

pub fn inject_touches(data: impl ToString + Send + 'static) {
    let scale = UIManager::scale();

    if human_mode() {
        for mut touch in Touch::vec_from_str(&data.to_string()) {
            touch.position *= scale;
            from_main(move || {
                inject_touch(touch);
            });

            if touch.is_moved() {
                human_pause_quick();
            } else {
                human_pause();
            }
        }
        return;
    }

    from_main(move || {
        for mut touch in Touch::vec_from_str(&data.to_string()) {
            touch.position *= scale;
            inject_touch(touch);
        }
    });
}

/// A right button press and release at `x y`, the desktop way to open a
/// context menu. The engine does not gate the right button by platform,
/// so this drives the same path on a device.
pub fn inject_right_click(x: impl ToF32, y: impl ToF32) {
    let position = Point::new(x.to_f32(), y.to_f32()) * UIManager::scale();

    for event in [TouchEvent::Began, TouchEvent::Ended] {
        from_main(move || {
            inject_touch(Touch {
                id: 1,
                position,
                event,
                button: MouseButton::Right,
            });
        });
    }

    human_pause();
}

/// Holds finger 1 at `x y` past the long press threshold and lets go. The
/// hold fires `secondary` on the view under it, the release is no tap.
pub fn inject_long_press(x: impl ToF32, y: impl ToF32) {
    let position = Point::new(x.to_f32(), y.to_f32()) * UIManager::scale();

    let touch = Touch {
        id: 1,
        position,
        event: TouchEvent::Began,
        button: MouseButton::Left,
    };

    from_main(move || inject_touch(touch));
    sleep(Duration::from_secs_f32(LongPress::DURATION + 0.2));
    wait_for_next_frame();

    from_main(move || {
        inject_touch(Touch {
            event: TouchEvent::Ended,
            ..touch
        });
    });

    human_pause();
}

/// Waits past the tooltip delay, so a tooltip the cursor is resting on
/// has shown by the time the next check runs.
pub fn wait_for_tooltip() {
    sleep(Duration::from_secs_f32(Tooltip::DELAY + 0.2));
    wait_for_next_frame();
    human_pause();
}

pub fn inject_touches_delayed(data: &str) {
    for touch in Touch::vec_from_str(data) {
        wait_for_next_frame();
        from_main(move || {
            inject_touch(touch);
        });
        wait_for_next_frame();

        if touch.is_moved() {
            human_pause_quick();
        } else {
            human_pause();
        }
    }
}

/// Types a string. Outside human mode the whole string goes to the main
/// thread in one trip, a trip per char would cost a frame per char.
pub fn inject_keys(s: impl ToString) {
    let s = s.to_string();

    if human_mode() {
        for ch in s.chars() {
            inject_key(ch);
        }
        return;
    }

    from_main(move || {
        for ch in s.chars() {
            Input::on_char(ch);
        }
    });
}

pub fn inject_key(key: char) {
    from_main(move || Input::on_char(key));
    human_pause_key();
}

pub fn inject_named_key(key: NamedKey) {
    from_main(move || Input::on_key(key));
    human_pause_key();
}

/// Sets the held modifier keys for the injected keys that follow, until
/// the next call. `ModifiersState::empty()` releases them.
pub fn inject_modifiers(modifiers: ModifiersState) {
    from_main(move || Input::set_modifiers(modifiers));
}

#[allow(dead_code)]
pub(crate) fn record_touches() {
    record_touches_internal(true);
}

#[allow(dead_code)]
pub(crate) fn record_moved_touches() {
    record_touches_internal(false);
}

fn record_touches_internal(skip_moved: bool) {
    let touches_own: Own<_> = Vec::<Touch>::new().into();
    let mut touches = touches_own.weak();

    let (s, r) = channel::<()>();

    let moved_record_skip = 10;

    let moved_counter = Arc::new(Mutex::new(0));

    on_main(move || {
        UIEvents::on_touch().val(move |touch| {
            if touch.is_moved() {
                let mut counter = moved_counter.lock();
                *counter += 1;
                if *counter == moved_record_skip {
                    *counter = 0;
                } else {
                    return;
                }
            }

            if skip_moved && touch.is_moved() {
                return;
            }

            touches.push(touch);
        });

        UIManager::keymap().add(UIManager::root_view(), 'a', move || {
            _ = s.send(());
        });
    });

    if r.recv().is_err() {
        warn!("Failed to receive record_touches result");
    }

    from_main(|| {
        UIEvents::on_touch().remove_subscribers();
    });

    println!(
        r#"
        inject_touches(
        "
{}
        ",
    );
    "#,
        Touch::str_from_vec(touches.to_vec()),
    );

    drop_on_main(touches_own);
}

#[allow(dead_code)]
pub(crate) fn record_ui_test() {
    if is_main_thread() {
        panic!("record_ui_test is blocking function. It shouldn't be called on main thread.");
    }

    loop {
        Window::set_title("Recording touches");
        record_touches();
        Window::set_title("Recording colors");
        record_colors().unwrap();
    }
}

#[allow(dead_code)]
pub(crate) fn record_colors() -> Result<()> {
    let touch_lock = Touch::lock();

    let screenshot = AppRunner::take_screenshot()?;

    let touches_own: Own<_> = Vec::<(Touch, U8Color)>::new().into();
    let mut touches = touches_own.weak();

    let (s, r) = channel::<()>();

    on_main(move || {
        UIEvents::on_debug_touch().val(move |touch| {
            if !touch.is_began() {
                return;
            }

            touches.push((touch, screenshot.get_pixel(touch.position)));
        });

        UIManager::keymap().add(UIManager::root_view(), 'a', move || {
            _ = s.send(());
        });
    });

    if r.recv().is_err() {
        warn!("Failed to receive record_touches_with_colors result");
    }

    on_main(|| {
        UIEvents::on_touch().remove_subscribers();
        UIEvents::on_debug_touch().remove_subscribers();
    });

    println!("check_colors( r\"");

    for (touch, color) in touches.deref() {
        let x: u32 = touch.position.x.lossy_convert();
        let y: u32 = touch.position.y.lossy_convert();
        println!("            {:>4} {:>4} - {}", x, y, color.as_hex());
    }

    println!("        \"");
    println!(")?;");

    drop(touch_lock);

    drop_on_main(touches_own);

    Ok(())
}
