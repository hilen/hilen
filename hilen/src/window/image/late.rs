//! A picture that a browser build gets later than its image: an svg drawn
//! by the page, and a bundled jpeg that the page decodes, since the wasm has
//! no jpeg decoder. The image is made at once with its size and draws
//! transparent until the pixels land.

use std::{
    future::Future,
    sync::{
        Arc, OnceLock,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
};

use anyhow::Result;
use parking_lot::Mutex;

use crate::{
    deps::hreads::spawn,
    gm::flat::Size,
    window::image::{Image, ImageBind, ImageFilter, Texture, TextureRawData},
};

/// Pictures the last frame drew that the browser still makes, counted by
/// the main thread at the end of each frame, see `Svg::drop_stale_everywhere`.
static PENDING: AtomicUsize = AtomicUsize::new(0);

/// How many pictures the last frame drew that the browser still makes, for
/// a test that reads the frame. A picture of a screen that is gone does not
/// count, a scene that moves every frame would move on while it lands. The
/// tests of a page run on a worker, and a browser does not let the main
/// thread wait for a lock the worker holds, so this reads only an atomic.
pub fn pictures_pending() -> usize {
    PENDING.load(Ordering::SeqCst)
}

pub(crate) fn set_pictures_pending(count: usize) {
    PENDING.store(count, Ordering::SeqCst);
}

/// The names of the images with a pending picture, for the warning of a
/// shot that stopped waiting. On the main thread only.
pub(crate) fn pending_names() -> Vec<String> {
    use crate::deps::refs::manage::DataManager;

    let frame = crate::window::Window::render_frame();
    Image::storage()
        .iter()
        .filter(|(_, image)| image.pending_since(frame.saturating_sub(1)) > 0)
        .map(|(name, _)| name.clone())
        .collect()
}

/// A bind that is filled once, by a task of the page.
#[derive(Debug, Default)]
pub(crate) struct Late {
    bind:    Arc<OnceLock<ImageBind>>,
    started: AtomicBool,
    /// The filter of the image, read when the pixels land.
    filter:  Arc<Mutex<ImageFilter>>,
}

impl Late {
    pub(crate) fn get(&self) -> Option<&ImageBind> {
        self.bind.get()
    }

    /// Started and not landed.
    pub(crate) fn is_pending(&self) -> bool {
        self.started.load(Ordering::SeqCst) && self.bind.get().is_none()
    }

    /// Binds the landed pixels again with `filter`, or keeps it for the
    /// landing. `Image::set_filter` holds the image mutably, so no task
    /// holds the bind but a weak one.
    pub(crate) fn set_filter(&mut self, filter: ImageFilter) {
        *self.filter.lock() = filter;
        if let Some(bind) = Arc::get_mut(&mut self.bind).and_then(OnceLock::get_mut) {
            let view = bind.view.clone();
            *bind = Image::bind_view(&view, &Texture::image_sampler(filter));
        }
    }

    /// Runs `pixels` on the first call and binds what it gives. Later calls
    /// do nothing.
    pub(crate) fn start(&self, what: String, pixels: impl Future<Output = Result<TextureRawData>> + 'static) {
        if self.started.swap(true, Ordering::SeqCst) {
            return;
        }
        let bind = Arc::downgrade(&self.bind);
        let filter = self.filter.clone();
        spawn(async move {
            match pixels.await {
                Ok(raw) => {
                    if let Some(bind) = bind.upgrade() {
                        let texture = Texture::from_raw_data(raw, &what);
                        let sampler = Texture::image_sampler(*filter.lock());
                        let _ = bind.set(Image::bind_view(&texture.view, &sampler));
                    }
                }
                Err(err) => log::error!("The browser did not make {what}: {err}"),
            }
        });
    }
}

/// A 1 by 1 transparent texture, bound while a picture is made.
pub(crate) fn transparent() -> &'static ImageBind {
    static BIND: OnceLock<ImageBind> = OnceLock::new();
    BIND.get_or_init(|| Image::bind_texture(&transparent_texture("transparent")))
}

/// A 1 by 1 transparent texture of its own, for an image whose pixels
/// come later.
pub(crate) fn transparent_texture(label: &str) -> Texture {
    Texture::from_raw_data(
        TextureRawData {
            data:     vec![0; 4],
            size:     Size::new(1, 1),
            channels: 4,
        },
        label,
    )
}
