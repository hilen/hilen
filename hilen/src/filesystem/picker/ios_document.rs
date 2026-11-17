//! `UIDocumentPickerViewController`, the Files browser of iOS. It hands
//! over a copy of the picked file in the temp folder of the app, so the
//! read needs no security scope and no permission.

use std::{
    cell::RefCell,
    fs::{read, remove_file},
    path::Path,
};

use anyhow::{Result, anyhow};
use log::{error, warn};
use objc2::{
    DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send,
    rc::Retained,
    runtime::{AnyClass, AnyObject, NSObject, NSObjectProtocol, ProtocolObject},
};
use objc2_foundation::{NSArray, NSString, NSURL};
use objc2_ui_kit::{UIDocumentPickerDelegate, UIDocumentPickerViewController};
use tokio::sync::oneshot::{Sender, channel};

use crate::{
    deps::{hreads::on_main, refs::main_lock::MainLock},
    filesystem::picker::{PickedFile, ios::root_view_controller},
};

type Answer = Sender<Option<PickedFile>>;

/// The type every file and folder conforms to.
const ANY_FILE: &str = "public.item";

/// `UIDocumentPickerModeImport`, the mode that copies the picked file.
const IMPORT_MODE: usize = 0;

/// The picker holds its delegate weakly, so the last one lives here until
/// the next pick replaces it.
static DELEGATE: MainLock<Option<Retained<PickerDelegate>>> = MainLock::new();

/// Lets go of the delegate, so its class can be deleted when a hot build
/// stops.
#[cfg(hot)]
pub(super) fn stop() {
    *DELEGATE.get_mut() = None;
}

pub(super) async fn pick(extensions: Vec<String>) -> Option<PickedFile> {
    let (answer, answered) = channel();

    on_main(move || {
        if let Err(err) = present(&extensions, answer) {
            error!("file picker failed to open: {err}");
        }
    });

    // A dropped sender, after a failed open or a second pick, is no file.
    answered.await.ok().flatten()
}

fn present(extensions: &[String], answer: Answer) -> Result<()> {
    let mtm = MainThreadMarker::new().ok_or_else(|| anyhow!("not on the main thread"))?;
    let root = root_view_controller()?;

    let picker = picker(mtm, extensions)?;

    let delegate = PickerDelegate::new(mtm, answer);
    picker.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
    *DELEGATE.get_mut() = Some(delegate);

    root.presentViewController_animated_completion(&picker, true, None);
    Ok(())
}

/// Both initializers go out as plain messages. The one with `UTType`
/// starts at iOS 14 and its binding needs a framework the older systems
/// do not have, the one before it is marked deprecated.
fn picker(mtm: MainThreadMarker, extensions: &[String]) -> Result<Retained<UIDocumentPickerViewController>> {
    let picker: Option<Retained<UIDocumentPickerViewController>> =
        if let Some(uttype) = AnyClass::get(c"UTType") {
            let types = content_types(uttype, extensions)?;
            unsafe {
                msg_send![
                    UIDocumentPickerViewController::alloc(mtm),
                    initForOpeningContentTypes: &*types,
                    asCopy: true,
                ]
            }
        } else {
            // iOS 12 and 13 have no call that turns an extension into a
            // type without one more framework, so they show every file.
            let types = NSArray::from_retained_slice(&[NSString::from_str(ANY_FILE)]);
            unsafe {
                msg_send![
                    UIDocumentPickerViewController::alloc(mtm),
                    initWithDocumentTypes: &*types,
                    inMode: IMPORT_MODE,
                ]
            }
        };

    picker.ok_or_else(|| anyhow!("the document picker is null"))
}

/// The `UTType` of every extension. No extension, or one the system makes
/// no type for, gives the type of every file.
fn content_types(uttype: &AnyClass, extensions: &[String]) -> Result<Retained<NSArray<AnyObject>>> {
    let mut types = Vec::with_capacity(extensions.len());

    for extension in extensions {
        let extension = NSString::from_str(extension);
        let found: Option<Retained<AnyObject>> =
            unsafe { msg_send![uttype, typeWithFilenameExtension: &*extension] };
        let Some(found) = found else {
            types.clear();
            break;
        };
        types.push(found);
    }

    if types.is_empty() {
        let identifier = NSString::from_str(ANY_FILE);
        let any: Option<Retained<AnyObject>> = unsafe { msg_send![uttype, typeWithIdentifier: &*identifier] };
        types.push(any.ok_or_else(|| anyhow!("no type for {ANY_FILE}"))?);
    }

    Ok(NSArray::from_retained_slice(&types))
}

fn picked_file(urls: &NSArray<NSURL>) -> Result<PickedFile> {
    let path = urls
        .firstObject()
        .and_then(|url| url.path())
        .ok_or_else(|| anyhow!("the pick has no file path"))?
        .to_string();
    let name = Path::new(&path)
        .file_name()
        .ok_or_else(|| anyhow!("{path} has no file name"))?
        .to_string_lossy()
        .into_owned();

    let bytes = read(&path).map_err(|err| anyhow!("{path}: {err}"))?;

    // The copy is only for this read, and the temp folder is emptied by
    // the system at a time nobody knows.
    if let Err(err) = remove_file(&path) {
        warn!("the copy of the picked file {path} stays: {err}");
    }

    Ok(PickedFile::new(name, None, bytes))
}

struct Ivars {
    answer: RefCell<Option<Answer>>,
}

define_class!(
    // SAFETY: NSObject has no subclassing requirements and the class has no
    // Drop.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "HilenDocumentPickerDelegate"]
    #[ivars = Ivars]
    struct PickerDelegate;

    unsafe impl NSObjectProtocol for PickerDelegate {}

    // The picker closes by itself after both answers.
    unsafe impl UIDocumentPickerDelegate for PickerDelegate {
        #[unsafe(method(documentPicker:didPickDocumentsAtURLs:))]
        fn did_pick(&self, _: &UIDocumentPickerViewController, urls: &NSArray<NSURL>) {
            let picked = picked_file(urls)
                .inspect_err(|err| error!("picked file failed to read: {err}"))
                .ok();
            self.answer(picked);
        }

        #[unsafe(method(documentPickerWasCancelled:))]
        fn was_cancelled(&self, _: &UIDocumentPickerViewController) {
            self.answer(None);
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

    fn answer(&self, file: Option<PickedFile>) {
        if let Some(answer) = self.ivars().answer.borrow_mut().take()
            && answer.send(file).is_err()
        {
            warn!("file picked after the caller stopped waiting");
        }
    }
}
