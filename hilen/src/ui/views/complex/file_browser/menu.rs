use super::{
    browser::FileBrowser,
    name_prompt::{NamePrompt, NameRequest},
};
use crate::{
    deps::{hreads::on_main, refs::Weak},
    filesystem::{Changed, FilePath},
    ui::{Alert, ContextMenu, MenuItem, ModalView, Question},
};

/// The right click menu of the browser, and the changes it offers when
/// the source can write.
impl FileBrowser {
    /// A right click or a long press, on the row `index` or on the empty
    /// area. A click on a row that is not picked picks only that row.
    pub(super) fn show_menu(mut self: Weak<Self>, index: Option<usize>) {
        self.take_keys();

        match index {
            Some(index) if self.selection.contains(index) => (),
            Some(index) if self.can_pick(index) => {
                self.selection.only(index);
                self.selection_moved();
            }
            _ => {
                self.selection.clear();
                self.selection_moved();
            }
        }

        let index = index.filter(|index| self.selection.contains(*index));
        let targets = self.selected();
        let writable = self.source.as_ref().is_some_and(|source| source.can_write());

        let mut items = vec![];

        if let Some(index) = index {
            items.push(MenuItem::new("Open", move || self.open_entry(index)));
        }

        let extra = match &mut self.menu_items {
            Some(provider) => provider(&targets),
            None => vec![],
        };
        add_group(&mut items, extra);

        if writable {
            let mut changes = vec![];
            if targets.is_empty() {
                changes.push(MenuItem::new("New Folder", move || self.ask_new_folder()));
            } else {
                if let [target] = targets.as_slice() {
                    let target = target.clone();
                    changes.push(MenuItem::new("Rename", move || self.ask_rename(target.clone())));
                }
                changes.push(MenuItem::new("Delete", move || self.ask_delete(targets.clone())).danger());
            }
            add_group(&mut items, changes);
        }

        if !items.is_empty() {
            ContextMenu::show_at_cursor(items);
        }
    }

    fn ask_new_folder(self: Weak<Self>) {
        let request = NameRequest {
            title:   "New Folder".to_string(),
            name:    "New Folder".to_string(),
            confirm: "Create".to_string(),
        };
        NamePrompt::show_modally_with_input(request, move |name| {
            let (Some(name), Some(source)) = (name, self.source.clone()) else {
                return;
            };
            source.make_folder(&self.path, &name, self.after_change("create the folder"));
        });
    }

    fn ask_rename(self: Weak<Self>, target: FilePath) {
        let request = NameRequest {
            title:   "Rename".to_string(),
            name:    target.name().to_string(),
            confirm: "Rename".to_string(),
        };
        NamePrompt::show_modally_with_input(request, move |name| {
            let (Some(name), Some(source)) = (name, self.source.clone()) else {
                return;
            };
            if name != target.name() {
                source.rename(&target, &name, self.after_change("rename"));
            }
        });
    }

    fn ask_delete(self: Weak<Self>, targets: Vec<FilePath>) {
        let what = match targets.as_slice() {
            [one] => format!("\"{}\"", one.name()),
            many => format!("{} items", many.len()),
        };
        Question::ask(format!("Delete {what}?"))
            .options("Cancel", "Delete")
            .destructive()
            .on_yes(move || {
                let Some(source) = self.source.clone() else {
                    return;
                };
                for target in &targets {
                    source.delete(target, self.after_change("delete"));
                }
            });
    }

    /// The answer of a change: the folder is listed again, or the error
    /// is shown.
    fn after_change(self: Weak<Self>, what: &'static str) -> Changed {
        Box::new(move |result| {
            on_main(move || {
                if self.is_null() {
                    return;
                }
                match result {
                    Ok(()) => {
                        log::info!("file browser: {what} done in {}", self.text_of(&self.path));
                        self.reload();
                    }
                    Err(error) => {
                        log::warn!("file browser: failed to {what}: {error:#}");
                        Alert::show(format!("{error:#}"));
                    }
                }
            });
        })
    }
}

/// Adds a group of items after a separator line, nothing for no items.
fn add_group(items: &mut Vec<MenuItem>, mut group: Vec<MenuItem>) {
    if group.is_empty() {
        return;
    }
    if !items.is_empty() {
        items.push(MenuItem::separator());
    }
    items.append(&mut group);
}
