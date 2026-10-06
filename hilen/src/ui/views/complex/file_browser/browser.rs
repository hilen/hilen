use std::{mem::take, sync::Arc};

use anyhow::Result;

use super::{
    cells::Row,
    choose_bar::{CHOOSE_BAR_HEIGHT, ChooseBar},
    list::{FileList, ListStatus},
    look::{BACKGROUND, BORDER},
    model::{FileSort, Filter, Selection, SortKey},
    sidebar::{SIDEBAR_WIDTH, Sidebar},
    toolbar::Toolbar,
};
use crate::{
    self as hilen,
    deps::{hreads::on_main, refs::Weak, vents::Event},
    filesystem::{FileEntry, FilePath, FilePlace, FileSource},
    gm::flat::Point,
    ui::{
        Container, Input, KeyCombo, MenuItem, Setup, UIEvents, UIManager, ViewData, ViewFrame, view,
        view::DoubleTap,
    },
};

/// Below this width the sidebar gives its room to the list.
const SIDEBAR_MIN_WIDTH: f32 = 480.0;

/// What a [`FileBrowser`] is for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FileBrowserMode {
    /// No bar at the bottom. Opening a file fires `opened`.
    #[default]
    Browse,
    /// Choose gives the picked folder, or the open one with none picked.
    PickFolder,
    PickFile,
    PickFiles,
}

/// A view of a [`FileBrowser`] a test or a script can aim a tap at, see
/// [`FileBrowser::control_center`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileBrowserControl {
    Back,
    Forward,
    Up,
    /// The free part of the path bar, a tap turns it into a text field.
    PathBar,
    Search,
    HiddenSwitch,
    GridSwitch,
    Header(SortKey),
    Retry,
    Cancel,
    Choose,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) enum LoadState {
    #[default]
    Idle,
    Loading,
    Loaded,
    Failed(String),
}

pub(super) type MenuProvider = Box<dyn FnMut(&[FilePath]) -> Vec<MenuItem> + Send>;

/// A file browser like Finder or Explorer, over any [`FileSource`]: the
/// disk of this machine with `LocalFiles`, or a source of the app for
/// another machine, an archive or a cloud drive.
///
/// A toolbar with back, forward and up, a path bar of crumbs that turns
/// into a text field, a search field, a hidden files switch and a list or
/// grid switch. A sidebar of places. The entries as a list with sortable
/// columns or as a grid of icons. With a pick mode a bar at the bottom
/// with Cancel and Choose. Place it as a part of a page, or open it as a
/// dialog with `FilePicker`.
///
/// A tap picks, a double tap or Enter opens. Shift and Cmd or Ctrl pick
/// several. After a press on the list the arrow keys move, letters jump
/// to a name, and Backspace or Cmd Up goes to the parent. On a phone a
/// single tap opens, see `set_open_on_tap`.
#[view]
pub struct FileBrowser {
    pub(super) source:     Option<Arc<dyn FileSource>>,
    pub(super) path:       FilePath,
    history:               Vec<FilePath>,
    history_at:            usize,
    /// Everything the source listed for the open folder.
    all:                   Vec<FileEntry>,
    /// What the filter lets through, in sort order. Rows are indices
    /// into this.
    pub(super) shown:      Vec<FileEntry>,
    pub(super) selection:  Selection,
    sort:                  FileSort,
    filter:                Filter,
    pub(super) mode:       FileBrowserMode,
    open_on_tap:           bool,
    sidebar_hidden:        bool,
    extra_places:          Vec<FilePlace>,
    state:                 LoadState,
    /// Counts the listings asked, an answer to an older one is dropped.
    request:               u64,
    /// Tells the second tap of a double tap on a row.
    taps:                  DoubleTap,
    /// The entry to pick once the folder being opened has listed, the
    /// folder a move to the parent came from.
    pick_when_loaded:      Option<String>,
    pub(super) keys_held:  bool,
    /// The letters typed to jump to a name, with the time of the last.
    pub(super) typed:      (String, f64),
    pub(super) menu_items: Option<MenuProvider>,

    /// The open folder changed.
    pub path_changed:      Event<FilePath>,
    pub selection_changed: Event<Vec<FilePath>>,
    /// A file was opened in `Browse` mode.
    pub opened:            Event<FilePath>,
    /// Choose in a pick mode, with what was picked.
    pub chosen:            Event<Vec<FilePath>>,
    pub cancelled:         Event,

    #[init]
    toolbar:         Toolbar,
    sidebar:         Sidebar,
    sidebar_line:    Container,
    pub(super) list: FileList,
    choose_bar:      ChooseBar,
}

impl FileBrowser {
    /// The source to browse. Opens its first place, unless a folder is
    /// already open.
    pub fn set_source(mut self: Weak<Self>, source: Arc<dyn FileSource>) -> Weak<Self> {
        let places = source.places();
        self.source = Some(source);
        self.refresh_places();

        if self.path.is_empty() {
            if let Some(first) = places.first() {
                self.open(first.path.clone());
            }
        } else {
            self.reload();
        }
        self
    }

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

    pub fn mode(&self) -> FileBrowserMode {
        self.mode
    }

    pub fn set_mode(mut self: Weak<Self>, mode: FileBrowserMode) -> Weak<Self> {
        self.mode = mode;
        self.selection.clear();
        self.layout();
        self.push_rows();
        self
    }

    /// Only files with one of these extensions are shown, folders always
    /// are. No dot, any case. An empty list shows every file.
    pub fn set_extensions(mut self: Weak<Self>, extensions: &[&str]) -> Weak<Self> {
        self.filter.extensions = extensions.iter().map(|extension| extension.to_lowercase()).collect();
        self.refresh_rows();
        self
    }

    pub fn sort(&self) -> FileSort {
        self.sort
    }

    pub fn set_sort(mut self: Weak<Self>, sort: FileSort) -> Weak<Self> {
        self.sort = sort;
        self.list.set_sort(sort);
        self.refresh_rows();
        self
    }

    pub fn is_grid(&self) -> bool {
        self.list.is_grid()
    }

    /// The grid of icons in place of the list with columns.
    pub fn set_grid(self: Weak<Self>, grid: bool) -> Weak<Self> {
        self.list.set_grid(grid);
        self.toolbar.set_grid(grid);
        self
    }

    pub fn shows_hidden(&self) -> bool {
        self.filter.show_hidden
    }

    pub fn set_show_hidden(mut self: Weak<Self>, show: bool) -> Weak<Self> {
        self.filter.show_hidden = show;
        self.toolbar.hidden_button.set_lit(show);
        self.refresh_rows();
        self
    }

    /// The text of the search field, it filters the open folder by name.
    pub fn search(&self) -> &str {
        self.toolbar.search.text()
    }

    pub fn set_search(self: Weak<Self>, text: &str) -> Weak<Self> {
        // The field reports the change, which filters.
        self.toolbar.search.set_text(text);
        self
    }

    /// A single tap opens a folder, and a file while browsing, the way a
    /// touch screen works. On by default on a phone, off on desktop,
    /// where a tap picks and a double tap opens.
    pub fn set_open_on_tap(mut self: Weak<Self>, open: bool) -> Weak<Self> {
        self.open_on_tap = open;
        self
    }

    pub fn set_sidebar_hidden(mut self: Weak<Self>, hidden: bool) -> Weak<Self> {
        self.sidebar_hidden = hidden;
        self.layout();
        self
    }

    /// Another row in the sidebar, after the places of the source: a
    /// recent or a pinned folder.
    pub fn add_place(mut self: Weak<Self>, place: FilePlace) -> Weak<Self> {
        self.extra_places.push(place);
        self.refresh_places();
        self
    }

    /// The app's own entries for the right click menu. It gets the picked
    /// paths, none for a click on the empty area.
    pub fn set_menu_items(
        mut self: Weak<Self>,
        items: impl FnMut(&[FilePath]) -> Vec<MenuItem> + Send + 'static,
    ) -> Weak<Self> {
        self.menu_items = Some(Box::new(items));
        self
    }

    /// The title of the Choose button, `Open` or `Select`.
    pub fn set_choose_title(self: Weak<Self>, title: &str) -> Weak<Self> {
        self.choose_bar.set_choose_title(title);
        self
    }

    /// The entries on screen, filtered and sorted.
    pub fn entries(&self) -> &[FileEntry] {
        &self.shown
    }

    pub fn selected(&self) -> Vec<FilePath> {
        self.selection
            .indices()
            .into_iter()
            .filter_map(|index| self.shown.get(index))
            .map(|entry| self.path.join(&entry.name))
            .collect()
    }

    /// Picks the entries with these names, in place of what was picked.
    /// A name that is not on screen, or that the mode cannot pick, is
    /// left out.
    pub fn select(mut self: Weak<Self>, names: &[&str]) {
        self.selection.clear();
        for name in names {
            let found = self.shown.iter().position(|entry| entry.name == *name);
            if let Some(index) = found.filter(|index| self.can_pick(*index)) {
                self.selection.toggle(index);
            }
        }
        self.selection_moved();
    }

    pub fn is_loading(&self) -> bool {
        self.state == LoadState::Loading
    }

    /// The error of the last listing, `None` when it worked.
    pub fn error(&self) -> Option<&str> {
        match &self.state {
            LoadState::Failed(error) => Some(error),
            _ => None,
        }
    }

    /// What the list shows in place of rows: the wait, the error, or that
    /// nothing is there. Empty while rows show.
    pub fn status_text(&self) -> &str {
        self.list.message()
    }

    /// What Choose would give right now, by the mode.
    pub fn choice(&self) -> Vec<FilePath> {
        let picked: Vec<(FilePath, bool)> = self
            .selection
            .indices()
            .into_iter()
            .filter_map(|index| self.shown.get(index))
            .map(|entry| (self.path.join(&entry.name), entry.is_folder()))
            .collect();

        let files = || picked.iter().filter(|(_, folder)| !folder).map(|(path, _)| path.clone());

        match self.mode {
            FileBrowserMode::Browse => picked.into_iter().map(|(path, _)| path).collect(),
            FileBrowserMode::PickFolder => {
                let folder = picked.into_iter().find(|(_, folder)| *folder).map(|(path, _)| path);
                let open = (!self.path.is_empty()).then(|| self.path.clone());
                folder.or(open).into_iter().collect()
            }
            FileBrowserMode::PickFile => {
                if picked.len() == 1 {
                    files().collect()
                } else {
                    vec![]
                }
            }
            FileBrowserMode::PickFiles => files().collect(),
        }
    }

    /// The same as a tap on Choose. Does nothing with nothing to give.
    pub fn choose(self: Weak<Self>) {
        let choice = self.choice();
        if choice.is_empty() {
            return;
        }
        log::info!(
            "file browser: chose {} path(s), first {}",
            choice.len(),
            self.text_of(&choice[0])
        );
        self.chosen.trigger(choice);
    }

    pub fn cancel(self: Weak<Self>) {
        self.cancelled.trigger(());
    }

    /// The titles of the crumbs of the path bar, left to right.
    pub fn crumb_titles(&self) -> Vec<String> {
        self.toolbar.path_bar.crumb_titles()
    }

    /// The titles of the sidebar rows, top to bottom.
    pub fn place_titles(&self) -> Vec<String> {
        self.sidebar.rows().iter().map(|row| row.title().to_string()).collect()
    }

    /// The middle of the row of this entry in the points of the window,
    /// `None` while it is scrolled out of view. For a test or a script
    /// that taps it.
    pub fn row_center(&self, name: &str) -> Option<Point> {
        let index = self.shown.iter().position(|entry| entry.name == name)?;
        self.list.cell_center(index)
    }

    pub fn crumb_center(&self, title: &str) -> Option<Point> {
        self.toolbar.path_bar.crumb_center(title)
    }

    pub fn place_center(&self, title: &str) -> Option<Point> {
        self.sidebar.row_center(title)
    }

    /// The middle of a control in the points of the window.
    pub fn control_center(&self, control: FileBrowserControl) -> Point {
        let bar = &self.toolbar;
        match control {
            FileBrowserControl::Back => bar.back_button.absolute_frame().center(),
            FileBrowserControl::Forward => bar.forward_button.absolute_frame().center(),
            FileBrowserControl::Up => bar.up_button.absolute_frame().center(),
            FileBrowserControl::PathBar => {
                // The right end, the crumbs start at the left.
                let frame = bar.path_bar.absolute_frame();
                Point::new(frame.max_x() - 8.0, frame.center().y)
            }
            FileBrowserControl::Search => bar.search.absolute_frame().center(),
            FileBrowserControl::HiddenSwitch => bar.hidden_button.absolute_frame().center(),
            FileBrowserControl::GridSwitch => bar.grid_button.absolute_frame().center(),
            FileBrowserControl::Header(key) => self.list.header(key).absolute_frame().center(),
            FileBrowserControl::Retry => self.list.retry_button().absolute_frame().center(),
            FileBrowserControl::Cancel => self.choose_bar.cancel.absolute_frame().center(),
            FileBrowserControl::Choose => self.choose_bar.choose.absolute_frame().center(),
        }
    }

    /// Whether the path bar is a text field right now.
    pub fn is_editing_path(&self) -> bool {
        self.toolbar.path_bar.is_editing()
    }

    /// The text under the list in a pick mode, what Choose would give.
    pub fn choice_text(&self) -> &str {
        self.choose_bar.picked.text()
    }
}

impl FileBrowser {
    pub(super) fn text_of(&self, path: &FilePath) -> String {
        match &self.source {
            Some(source) => source.path_text(path),
            None => path.slash_text(),
        }
    }

    pub(super) fn multi(&self) -> bool {
        matches!(self.mode, FileBrowserMode::Browse | FileBrowserMode::PickFiles)
    }

    /// A file cannot be picked while only a folder is wanted.
    pub(super) fn can_pick(&self, index: usize) -> bool {
        self.shown
            .get(index)
            .is_some_and(|entry| self.mode != FileBrowserMode::PickFolder || entry.is_folder())
    }

    /// Shows a folder, the history untouched. `pick` is the entry to
    /// pick once it has listed.
    fn show(mut self: Weak<Self>, path: FilePath, pick: Option<String>) {
        let changed = self.path != path;
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

    /// Filters and sorts again. The picked entries stay picked by name.
    pub(super) fn refresh_rows(mut self: Weak<Self>) {
        let before = self.selection.indices();
        let old = take(&mut self.shown);
        let shown = self.filter.apply(&self.all, self.sort);

        self.selection.remap(|index| {
            let name = &old.get(index)?.name;
            shown.iter().position(|entry| entry.name == *name)
        });
        self.shown = shown;
        self.push_rows();

        if before != self.selection.indices() {
            self.selection_changed.trigger(self.selected());
        }
    }

    /// Hands the rows with their picked state to the list.
    pub(super) fn push_rows(self: Weak<Self>) {
        let rows: Vec<Row> = self
            .shown
            .iter()
            .enumerate()
            .map(|(index, entry)| Row {
                entry:    entry.clone(),
                selected: self.selection.contains(index),
                enabled:  self.can_pick(index),
            })
            .collect();

        let status = match &self.state {
            LoadState::Failed(error) => ListStatus::Failed(error.clone()),
            LoadState::Loading if rows.is_empty() => ListStatus::Loading,
            LoadState::Loaded if rows.is_empty() => ListStatus::Empty(self.empty_text().to_string()),
            _ => ListStatus::Rows,
        };

        self.list.set_rows(rows, status);
        self.refresh_choice();
    }

    fn empty_text(&self) -> &'static str {
        if self.all.is_empty() {
            "This folder is empty"
        } else {
            "No matches"
        }
    }

    /// After the selection changed: the rows, the bar and the event.
    pub(super) fn selection_moved(mut self: Weak<Self>) {
        let this = self;
        self.selection.retain(|index| this.can_pick(index));
        self.push_rows();
        self.selection_changed.trigger(self.selected());
    }

    fn refresh_choice(self: Weak<Self>) {
        if self.mode == FileBrowserMode::Browse {
            return;
        }
        let choice = self.choice();
        let text = match choice.as_slice() {
            [] => String::new(),
            [one] => self.text_of(one),
            many => format!("{} files", many.len()),
        };
        self.choose_bar.set_picked(&text);
    }

    fn refresh_toolbar(self: Weak<Self>) {
        self.toolbar.back_button.set_enabled(self.can_go_back());
        self.toolbar.forward_button.set_enabled(self.can_go_forward());
        self.toolbar.up_button.set_enabled(self.can_go_up());
        self.toolbar
            .path_bar
            .set_path(self.path.parts().to_vec(), self.text_of(&self.path));
    }

    fn refresh_places(self: Weak<Self>) {
        let mut places = self.source.as_ref().map_or_default(|source| source.places());
        places.extend(self.extra_places.iter().cloned());
        self.sidebar.set_places(places);
        self.sidebar.set_current(self.path.clone());
    }

    fn row_tapped(mut self: Weak<Self>, index: usize) {
        let Some(entry) = self.shown.get(index).cloned() else {
            return;
        };
        self.take_keys();

        let double = self.taps.tap(UIManager::cursor_position()).is_none();

        let browsing = self.mode == FileBrowserMode::Browse;
        if double || (self.open_on_tap && (entry.is_folder() || browsing)) {
            self.open_entry(index);
            return;
        }
        if !self.can_pick(index) {
            return;
        }

        let multi = self.multi();
        // With no keyboard a tap on a file adds it to the picked ones.
        let touch_toggle = self.open_on_tap && self.mode == FileBrowserMode::PickFiles;

        if multi && (Input::command_held() || touch_toggle) {
            self.selection.toggle(index);
        } else if multi && Input::modifiers().shift_key() {
            self.selection.extend_to(index);
        } else {
            self.selection.only(index);
        }
        self.selection_moved();
    }

    /// A double tap or Enter on a row: a folder opens, a file is opened
    /// or chosen by the mode.
    pub(super) fn open_entry(mut self: Weak<Self>, index: usize) {
        let Some(entry) = self.shown.get(index).cloned() else {
            return;
        };
        let path = self.path.join(&entry.name);

        if entry.is_folder() {
            self.open(path);
            return;
        }

        match self.mode {
            FileBrowserMode::Browse => {
                log::info!("file browser: opened the file {}", self.text_of(&path));
                self.opened.trigger(path);
            }
            FileBrowserMode::PickFile | FileBrowserMode::PickFiles => {
                if !self.selection.contains(index) {
                    self.selection.only(index);
                    self.selection_moved();
                }
                self.choose();
            }
            FileBrowserMode::PickFolder => (),
        }
    }

    fn typed_path(self: Weak<Self>, text: &str) {
        let parsed = self.source.as_ref().and_then(|source| source.parse_path(text));
        match parsed {
            Some(path) => self.open(path),
            None => log::warn!("file browser: \"{text}\" is not a path"),
        }
    }

    fn layout(self: Weak<Self>) {
        let width = self.width();
        let height = self.height();
        if width <= 0.0 || height <= 0.0 {
            return;
        }

        let top = Toolbar::height_for(width);
        self.toolbar.set_frame((0.0, 0.0, width, top));

        let picking = self.mode != FileBrowserMode::Browse;
        let bar = if picking { CHOOSE_BAR_HEIGHT } else { 0.0 };
        self.choose_bar.set_hidden(!picking);
        self.choose_bar.set_frame((0.0, height - bar, width, bar));

        let with_sidebar = !self.sidebar_hidden && width >= SIDEBAR_MIN_WIDTH;
        let side = if with_sidebar { SIDEBAR_WIDTH } else { 0.0 };
        let middle = (height - top - bar).max(0.0);

        self.sidebar.set_hidden(!with_sidebar);
        self.sidebar_line.set_hidden(!with_sidebar);
        self.sidebar.set_frame((0.0, top, side, middle));
        self.sidebar_line.set_frame((side - 1.0, top, 1.0, middle));
        self.list.set_frame((side, top, width - side, middle));
    }
}

impl Setup for FileBrowser {
    fn setup(mut self: Weak<Self>) {
        self.open_on_tap = cfg!(mobile);

        self.set_color(BACKGROUND);
        self.sidebar_line.set_color(BORDER);

        self.toolbar.back.sub(move || self.go_back());
        self.toolbar.forward.sub(move || self.go_forward());
        self.toolbar.up.sub(move || self.go_up());
        self.toolbar.hidden_tapped.sub(move || {
            self.set_show_hidden(!self.shows_hidden());
        });
        self.toolbar.grid_tapped.sub(move || {
            self.set_grid(!self.is_grid());
        });
        self.toolbar.search_changed.val(move |text| {
            self.filter.search = text.to_lowercase();
            self.refresh_rows();
        });
        self.toolbar
            .path_bar
            .crumb_tapped
            .val(move |depth| self.open(self.path.prefix(depth)));
        self.toolbar.path_bar.typed.val(move |text| self.typed_path(&text));

        self.sidebar.picked.val(move |path| self.open(path));

        self.list.row_tapped.val(move |index| self.row_tapped(index));
        self.list.row_secondary.val(move |index| self.show_menu(index));
        self.list.empty_tapped.sub(move || {
            self.take_keys();
            self.selection.clear();
            self.selection_moved();
        });
        self.list.sort_tapped.val(move |key| {
            self.set_sort(self.sort.tapped(key));
        });
        self.list.retry.sub(move || self.reload());

        self.choose_bar.chosen.sub(move || self.choose());
        self.choose_bar.cancelled.sub(move || self.cancel());

        UIEvents::touch_began().val(self, move |touch| self.pointer_pressed(touch.position));
        UIEvents::keyboard_key().val(self, move |key| self.on_key(key));
        UIEvents::keyboard_input().val(self, move |key| self.on_char(key));
        UIManager::keymap().add(self, KeyCombo::cmd('a'), move || self.select_all());

        self.size_changed().sub(move || self.layout());
    }
}
