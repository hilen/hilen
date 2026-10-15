use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use image::{ImageBuffer, Rgba, imageops::thumbnail};
use log::warn;
use parking_lot::Mutex;

use crate::{gm::flat::Size, window::image::fitted_size};

/// A picture of the live stream of a camera, RGBA with 4 bytes a pixel.
#[derive(Debug, Clone)]
pub struct CameraFrame {
    pub rgba:   Arc<Vec<u8>>,
    pub width:  u32,
    pub height: u32,
    /// Counts up by 1 with each new picture of the camera.
    pub number: u64,
}

impl CameraFrame {
    /// The longest side a picture of the stream keeps. A photo has the full
    /// size, this is only for the screen.
    pub const MAX_SIDE: u32 = 1280;
}

/// The newest picture of 1 open camera. The backend puts, the app and the
/// `CameraView` read.
#[derive(Clone, Default)]
pub(super) struct FrameSlot {
    inner: Arc<Mutex<Slot>>,
}

#[derive(Default)]
struct Slot {
    frame: Option<CameraFrame>,
    count: u64,
}

/// A stream sends the same broken picture many times a second, 1 line in
/// the log is enough.
static WARNED: AtomicBool = AtomicBool::new(false);

impl FrameSlot {
    /// Keeps `rgba`, shrunk to `CameraFrame::MAX_SIDE`.
    pub(super) fn put(&self, rgba: Vec<u8>, width: u32, height: u32) {
        let Some((rgba, width, height)) = fit(rgba, width, height) else {
            if !WARNED.swap(true, Ordering::Relaxed) {
                warn!("a camera picture of {width} by {height} has a wrong length, it is dropped");
            }
            return;
        };
        let mut slot = self.inner.lock();
        slot.count += 1;
        slot.frame = Some(CameraFrame {
            rgba: Arc::new(rgba),
            width,
            height,
            number: slot.count,
        });
    }

    pub(super) fn latest(&self) -> Option<CameraFrame> {
        self.inner.lock().frame.clone()
    }

    /// How many pictures came.
    #[cfg(any(target_os = "macos", target_os = "ios", target_os = "android"))]
    pub(super) fn count(&self) -> u64 {
        self.inner.lock().count
    }

    /// The same slot, not an equal one.
    #[cfg(any(test, feature = "ui-tests"))]
    pub(super) fn is(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }
}

/// None when `rgba` does not hold `width` by `height` pixels.
fn fit(rgba: Vec<u8>, width: u32, height: u32) -> Option<(Vec<u8>, u32, u32)> {
    let buffer = ImageBuffer::<Rgba<u8>, _>::from_raw(width, height, rgba)?;
    let Some(size) = fitted_size(Size::new(width, height), CameraFrame::MAX_SIDE) else {
        return Some((buffer.into_raw(), width, height));
    };
    let small = thumbnail(&buffer, size.width, size.height);
    Some((small.into_raw(), size.width, size.height))
}

/// One plane of a YUV picture as the camera gives it.
#[cfg(any(target_os = "android", target_os = "windows", test))]
pub(super) struct Plane<'a> {
    pub data:         &'a [u8],
    pub row_stride:   usize,
    pub pixel_stride: usize,
}

#[cfg(any(target_os = "android", target_os = "windows", test))]
impl Plane<'_> {
    fn at(&self, (row, column): (usize, usize)) -> Option<u8> {
        self.data.get(row * self.row_stride + column * self.pixel_stride).copied()
    }
}

/// RGBA from the 3 planes of a YUV 4:2:0 picture, the color plane at half
/// size each way. NV12 is this with 1 plane for both colors.
#[cfg(any(target_os = "android", target_os = "windows", test))]
pub(super) fn yuv420_to_rgba(
    y: &Plane,
    u: &Plane,
    v: &Plane,
    width: usize,
    height: usize,
) -> Option<Vec<u8>> {
    let mut rgba = Vec::with_capacity(width * height * 4);
    for row in 0..height {
        for column in 0..width {
            let luma = *y.data.get(row * y.row_stride + column * y.pixel_stride)?;
            let color = (row / 2, column / 2);
            rgba.extend(yuv(luma, u.at(color)?, v.at(color)?));
        }
    }
    Some(rgba)
}

/// RGBA from YUYV, 2 pixels in 4 bytes that share their color, the way a
/// webcam sends an uncompressed picture.
#[cfg(any(target_os = "windows", target_os = "linux", test))]
pub(super) fn yuyv_to_rgba(bytes: &[u8], width: usize, height: usize) -> Option<Vec<u8>> {
    let pairs = bytes.get(..width * height * 2)?.as_chunks::<4>().0;
    let mut rgba = Vec::with_capacity(width * height * 4);
    for [first, blue, second, red] in pairs {
        rgba.extend(yuv(*first, *blue, *red));
        rgba.extend(yuv(*second, *blue, *red));
    }
    Some(rgba)
}

/// 1 pixel. A camera writes the full range of BT.601, the JPEG way.
#[cfg(any(target_os = "android", target_os = "windows", target_os = "linux", test))]
fn yuv(luma: u8, blue: u8, red: u8) -> [u8; 4] {
    let luma = i32::from(luma);
    let blue_diff = i32::from(blue) - 128;
    let red_diff = i32::from(red) - 128;
    // 1.402, 0.344, 0.714 and 1.772 in steps of 1/1024.
    let red = luma + ((1436 * red_diff) >> 10);
    let green = luma - ((352 * blue_diff + 731 * red_diff) >> 10);
    let blue = luma + ((1815 * blue_diff) >> 10);
    [clamp(red), clamp(green), clamp(blue), u8::MAX]
}

#[cfg(any(target_os = "android", target_os = "windows", target_os = "linux", test))]
fn clamp(value: i32) -> u8 {
    u8::try_from(value.clamp(0, 255)).unwrap_or(u8::MAX)
}

/// RGBA turned clockwise by `quarters` quarter turns. A phone sensor lies
/// on its side, its picture is turned upright for a phone held up.
#[cfg(any(target_os = "android", target_os = "ios", test))]
pub(super) fn turn(rgba: &[u8], width: usize, height: usize, quarters: u32) -> (Vec<u8>, usize, usize) {
    let quarters = quarters % 4;
    if quarters == 0 {
        return (rgba.to_vec(), width, height);
    }
    let (new_width, new_height) = if quarters == 2 {
        (width, height)
    } else {
        (height, width)
    };
    let mut turned = vec![0; rgba.len()];
    for row in 0..height {
        for column in 0..width {
            let (to_column, to_row) = match quarters {
                1 => (height - 1 - row, column),
                2 => (width - 1 - column, height - 1 - row),
                _ => (row, width - 1 - column),
            };
            let from = (row * width + column) * 4;
            let to = (to_row * new_width + to_column) * 4;
            turned[to..to + 4].copy_from_slice(&rgba[from..from + 4]);
        }
    }
    (turned, new_width, new_height)
}
