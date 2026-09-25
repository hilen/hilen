use crate::{
    deps::hreads::from_main,
    inspect::{
        AppCommand, InspectService,
        wait::{MAX_WAIT_MS, wait_ms},
    },
    ui::Keys,
    window::KeyCode,
};

impl InspectService {
    /// Holds physical keys down for `ms`, then lets them go. `Keys` is
    /// what a walking player reads every step, and a key pressed and
    /// released in one request is never seen held there.
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
            }
        });
        let waited = wait_ms(ms);
        from_main(move || {
            for key in keys {
                Keys::set(key, false);
            }
        });

        match waited {
            Ok(()) => Self::send_ui_with(None),
            Err(error) => AppCommand::Error(format!("Hold {error}")),
        }
    }
}
