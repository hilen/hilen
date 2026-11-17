//! The state of a self update, for the views that show it. An app turns
//! self update on by returning its public key from `App::update_key`. The
//! engine then checks once after launch, and a view reads the phase, starts
//! the install and hears about every change. `Updater` stays the layer
//! below, the calls that talk to the network and swap the binary.

use std::mem::take;

use anyhow::Result;

use crate::{
    deps::refs::{Weak, main_lock::MainLock},
    system::UpdateInfo,
};

/// Seconds between launch and the first check, so the check does not
/// compete with the first frames and the first requests of the app.
#[cfg(desktop)]
const FIRST_CHECK_DELAY: f32 = 3.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum UpdatePhase {
    #[default]
    Idle,
    Checking,
    Available,
    Installing,
}

struct Subscriber {
    alive:  Box<dyn Fn() -> bool>,
    action: Box<dyn FnMut()>,
}

#[derive(Default)]
pub struct UpdateState {
    phase:       UpdatePhase,
    version:     Option<String>,
    progress:    u8,
    error:       Option<String>,
    info:        Option<UpdateInfo>,
    subscribers: Vec<Subscriber>,
}

static STATE: MainLock<UpdateState> = MainLock::new();

fn state() -> &'static mut UpdateState {
    STATE.get_mut()
}

impl UpdateState {
    /// The one state of the app. Main thread only.
    pub fn get() -> &'static Self {
        state()
    }

    pub fn phase(&self) -> UpdatePhase {
        self.phase
    }

    /// The version a check found, `None` when the app is up to date.
    pub fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }

    /// Whole percent of the download, 0 until the first chunk lands.
    pub fn progress(&self) -> u8 {
        self.progress
    }

    /// Why the last check or install failed. The next one clears it.
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn has_update(&self) -> bool {
        matches!(self.phase, UpdatePhase::Available | UpdatePhase::Installing)
    }

    pub fn busy(&self) -> bool {
        matches!(self.phase, UpdatePhase::Checking | UpdatePhase::Installing)
    }

    /// Runs `action` on the main thread after every change of the state,
    /// for as long as `view` lives. Any number of views can subscribe.
    pub fn on_change<V: ?Sized + 'static>(view: Weak<V>, action: impl FnMut() + 'static) {
        state().subscribe(move || view.is_ok(), action);
    }

    /// Asks the manifest for a newer version. Does nothing while a check
    /// or an install runs. Outside the desktop it finds no update.
    pub fn check() {
        use crate::{
            deps::hreads::{on_main, spawn},
            system::Updater,
        };

        if !state().begin_check() {
            return;
        }
        log::info!("updater: check start");
        spawn(async move {
            let result = Updater::check().await;
            on_main(move || state().finish_check(result));
        });
    }

    /// Downloads, verifies and swaps the binary of the found update, then
    /// starts the new one. A failure lands in `error`. Does nothing when
    /// no update is waiting.
    pub fn install() {
        use crate::{
            deps::hreads::{on_main, spawn},
            system::Updater,
        };

        let Some(info) = state().begin_install() else {
            return;
        };
        let version = info.version.clone();
        log::info!("updater: install start latest={version}");
        spawn(async move {
            let mut last = 0;
            let result = Updater::install_with_progress(info, move |done, total| {
                if total.is_none() && done == 0 {
                    log::warn!(
                        "updater: Content-Length missing, the server may be returning an HTML fallback instead \
                         of the artifact"
                    );
                }
                let Some(now) = percent(done, total) else {
                    return;
                };
                if now == last {
                    return;
                }
                last = now;
                on_main(move || state().set_progress(now));
            })
            .await;
            on_main(move || {
                let result = result.and_then(|()| {
                    log::info!("updater: installed {version}, relaunching");
                    Updater::relaunch()
                });
                if let Err(error) = result {
                    log::error!("updater: install of {version} failed: {error:#}");
                    state().fail_install(format!("{error:#}"));
                }
            });
        });
    }

    /// The first check, a few seconds after launch. Only an app that gives
    /// an update key gets it, so an app with its own update code and a
    /// test run make no request.
    #[cfg(desktop)]
    pub(crate) fn check_after_launch() {
        use crate::deps::hreads::after;

        if crate::app::app().update_key().is_none() {
            return;
        }
        after(FIRST_CHECK_DELAY, Self::check);
    }

    fn subscribe(&mut self, alive: impl Fn() -> bool + 'static, action: impl FnMut() + 'static) {
        self.subscribers.push(Subscriber {
            alive:  Box::new(alive),
            action: Box::new(action),
        });
    }

    /// Calls every subscriber whose view still lives and drops the rest.
    /// The list is taken out for the calls, a subscriber is free to read
    /// the state and to subscribe another view.
    fn changed(&mut self) {
        let mut current = take(&mut self.subscribers);
        current.retain(|subscriber| (subscriber.alive)());
        for subscriber in &mut current {
            (subscriber.action)();
        }
        current.append(&mut self.subscribers);
        self.subscribers = current;
    }

    /// False when a check or an install already runs.
    fn begin_check(&mut self) -> bool {
        if self.busy() {
            return false;
        }
        self.phase = UpdatePhase::Checking;
        self.error = None;
        self.changed();
        true
    }

    fn finish_check(&mut self, result: Result<Option<UpdateInfo>>) {
        match result {
            Ok(Some(info)) => {
                log::info!("updater: update available latest={}", info.version);
                self.version = Some(info.version.clone());
                self.info = Some(info);
                self.phase = UpdatePhase::Available;
            }
            Ok(None) => {
                log::info!("updater: no update available");
                self.version = None;
                self.info = None;
                self.phase = UpdatePhase::Idle;
            }
            Err(error) => {
                log::error!("updater: check failed: {error:#}");
                self.error = Some(format!("{error:#}"));
                self.phase = UpdatePhase::Idle;
            }
        }
        self.changed();
    }

    /// The update to install, `None` when no update is waiting.
    fn begin_install(&mut self) -> Option<UpdateInfo> {
        let info = self.info.take()?;
        self.phase = UpdatePhase::Installing;
        self.progress = 0;
        self.error = None;
        self.changed();
        Some(info)
    }

    fn set_progress(&mut self, percent: u8) {
        self.progress = percent;
        self.changed();
    }

    /// The found update is gone with the failed install, the next check
    /// finds it again.
    fn fail_install(&mut self, error: String) {
        self.error = Some(error);
        self.phase = UpdatePhase::Idle;
        self.version = None;
        self.changed();
    }
}

/// Whole percent of a download, `None` while the total is not known.
fn percent(done: u64, total: Option<u64>) -> Option<u8> {
    let total = total.filter(|total| *total > 0)?;
    let percent = (u128::from(done) * 100 / u128::from(total)).min(100);
    u8::try_from(percent).ok()
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use anyhow::anyhow;

    use super::{UpdatePhase, UpdateState, percent};
    use crate::system::{UpdateArtifact, UpdateInfo};

    fn info(version: &str) -> UpdateInfo {
        UpdateInfo {
            version:    version.to_string(),
            notes:      String::new(),
            artifact:   UpdateArtifact {
                url:    String::new(),
                size:   0,
                sha256: String::new(),
                sig:    String::new(),
            },
            verify_key: String::new(),
        }
    }

    /// A state with one subscriber that counts its calls.
    fn counted() -> (UpdateState, Rc<Cell<u32>>) {
        let mut state = UpdateState::default();
        let calls = Rc::new(Cell::new(0));
        let seen = calls.clone();
        state.subscribe(|| true, move || seen.set(seen.get() + 1));
        (state, calls)
    }

    #[test]
    fn a_check_that_finds_an_update_makes_it_available() {
        let (mut state, calls) = counted();

        assert!(state.begin_check());
        assert_eq!(state.phase(), UpdatePhase::Checking);
        assert!(state.busy());
        assert_eq!(calls.get(), 1);

        state.finish_check(Ok(Some(info("1.2.0"))));

        assert_eq!(state.phase(), UpdatePhase::Available);
        assert_eq!(state.version(), Some("1.2.0"));
        assert!(state.has_update());
        assert!(!state.busy());
        assert_eq!(calls.get(), 2);
    }

    #[test]
    fn a_check_that_finds_nothing_goes_back_to_idle() {
        let (mut state, calls) = counted();
        state.begin_check();
        state.finish_check(Ok(Some(info("1.2.0"))));

        state.begin_check();
        state.finish_check(Ok(None));

        assert_eq!(state.phase(), UpdatePhase::Idle);
        assert_eq!(state.version(), None);
        assert!(!state.has_update());
        assert!(state.begin_install().is_none());
        assert_eq!(calls.get(), 4);
    }

    #[test]
    fn a_failed_check_keeps_the_error_until_the_next_check() {
        let (mut state, _) = counted();
        state.begin_check();

        state.finish_check(Err(anyhow!("no network")));

        assert_eq!(state.phase(), UpdatePhase::Idle);
        assert_eq!(state.error(), Some("no network"));

        state.begin_check();
        assert_eq!(state.error(), None);
    }

    #[test]
    fn no_second_check_starts_while_one_runs_or_an_install_runs() {
        let (mut state, calls) = counted();

        assert!(state.begin_check());
        assert!(!state.begin_check());
        assert_eq!(calls.get(), 1);

        state.finish_check(Ok(Some(info("1.2.0"))));
        assert!(state.begin_install().is_some());
        assert!(!state.begin_check());
        assert_eq!(state.phase(), UpdatePhase::Installing);
    }

    #[test]
    fn an_install_reports_its_progress_and_cannot_start_twice() {
        let (mut state, calls) = counted();
        state.begin_check();
        state.finish_check(Ok(Some(info("1.2.0"))));

        let taken = state.begin_install();

        assert_eq!(taken.map(|info| info.version).as_deref(), Some("1.2.0"));
        assert_eq!(state.phase(), UpdatePhase::Installing);
        assert_eq!(state.progress(), 0);
        assert!(state.has_update());
        assert!(state.begin_install().is_none());

        state.set_progress(40);
        assert_eq!(state.progress(), 40);
        assert_eq!(calls.get(), 4);
    }

    #[test]
    fn a_failed_install_goes_back_to_idle_with_the_error() {
        let (mut state, _) = counted();
        state.begin_check();
        state.finish_check(Ok(Some(info("1.2.0"))));
        state.begin_install();

        state.fail_install("checksum mismatch".to_string());

        assert_eq!(state.phase(), UpdatePhase::Idle);
        assert_eq!(state.error(), Some("checksum mismatch"));
        assert_eq!(state.version(), None);
        assert!(!state.has_update());
    }

    #[test]
    fn every_live_subscriber_is_called_and_a_dead_one_is_dropped() {
        let mut state = UpdateState::default();
        let first = Rc::new(Cell::new(0));
        let second = Rc::new(Cell::new(0));
        let dead = Rc::new(Cell::new(0));
        let alive = Rc::new(Cell::new(true));

        let seen = first.clone();
        state.subscribe(|| true, move || seen.set(seen.get() + 1));
        let seen = dead.clone();
        let lives = alive.clone();
        state.subscribe(move || lives.get(), move || seen.set(seen.get() + 1));
        let seen = second.clone();
        state.subscribe(|| true, move || seen.set(seen.get() + 1));

        state.set_progress(1);
        alive.set(false);
        state.set_progress(2);
        state.set_progress(3);

        assert_eq!((first.get(), second.get(), dead.get()), (3, 3, 1));
        assert_eq!(state.subscribers.len(), 2);
    }

    #[test]
    fn percent_is_whole_capped_and_unknown_without_a_total() {
        assert_eq!(percent(0, Some(200)), Some(0));
        assert_eq!(percent(50, Some(200)), Some(25));
        assert_eq!(percent(199, Some(200)), Some(99));
        assert_eq!(percent(200, Some(200)), Some(100));
        assert_eq!(percent(300, Some(200)), Some(100));
        assert_eq!(percent(u64::MAX, Some(u64::MAX)), Some(100));
        assert_eq!(percent(10, None), None);
        assert_eq!(percent(10, Some(0)), None);
    }
}
