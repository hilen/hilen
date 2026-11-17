use crate::{
    inspect::{
        AppCommand, InspectService,
        wait::{MAX_WAIT_MS, wait_ms},
        window::from_main,
    },
    ui::{Code, Input, Keys, NamedKey},
};

impl InspectService {
    /// Holds physical keys down for `ms`, then lets them go. `Keys` is
    /// what a walking player reads every step, and a key pressed and
    /// released in one request is never seen held there. A key with a
    /// name also goes down and up as that named key, the way a real key
    /// does, so a held Enter is a long press on the key focus.
    pub(super) fn hold(keys: Vec<Code>, ms: u32) -> AppCommand {
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
fn named_key(code: Code) -> Option<NamedKey> {
    Some(match code {
        Code::Enter | Code::NumpadEnter => NamedKey::Enter,
        Code::Escape => NamedKey::Escape,
        Code::ArrowUp => NamedKey::ArrowUp,
        Code::ArrowDown => NamedKey::ArrowDown,
        Code::ArrowLeft => NamedKey::ArrowLeft,
        Code::ArrowRight => NamedKey::ArrowRight,
        _ => return None,
    })
}
