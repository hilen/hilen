use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    filesystem::PickedFile,
    refs::Weak,
    ui::{Label, Setup, UIManager, ViewData, ViewTest, view},
    ui_test::system_input::wait_until,
};
use web_sys::{
    DataTransfer, DragEvent, DragEventInit, File,
    js_sys::{Array, Uint8Array},
};

/// Files dragged from the desktop onto a page arrive with their names and
/// their bytes, a page gets no path. A drop with no files stays with the
/// browser.
#[view]
struct FileDropBrowser {
    dropped: Vec<PickedFile>,
    holds:   usize,
    ended:   usize,

    #[init]
    title: Label,
}

impl Setup for FileDropBrowser {
    fn setup(mut self: Weak<Self>) {
        self.title.set_text("drop files on the page");
        self.title.place().t(40).lr(20).h(30);
        UIManager::on_drop_bytes(self, move |file| self.dropped.push(file));
        UIManager::on_hover_file(self, move |path| {
            assert!(path.as_os_str().is_empty(), "a page got a path");
            self.holds += 1;
        });
        UIManager::on_hover_file_end(self, move || self.ended += 1);
    }
}

fn data_with_files() -> DataTransfer {
    let data = DataTransfer::new().expect("Failed to make a data transfer");
    for (name, text) in [("LogOutput.log", "a log"), ("shot.png", "a picture")] {
        let parts = Array::new();
        parts.push(&Uint8Array::from(text.as_bytes()));
        let file = File::new_with_u8_array_sequence(&parts, name).expect("Failed to make a file");
        data.items().add_with_file(&file).expect("Failed to add a file");
    }
    data
}

/// False when a listener took the event from the browser.
fn send(name: &str, data: &DataTransfer) -> bool {
    let init = DragEventInit::new();
    init.set_bubbles(true);
    init.set_cancelable(true);
    init.set_data_transfer(Some(data));
    let event = DragEvent::new_with_event_init_dict(name, &init).expect("Failed to make a drag event");
    web_sys::window()
        .expect("Failed to get browser window")
        .document()
        .expect("Failed to get browser document")
        .query_selector("canvas")
        .expect("Failed to query the canvas")
        .expect("No canvas on the page")
        .dispatch_event(&event)
        .expect("Failed to send a drag event")
}

impl ViewTest for FileDropBrowser {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        from_main(move || -> Result<()> {
            let text = DataTransfer::new().expect("Failed to make a data transfer");
            text.set_data("text/plain", "words").expect("Failed to set a text");
            ensure!(
                send("dragover", &text),
                "a drag of a text was taken from the browser"
            );
            ensure!(view.holds == 0, "a drag of a text counts as a hold");

            let data = data_with_files();
            ensure!(!send("dragover", &data), "the browser kept the drag of files");
            ensure!(!send("dragover", &data), "the browser kept the second move");
            ensure!(view.holds == 1, "{} holds for 1 drag", view.holds);
            ensure!(!send("drop", &data), "the browser kept the drop");
            ensure!(view.ended == 1, "the hold ended {} times", view.ended);
            Ok(())
        })?;
        wait_until("the dropped files are read", move || view.dropped.len() == 2)?;
        from_main(move || -> Result<()> {
            let first = &view.dropped[0];
            ensure!(first.name == "LogOutput.log", "the first file is {}", first.name);
            ensure!(first.bytes == b"a log", "the first file holds {:?}", first.bytes);
            ensure!(first.folder.is_empty(), "a lone file lay in {}", first.folder);
            let second = &view.dropped[1];
            ensure!(second.name == "shot.png", "the second file is {}", second.name);
            ensure!(
                second.content_type == "image/png",
                "the second file is {}",
                second.content_type
            );
            Ok(())
        })
    }
}
