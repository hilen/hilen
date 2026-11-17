use std::sync::Arc;

use super::{
    browser::{FileBrowser, FileBrowserMode},
    look::{BORDER, RADIUS},
};
use crate::{
    self as hilen,
    deps::{refs::Weak, vents::OnceEvent},
    filesystem::{FilePath, FileSource},
    gm::{color::BLACK, flat::Size},
    ui::{ModalView, Setup, UIColor, UIManager, ViewData, ViewFrame, view},
};

const WIDTH: f32 = 760.0;
const HEIGHT: f32 = 520.0;
/// The dialog never comes closer than this to the edge of the window.
const SCREEN_MARGIN: f32 = 16.0;

/// What a [`FilePicker`] opens with: the source, what to pick, and
/// optionally where to start and which files to show.
pub struct FilePick {
    source:     Arc<dyn FileSource>,
    mode:       FileBrowserMode,
    start:      Option<FilePath>,
    extensions: Vec<String>,
    title:      Option<String>,
}

impl FilePick {
    pub fn folder(source: Arc<dyn FileSource>) -> Self {
        Self::new(source, FileBrowserMode::PickFolder)
    }

    pub fn file(source: Arc<dyn FileSource>) -> Self {
        Self::new(source, FileBrowserMode::PickFile)
    }

    pub fn files(source: Arc<dyn FileSource>) -> Self {
        Self::new(source, FileBrowserMode::PickFiles)
    }

    fn new(source: Arc<dyn FileSource>, mode: FileBrowserMode) -> Self {
        Self {
            source,
            mode,
            start: None,
            extensions: vec![],
            title: None,
        }
    }

    /// The folder the dialog opens in, the first place of the source
    /// otherwise.
    #[must_use]
    pub fn start(mut self, path: FilePath) -> Self {
        self.start = Some(path);
        self
    }

    /// Only files with one of these extensions show, no dot, any case.
    #[must_use]
    pub fn extensions(mut self, extensions: &[&str]) -> Self {
        self.extensions = extensions.iter().map(ToString::to_string).collect();
        self
    }

    /// The title of the Choose button.
    #[must_use]
    pub fn choose_title(mut self, title: impl ToString) -> Self {
        self.title = Some(title.to_string());
        self
    }
}

/// A [`FileBrowser`] as a dialog. It gives the picked paths, or `None`
/// on Cancel and Escape:
///
/// ```ignore
/// FilePicker::pick(FilePick::folder(source), move |picked| {
///     if let Some(paths) = picked { ... }
/// });
/// ```
#[view]
pub struct FilePicker {
    event: OnceEvent<Option<Vec<FilePath>>>,

    #[init]
    browser: FileBrowser,
}

impl FilePicker {
    /// Opens the dialog and returns it. Call it on the main thread, the
    /// callback runs there too.
    pub fn pick(request: FilePick, done: impl FnOnce(Option<Vec<FilePath>>) + Send + 'static) -> Weak<Self> {
        let picker = Self::prepare_modally_with_input(request);
        picker.modal_event().val(done);
        picker
    }

    pub fn browser(&self) -> Weak<FileBrowser> {
        self.browser
    }
}

impl ModalView<FilePick, Option<Vec<FilePath>>> for FilePicker {
    fn modal_event(&self) -> &OnceEvent<Option<Vec<FilePath>>> {
        &self.event
    }

    fn modal_size() -> Size {
        (WIDTH, HEIGHT).into()
    }

    fn modal_scrim_color() -> UIColor {
        BLACK.with_alpha(0.25).into()
    }

    fn modal_cancel(self: Weak<Self>) -> Option<Option<Vec<FilePath>>> {
        Some(None)
    }

    fn setup_input(self: Weak<Self>, request: FilePick) {
        // A phone or a small window gets a dialog that fits it.
        let screen = UIManager::root_view().frame().size;
        self.set_modal_size((
            WIDTH.min(screen.width - SCREEN_MARGIN * 2.0),
            HEIGHT.min(screen.height - SCREEN_MARGIN * 2.0),
        ));

        let extensions: Vec<&str> = request.extensions.iter().map(String::as_str).collect();
        self.browser.set_mode(request.mode).set_extensions(&extensions);
        if let Some(title) = &request.title {
            self.browser.set_choose_title(title);
        }
        if let Some(start) = request.start {
            self.browser.open(start);
        }
        self.browser.set_source(request.source);
        self.browser.take_keys();
    }
}

impl Setup for FilePicker {
    fn setup(self: Weak<Self>) {
        self.set_corner_radius(RADIUS + 4.0)
            .set_border_width(1)
            .set_border_color(BORDER);

        self.browser.place().all_sides(1);
        self.browser.chosen.val(move |paths| self.hide_modal(Some(paths)));
        self.browser.cancelled.sub(move || self.hide_modal(None));
    }
}
