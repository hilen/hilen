use super::ImageViewer;
use crate::{
    deps::refs::Weak,
    image::Image,
    ui::{View, ViewData, ViewSubviews},
};

/// A page of an open `ImageViewer`, given to the app when the page opens.
/// The app fills it now or later, when a picture has loaded. A call on a
/// page that was left, by a step or a close, does nothing, so a late load
/// never lands on another page.
#[derive(Clone, Copy)]
pub struct ImageViewerPage {
    pub(super) viewer: Weak<ImageViewer>,
    pub(super) index:  usize,
}

impl ImageViewerPage {
    pub fn index(&self) -> usize {
        self.index
    }

    /// Whether this page still shows.
    pub fn is_open(&self) -> bool {
        self.viewer.is_ok() && self.viewer.at == self.index
    }

    /// The picture at full size. Its pixels set how far it zooms. It is
    /// never drawn bigger than 1 point a pixel until the user zooms.
    pub fn show_image(&self, image: Weak<Image>) {
        if self.is_open() {
            self.viewer.show(image, true);
        }
    }

    /// A small copy that shows while the full picture loads, fitted to
    /// the window. `show_image` takes its place.
    pub fn show_preview(&self, image: Weak<Image>) {
        if self.is_open() {
            self.viewer.show(image, false);
        }
    }

    /// A line of text at the bottom, like why the picture did not load.
    /// It stays until the page is left or a picture comes.
    pub fn show_note(&self, text: impl ToString) {
        if self.is_open() {
            self.viewer.show_note(text.to_string());
        }
    }

    /// A view of the app's own in place of a picture, like a 3D model. It
    /// fills the page between the step buttons and goes when the page is
    /// left, and the note goes when it comes. It does not zoom, a view
    /// that zooms brings its own touches. None when the page was left
    /// already.
    pub fn add_view<V: 'static + View + Default>(&self) -> Option<Weak<V>> {
        if !self.is_open() {
            return None;
        }
        self.viewer.hide_note();
        let view = self.viewer.page.add_view::<V>();
        view.place().back();
        Some(view)
    }

    /// Runs when the page is left, by a step or a close, like to stop a
    /// scene the page started.
    pub fn on_leave(&self, action: impl FnOnce() + Send + 'static) {
        if self.is_open() {
            let mut viewer = self.viewer;
            viewer.leave.push(Box::new(action));
        }
    }
}
