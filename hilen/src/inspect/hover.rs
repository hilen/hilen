use crate::{
    deps::hreads::from_main,
    gm::flat::Point,
    inspect::{
        AppCommand, InspectService,
        inspect_service::find_view,
        wait::{MAX_WAIT_MS, wait_ms},
        weak_to_id,
    },
    ui::{Hover, Input, Touch, TouchEvent, UIManager, ViewData, ViewFrame},
    window::MouseButton,
};

impl InspectService {
    pub(super) fn hover(view_id: Option<String>, wait: u32) -> AppCommand {
        if wait > MAX_WAIT_MS {
            return AppCommand::Error(format!("Hover wait must be at most {MAX_WAIT_MS} milliseconds"));
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
                if !view.contains_visible(center) {
                    return Err(format!(
                        "View {} center is cut off by a scroll view. Scroll it into view first.",
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

        if wait > 0
            && let Err(error) = wait_ms(wait)
        {
            return AppCommand::Error(format!("Hover {error}"));
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
