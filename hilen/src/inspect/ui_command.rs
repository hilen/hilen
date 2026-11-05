//! The commands that read and change the views, `UIRequest`. Each main
//! thread call goes through `window::from_main`, so in `InWindow` it works
//! on the views of that window.

#[cfg(not_wasm)]
use chrono::Local;

use crate::{
    gm::flat::Point,
    inspect::{
        InspectService, edit_log,
        protocol::{AppCommand, EditEntry, Key, UIRequest, UIResponse},
        view_conversion::{ViewToInspect, weak_to_id},
        window::from_main,
    },
    ui::{
        Button, Input, Label, ModifiersState, TextField, Touch, TouchEvent, UIManager, ViewData, ViewFrame,
        ViewSubviews, WeakView,
    },
    window::MouseButton,
};

impl InspectService {
    pub(super) fn process_ui_command(command: UIRequest) -> AppCommand {
        match command {
            UIRequest::SetScale(scale) => {
                from_main(move || {
                    UIManager::set_scale(scale);
                });

                // send_ui dispatches a fresh from_main, so the snapshot runs
                // one frame later, after layout applied the new scale.
                Self::send_ui()
            }
            UIRequest::GetUI => Self::send_ui(),
            UIRequest::EditRule {
                view_id,
                rule_index,
                offset,
                enabled,
            } => Self::apply(move || {
                let view = find_view(&view_id)?;

                let rules_count = view.place().get_rules().len();
                if rule_index >= rules_count {
                    return Err(format!(
                        "Rule index {rule_index} is out of range, view has {rules_count} rules"
                    ));
                }

                let mut rule = view.place().edit_rule(rule_index);
                let old = format!("offset: {}, enabled: {}", rule.offset(), rule.enabled);
                rule.set_offset(offset);
                rule.enabled = enabled;

                Ok(entry(
                    view,
                    format!("rule {rule_index}"),
                    old,
                    format!("offset: {offset}, enabled: {enabled}"),
                ))
            }),
            UIRequest::SetText { view_id, text } => Self::apply(move || {
                let view = find_view(&view_id)?;
                let old = set_text(view, &text)?;
                Ok(entry(view, "text", old, text))
            }),
            UIRequest::SetColor { view_id, color } => Self::apply(move || {
                let view = find_view(&view_id)?;
                let old = *view.color();
                view.set_color(color);
                Ok(entry(view, "color", format!("{old}"), format!("{color}")))
            }),
            UIRequest::Tap {
                view_id,
                modifiers,
                right,
                force,
            } => match from_main(move || Self::tap(&view_id, modifiers, right, force)) {
                // The snapshot runs a frame later, so a page swap or modal
                // the tap triggered is already in the tree.
                Ok(note) => Self::send_ui_with(note),
                Err(err) => AppCommand::Error(err),
            },
            #[cfg(any(desktop, wasm))]
            UIRequest::Hover { view_id, wait_ms } => Self::hover(view_id, wait_ms),
            #[cfg(not(any(desktop, wasm)))]
            UIRequest::Hover { .. } => AppCommand::Error("Hover needs a pointer on desktop or web".into()),
            UIRequest::Drag {
                from,
                to,
                steps,
                step_ms,
            } => Self::drag(from.into(), to.into(), steps, step_ms),
            UIRequest::Scroll { view_id, dx, dy } => match from_main(move || Self::scroll(view_id, dx, dy)) {
                Ok(()) => Self::send_ui(),
                Err(err) => AppCommand::Error(err),
            },
            #[cfg(desktop)]
            UIRequest::Resize { width, height } => Self::resize(width, height),
            #[cfg(not(desktop))]
            UIRequest::Resize { .. } => AppCommand::Error("Resize works only on desktop".into()),
            UIRequest::Hold { keys, ms } => Self::hold(keys, ms),
            UIRequest::Keys { keys, modifiers } => {
                // The modifiers hold only for this one request and release
                // right after it, so a Cmd from an inspect request can never
                // stay stuck on whatever the app receives next.
                from_main(move || {
                    Input::set_modifiers(modifiers);
                    for key in keys {
                        match key {
                            Key::Char(ch) => Input::on_char(ch, None),
                            // A real named key also arrives as its text,
                            // Backspace as 0x08, and the text field edits
                            // on that char. Same two calls as the winit
                            // key handler in `app_runner`.
                            Key::Named(key) => {
                                Input::on_key(key);
                                if let Some(text) = key.to_text() {
                                    Input::on_char(text.chars().last().expect("Key text is empty"), None);
                                }
                                Input::on_key_up(key);
                            }
                        }
                    }
                    Input::set_modifiers(ModifiersState::empty());
                });

                // The snapshot runs a frame later, so a palette or page a
                // hotkey opened is already in the tree.
                Self::send_ui()
            }
        }
    }

    fn scroll(view_id: Option<String>, dx: f32, dy: f32) -> Result<(), String> {
        let position = if let Some(id) = view_id {
            find_view(&id)?.absolute_frame().center()
        } else {
            let window = UIManager::root_view().frame().size;
            Point::new(window.width / 2.0, window.height / 2.0)
        };
        // The wheel goes to the scroll view under the cursor, so the
        // cursor moves to the aim point first.
        UIManager::set_cursor_position(position);
        Input::on_scroll(Point::new(dx, dy));
        Ok(())
    }

    /// The tap itself, on the main thread. Refuses hidden and offscreen
    /// targets, since such a tap lands nowhere while looking like a
    /// success to the client. Refuses a tap point another view sits over
    /// too, that touch once pressed a reset button, `force` sends it
    /// anyway and the reply then carries the covering warning.
    fn tap(
        view_id: &str,
        modifiers: ModifiersState,
        right: bool,
        force: bool,
    ) -> Result<Option<String>, String> {
        let view = find_view(view_id)?;

        if view.is_hidden_in_tree() {
            return Err(format!("View {} is hidden", view.label()));
        }

        let center = view.absolute_frame().center();
        let window = UIManager::root_view().frame().size;
        if center.x < 0.0 || center.y < 0.0 || center.x > window.width || center.y > window.height {
            return Err(format!(
                "View {} center ({}, {}) is outside the {}x{} window. Scroll it into view or resize the window.",
                view.label(),
                center.x,
                center.y,
                window.width,
                window.height,
            ));
        }
        if !view.contains_visible(center) {
            return Err(format!(
                "View {} center is cut off by a scroll view. Scroll it into view first.",
                view.label()
            ));
        }

        let note = covering_note(view, center);
        if let Some(note) = &note
            && !force
        {
            return Err(format!(
                "View {} is not tapped, {note}. Pass --force to send the touch anyway.",
                view.label()
            ));
        }

        // The touch pipeline takes physical pixels and converts to
        // points itself, so the center scales up first.
        let position = center * UIManager::scale();

        // Held for this tap only and released right after, like the keys
        // request, so a Cmd cannot leak into later input.
        Input::set_modifiers(modifiers);
        let button = if right {
            MouseButton::Right
        } else {
            MouseButton::Left
        };
        for event in [TouchEvent::Began, TouchEvent::Ended] {
            Input::process_touch_event(Touch {
                id: 1,
                position,
                event,
                button,
            });
        }
        Input::set_modifiers(ModifiersState::empty());

        Ok(note)
    }

    // Applies an edit on the main thread. On success records it to the edit
    // log and replies with a fresh tree. send_ui dispatches its own
    // from_main, so the snapshot runs one frame later, after layout.
    fn apply(edit: impl FnOnce() -> Result<EditEntry, String> + Send + 'static) -> AppCommand {
        match from_main(edit) {
            Ok(entry) => {
                edit_log::record(entry);
                Self::send_ui()
            }
            Err(err) => AppCommand::Error(err),
        }
    }

    /// A size the OS does not give, like one above the screen, is an
    /// error answer with the size the window has instead.
    #[cfg(desktop)]
    fn resize(width: f32, height: f32) -> AppCommand {
        use crate::gm::LossyConvert;

        let (size, scale) = from_main(move || {
            let scale = UIManager::scale();
            let size: (u32, u32) = ((width * scale).lossy_convert(), (height * scale).lossy_convert());
            (size, scale)
        });
        let Err(actual) = crate::AppRunner::request_window_size(size.into()) else {
            return Self::send_ui();
        };
        let actual_width: f32 = actual.width.lossy_convert();
        let actual_height: f32 = actual.height.lossy_convert();
        let has = format!("{} by {}", actual_width / scale, actual_height / scale);
        AppCommand::Error(format!(
            "The window did not take {width} by {height} points, it is {has}. A window cannot be bigger than the screen."
        ))
    }

    pub(super) fn send_ui() -> AppCommand {
        Self::send_ui_with(None)
    }

    pub(super) fn send_ui_with(note: Option<String>) -> AppCommand {
        Self::lay_out_covered();
        from_main(move || {
            let scale = UIManager::scale();
            let root = UIManager::root_view().view_to_inspect();
            UIResponse::SendUI { scale, root, note }.into()
        })
    }
}

/// Names the deepest visible view under the tap point when it is not the
/// target or inside it, so a tap that would land on a covering view is
/// refused instead of silently doing the wrong thing. Frame containment is
/// an approximation of the draw order, `force` is the way past a wrong call.
fn covering_note(target: WeakView, point: Point) -> Option<String> {
    let top = deepest_at(UIManager::root_view(), point)?;

    if weak_to_id(top) == weak_to_id(target) || is_inside(target, top) || is_inside(top, target) {
        return None;
    }

    Some(format!(
        "the deepest view at the tap point is {} {}, the touch would land there instead",
        top.label(),
        weak_to_id(top),
    ))
}

fn deepest_at(view: WeakView, point: Point) -> Option<WeakView> {
    if view.is_hidden() || !view.absolute_frame().contains(point) {
        return None;
    }
    if let Some(deepest_child) = view.subviews().iter().rev().find_map(|sub| deepest_at(sub.weak(), point)) {
        return Some(deepest_child);
    }
    // A fully transparent view with nothing in it, like an empty overlay
    // host, draws nothing at the point and would only produce false
    // covering warnings.
    if view.color().a == 0.0 && view.subviews().is_empty() {
        return None;
    }
    Some(view)
}

fn is_inside(ancestor: WeakView, view: WeakView) -> bool {
    fn contains(view: WeakView, id: &str) -> bool {
        if weak_to_id(view) == id {
            return true;
        }
        view.subviews().iter().any(|sub| contains(sub.weak(), id))
    }

    contains(ancestor, &weak_to_id(view))
}

fn entry(view: WeakView, what: impl ToString, old: impl ToString, new: impl ToString) -> EditEntry {
    EditEntry {
        timestamp: timestamp(),
        view:      view.label().to_string(),
        view_id:   weak_to_id(view),
        what:      what.to_string(),
        old:       old.to_string(),
        new:       new.to_string(),
    }
}

fn timestamp() -> String {
    #[cfg(not_wasm)]
    {
        Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
    }
    #[cfg(wasm)]
    {
        String::from(web_sys::js_sys::Date::new_0().to_iso_string())
    }
}

fn set_text(view: WeakView, text: &str) -> Result<String, String> {
    if let Some(label) = view.downcast::<Label>() {
        let old = label.text().to_string();
        label.set_text(text);
        return Ok(old);
    }
    if let Some(button) = view.downcast::<Button>() {
        let old = button.text().to_string();
        button.set_text(text);
        return Ok(old);
    }
    if let Some(field) = view.downcast::<TextField>() {
        let old = field.text().to_string();
        field.set_text(text);
        return Ok(old);
    }
    Err(format!("View {} has no text", view.label()))
}

pub(super) fn find_view(id: &str) -> Result<WeakView, String> {
    fn search(view: WeakView, id: &str) -> Option<WeakView> {
        if weak_to_id(view) == id {
            return Some(view);
        }

        view.subviews().iter().find_map(|sub| search(sub.weak(), id))
    }

    search(UIManager::root_view(), id).ok_or_else(|| format!("View not found: {id}"))
}
