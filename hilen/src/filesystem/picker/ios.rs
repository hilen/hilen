//! `UIImagePickerController` on its default source, the photo library. It
//! runs out of process, so it needs no photo permission, and it exists on
//! every iOS the engine runs on. `PHPickerViewController` starts at iOS 14.

use std::{cell::RefCell, fs::read, path::Path};

use anyhow::{Result, anyhow};
use log::{error, warn};
use objc2::{
    DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send,
    rc::Retained,
    runtime::{AnyObject, NSObject, NSObjectProtocol},
};
use objc2_foundation::{NSDictionary, NSURL};
use objc2_ui_kit::{
    UIImage, UIImagePickerController, UIImagePickerControllerDelegate, UIImagePickerControllerImageURL,
    UIImagePickerControllerInfoKey, UIImagePickerControllerOriginalImage, UINavigationControllerDelegate,
    UIViewController,
};
use tokio::sync::oneshot::{Sender, channel};
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

use crate::{
    deps::{hreads::on_main, refs::main_lock::MainLock},
    filesystem::picker::PickedFile,
    window::Window,
};

type Answer = Sender<Option<PickedFile>>;

/// The picker holds its delegate weakly, so the last one lives here until
/// the next pick replaces it.
static DELEGATE: MainLock<Option<Retained<PickerDelegate>>> = MainLock::new();

/// Lets go of the delegate, so its class can be deleted when a hot build
/// stops.
#[cfg(hot)]
pub(crate) fn stop() {
    *DELEGATE.get_mut() = None;
}

pub(super) async fn pick() -> Option<PickedFile> {
    let (answer, answered) = channel();

    on_main(move || {
        if let Err(err) = present(answer) {
            error!("image picker failed to open: {err}");
        }
    });

    // A dropped sender, after a failed open or a second pick, is no image.
    answered.await.ok().flatten()
}

fn present(answer: Answer) -> Result<()> {
    let mtm = MainThreadMarker::new().ok_or_else(|| anyhow!("not on the main thread"))?;
    let root = root_view_controller()?;

    let picker = UIImagePickerController::new(mtm);

    let delegate = PickerDelegate::new(mtm, answer);
    let object: &AnyObject = &delegate;
    unsafe { picker.setDelegate(Some(object)) };
    *DELEGATE.get_mut() = Some(delegate);

    root.presentViewController_animated_completion(&picker, true, None);
    Ok(())
}

/// The controller winit made for the window, the engine has no other.
pub(super) fn root_view_controller() -> Result<Retained<UIViewController>> {
    let window = Window::winit_window().ok_or_else(|| anyhow!("no window"))?;
    let handle = window.window_handle()?;

    let RawWindowHandle::UiKit(handle) = handle.as_raw() else {
        return Err(anyhow!("the window is not a UIKit window"));
    };
    let controller = handle
        .ui_view_controller
        .ok_or_else(|| anyhow!("the window has no view controller"))?;

    unsafe { Retained::retain(controller.as_ptr().cast::<UIViewController>()) }
        .ok_or_else(|| anyhow!("the view controller is null"))
}

/// The file iOS writes for the pick, a JPEG export of the photo. A photo
/// with no such file still has the image itself, sent as a PNG.
fn picked_file(info: &NSDictionary<UIImagePickerControllerInfoKey, AnyObject>) -> Result<PickedFile> {
    if let Some(path) = info
        .objectForKey(unsafe { UIImagePickerControllerImageURL })
        .and_then(|url| url.downcast::<NSURL>().ok())
        .and_then(|url| url.path())
    {
        let path = path.to_string();
        let name = Path::new(&path).file_name().map_or_else(
            || "image.jpeg".to_string(),
            |name| name.to_string_lossy().into_owned(),
        );
        let bytes = read(&path).map_err(|err| anyhow!("{path}: {err}"))?;
        return Ok(PickedFile::new(name, None, bytes));
    }

    let image = info
        .objectForKey(unsafe { UIImagePickerControllerOriginalImage })
        .and_then(|image| image.downcast::<UIImage>().ok())
        .ok_or_else(|| anyhow!("the pick has neither a file nor an image"))?;
    let png = image.png_representation().ok_or_else(|| anyhow!("the image has no PNG form"))?;

    Ok(PickedFile::new(
        "image.png".to_string(),
        Some("image/png".to_string()),
        png.to_vec(),
    ))
}

struct Ivars {
    answer: RefCell<Option<Answer>>,
}

define_class!(
    // SAFETY: NSObject has no subclassing requirements and the class has no
    // Drop.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "HilenImagePickerDelegate"]
    #[ivars = Ivars]
    struct PickerDelegate;

    unsafe impl NSObjectProtocol for PickerDelegate {}

    unsafe impl UINavigationControllerDelegate for PickerDelegate {}

    unsafe impl UIImagePickerControllerDelegate for PickerDelegate {
        #[unsafe(method(imagePickerController:didFinishPickingMediaWithInfo:))]
        fn did_finish(
            &self,
            picker: &UIImagePickerController,
            info: &NSDictionary<UIImagePickerControllerInfoKey, AnyObject>,
        ) {
            let picked = picked_file(info)
                .inspect_err(|err| error!("picked image failed to read: {err}"))
                .ok();
            self.close(picker, picked);
        }

        #[unsafe(method(imagePickerControllerDidCancel:))]
        fn did_cancel(&self, picker: &UIImagePickerController) {
            self.close(picker, None);
        }
    }
);

impl PickerDelegate {
    fn new(mtm: MainThreadMarker, answer: Answer) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(Ivars {
            answer: RefCell::new(Some(answer)),
        });
        unsafe { msg_send![super(this), init] }
    }

    fn close(&self, picker: &UIImagePickerController, file: Option<PickedFile>) {
        picker.dismissViewControllerAnimated_completion(true, None);

        if let Some(answer) = self.ivars().answer.borrow_mut().take()
            && answer.send(file).is_err()
        {
            warn!("image picked after the caller stopped waiting");
        }
    }
}
