use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{Label, Setup, TextField, UIManager, ViewData, ViewTest, view},
};
use web_sys::{
    CompositionEvent, CompositionEventInit, Document, Event, HtmlElement, HtmlInputElement,
    HtmlTextAreaElement, KeyboardEvent, KeyboardEventInit, wasm_bindgen::JsCast,
};

/// A field in a browser edits through a real element of the page. So what
/// only a browser can do reaches the engine: a paste, an undo and composed
/// text, and a phone can open its keyboard. `execCommand` puts text in the
/// way a paste and a keyboard do, with a real `input` event.
#[view]
struct TextFieldBrowser {
    #[init]
    title: Label,
    line:  TextField,
    area:  TextField,
}

impl Setup for TextFieldBrowser {
    fn setup(self: Weak<Self>) {
        self.title.set_text("text fields edited through the page");
        self.title.place().t(40).lr(20).h(30);
        self.line.set_text_size(24);
        self.line.place().t(90).lr(20).h(50);
        self.area.set_multiline(true).set_text_size(24);
        self.area.place().t(160).lr(20).h(160);
    }
}

fn document() -> Document {
    web_sys::window()
        .expect("Failed to get browser window")
        .document()
        .expect("Failed to get browser document")
}

fn active() -> HtmlElement {
    document()
        .active_element()
        .expect("Nothing has the focus")
        .dyn_into()
        .expect("The focus is on no html element")
}

fn active_tag() -> String {
    document()
        .active_element()
        .map_or_else(String::new, |element| element.tag_name().to_lowercase())
}

/// What a paste or a key of a phone does to the focused element.
fn put_in(text: &str) {
    let html: web_sys::HtmlDocument = document().dyn_into().expect("The document is no html document");
    assert!(
        html.exec_command_with_show_ui_and_value("insertText", false, text)
            .expect("insertText failed"),
        "the browser refused insertText"
    );
}

fn command(name: &str) {
    let html: web_sys::HtmlDocument = document().dyn_into().expect("The document is no html document");
    html.exec_command(name).expect("the edit command failed");
}

fn key(name: &str, shift: bool) -> bool {
    let init = KeyboardEventInit::new();
    init.set_bubbles(true);
    init.set_cancelable(true);
    init.set_key(name);
    init.set_shift_key(shift);
    let event =
        KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init).expect("Failed to build a key");
    active().dispatch_event(&event).expect("Failed to send a key")
}

fn compose(name: &str) {
    let init = CompositionEventInit::new();
    init.set_bubbles(true);
    let event =
        CompositionEvent::new_with_event_init_dict(name, &init).expect("Failed to build a composition");
    active().dispatch_event(&event).expect("Failed to send a composition");
}

/// The element takes composing text with no end of the composition.
fn composing_value(value: &str) {
    let element = active();
    match element.dyn_ref::<HtmlInputElement>() {
        Some(input) => input.set_value(value),
        None => element
            .dyn_ref::<HtmlTextAreaElement>()
            .expect("The focus is on no text element")
            .set_value(value),
    }
    let event = Event::new("input").expect("Failed to build an input event");
    element.dispatch_event(&event).expect("Failed to send an input event");
}

fn selection() -> (u32, u32) {
    let element = active();
    if let Some(input) = element.dyn_ref::<HtmlInputElement>() {
        return (
            input.selection_start().expect("no start").expect("no start"),
            input.selection_end().expect("no end").expect("no end"),
        );
    }
    let area = element
        .dyn_ref::<HtmlTextAreaElement>()
        .expect("The focus is on no text element");
    (
        area.selection_start().expect("no start").expect("no start"),
        area.selection_end().expect("no end").expect("no end"),
    )
}

impl ViewTest for TextFieldBrowser {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        from_main(move || -> Result<()> {
            view.line.focus_with_keyboard();
            ensure!(view.line.is_editing(), "the field is not edited");
            ensure!(active_tag() == "input", "the focus is on {}", active_tag());

            put_in("Riga");
            ensure!(view.line.text() == "Riga", "a paste gave {:?}", view.line.text());
            ensure!(
                view.line.caret() == 4,
                "the caret after a paste is {}",
                view.line.caret()
            );

            // A place in bytes is not a place of the browser.
            put_in(" 日本😀");
            ensure!(view.line.text() == "Riga 日本😀", "gave {:?}", view.line.text());
            ensure!(
                view.line.caret() == "Riga 日本😀".len(),
                "the caret is {}",
                view.line.caret()
            );

            ensure!(!key("ArrowLeft", false), "the browser kept the arrow");
            ensure!(
                view.line.caret() == "Riga 日本".len(),
                "the arrow gave {}",
                view.line.caret()
            );
            ensure!(
                selection() == (7, 7),
                "the element holds the caret at {:?}",
                selection()
            );

            ensure!(!key("ArrowLeft", true), "the browser kept the shift arrow");
            ensure!(
                view.line.selected_text() == "本",
                "selected {:?}",
                view.line.selected_text()
            );
            ensure!(selection() == (6, 7), "the element selects {:?}", selection());

            put_in("x");
            ensure!(
                view.line.text() == "Riga 日x😀",
                "a paste over a selection gave {:?}",
                view.line.text()
            );

            command("undo");
            ensure!(
                view.line.text() == "Riga 日本😀",
                "undo gave {:?}",
                view.line.text()
            );

            // The element must not be changed while a text is composed.
            view.line.set_text("");
            compose("compositionstart");
            composing_value("に");
            ensure!(view.line.text() == "に", "composing gave {:?}", view.line.text());
            ensure!(
                key("Enter", false),
                "a key of the input method went to the engine"
            );
            ensure!(view.line.is_editing(), "composing ended the editing");
            composing_value("日");
            compose("compositionend");
            ensure!(view.line.text() == "日", "composed {:?}", view.line.text());

            // A press on the canvas takes the focus, it comes back. A browser
            // whose window has no focus of the system, under a bare Xvfb, may
            // fire no focus event, there the step says nothing.
            if document().has_focus().unwrap_or(false) {
                document()
                    .query_selector("canvas")
                    .expect("Failed to query the canvas")
                    .expect("No canvas on the page")
                    .dyn_into::<HtmlElement>()
                    .expect("The canvas is no html element")
                    .focus()
                    .expect("Failed to focus the canvas");
                ensure!(active_tag() == "input", "the focus stayed on {}", active_tag());
            } else {
                log::warn!("the window has no focus, the way back from the canvas is not checked");
            }

            // The code of the app changes the text under the element.
            view.line.set_text("set by code");
            let value = active().dyn_into::<HtmlInputElement>().expect("no input").value();
            ensure!(value == "set by code", "the element holds {value:?}");

            ensure!(!key("Tab", false), "the browser kept Tab");
            ensure!(view.area.is_editing(), "Tab did not move on");
            ensure!(active_tag() == "textarea", "the focus is on {}", active_tag());

            put_in("one");
            ensure!(!key("Enter", false), "the browser kept Enter");
            put_in("two");
            ensure!(
                view.area.text() == "one\ntwo",
                "the area holds {:?}",
                view.area.text()
            );
            ensure!(!key("ArrowUp", false), "the browser kept the arrow");
            ensure!(view.area.caret() == 3, "up gave {}", view.area.caret());

            ensure!(!key("Escape", false), "the browser kept Escape");
            ensure!(!view.area.is_editing(), "Escape did not end the editing");
            ensure!(active_tag() == "canvas", "the focus is on {}", active_tag());
            ensure!(
                document().query_selector("input, textarea").expect("query").is_none(),
                "an element stayed in the page"
            );
            UIManager::unselect_view();
            Ok(())
        })
    }
}
