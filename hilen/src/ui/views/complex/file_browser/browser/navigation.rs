//! Moving between folders, and the listing of the open one.

use anyhow::Result;

use super::{FileBrowser, LoadState};
use crate::{
    deps::{hreads::on_main, refs::Weak},
    filesystem::{FileEntry, FilePath},
    ui::view::DoubleTap,
};

impl FileBrowser {
    /// Opens a folder and adds it to the history.
    pub fn open(mut self: Weak<Self>, path: FilePath) {
        if path.is_empty() {
            return;
        }
        if self.history.get(self.history_at) != Some(&path) {
            let keep = (self.history_at + 1).min(self.history.len());
            self.history.truncate(keep);
            self.history.push(path.clone());
            self.history_at = self.history.len() - 1;
        }
        self.show(path, None);
    }

    pub fn path(&self) -> &FilePath {
        &self.path
    }

    /// Asks the source for the open folder again. The picked entries stay
    /// picked when they are still there.
    pub fn reload(self: Weak<Self>) {
        self.load();
    }

    pub fn can_go_back(&self) -> bool {
        self.history_at > 0
    }

    pub fn can_go_forward(&self) -> bool {
        self.history_at + 1 < self.history.len()
    }

    pub fn can_go_up(&self) -> bool {
        self.path.parent().is_some()
    }

    pub fn go_back(mut self: Weak<Self>) {
        if self.can_go_back() {
            self.history_at -= 1;
            let path = self.history[self.history_at].clone();
            self.show(path, None);
        }
    }

    pub fn go_forward(mut self: Weak<Self>) {
        if self.can_go_forward() {
            self.history_at += 1;
            let path = self.history[self.history_at].clone();
            self.show(path, None);
        }
    }

    /// Opens the parent and picks the folder it came from.
    pub fn go_up(mut self: Weak<Self>) {
        let Some(parent) = self.path.parent() else {
            return;
        };
        let from = self.path.name().to_string();
        let keep = (self.history_at + 1).min(self.history.len());
        self.history.truncate(keep);
        self.history.push(parent.clone());
        self.history_at = self.history.len() - 1;
        self.show(parent, Some(from));
    }

    /// Shows a folder, the history untouched. `pick` is the entry to
    /// pick once it has listed.
    fn show(mut self: Weak<Self>, path: FilePath, pick: Option<String>) {
        let changed = self.path != path;
        let had_picks = !self.selection.is_empty();
        self.pick_when_loaded = pick;
        self.path = path.clone();
        self.selection.clear();
        self.all.clear();
        // A tap in the new folder never pairs with one in the old.
        self.taps = DoubleTap::default();
        self.typed.0.clear();

        // A search belongs to the folder it was typed in.
        if !self.toolbar.search.is_empty() {
            self.toolbar.search.set_text("");
        }

        self.refresh_toolbar();
        self.sidebar.set_current(path.clone());
        self.list.scroll_to_top();
        // Before the listing, a source can answer at once, and the pick of
        // the folder a move up came from must come after the drop.
        if had_picks {
            self.selection_changed.trigger(vec![]);
        }
        self.load();

        if changed {
            log::info!("file browser: opened {}", self.text_of(&path));
            self.path_changed.trigger(path);
        }
    }

    fn load(mut self: Weak<Self>) {
        let Some(source) = self.source.clone() else {
            return;
        };
        if self.path.is_empty() {
            return;
        }

        self.request += 1;
        let request = self.request;
        self.state = LoadState::Loading;
        self.push_rows();

        let path = self.path.clone();
        log::debug!("file browser: list {} request {request}", self.text_of(&path));
        source.list(
            &path,
            Box::new(move |result| {
                on_main(move || {
                    if self.is_ok() {
                        self.loaded(request, result);
                    }
                });
            }),
        );
    }

    fn loaded(mut self: Weak<Self>, request: u64, result: Result<Vec<FileEntry>>) {
        if request != self.request {
            log::debug!("file browser: dropped the late answer of request {request}");
            return;
        }

        match result {
            Ok(entries) => {
                log::debug!("file browser: request {request} gave {} entries", entries.len());
                self.all = entries;
                self.state = LoadState::Loaded;
            }
            Err(error) => {
                log::warn!(
                    "file browser: failed to list {}: {error:#}",
                    self.text_of(&self.path)
                );
                self.all.clear();
                self.state = LoadState::Failed(format!("{error:#}"));
            }
        }
        self.refresh_rows();

        if let Some(name) = self.pick_when_loaded.take() {
            self.select(&[&name]);
            if let Some(index) = self.selection.cursor() {
                self.list.reveal(index);
            }
        }
    }
}
