//! `PHPickerViewController`, the photo picker of iOS 14 and later that
//! takes several photos. Like the older picker it runs out of process and
//! needs no photo permission. objc2 binds the controller only on macOS, so
//! it is made by name here, and its class is missing on iOS 12 and 13.

use std::{
    cell::{Cell, RefCell},
    ffi::CStr,
    fs::read,
    path::Path,
    ptr::null,
};

use anyhow::{Result, anyhow};
use block2::RcBlock;
use log::{debug, error, warn};
use objc2::{
    AnyThread, DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send,
    rc::{Allocated, Retained},
    runtime::{AnyClass, AnyObject, NSObject, NSObjectProtocol},
};
use objc2_foundation::{NSArray, NSError, NSInteger, NSItemProvider, NSString, NSURL};
use objc2_photos_ui::{
    PHPickerConfiguration, PHPickerConfigurationAssetRepresentationMode, PHPickerFilter, PHPickerResult,
    PHPickerViewControllerDelegate,
};
use tokio::sync::oneshot::{Receiver, Sender, channel};

use crate::{
    deps::{hreads::on_main, refs::main_lock::MainLock},
    filesystem::picker::{PickedFile, ios::root_view_controller},
};

/// 1 receiver per picked photo, in the order of the pick.
type Loads = Vec<Receiver<Option<PickedFile>>>;
type Answer = Sender<Loads>;

const CONTROLLER: &CStr = c"PHPickerViewController";

/// The types asked of a photo, the first one it has wins. A JPEG and a PNG
/// decode everywhere, `public.image` is whatever else the photo is.
const TYPES: [&str; 3] = ["public.jpeg", "public.png", "public.image"];

/// The picker holds its delegate weakly, so the last one lives here until
/// the next pick replaces it.
static DELEGATE: MainLock<Option<Retained<PhotosDelegate>>> = MainLock::new();

#[cfg(hot)]
pub(super) fn stop() {
    *DELEGATE.get_mut() = None;
}

/// False on iOS 12 and 13, which have only the picker of 1 photo.
pub(super) fn available() -> bool {
    AnyClass::get(CONTROLLER).is_some()
}

/// `max` 0 has no limit.
pub(super) async fn pick(max: usize) -> Vec<PickedFile> {
    let (answer, answered) = channel();

    on_main(move || {
        if let Err(err) = present(answer, max) {
            error!("photo picker failed to open: {err}");
        }
    });

    // A dropped sender, after a failed open or a second pick, is no photo.
    let loads = answered.await.unwrap_or_default();
    let mut picked = Vec::with_capacity(loads.len());
    for load in loads {
        if let Ok(Some(file)) = load.await {
            picked.push(file);
        }
    }
    picked
}

fn present(answer: Answer, max: usize) -> Result<()> {
    let mtm = MainThreadMarker::new().ok_or_else(|| anyhow!("not on the main thread"))?;
    let root = root_view_controller()?;
    let class = AnyClass::get(CONTROLLER).ok_or_else(|| anyhow!("no PHPickerViewController"))?;

    let configuration = unsafe { PHPickerConfiguration::init(PHPickerConfiguration::alloc()) };
    unsafe {
        configuration.setSelectionLimit(NSInteger::try_from(max)?);
        configuration.setFilter(Some(&PHPickerFilter::imagesFilter()));
        // A HEIC photo comes as a JPEG, like from the older picker.
        configuration
            .setPreferredAssetRepresentationMode(PHPickerConfigurationAssetRepresentationMode::Compatible);
    }

    let picker: Retained<AnyObject> = unsafe {
        let allocated: Allocated<AnyObject> = msg_send![class, alloc];
        msg_send![allocated, initWithConfiguration: &*configuration]
    };

    let delegate = PhotosDelegate::new(mtm, answer);
    let object: &AnyObject = &delegate;
    let () = unsafe { msg_send![&picker, setDelegate: object] };
    *DELEGATE.get_mut() = Some(delegate);

    let () = unsafe {
        msg_send![&root, presentViewController: &*picker, animated: true, completion: null::<AnyObject>()]
    };
    Ok(())
}

/// Starts the read of 1 photo. iOS answers on a queue of its own and
/// deletes the file once the handler returns, so it is read in there.
fn load(result: &PHPickerResult) -> Receiver<Option<PickedFile>> {
    let (sender, receiver) = channel();
    let provider: Retained<NSItemProvider> = unsafe { result.itemProvider() };

    let Some(kind) = TYPES
        .iter()
        .map(|kind| NSString::from_str(kind))
        .find(|kind| provider.hasItemConformingToTypeIdentifier(kind))
    else {
        warn!(
            "a picked photo has no image type: {:?}",
            provider.registeredTypeIdentifiers()
        );
        return receiver;
    };
    let name = provider.suggestedName().map(|name| name.to_string());

    let sender = Cell::new(Some(sender));
    let handler = RcBlock::new(move |url: *mut NSURL, err: *mut NSError| {
        let file = unsafe { url.as_ref() }
            .ok_or_else(|| {
                let err = unsafe { err.as_ref() }.map_or_else(|| "no file".to_string(), ToString::to_string);
                anyhow!("{err}")
            })
            .and_then(|url| picked_file(url, name.as_deref()))
            .inspect_err(|err| error!("picked photo failed to read: {err}"))
            .ok();
        if let Some(sender) = sender.take()
            && sender.send(file).is_err()
        {
            warn!("photo read after the caller stopped waiting");
        }
    });
    unsafe { provider.loadFileRepresentationForTypeIdentifier_completionHandler(&kind, &handler) };
    receiver
}

/// The name of the temp file has the extension of what iOS wrote, the
/// suggested name of the photo has none.
fn picked_file(url: &NSURL, suggested: Option<&str>) -> Result<PickedFile> {
    let path = url.path().ok_or_else(|| anyhow!("the file has no path"))?.to_string();
    let path = Path::new(&path);
    let bytes = read(path).map_err(|err| anyhow!("{}: {err}", path.display()))?;
    Ok(PickedFile::new(file_name(path, suggested), None, bytes))
}

fn file_name(path: &Path, suggested: Option<&str>) -> String {
    let extension = path.extension().map(|extension| extension.to_string_lossy());
    match (suggested, extension) {
        (Some(name), Some(extension)) if !name.is_empty() => format!("{name}.{extension}"),
        _ => path.file_name().map_or_else(
            || "image.jpeg".to_string(),
            |name| name.to_string_lossy().into_owned(),
        ),
    }
}

struct Ivars {
    answer: RefCell<Option<Answer>>,
}

define_class!(
    // SAFETY: NSObject has no subclassing requirements and the class has no
    // Drop.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "HilenPhotosPickerDelegate"]
    #[ivars = Ivars]
    struct PhotosDelegate;

    unsafe impl NSObjectProtocol for PhotosDelegate {}

    unsafe impl PHPickerViewControllerDelegate for PhotosDelegate {}

    impl PhotosDelegate {
        /// A cancel is an empty list.
        #[unsafe(method(picker:didFinishPicking:))]
        fn did_finish(&self, picker: &AnyObject, results: &NSArray<PHPickerResult>) {
            let () = unsafe { msg_send![picker, dismissViewControllerAnimated: true, completion: null::<AnyObject>()] };

            debug!("photo picker closed with {} photos", results.count());
            let loads: Loads = results.iter().map(|result| load(&result)).collect();
            if let Some(answer) = self.ivars().answer.borrow_mut().take()
                && answer.send(loads).is_err()
            {
                warn!("photos picked after the caller stopped waiting");
            }
        }
    }
);

impl PhotosDelegate {
    fn new(mtm: MainThreadMarker, answer: Answer) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(Ivars {
            answer: RefCell::new(Some(answer)),
        });
        unsafe { msg_send![super(this), init] }
    }
}
