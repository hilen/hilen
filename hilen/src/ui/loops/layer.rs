//! A Core Animation layer over the window view that shows the pictures of
//! a loop with a discrete keyframe animation on `contents`. The system
//! shows the next picture at its time, the app does nothing and its loop
//! can sleep.
//!
//! Only classes of the system are used, so a hot build has no class of its
//! own to delete at its stop, see `docs/hot-reload.md`. Every call is old
//! enough for iOS 12.

use std::ptr::{NonNull, from_ref, null};

use anyhow::{Context, Result};
use objc2::{msg_send, rc::Retained, runtime::AnyObject};
use objc2_core_foundation::{CFData, CFIndex, CFRetained, CGPoint, CGRect, CGSize};
use objc2_core_graphics::{
    CGBitmapInfo, CGColorRenderingIntent, CGColorSpace, CGDataProvider, CGImage, CGImageAlphaInfo,
    CGImageByteOrderInfo, kCGColorSpaceSRGB,
};
use objc2_foundation::{NSArray, NSNumber, NSString};
use objc2_quartz_core::{
    CAKeyframeAnimation, CALayer, CAMediaTiming, CATransaction, kCAAnimationDiscrete, kCAGravityResize,
};
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

use super::on_layer::{PixelOrder, RawFrames};
#[cfg(feature = "ui-tests")]
use crate::gm::LossyConvert;
use crate::{gm::flat::Rect, window::Window};

/// The key of the keyframe animation on the layer.
const ANIMATION: &str = "hilen.loop";

/// Siblings with a bigger z are in front, the layers of the surface keep 0.
const LAYER_Z: f64 = 1000.0;

pub(super) struct Layer {
    root:      Retained<CALayer>,
    layer:     Retained<CALayer>,
    /// The y axis of the root layer goes down, as in the window.
    y_down:    bool,
    images:    Vec<CFRetained<CGImage>>,
    key_times: Vec<f64>,
    period:    f64,
    /// Where the layer is, in the pixels of the window.
    rect:      Option<Rect>,
    /// The layer stays on the window view when it is dropped.
    stays:     bool,
}

impl Layer {
    /// A hidden layer on top of the layer of the window view.
    pub(super) fn new() -> Result<Self> {
        let view = window_view().context("the window has no view")?;
        // SAFETY: a view of winit, which lives with the window, asked on
        // the main thread.
        let view: &AnyObject = unsafe { view.as_ref() };
        // SAFETY: `layer` of an NSView or a UIView gives a CALayer or nil.
        let root: Option<Retained<CALayer>> = unsafe { msg_send![view, layer] };
        let root = root.context("the window view has no layer")?;

        let layer = CALayer::new();
        let y_down = transaction(|| {
            layer.setHidden(true);
            // Over the Metal layer of the surface, also over a new one that
            // a lost surface adds later.
            layer.setZPosition(LAYER_Z);
            // SAFETY: a constant of Core Animation.
            layer.setContentsGravity(unsafe { kCAGravityResize });
            root.addSublayer(&layer);
            // The pictures are rows from the top. Under an odd number of
            // flipped layers Core Animation draws such a picture upside
            // down, and a flip of the layer itself puts it back.
            if layer.contentsAreFlipped() {
                layer.setGeometryFlipped(true);
            }
            #[cfg(macos)]
            let y_down = y_down(view, &root);
            // The layers of UIKit have their 0 point at the top left.
            #[cfg(ios)]
            let y_down = true;
            y_down
        });

        Ok(Self {
            root,
            layer,
            y_down,
            images: vec![],
            key_times: vec![],
            period: 1.0,
            rect: None,
            stays: false,
        })
    }

    /// The pictures and their times, from 0 to 1 with 1 more time than
    /// pictures, the end of the last one.
    pub(super) fn set_pictures(
        &mut self,
        frames: &RawFrames,
        key_times: Vec<f64>,
        period: f64,
    ) -> Result<()> {
        // SAFETY: a constant of Core Graphics.
        let space =
            CGColorSpace::with_name(Some(unsafe { kCGColorSpaceSRGB })).context("no sRGB color space")?;
        let info = match frames.order {
            PixelOrder::BgraPremultiplied => {
                CGBitmapInfo(CGImageByteOrderInfo::Order32Little.0 | CGImageAlphaInfo::PremultipliedFirst.0)
            }
            PixelOrder::Rgba => CGBitmapInfo(CGImageAlphaInfo::Last.0),
        };
        let width = usize::try_from(frames.size.width)?;
        let height = usize::try_from(frames.size.height)?;

        let images = frames
            .pixels
            .iter()
            .map(|pixels| image(pixels, width, height, &space, info))
            .collect::<Result<Vec<_>>>()?;
        let first = images.first().context("a loop with no pictures")?;
        transaction(|| {
            // SAFETY: a CGImage is what `contents` takes.
            unsafe { self.layer.setContents(Some(object(first))) };
        });

        self.images = images;
        self.key_times = key_times;
        self.period = period;
        Ok(())
    }

    /// Puts the layer over `rect`, in the pixels of the window.
    pub(super) fn place(&mut self, rect: Rect) {
        if self.rect == Some(rect) {
            return;
        }
        self.rect = Some(rect);

        let scale = f64::from(Window::screen_scale());
        let x = f64::from(rect.x()) / scale;
        let top = f64::from(rect.y()) / scale;
        let width = f64::from(rect.width()) / scale;
        let height = f64::from(rect.height()) / scale;
        let y = if self.y_down {
            top
        } else {
            self.root.bounds().size.height - top - height
        };

        transaction(|| {
            self.layer.setContentsScale(scale);
            self.layer.setFrame(CGRect::new(CGPoint::new(x, y), CGSize::new(width, height)));
        });
    }

    /// Shows the pictures in a loop, `offset` seconds into it.
    pub(super) fn start(&self, offset: f64) {
        let values: Vec<&AnyObject> = self.images.iter().map(|image| object(image)).collect();
        let values = NSArray::from_slice(&values);
        let times: Vec<Retained<NSNumber>> =
            self.key_times.iter().map(|time| NSNumber::numberWithDouble(*time)).collect();
        let times = NSArray::from_retained_slice(&times);

        let animation = CAKeyframeAnimation::animationWithKeyPath(Some(&NSString::from_str("contents")));
        // SAFETY: the values of a `contents` animation are CGImages.
        unsafe { animation.setValues(Some(&values)) };
        animation.setKeyTimes(Some(&times));
        // SAFETY: a constant of Core Animation.
        animation.setCalculationMode(unsafe { kCAAnimationDiscrete });
        animation.setDuration(self.period);
        animation.setRepeatCount(f32::INFINITY);
        animation.setTimeOffset(offset);
        // Else the system takes it off when the app goes to the background.
        animation.setRemovedOnCompletion(false);

        transaction(|| {
            self.layer.removeAllAnimations();
            self.layer.addAnimation_forKey(&animation, Some(&NSString::from_str(ANIMATION)));
            self.layer.setHidden(false);
        });
    }

    pub(super) fn hide(&self) {
        transaction(|| {
            self.layer.removeAllAnimations();
            self.layer.setHidden(true);
        });
    }

    #[cfg(feature = "ui-tests")]
    pub(super) fn state(&self) -> super::on_layer::LayerState {
        let frame = self.layer.frame();
        let narrow = |value: f64| -> f32 { value.lossy_convert() };
        // SAFETY: the key is a string, the answer an animation or nil.
        let animation = unsafe { self.layer.animationForKey(&NSString::from_str(ANIMATION)) };
        super::on_layer::LayerState {
            frame:    Rect::new(
                narrow(frame.origin.x),
                narrow(frame.origin.y),
                narrow(frame.size.width),
                narrow(frame.size.height),
            ),
            pictures: self.images.len(),
            animated: animation.is_some() && !self.layer.isHidden(),
            flipped:  self.layer.contentsAreFlipped(),
            y_down:   self.y_down,
            builds:   0,
        }
    }
}

impl Layer {
    /// Lets the layer go with the window view, at the stop of a hot build.
    /// A removal would mark the view for a layout, and that layout runs
    /// after the stop, in the class of winit the stop deleted. The hot
    /// reload lane crashed in about half of its runs so.
    #[cfg(hot)]
    pub(super) fn leave_with_view(mut self) {
        self.stays = true;
    }
}

impl Drop for Layer {
    fn drop(&mut self) {
        if !self.stays {
            transaction(|| self.layer.removeFromSuperlayer());
        }
    }
}

/// Changes of layers with no implicit animation, a layer of its own would
/// glide to a new place for a quarter of a second.
fn transaction<T>(change: impl FnOnce() -> T) -> T {
    CATransaction::begin();
    CATransaction::setDisableActions(true);
    let result = change();
    CATransaction::commit();
    result
}

/// A copy of the pixels as a picture of Core Graphics. A copy and not the
/// bytes of the engine, a picture can outlive the loop in the system, and a
/// stopped hot build has no code left to free the bytes.
fn image(
    pixels: &[u8],
    width: usize,
    height: usize,
    space: &CGColorSpace,
    info: CGBitmapInfo,
) -> Result<CFRetained<CGImage>> {
    // SAFETY: the bytes are read here and copied.
    let data = unsafe { CFData::new(None, pixels.as_ptr(), CFIndex::try_from(pixels.len())?) }
        .context("no data for a picture")?;
    let provider = CGDataProvider::with_cf_data(Some(&data)).context("no data provider for a picture")?;
    // SAFETY: the provider holds `height` rows of `width` pixels of 4 bytes.
    unsafe {
        CGImage::new(
            width,
            height,
            8,
            32,
            width * 4,
            Some(space),
            info,
            Some(&provider),
            null(),
            false,
            CGColorRenderingIntent::RenderingIntentDefault,
        )
    }
    .context("no picture")
}

/// A `CGImage` as the object of the runtime it is.
fn object(image: &CGImage) -> &AnyObject {
    // SAFETY: a CGImage is a Core Foundation object, which is an object of
    // the runtime.
    unsafe { &*from_ref(image).cast::<AnyObject>() }
}

/// The `NSView` or the `UIView` of the window.
fn window_view() -> Option<NonNull<AnyObject>> {
    let handle = Window::winit_window()?.window_handle().ok()?;
    match handle.as_raw() {
        #[cfg(macos)]
        RawWindowHandle::AppKit(handle) => Some(handle.ns_view.cast()),
        #[cfg(ios)]
        RawWindowHandle::UiKit(handle) => Some(handle.ui_view.cast()),
        _ => None,
    }
}

/// Whether the y axis of the root layer goes down. The view of winit on a
/// Mac is flipped, its top left corner is its 0 point, and the layer of a
/// view need not be, so the view says where its top lands in the layer.
#[cfg(macos)]
fn y_down(view: &AnyObject, root: &CALayer) -> bool {
    // SAFETY: `convertPointToLayer:` of NSView takes and gives a point.
    let top: CGPoint = unsafe { msg_send![view, convertPointToLayer: CGPoint::new(0.0, 0.0)] };
    top.y < root.bounds().size.height / 2.0
}
