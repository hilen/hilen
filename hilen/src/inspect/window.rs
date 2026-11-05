//! The window an inspector command works in. `InWindow` runs 1 command in
//! an extra window: every call the command makes on the main thread runs
//! with the slot of that window current, see `window::slots`, so its root
//! view, its input and its frame are the ones of that window.

use std::{
    cell::Cell,
    panic::{AssertUnwindSafe, catch_unwind},
};

#[cfg(wasm)]
use log::debug;

use crate::{
    deps::hreads,
    inspect::{
        AppCommand, InspectService,
        protocol::{InspectorCommand, UIRequest, WindowRepr},
    },
    ui::{UIManager, ViewFrame},
    window::{AppWindow, Window, open_slots, slots::within},
};

thread_local! {
    /// The extra window of the command this thread runs, None for the main
    /// window. A command runs in 1 call on 1 thread, see `serve`.
    static TARGET: Cell<Option<AppWindow>> = const { Cell::new(None) };
}

/// The extra window of the running command.
pub(super) fn target() -> Option<AppWindow> {
    TARGET.get()
}

/// `hreads::from_main` in the window of the running command.
pub(super) fn from_main<T, A>(action: A) -> T
where
    A: FnOnce() -> T + Send + 'static,
    T: Send + 'static, {
    let Some(window) = target() else {
        return hreads::from_main(action);
    };
    match hreads::from_main(move || window.within(action)) {
        Ok(result) => result,
        // Caught by `in_window`, which answers with an error.
        Err(err) => panic!("The inspector lost its window: {err}"),
    }
}

/// The extra window in `slot` once it is up.
#[cfg(not_wasm)]
fn extra(slot: usize) -> Option<AppWindow> {
    AppWindow::of_slot(slot).filter(|window| window.is_ready())
}

/// A page has 1 window and no extra one.
#[cfg(wasm)]
fn extra(slot: usize) -> Option<AppWindow> {
    debug!("A page has no window {slot}");
    None
}

/// A command that reads or drives the views of 1 window.
fn works_in_window(command: &InspectorCommand) -> bool {
    match command {
        InspectorCommand::Screenshot => true,
        InspectorCommand::UI(request) => !matches!(request, UIRequest::Resize { .. }),
        _ => false,
    }
}

impl InspectService {
    pub(super) fn in_window(window: usize, command: InspectorCommand) -> AppCommand {
        if window == 0 {
            return Self::process_command(command);
        }
        if !works_in_window(&command) {
            return AppCommand::Error(format!(
                "This command works on the main window only, not on window {window}"
            ));
        }
        let Some(target) = hreads::from_main(move || extra(window)) else {
            return AppCommand::Error(format!(
                "No window {window} is open, `hilen-inspect windows` lists them"
            ));
        };

        TARGET.set(Some(target));
        let answer = catch_unwind(AssertUnwindSafe(|| Self::process_command(command)));
        TARGET.set(None);
        answer.unwrap_or_else(|_| AppCommand::Error(format!("Window {window} closed while the command ran")))
    }

    pub(super) fn windows() -> AppCommand {
        AppCommand::Windows(hreads::from_main(|| {
            open_slots()
                .into_iter()
                .filter_map(|slot| {
                    let title = if slot == 0 {
                        Window::winit_window().map_or_else(String::new, winit::window::Window::title)
                    } else {
                        extra(slot)?.title()?
                    };
                    let size = within(slot, || UIManager::root_view().size());
                    Some(WindowRepr {
                        number: slot,
                        title,
                        width: size.width,
                        height: size.height,
                    })
                })
                .collect()
        }))
    }
}
