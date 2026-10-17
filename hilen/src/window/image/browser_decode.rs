//! A browser decodes a downloaded picture in native code off the main
//! thread, and shrinks it on the way. The `image` crate in wasm took up to
//! 2.5 s for 1 poster on an LG C1 TV, on the main thread, at full size.
//!
//! 2 decodes run at a time and share 1 canvas. 49 posters decoded at once
//! put about 150 MB on top of the page of the same TV, each with its own
//! bitmap, canvas and pixel copies.

use std::{
    cell::RefCell,
    sync::atomic::{AtomicUsize, Ordering},
};

use anyhow::{Result, anyhow};
use event_listener::Event;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    Blob, BlobPropertyBag, CanvasRenderingContext2d, HtmlCanvasElement, HtmlImageElement, ImageBitmap,
    ImageBitmapOptions, ResizeQuality, Url, Window,
    js_sys::{Array, Uint8Array},
    wasm_bindgen::{JsCast, JsValue},
};

const AT_ONCE: usize = 2;

static RUNNING: AtomicUsize = AtomicUsize::new(0);
static FREED: Event = Event::new();

thread_local! {
    static CANVAS: RefCell<Option<(HtmlCanvasElement, CanvasRenderingContext2d)>> = const { RefCell::new(None) };
}

/// 1 of the decodes that may run at once, given back when dropped.
struct Slot;

impl Drop for Slot {
    fn drop(&mut self) {
        RUNNING.fetch_sub(1, Ordering::SeqCst);
        FREED.notify(1);
    }
}

async fn slot() -> Slot {
    loop {
        if RUNNING
            .try_update(Ordering::SeqCst, Ordering::SeqCst, |running| {
                (running < AT_ONCE).then_some(running + 1)
            })
            .is_ok()
        {
            return Slot;
        }
        let freed = FREED.listen();
        // A slot freed between the try and the listen sends no notice to it.
        if RUNNING.load(Ordering::SeqCst) < AT_ONCE {
            continue;
        }
        freed.await;
    }
}

/// The canvas the pixels are read back from, made once for the page.
fn canvas(window: &Window) -> Result<(HtmlCanvasElement, CanvasRenderingContext2d)> {
    if let Some(made) = CANVAS.with_borrow(Clone::clone) {
        return Ok(made);
    }
    let canvas: HtmlCanvasElement = window
        .document()
        .ok_or_else(|| anyhow!("No document to decode a picture in"))?
        .create_element("canvas")
        .map_err(js)?
        .dyn_into()
        .map_err(|_| anyhow!("A canvas element is no canvas"))?;
    let context: CanvasRenderingContext2d = canvas
        .get_context("2d")
        .map_err(js)?
        .ok_or_else(|| anyhow!("No 2d context to read a picture from"))?
        .dyn_into()
        .map_err(|_| anyhow!("A 2d context is no 2d context"))?;
    CANVAS.set(Some((canvas.clone(), context.clone())));
    Ok((canvas, context))
}

use crate::{
    gm::flat::Size,
    window::image::{TextureRawData, fitted_size, image_header},
};

fn js(err: JsValue) -> anyhow::Error {
    anyhow!("{err:?}")
}

/// The pixels of the picture in `bytes`, its longest side at most
/// `max_side`, 0 for its own size. Only the header is read in wasm, for the
/// size. A page with no window, a worker, gets an error and decodes in wasm,
/// which has no jpeg decoder.
pub(crate) async fn decode(bytes: &[u8], max_side: u32) -> Result<TextureRawData> {
    let (width, height) =
        image_header::dimensions(bytes).ok_or_else(|| anyhow!("Not a png, jpeg, gif or webp header"))?;
    let size = Size::new(width, height);
    let size = fitted_size(size, max_side).unwrap_or(size);

    let window = web_sys::window().ok_or_else(|| anyhow!("No window to decode a picture in"))?;
    let _slot = slot().await;
    let parts = Array::of1(&Uint8Array::from(bytes));
    let blob = Blob::new_with_u8_array_sequence(&parts).map_err(js)?;

    let options = ImageBitmapOptions::new();
    options.set_resize_width(size.width);
    options.set_resize_height(size.height);
    options.set_resize_quality(ResizeQuality::High);
    let promise = window
        .create_image_bitmap_with_blob_and_image_bitmap_options(&blob, &options)
        .map_err(js)?;
    let bitmap: ImageBitmap = JsFuture::from(promise).await.map_err(js)?.dyn_into().map_err(js)?;

    // A new size also clears what the last picture left on the canvas.
    let (canvas, context) = canvas(&window)?;
    canvas.set_width(size.width);
    canvas.set_height(size.height);
    context.draw_image_with_image_bitmap(&bitmap, 0.0, 0.0).map_err(js)?;
    bitmap.close();
    let data = context
        .get_image_data(0.0, 0.0, f64::from(size.width), f64::from(size.height))
        .map_err(js)?
        .data()
        .to_vec();

    Ok(TextureRawData {
        data,
        size,
        channels: 4,
    })
}

/// The svg in `bytes` drawn by the browser at `size`, straight alpha RGBA
/// like a picture. An `<img>` draws it, `createImageBitmap` takes no svg in
/// every browser. The root tag must carry `size`, see `svg_size::sized`.
pub(crate) async fn rasterize_svg(bytes: &[u8], size: Size<u32>) -> Result<TextureRawData> {
    let window = web_sys::window().ok_or_else(|| anyhow!("No window to draw an svg in"))?;
    let parts = Array::of1(&Uint8Array::from(bytes));
    let options = BlobPropertyBag::new();
    options.set_type("image/svg+xml");
    let blob = Blob::new_with_u8_array_sequence_and_options(&parts, &options).map_err(js)?;
    let url = Url::create_object_url_with_blob(&blob).map_err(js)?;
    let image = HtmlImageElement::new().map_err(js)?;
    image.set_src(&url);
    let decoded = JsFuture::from(image.decode()).await;
    let _ = Url::revoke_object_url(&url);
    decoded.map_err(js)?;

    // A new size also clears what the last picture left on the canvas.
    let (canvas, context) = canvas(&window)?;
    canvas.set_width(size.width);
    canvas.set_height(size.height);
    context
        .draw_image_with_html_image_element_and_dw_and_dh(
            &image,
            0.0,
            0.0,
            f64::from(size.width),
            f64::from(size.height),
        )
        .map_err(js)?;
    let data = context
        .get_image_data(0.0, 0.0, f64::from(size.width), f64::from(size.height))
        .map_err(js)?
        .data()
        .to_vec();

    Ok(TextureRawData {
        data,
        size,
        channels: 4,
    })
}
