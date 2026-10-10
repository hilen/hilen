use crate::{
    deps::hreads::from_main,
    inspect::{
        AppCommand, InspectService,
        wait::{MAX_WAIT_MS, wait_ms},
    },
    ui::{Input, Keys, NamedKey},
    window::KeyCode,
};

impl InspectService {
    /// Holds physical keys down for `ms`, then lets them go. `Keys` is
    /// what a walking player reads every step, and a key pressed and
    /// released in one request is never seen held there. A key with a
    /// name also goes down and up as that named key, the way a real key
    /// does, so a held Enter is a long press on the key focus.
    pub(super) fn hold(keys: Vec<KeyCode>, ms: u32) -> AppCommand {
        if ms > MAX_WAIT_MS {
            return AppCommand::Error(format!("Hold must be at most {MAX_WAIT_MS} milliseconds"));
        }
        if keys.is_empty() {
            return AppCommand::Error("Hold needs at least one key".into());
        }

        let held = keys.clone();
        from_main(move || {
            for key in held {
                Keys::set(key, true);
                if let Some(named) = named_key(key) {
                    Input::on_key(named);
                }
            }
        });
        let waited = wait_ms(ms);
        from_main(move || {
            for key in keys {
                Keys::set(key, false);
                if let Some(named) = named_key(key) {
                    Input::on_key_up(named);
                }
            }
        });

        match waited {
            Ok(()) => Self::send_ui_with(None),
            Err(error) => AppCommand::Error(format!("Hold {error}")),
        }
    }
}

/// The named key a physical key also arrives as. Only the keys the key
/// focus reads, a letter has no name.
fn named_key(code: KeyCode) -> Option<NamedKey> {
    Some(match code {
        KeyCode::Enter | KeyCode::NumpadEnter => NamedKey::Enter,
        KeyCode::Escape => NamedKey::Escape,
        KeyCode::ArrowUp => NamedKey::ArrowUp,
        KeyCode::ArrowDown => NamedKey::ArrowDown,
        KeyCode::ArrowLeft => NamedKey::ArrowLeft,
        KeyCode::ArrowRight => NamedKey::ArrowRight,
        _ => return None,
    })
}
