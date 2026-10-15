//! The save sheet of iOS, `UIDocumentPickerViewController` made to
//! export. The bytes go into a file in the temp folder of the app first,
//! the sheet copies that file to where the user says, in Files or in
//! another app. The copy in the temp folder is deleted after.

use std::{
    cell::RefCell,
    env::temp_dir,
    fs::{remove_file, write},
    path::{Path, PathBuf},
};

use anyhow::{Result, anyhow};
use log::{error, warn};
use objc2::{
    DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send,
    rc::Retained,
    runtime::{AnyClass, NSObject, NSObjectProtocol, ProtocolObject},
};
use objc2_foundation::{NSArray, NSString, NSURL};
use objc2_ui_kit::{UIDocumentPickerDelegate, UIDocumentPickerViewController};
use tokio::sync::oneshot::{Sender, channel};

use crate::{
    deps::{hreads::on_main, refs::main_lock::MainLock},
    filesystem::picker::{ios::root_view_controller, save::SavedFile},
};

type Answer = Sender<Option<SavedFile>>;

/// `UIDocumentPickerModeExportToService`, the mode that copies a file out.
const EXPORT_MODE: usize = 2;

/// The sheet holds its delegate weakly, so the last one lives here until
/// the next save replaces it.
static DELEGATE: MainLock<Option<Retained<SaveDelegate>>> = MainLock::new();

/// Lets go of the delegate, so its class can be deleted when a hot build
/// stops.
#[cfg(hot)]
pub(super) fn stop() {
    *DELEGATE.get_mut() = None;
}

pub(super) async fn save(name: &str, bytes: &[u8]) -> Option<SavedFile> {
    let path = temp_dir().join(name);
    if let Err(err) = write(&path, bytes) {
        error!(
            "file {} failed to write for the save sheet: {err}",
            path.display()
        );
        return None;
    }

    let (answer, answered) = channel();
    let shown = path.clone();
    on_main(move || {
        if let Err(err) = present(&shown, answer) {
            error!("save sheet failed to open: {err}");
        }
    });

    // A dropped sender, after a failed open or a second save, is no file.
    let saved = answered.await.ok().flatten();

    if let Err(err) = remove_file(&path) {
        warn!("the copy of the saved file {} stays: {err}", path.display());
    }
    saved
}

fn present(path: &Path, answer: Answer) -> Result<()> {
    let mtm = MainThreadMarker::new().ok_or_else(|| anyhow!("not on the main thread"))?;
    let root = root_view_controller()?;

    let path = path
        .to_str()
        .ok_or_else(|| anyhow!("the path {} is not text", path.display()))?;
    let url = NSURL::fileURLWithPath(&NSString::from_str(path));
    let sheet = sheet(mtm, &url)?;

    let delegate = SaveDelegate::new(mtm, answer);
    sheet.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
    *DELEGATE.get_mut() = Some(delegate);

    root.presentViewController_animated_completion(&sheet, true, None);
    Ok(())
}

/// Both initializers go out as plain messages. The one for many files
/// starts at iOS 14, the one before it is marked deprecated.
fn sheet(mtm: MainThreadMarker, url: &NSURL) -> Result<Retained<UIDocumentPickerViewController>> {
    // `UTType` came with iOS 14 too, its class tells the version.
    let sheet: Option<Retained<UIDocumentPickerViewController>> = if AnyClass::get(c"UTType").is_some() {
        let urls = NSArray::from_slice(&[url]);
        unsafe {
            msg_send![
                UIDocumentPickerViewController::alloc(mtm),
                initForExportingURLs: &*urls,
                asCopy: true,
            ]
        }
    } else {
        unsafe {
            msg_send![
                UIDocumentPickerViewController::alloc(mtm),
                initWithURL: url,
                inMode: EXPORT_MODE,
            ]
        }
    };

    sheet.ok_or_else(|| anyhow!("the save sheet is null"))
}

/// The name of the file where it was saved.
fn saved_file(urls: &NSArray<NSURL>) -> Result<SavedFile> {
    let path = urls
        .firstObject()
        .and_then(|url| url.path())
        .ok_or_else(|| anyhow!("the save has no file path"))?
        .to_string();
    let name = PathBuf::from(&path)
        .file_name()
        .ok_or_else(|| anyhow!("{path} has no file name"))?
        .to_string_lossy()
        .into_owned();
    Ok(SavedFile { name, path: None })
}

struct Ivars {
    answer: RefCell<Option<Answer>>,
}

define_class!(
    // SAFETY: NSObject has no subclassing requirements and the class has no
    // Drop.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "HilenDocumentSaveDelegate"]
    #[ivars = Ivars]
    struct SaveDelegate;

    unsafe impl NSObjectProtocol for SaveDelegate {}

    // The sheet closes by itself after both answers.
    unsafe impl UIDocumentPickerDelegate for SaveDelegate {
        #[unsafe(method(documentPicker:didPickDocumentsAtURLs:))]
        fn did_pick(&self, _: &UIDocumentPickerViewController, urls: &NSArray<NSURL>) {
            let saved = saved_file(urls).inspect_err(|err| error!("saved file has no name: {err}")).ok();
            self.answer(saved);
        }

        #[unsafe(method(documentPickerWasCancelled:))]
        fn was_cancelled(&self, _: &UIDocumentPickerViewController) {
            self.answer(None);
        }
    }
);

impl SaveDelegate {
    fn new(mtm: MainThreadMarker, answer: Answer) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(Ivars {
            answer: RefCell::new(Some(answer)),
        });
        unsafe { msg_send![super(this), init] }
    }

    fn answer(&self, file: Option<SavedFile>) {
        if let Some(answer) = self.ivars().answer.borrow_mut().take()
            && answer.send(file).is_err()
        {
            warn!("file saved after the caller stopped waiting");
        }
    }
}
