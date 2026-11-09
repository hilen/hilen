//! Global keys through [global-hotkey](https://github.com/tauri-apps/global-hotkey),
//! the Carbon hot key API underneath.

use std::collections::HashMap;

use anyhow::{Context, Result};
use global_hotkey::{
    GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState,
    hotkey::{HotKey, Modifiers},
};
use log::warn;

use super::GlobalCombo;
use crate::deps::{
    hreads::{assert_main_thread, on_main_later},
    refs::main_lock::MainLock,
};

type Action = Box<dyn FnMut()>;

/// The manager has to be made on the main thread and lives as long as a key
/// is registered.
static MANAGER: MainLock<Option<GlobalHotKeyManager>> = MainLock::new();
static ACTIONS: MainLock<HashMap<u32, Action>> = MainLock::new();

pub(super) struct Registered {
    hot_key: HotKey,
}

impl Registered {
    pub(super) fn new(combo: GlobalCombo, action: Action) -> Result<Self> {
        assert_main_thread();
        let manager = MANAGER.get_mut();
        if manager.is_none() {
            *manager = Some(GlobalHotKeyManager::new().context("The system gave no global key manager")?);
            // The system calls this inside its own event. The action runs in
            // the next turn of the loop, where the engine expects app code.
            GlobalHotKeyEvent::set_event_handler(Some(|event: GlobalHotKeyEvent| {
                if event.state == HotKeyState::Pressed {
                    on_main_later(move || run(event.id));
                }
            }));
        }

        let hot_key = HotKey::new(Some(modifiers(combo)), combo.key);
        manager
            .as_ref()
            .expect("The manager was just made")
            .register(hot_key)
            .with_context(|| format!("{combo} is taken by another app or this one"))?;
        ACTIONS.get_mut().insert(hot_key.id(), action);
        Ok(Self { hot_key })
    }

    pub(super) fn unregister(&self) {
        ACTIONS.get_mut().remove(&self.hot_key.id());
        if let Some(manager) = MANAGER.get_mut().as_ref()
            && let Err(err) = manager.unregister(self.hot_key)
        {
            warn!("The system did not take the global key back: {err}");
        }
    }
}

fn run(id: u32) {
    if let Some(action) = ACTIONS.get_mut().get_mut(&id) {
        action();
    }
}

fn modifiers(combo: GlobalCombo) -> Modifiers {
    let mut modifiers = Modifiers::empty();
    for (held, modifier) in [
        (combo.command, Modifiers::META),
        (combo.shift, Modifiers::SHIFT),
        (combo.control, Modifiers::CONTROL),
        (combo.option, Modifiers::ALT),
    ] {
        if held {
            modifiers |= modifier;
        }
    }
    modifiers
}
