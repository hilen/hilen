use super::screen_awake;
use crate::{deps::refs::main_lock::MainLock, ui::UIEvent};

static STATE: MainLock<ActivityState> = MainLock::new();
static CHANGED: UIEvent<bool> = UIEvent::const_new();

/// Foreground activity of the app. All access and callbacks run on the main
/// thread.
pub struct AppActivity;

impl AppActivity {
    pub fn is_active() -> bool {
        STATE.get_mut().active()
    }

    /// False on focus loss, hiding or suspension, true once all three recover.
    /// A mobile lock or app switch suspends the app. Returning does not imply
    /// that work stopped by a subscriber should restart.
    pub fn changed() -> &'static UIEvent<bool> {
        &CHANGED
    }
}

#[derive(Clone, Copy)]
pub(crate) enum ActivityChange {
    Focused(bool),
    Visible(bool),
    Resumed(bool),
}

pub(crate) fn update(change: ActivityChange) {
    let changed = STATE.get_mut().update(change);
    if let Some(active) = changed {
        screen_awake::set_active(active);
        CHANGED.trigger(active);
    }
}

struct ActivityState {
    focused: bool,
    visible: bool,
    resumed: bool,
}

impl Default for ActivityState {
    fn default() -> Self {
        Self {
            focused: true,
            visible: true,
            resumed: true,
        }
    }
}

impl ActivityState {
    fn active(&self) -> bool {
        self.focused && self.visible && self.resumed
    }

    fn update(&mut self, change: ActivityChange) -> Option<bool> {
        let was_active = self.active();
        match change {
            ActivityChange::Focused(value) => self.focused = value,
            ActivityChange::Visible(value) => self.visible = value,
            ActivityChange::Resumed(value) => self.resumed = value,
        }
        (was_active != self.active()).then(|| self.active())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ActivityChange::{Focused, Resumed, Visible},
        ActivityState,
    };

    #[test]
    fn lock_and_focus_events_only_notify_once() {
        let mut state = ActivityState::default();
        assert_eq!(state.update(Resumed(false)), Some(false));
        assert_eq!(state.update(Focused(false)), None);
        assert_eq!(state.update(Visible(false)), None);
        assert_eq!(state.update(Resumed(true)), None);
        assert_eq!(state.update(Visible(true)), None);
        assert_eq!(state.update(Focused(true)), Some(true));
        assert_eq!(state.update(Focused(true)), None);
    }

    #[test]
    fn android_focus_loss_stops_before_suspension() {
        let mut state = ActivityState::default();
        assert_eq!(state.update(Focused(false)), Some(false));
        assert_eq!(state.update(Resumed(false)), None);
        assert_eq!(state.update(Focused(true)), None);
        assert_eq!(state.update(Resumed(true)), Some(true));
    }
}
