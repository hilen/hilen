//! A webcam on Windows and Linux: Media Foundation through nokhwa, Video
//! for Linux through linuxvideo. The camera runs on a thread of its own at
//! its highest resolution, a photo is the next picture of that stream. A
//! camera that sends JPEG pictures gives that JPEG as the photo as it is.

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use linux as webcam;

#[cfg(target_os = "windows")]
mod windows;
use std::{
    sync::mpsc::{Receiver, Sender, TryRecvError, channel},
    thread::{JoinHandle, sleep, spawn},
    time::Duration,
};

use anyhow::{Result, anyhow};
use image::{ExtendedColorType, ImageFormat, load_from_memory_with_format};
use log::{debug, error, warn};
#[cfg(target_os = "windows")]
use windows as webcam;

#[cfg(target_os = "windows")]
use crate::camera::frame::{Plane, yuv420_to_rgba};
use crate::camera::{
    CameraDevice, CameraLocks, Photo, PhotoFormat,
    backend::{Backend, Done, Session},
    frame::{FrameSlot, yuyv_to_rgba},
    photo::encode_jpeg,
};

/// A camera that fails this many pictures in a row is taken as gone.
const FAILURES: u32 = 50;
const RETRY: Duration = Duration::from_millis(100);

/// How the bytes of a picture of a webcam hold its pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Pixels {
    Jpeg,
    Yuyv,
    #[cfg(target_os = "windows")]
    Nv12,
    #[cfg(target_os = "windows")]
    Gray,
    Rgb,
    Bgr,
}

/// 1 picture of a webcam, as it sent it.
pub(super) struct Picture {
    pub pixels: Pixels,
    pub bytes:  Vec<u8>,
    pub width:  u32,
    pub height: u32,
}

/// An open webcam, on the thread of its session. Dropping it closes it.
pub(super) trait Webcam {
    /// Waits for the next picture.
    fn picture(&mut self) -> Result<Picture>;

    /// Answers with the locks the camera holds after the call.
    fn set_locks(&mut self, locks: CameraLocks) -> Result<CameraLocks>;
}

pub(super) struct System;

impl Backend for System {
    /// Windows and Linux ask no question, a camera the user turned off
    /// fails to open.
    fn request_access(&self, done: Done<()>) {
        done(Ok(()));
    }

    fn devices(&self, done: Done<Vec<CameraDevice>>) {
        done(webcam::list());
    }

    /// Opening a camera takes up to a second, it waits on a thread of its
    /// own and not on the one of the caller.
    fn open(&self, device: &CameraDevice, frames: FrameSlot, done: Done<Box<dyn Session>>) {
        let device = device.clone();
        spawn(move || {
            let (commands, receive) = channel();
            let (ready, readied) = channel();
            let thread = spawn(move || run(&device, &frames, &receive, &ready));
            match readied.recv() {
                Ok(Ok(())) => done(Ok(Box::new(DesktopSession {
                    commands: Some(commands),
                    thread:   Some(thread),
                }))),
                Ok(Err(err)) => {
                    join(thread);
                    done(Err(err));
                }
                Err(err) => {
                    join(thread);
                    done(Err(anyhow!(
                        "the camera thread ended before the camera opened: {err}"
                    )));
                }
            }
        });
    }
}

enum Command {
    Photo(PhotoFormat, Done<Photo>),
    Locks(CameraLocks, Done<CameraLocks>),
}

impl Command {
    fn fail(self, reason: &str) {
        match self {
            Self::Photo(_, done) => done(Err(anyhow!("{reason}"))),
            Self::Locks(_, done) => done(Err(anyhow!("{reason}"))),
        }
    }
}

struct DesktopSession {
    commands: Option<Sender<Command>>,
    thread:   Option<JoinHandle<()>>,
}

impl DesktopSession {
    fn send(&self, command: Command) {
        let Some(commands) = &self.commands else {
            command.fail("the camera is closed");
            return;
        };
        if let Err(err) = commands.send(command) {
            err.0.fail("the camera stopped, it failed too many pictures in a row");
        }
    }
}

impl Session for DesktopSession {
    fn take_photo(&mut self, format: PhotoFormat, done: Done<Photo>) {
        self.send(Command::Photo(format, done));
    }

    fn set_locks(&mut self, locks: CameraLocks, done: Done<CameraLocks>) {
        self.send(Command::Locks(locks, done));
    }
}

impl Drop for DesktopSession {
    /// The closed channel ends the thread after its current picture. The
    /// wait for that is not put on the thread that drops the camera, often
    /// the main thread.
    fn drop(&mut self) {
        self.commands = None;
        if let Some(thread) = self.thread.take() {
            spawn(move || join(thread));
        }
    }
}

fn join(thread: JoinHandle<()>) {
    if thread.join().is_err() {
        error!("the camera thread panicked");
    }
}

/// The thread of 1 open camera. It ends when the session closes its
/// channel, or when the camera fails `FAILURES` pictures in a row.
fn run(device: &CameraDevice, frames: &FrameSlot, commands: &Receiver<Command>, ready: &Sender<Result<()>>) {
    let mut camera = match webcam::open(device) {
        Ok(camera) => camera,
        Err(err) => {
            if ready.send(Err(err)).is_err() {
                debug!("nobody waits for camera {} any more", device.name);
            }
            return;
        }
    };
    if ready.send(Ok(())).is_err() {
        debug!("nobody waits for camera {} any more", device.name);
        return;
    }

    let mut waiting: Vec<(PhotoFormat, Done<Photo>)> = Vec::new();
    let mut failures = 0;
    let mut decode_failed = false;
    loop {
        loop {
            match commands.try_recv() {
                Ok(Command::Photo(format, done)) => waiting.push((format, done)),
                Ok(Command::Locks(locks, done)) => done(camera.set_locks(locks)),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    debug!("camera {} closed", device.name);
                    return;
                }
            }
        }
        match camera.picture() {
            Ok(picture) => {
                failures = 0;
                // A photo is the first picture after the ask, so it shows
                // the locks set before it.
                for (format, done) in waiting.drain(..) {
                    done(photo_of(&picture, format));
                }
                match rgba_of(&picture) {
                    Ok(rgba) => frames.put(rgba, picture.width, picture.height),
                    // The next pictures fail the same way, 1 line is enough.
                    Err(err) if !decode_failed => {
                        decode_failed = true;
                        warn!("a picture of camera {} failed to decode: {err}", device.name);
                    }
                    Err(_) => {}
                }
            }
            Err(err) => {
                failures += 1;
                if failures == 1 {
                    warn!("camera {} failed a picture: {err}", device.name);
                }
                if failures >= FAILURES {
                    error!(
                        "camera {} failed {failures} pictures in a row, it is closed",
                        device.name
                    );
                    for (_, done) in waiting.drain(..) {
                        done(Err(anyhow!("the camera stopped sending pictures")));
                    }
                    return;
                }
                sleep(RETRY);
            }
        }
    }
}

fn rgba_of(picture: &Picture) -> Result<Vec<u8>> {
    let bytes = &picture.bytes;
    let width = usize::try_from(picture.width)?;
    let height = usize::try_from(picture.height)?;
    let short = || {
        anyhow!(
            "a {:?} picture is shorter than {width} by {height}",
            picture.pixels
        )
    };
    Ok(match picture.pixels {
        Pixels::Jpeg => load_from_memory_with_format(bytes, ImageFormat::Jpeg)?.into_rgba8().into_raw(),
        Pixels::Yuyv => yuyv_to_rgba(bytes, width, height).ok_or_else(short)?,
        #[cfg(target_os = "windows")]
        Pixels::Nv12 => {
            let (luma, color) = bytes.split_at_checked(width * height).ok_or_else(short)?;
            let plane = |data, pixel_stride| Plane {
                data,
                row_stride: width,
                pixel_stride,
            };
            let blue = plane(color, 2);
            let red = plane(color.get(1..).ok_or_else(short)?, 2);
            yuv420_to_rgba(&plane(luma, 1), &blue, &red, width, height).ok_or_else(short)?
        }
        #[cfg(target_os = "windows")]
        Pixels::Gray => bytes.iter().flat_map(|gray| [*gray, *gray, *gray, u8::MAX]).collect(),
        Pixels::Rgb => bytes
            .as_chunks::<3>()
            .0
            .iter()
            .flat_map(|[red, green, blue]| [*red, *green, *blue, u8::MAX])
            .collect(),
        Pixels::Bgr => bytes
            .as_chunks::<3>()
            .0
            .iter()
            .flat_map(|[blue, green, red]| [*red, *green, *blue, u8::MAX])
            .collect(),
    })
}

/// A webcam has no HEIC, every photo is a JPEG.
fn photo_of(picture: &Picture, format: PhotoFormat) -> Result<Photo> {
    if format == PhotoFormat::Heic {
        debug!("a webcam has no HEIC, the photo is a JPEG");
    }
    if picture.pixels == Pixels::Jpeg {
        return Ok(Photo {
            bytes:  picture.bytes.clone(),
            format: PhotoFormat::Jpeg,
            width:  picture.width,
            height: picture.height,
        });
    }
    let rgb: Vec<u8> = rgba_of(picture)?
        .as_chunks::<4>()
        .0
        .iter()
        .flat_map(|[red, green, blue, _]| [*red, *green, *blue])
        .collect();
    encode_jpeg(&rgb, picture.width, picture.height, ExtendedColorType::Rgb8)
}
