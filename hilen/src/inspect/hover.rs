use std::sync::mpsc::channel;

use crate::{
    deps::hreads::{after, from_main},
    gm::{LossyConvert, flat::Point},
    inspect::{AppCommand, InspectService, inspect_service::find_view, weak_to_id},
    ui::{Hover, Input, Touch, TouchEvent, UIManager, ViewData, ViewFrame},
    window::MouseButton,
};

impl InspectService {
    pub(super) fn hover(view_id: Option<String>, wait_ms: u32) -> AppCommand {
        if wait_ms > 60_000 {
            return AppCommand::Error("Hover wait must be at most 60000 milliseconds".into());
        }
        let result = from_main(move || {
            let position = if let Some(id) = &view_id {
                let view = find_view(id)?;
                if view.is_hidden_in_tree() {
                    return Err(format!("View {} is hidden", view.label()));
                }
                let center = view.absolute_frame().center();
                let window = UIManager::root_view().frame().size;
                if center.x < 0.0 || center.y < 0.0 || center.x > window.width || center.y > window.height {
                    return Err(format!(
                        "View {} center is outside the window. Scroll it into view first.",
                        view.label()
                    ));
                }
                center
            } else {
                // CursorLeft clears hover without dispatching a moved event
                // to views whose frames happen to extend outside the window.
                UIManager::set_cursor_position(Point::new(-1.0, -1.0));
                Hover::clear();
                return Ok(());
            };

            // Winit cursor moves enter this same pipeline as a moved mouse
            // touch, in physical pixels, without a began or ended event.
            Input::process_touch_event(Touch {
                id:       1,
                position: position * UIManager::scale(),
                event:    TouchEvent::Moved,
                button:   MouseButton::Left,
            });
            Ok(())
        });
        if let Err(error) = result {
            return AppCommand::Error(error);
        }

        if wait_ms > 0 {
            let (send, recv) = channel();
            from_main(move || {
                let delay: f32 = wait_ms.lossy_convert();
                after(delay / 1000.0, move || {
                    send.send(()).expect("Hover wait receiver is gone");
                });
            });
            if let Err(error) = recv.recv() {
                return AppCommand::Error(format!("Hover wait failed: {error}"));
            }
        }
        let note = from_main(|| {
            let hovered = Hover::hovered();
            if hovered.is_ok() {
                format!("hovered {} {}", hovered.label(), weak_to_id(hovered))
            } else {
                "hovered none".into()
            }
        });
        Self::send_ui_with(Some(note))
    }
}
