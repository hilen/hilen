//! A camera in a browser through `getUserMedia`. The stream plays in a
//! video element that is never added to the page. A frame of the live
//! picture is drawn into a small canvas when the view asks for it, a photo
//! is the video at its full size drawn into a canvas and saved as a JPEG.
//! Every page object lives on the main thread, a session holds only its
//! number.

use std::{cell::RefCell, collections::HashMap};

use anyhow::{Error, Result, anyhow};
use log::{debug, warn};
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{
    Blob, CanvasRenderingContext2d, HtmlCanvasElement, HtmlVideoElement, MediaDeviceInfo, MediaDeviceKind,
    MediaDevices, MediaStream, MediaStreamConstraints, MediaStreamTrack, MediaTrackConstraints,
    js_sys::{Array, Function, Object, Promise, Reflect, Uint8Array},
    wasm_bindgen::{JsCast, JsValue, closure::Closure},
};

use crate::{
    camera::{
        CameraDevice, CameraError, CameraFrame, CameraLens, CameraLocks, CameraPosition, Photo, PhotoFormat,
        backend::{Backend, Done, Session},
        frame::FrameSlot,
    },
    deps::hreads::on_main,
    gm::flat::Size,
    window::image::fitted_size,
};

/// The size the browser is asked for, it gives the largest it has up to it.
const IDEAL_SIDE: i32 = 4096;
const JPEG_QUALITY: f64 = 0.95;
/// `HAVE_CURRENT_DATA` of a media element, a frame can be drawn.
const HAVE_CURRENT_DATA: u16 = 2;

thread_local! {
    static CAMERAS: RefCell<HashMap<u64, Open>> = RefCell::new(HashMap::new());
    static NEXT: RefCell<u64> = const { RefCell::new(0) };
}

/// 1 open camera, on the main thread.
struct Open {
    stream:  MediaStream,
    video:   HtmlVideoElement,
    preview: (HtmlCanvasElement, CanvasRenderingContext2d),
    frames:  FrameSlot,
    /// The time of the video at the last frame, the same time is the same
    /// frame.
    drawn:   f64,
    /// A frame failed to draw. The next fail the same way, 1 line in the
    /// log is enough.
    warned:  bool,
}

pub(super) struct System;

impl Backend for System {
    fn request_access(&self, done: Done<()>) {
        on_main(move || {
            spawn_local(async move {
                let result = stream(None).await.map(|stream| stop_tracks(&stream));
                done(result);
            });
        });
    }

    fn devices(&self, done: Done<Vec<CameraDevice>>) {
        on_main(move || spawn_local(async move { done(devices().await) }));
    }

    fn open(&self, device: &CameraDevice, frames: FrameSlot, done: Done<Box<dyn Session>>) {
        let device = device.clone();
        on_main(move || {
            spawn_local(async move {
                done(
                    open(&device, frames)
                        .await
                        .map(|id| Box::new(WebSession { id }) as Box<dyn Session>),
                );
            });
        });
    }
}

fn js(err: JsValue) -> Error {
    anyhow!("{err:?}")
}

fn media_devices() -> Result<MediaDevices> {
    web_sys::window()
        .ok_or_else(|| anyhow!("no window"))?
        .navigator()
        .media_devices()
        .map_err(|err| anyhow!("this page has no camera, a page needs https for one: {err:?}"))
}

/// A stream of the camera with `device_id`, any camera for none. A refusal
/// of the user is `NotAllowedError`.
async fn stream(device_id: Option<&str>) -> Result<MediaStream> {
    let constraints = MediaStreamConstraints::new();
    match device_id {
        Some(id) => {
            let video = MediaTrackConstraints::new();
            let exact = Object::new();
            Reflect::set(&exact, &"exact".into(), &id.into()).map_err(js)?;
            video.set_device_id(&exact);
            video.set_width_i32(IDEAL_SIDE);
            video.set_height_i32(IDEAL_SIDE);
            constraints.set_video_media_track_constraints(&video);
        }
        None => constraints.set_video_bool(true),
    }
    let promise = media_devices()?.get_user_media_with_constraints(&constraints).map_err(js)?;
    match JsFuture::from(promise).await {
        Ok(stream) => stream.dyn_into().map_err(|_| anyhow!("getUserMedia gave no stream")),
        Err(err) if error_name(&err).as_deref() == Some("NotAllowedError") => {
            Err(CameraError::Refused.into())
        }
        Err(err) if error_name(&err).as_deref() == Some("NotFoundError") => {
            Err(CameraError::NotFound(device_id.unwrap_or("any camera").to_string()).into())
        }
        Err(err) => Err(js(err)),
    }
}

fn error_name(err: &JsValue) -> Option<String> {
    Reflect::get(err, &"name".into()).ok()?.as_string()
}

fn stop_tracks(stream: &MediaStream) {
    for track in stream.get_tracks() {
        if let Ok(track) = track.dyn_into::<MediaStreamTrack>() {
            track.stop();
        }
    }
}

fn video_track(stream: &MediaStream) -> Option<MediaStreamTrack> {
    stream.get_video_tracks().get(0).dyn_into().ok()
}

async fn devices() -> Result<Vec<CameraDevice>> {
    let promise = media_devices()?.enumerate_devices().map_err(js)?;
    let list: Array = JsFuture::from(promise).await.map_err(js)?.unchecked_into();
    let mut devices = Vec::new();
    for info in list {
        let Ok(info) = info.dyn_into::<MediaDeviceInfo>() else {
            continue;
        };
        if info.kind() != MediaDeviceKind::Videoinput {
            continue;
        }
        let label = info.label();
        let name = if label.is_empty() {
            format!("Camera {}", devices.len() + 1)
        } else {
            label
        };
        devices.push(describe(info.device_id(), name));
    }
    Ok(devices)
}

/// A page learns no position and no lens of a camera, only its name. The
/// names of a phone say both, like "Back Ultra Wide Camera".
fn describe(id: String, name: String) -> CameraDevice {
    let lower = name.to_lowercase();
    let position = if ["back", "rear", "environment"].iter().any(|word| lower.contains(word)) {
        CameraPosition::Back
    } else if ["front", "user", "facetime"].iter().any(|word| lower.contains(word)) {
        CameraPosition::Front
    } else {
        CameraPosition::Unknown
    };
    let lens = if lower.contains("ultra wide") {
        CameraLens::UltraWide
    } else if lower.contains("telephoto") {
        CameraLens::Telephoto
    } else {
        CameraLens::Unknown
    };
    CameraDevice {
        id,
        name,
        position,
        lens,
        near_focus_mm: None,
    }
}

async fn open(device: &CameraDevice, frames: FrameSlot) -> Result<u64> {
    let stream = stream(Some(&device.id)).await?;
    let document = web_sys::window()
        .and_then(|window| window.document())
        .ok_or_else(|| anyhow!("no document"))?;
    let video: HtmlVideoElement = document
        .create_element("video")
        .map_err(js)?
        .dyn_into()
        .map_err(|_| anyhow!("a video element is no video"))?;
    video.set_muted(true);
    // iOS plays a video that is not in the page only inline.
    video.set_attribute("playsinline", "").map_err(js)?;
    video.set_src_object(Some(&stream));
    JsFuture::from(video.play().map_err(js)?).await.map_err(js)?;
    debug!(
        "camera {} streams {} by {}",
        device.name,
        video.video_width(),
        video.video_height()
    );

    let preview = canvas()?;
    let id = NEXT.with_borrow_mut(|next| {
        *next += 1;
        *next
    });
    CAMERAS.with_borrow_mut(|cameras| {
        cameras.insert(
            id,
            Open {
                stream,
                video,
                preview,
                frames,
                drawn: -1.0,
                warned: false,
            },
        )
    });
    Ok(id)
}

fn canvas() -> Result<(HtmlCanvasElement, CanvasRenderingContext2d)> {
    let canvas: HtmlCanvasElement = web_sys::window()
        .and_then(|window| window.document())
        .ok_or_else(|| anyhow!("no document"))?
        .create_element("canvas")
        .map_err(js)?
        .dyn_into()
        .map_err(|_| anyhow!("a canvas element is no canvas"))?;
    let context = canvas
        .get_context("2d")
        .map_err(js)?
        .ok_or_else(|| anyhow!("no 2d context"))?
        .dyn_into()
        .map_err(|_| anyhow!("a 2d context is no 2d context"))?;
    Ok((canvas, context))
}

impl Open {
    /// Puts the frame of the video into the slot, when it is a new one.
    fn pull(&mut self) -> Result<()> {
        let time = self.video.current_time();
        if self.video.ready_state() < HAVE_CURRENT_DATA || time.to_bits() == self.drawn.to_bits() {
            return Ok(());
        }
        self.drawn = time;
        let full = Size::new(self.video.video_width(), self.video.video_height());
        if full.width == 0 || full.height == 0 {
            return Ok(());
        }
        let size = fitted_size(full, CameraFrame::MAX_SIDE).unwrap_or(full);
        let (canvas, context) = &self.preview;
        if canvas.width() != size.width || canvas.height() != size.height {
            canvas.set_width(size.width);
            canvas.set_height(size.height);
        }
        let (width, height) = (f64::from(size.width), f64::from(size.height));
        context
            .draw_image_with_html_video_element_and_dw_and_dh(&self.video, 0.0, 0.0, width, height)
            .map_err(js)?;
        let rgba = context.get_image_data(0.0, 0.0, width, height).map_err(js)?.data().to_vec();
        self.frames.put(rgba, size.width, size.height);
        Ok(())
    }
}

struct WebSession {
    id: u64,
}

impl Session for WebSession {
    fn take_photo(&mut self, format: PhotoFormat, done: Done<Photo>) {
        if format == PhotoFormat::Heic {
            debug!("a browser has no HEIC, the photo is a JPEG");
        }
        let id = self.id;
        on_main(move || spawn_local(async move { done(photo(id).await) }));
    }

    fn set_locks(&mut self, locks: CameraLocks, done: Done<CameraLocks>) {
        let id = self.id;
        on_main(move || spawn_local(async move { done(set_locks(id, locks).await) }));
    }

    fn pull_frame(&mut self) {
        let id = self.id;
        CAMERAS.with_borrow_mut(|cameras| {
            if let Some(open) = cameras.get_mut(&id)
                && let Err(err) = open.pull()
                && !open.warned
            {
                open.warned = true;
                warn!("a camera frame failed to draw: {err}");
            }
        });
    }
}

impl Drop for WebSession {
    fn drop(&mut self) {
        let id = self.id;
        on_main(move || {
            if let Some(open) = CAMERAS.with_borrow_mut(|cameras| cameras.remove(&id)) {
                stop_tracks(&open.stream);
                open.video.set_src_object(None);
            }
        });
    }
}

fn with_open<T>(id: u64, action: impl FnOnce(&Open) -> Result<T>) -> Result<T> {
    CAMERAS.with_borrow(|cameras| action(cameras.get(&id).ok_or_else(|| anyhow!("the camera is closed"))?))
}

async fn photo(id: u64) -> Result<Photo> {
    let (canvas, width, height) = with_open(id, |open| {
        let (width, height) = (open.video.video_width(), open.video.video_height());
        let (canvas, context) = canvas()?;
        canvas.set_width(width);
        canvas.set_height(height);
        context.draw_image_with_html_video_element(&open.video, 0.0, 0.0).map_err(js)?;
        Ok((canvas, width, height))
    })?;
    let bytes = jpeg(&canvas).await?;
    Ok(Photo {
        bytes,
        format: PhotoFormat::Jpeg,
        width,
        height,
    })
}

async fn jpeg(canvas: &HtmlCanvasElement) -> Result<Vec<u8>> {
    let mut failed = None;
    let blob = Promise::new(&mut |resolve: Function, reject: Function| {
        let callback = Closure::once_into_js(move |blob: JsValue| {
            let call = if blob.is_null() {
                reject.call1(&JsValue::NULL, &"the canvas gave no JPEG".into())
            } else {
                resolve.call1(&JsValue::NULL, &blob)
            };
            if let Err(err) = call {
                warn!("a photo was lost on its way: {err:?}");
            }
        });
        if let Err(err) = canvas.to_blob_with_type_and_encoder_options(
            callback.unchecked_ref(),
            "image/jpeg",
            &JPEG_QUALITY.into(),
        ) {
            failed = Some(err);
        }
    });
    if let Some(err) = failed {
        return Err(js(err));
    }
    let blob: Blob = JsFuture::from(blob).await.map_err(js)?.unchecked_into();
    let buffer = JsFuture::from(blob.array_buffer()).await.map_err(js)?;
    Ok(Uint8Array::new(&buffer).to_vec())
}

/// The camera settings of a page are constraints of the track. A browser
/// skips the ones it does not know, so the held locks are read back from
/// the settings of the track.
async fn set_locks(id: u64, locks: CameraLocks) -> Result<CameraLocks> {
    let track = with_open(id, |open| {
        video_track(&open.stream).ok_or_else(|| anyhow!("the stream has no video"))
    })?;
    let wanted = Object::new();
    for (key, locked, auto) in [
        ("focusMode", locks.focus, "continuous"),
        ("exposureMode", locks.exposure, "continuous"),
        ("whiteBalanceMode", locks.white_balance, "continuous"),
    ] {
        let mode = if locked { "manual" } else { auto };
        Reflect::set(&wanted, &key.into(), &mode.into()).map_err(js)?;
    }
    let constraints = MediaTrackConstraints::new();
    constraints.set_advanced(&Array::of1(&wanted));
    JsFuture::from(track.apply_constraints_with_constraints(&constraints).map_err(js)?)
        .await
        .map_err(js)?;

    let settings = track.get_settings();
    let manual = |key: &str| {
        Reflect::get(&settings, &key.into())
            .ok()
            .and_then(|mode| mode.as_string())
            .is_some_and(|mode| mode == "manual")
    };
    Ok(CameraLocks {
        focus:         manual("focusMode"),
        exposure:      manual("exposureMode"),
        white_balance: manual("whiteBalanceMode"),
    })
}
