use crate::{
    deps::hreads::from_main,
    gm::{LossyConvert, flat::Point},
    inspect::{
        AppCommand, InspectService,
        wait::{MAX_WAIT_MS, wait_ms},
    },
    ui::{Input, Touch, TouchEvent, UIManager},
    window::MouseButton,
};

impl InspectService {
    /// A left drag through the real input pipeline: began, `steps` moves
    /// and ended. With `step_ms` 0 all of them go in 1 call on the main
    /// thread, so no frame and no other thread runs between the moves. A
    /// pointer moves about every 16 ms, and a drag that seeks a video on
    /// every move shows a picture during the drag only with such a gap, so
    /// above 0 each touch is its own call, `step_ms` after the one before.
    pub(super) fn drag(from: Point, to: Point, steps: usize, step_ms: u32) -> AppCommand {
        let steps = steps.max(2);
        let gaps = u32::try_from(steps + 1).unwrap_or(u32::MAX);
        if step_ms.saturating_mul(gaps) > MAX_WAIT_MS {
            return AppCommand::Error(format!(
                "A drag of {steps} steps {step_ms} ms apart is longer than {MAX_WAIT_MS} milliseconds"
            ));
        }

        let all: f32 = steps.lossy_convert();
        let moves = (1..=steps).map(|step| {
            let t: f32 = step.lossy_convert();
            (from + (to - from) * (t / all), TouchEvent::Moved)
        });
        let touches: Vec<(Point, TouchEvent)> = [(from, TouchEvent::Began)]
            .into_iter()
            .chain(moves)
            .chain([(to, TouchEvent::Ended)])
            .collect();

        if step_ms == 0 {
            from_main(move || touches.into_iter().for_each(|(position, event)| touch(position, event)));
            return Self::send_ui();
        }
        for (position, event) in touches {
            from_main(move || touch(position, event));
            if event == TouchEvent::Ended {
                break;
            }
            if let Err(error) = wait_ms(step_ms) {
                // The button is let go, a drag left down would hold every
                // later input of the app.
                from_main(move || touch(position, TouchEvent::Ended));
                return AppCommand::Error(format!("Drag {error}"));
            }
        }
        Self::send_ui()
    }
}

// The touch pipeline takes physical pixels, so every point scales up.
fn touch(position: Point, event: TouchEvent) {
    Input::process_touch_event(Touch {
        id: 1,
        position: position * UIManager::scale(),
        event,
        button: MouseButton::Left,
    });
}
