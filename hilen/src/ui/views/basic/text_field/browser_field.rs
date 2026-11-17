//! The element a browser edits an engine field through.
//!
//! A canvas takes no text from the system, only keys. While a field is edited,
//! a real `<input>`, or a `<textarea>` for a multiline field, has the focus and
//! lies over it. So the keyboard of a phone, a paste, undo and composed text
//! all come from the browser. The element cannot be seen and takes no touches.
//! The engine still draws the text, the caret and the selection, and hears of
//! every change at once. The 2 are kept equal in both ways, see `sync`.

use std::{cell::Cell, rc::Rc};

use web_sys::{
    ClipboardEvent, Event, File, FocusEvent, FocusOptions, HtmlElement, HtmlInputElement,
    HtmlTextAreaElement, KeyboardEvent,
    js_sys::{Function, Uint8Array},
    wasm_bindgen::{JsCast, closure::Closure},
    window,
};
use zeroize::Zeroizing;

use super::{
    TextField,
    utf16::{byte_of_units, units_of_byte},
};
use crate::{
    deps::refs::{Weak, main_lock::MainLock},
    gm::LossyConvert,
    system::ClipboardImage,
    ui::{Input, Modifiers, NamedKey, ScreenKeyboard, UIManager, view::ViewFrame},
    web::{canvas, canvas_css_origin},
    window::request_frame,
};

/// A smaller text makes Safari on an iPhone zoom the page in on a focus.
const NO_ZOOM_TEXT_SIZE: &str = "16px";
/// In CSS pixels. A smaller loss of the visible page is a bar of the browser
/// that hides itself.
const KEYBOARD_AT_LEAST: f64 = 120.0;
/// A browser does not tell how long the move takes.
const KEYBOARD_SECONDS: f32 = 0.25;

#[derive(Clone)]
enum Editor {
    Line(HtmlInputElement),
    Area(HtmlTextAreaElement),
}

impl Editor {
    fn html(&self) -> &HtmlElement {
        match self {
            Self::Line(element) => element,
            Self::Area(element) => element,
        }
    }

    fn value(&self) -> String {
        match self {
            Self::Line(element) => element.value(),
            Self::Area(element) => element.value(),
        }
    }

    fn set_value(&self, value: &str) {
        match self {
            Self::Line(element) => element.set_value(value),
            Self::Area(element) => element.set_value(value),
        }
    }

    /// The fixed end and the moving end, in UTF-16 units.
    fn selection(&self) -> Option<(u32, u32)> {
        let (start, end, direction) = match self {
            Self::Line(element) => (
                element.selection_start().ok()??,
                element.selection_end().ok()??,
                element.selection_direction().ok()?,
            ),
            Self::Area(element) => (
                element.selection_start().ok()??,
                element.selection_end().ok()??,
                element.selection_direction().ok()?,
            ),
        };
        Some(if direction.as_deref() == Some("backward") {
            (end, start)
        } else {
            (start, end)
        })
    }

    fn set_selection(&self, anchor: u32, caret: u32) {
        let (start, end) = (anchor.min(caret), anchor.max(caret));
        let direction = if caret < anchor { "backward" } else { "forward" };
        let result = match self {
            Self::Line(element) => element.set_selection_range_with_direction(start, end, direction),
            Self::Area(element) => element.set_selection_range_with_direction(start, end, direction),
        };
        if let Err(error) = result {
            log::warn!("Failed to set the selection of a text field in the page: {error:?}");
        }
    }
}

type Listener<E> = Closure<dyn FnMut(E)>;

/// Every listener lives as long as the element, `end_listening` takes them off.
struct Edited {
    editor:          Editor,
    /// A text being composed, the element must not be changed under it.
    composing:       Rc<Cell<bool>>,
    /// An event of the element is at work, the engine must not write back
    /// before it took all of the event in.
    in_event:        Rc<Cell<bool>>,
    on_input:        Listener<Event>,
    on_key:          Listener<KeyboardEvent>,
    on_key_up:       Listener<KeyboardEvent>,
    on_select:       Listener<Event>,
    on_compose:      Listener<Event>,
    on_composed:     Listener<Event>,
    on_paste:        Listener<ClipboardEvent>,
    on_canvas_focus: Listener<FocusEvent>,
}

static EDITED: MainLock<Edited> = MainLock::new();

fn set_style(element: &HtmlElement, name: &str, value: &str) {
    if let Err(error) = element.style().set_property(name, value) {
        log::warn!("Failed to set {name} of a text field in the page: {error:?}");
    }
}

fn focus(element: &HtmlElement) {
    let options = FocusOptions::new();
    // The element stands where the field is, the page itself never scrolls.
    options.set_prevent_scroll(true);
    if let Err(error) = element.focus_with_options(&options) {
        log::warn!("Failed to focus a text field in the page: {error:?}");
    }
}

fn make_editor(field: Weak<TextField>) -> Option<Editor> {
    let document = window()?.document()?;
    let name = if field.multiline { "textarea" } else { "input" };
    let element = document
        .create_element(name)
        .inspect_err(|error| log::error!("Failed to make a text field in the page: {error:?}"))
        .ok()?;
    let editor = if field.multiline {
        Editor::Area(element.dyn_into().ok()?)
    } else {
        let input: HtmlInputElement = element.dyn_into().ok()?;
        input.set_type(if field.is_secure() { "password" } else { "text" });
        Editor::Line(input)
    };

    let html = editor.html();
    for (name, value) in [
        ("autocomplete", "off"),
        ("spellcheck", "false"),
        (
            "inputmode",
            if field.constraint.is_some() {
                "decimal"
            } else {
                "text"
            },
        ),
    ] {
        if let Err(error) = html.set_attribute(name, value) {
            log::warn!("Failed to set {name} of a text field in the page: {error:?}");
        }
    }
    for (name, value) in [
        ("position", "fixed"),
        ("margin", "0"),
        ("padding", "0"),
        ("border", "0"),
        ("outline", "none"),
        ("resize", "none"),
        ("overflow", "hidden"),
        ("opacity", "0"),
        ("color", "transparent"),
        ("background", "transparent"),
        ("caret-color", "transparent"),
        ("font-size", NO_ZOOM_TEXT_SIZE),
        // The canvas takes every touch, it knows where the letters are.
        ("pointer-events", "none"),
    ] {
        set_style(html, name, value);
    }
    if let Err(error) = document.body()?.append_child(html) {
        log::error!("Failed to add a text field to the page: {error:?}");
        return None;
    }
    Some(editor)
}

fn take_text(field: Weak<TextField>, editor: &Editor) {
    let text = Zeroizing::new(editor.value());
    let (anchor, caret) = editor.selection().unwrap_or((0, 0));
    let anchor = byte_of_units(&text, anchor);
    let caret = byte_of_units(&text, caret);
    field.system_text_changed(text);
    field.system_selection_changed(anchor, caret);
}

/// A key the browser must not handle in the element. Letters, Backspace and
/// Delete stay with the element unless the owner of the field takes them, so
/// they reach the engine as a change of the text.
fn engine_key(field: Weak<TextField>, key: &str) -> Option<NamedKey> {
    let named = match key {
        "Enter" => NamedKey::Enter,
        "Escape" => NamedKey::Escape,
        "Tab" => NamedKey::Tab,
        "ArrowLeft" => NamedKey::ArrowLeft,
        "ArrowRight" => NamedKey::ArrowRight,
        "ArrowUp" => NamedKey::ArrowUp,
        "ArrowDown" => NamedKey::ArrowDown,
        "Home" => NamedKey::Home,
        "End" => NamedKey::End,
        "Backspace" if field.takes_key(NamedKey::Backspace) => NamedKey::Backspace,
        "Delete" if field.takes_key(NamedKey::Delete) => NamedKey::Delete,
        _ => return None,
    };
    Some(named)
}

fn set_modifiers(event: &KeyboardEvent) {
    let mut modifiers = Modifiers::empty();
    modifiers.set(Modifiers::SHIFT, event.shift_key());
    modifiers.set(Modifiers::CONTROL, event.ctrl_key());
    modifiers.set(Modifiers::ALT, event.alt_key());
    modifiers.set(Modifiers::META, event.meta_key());
    Input::set_modifiers(modifiers);
}

/// Copy, cut, paste, undo and select all are the work of the element.
fn is_edit_command(key: &str) -> bool {
    matches!(
        key.to_ascii_lowercase().as_str(),
        "a" | "c" | "v" | "x" | "y" | "z"
    )
}

/// Tells `ScreenKeyboard` where the keyboard of a phone is. A browser gives
/// no such event, the keyboard only makes the visible part of the page smaller.
fn watch_keyboard() {
    static WATCH: MainLock<Listener<Event>> = MainLock::new();

    if WATCH.is_set() || !ScreenKeyboard::on_this_device() {
        return;
    }
    let Some(window) = window() else {
        return;
    };
    let Some(viewport) = window.visual_viewport() else {
        log::warn!("This browser has no visual viewport, nothing follows its keyboard");
        return;
    };
    let listener = Closure::<dyn FnMut(Event)>::new({
        let viewport = viewport.clone();
        move |_| {
            let whole = window.inner_height().ok().and_then(|height| height.as_f64()).unwrap_or(0.0);
            let bottom = viewport.offset_top() + viewport.height();
            let ratio = window.device_pixel_ratio();
            let top = (whole - bottom > KEYBOARD_AT_LEAST).then(|| {
                let points = bottom * ratio / f64::from(UIManager::scale());
                points.lossy_convert()
            });
            ScreenKeyboard::moves_to(top, KEYBOARD_SECONDS);
        }
    });
    if let Err(error) = viewport.add_event_listener_with_callback("resize", listener.as_ref().unchecked_ref())
    {
        log::error!("Failed to watch the keyboard of this browser: {error:?}");
    }
    WATCH.set(listener);
}

/// The keys the element gets. A key the engine handles itself, like Escape
/// or a key binding of the screen, goes to the engine, a text key stays with
/// the browser.
fn key_listener(field: Weak<TextField>, composing: Rc<Cell<bool>>) -> Closure<dyn FnMut(KeyboardEvent)> {
    Closure::<dyn FnMut(KeyboardEvent)>::new(move |event: KeyboardEvent| {
        set_modifiers(&event);
        // 229 is a key an input method is busy with.
        if field.is_null() || composing.get() || event.is_composing() || event.key_code() == 229 {
            return;
        }
        let key = event.key();
        if let Some(named) = engine_key(field, &key) {
            event.prevent_default();
            Input::on_key(named);
            request_frame();
        } else if Input::command_held() && !is_edit_command(&key) && key.chars().count() == 1 {
            // A key binding of the screen, like Cmd and S.
            if let Some(ch) = key.chars().next() {
                event.prevent_default();
                Input::on_char(ch, None);
                request_frame();
            }
        }
    })
}

/// A paste with a picture and no text gives the picture to `image_pasted`,
/// as the paste shortcut of a desktop does. The file of a `paste` event can
/// be read without a prompt, the clipboard of the page cannot. A browser hands
/// a copied picture or a screenshot over as a png. Another type is not
/// decoded here, the page links no decoder for it, see `window/image/late.rs`.
fn paste_listener(field: Weak<TextField>) -> Closure<dyn FnMut(ClipboardEvent)> {
    Closure::<dyn FnMut(ClipboardEvent)>::new(move |event: ClipboardEvent| {
        let Some(data) = event.clipboard_data() else {
            return;
        };
        // A text pastes as always, also when a picture comes with it.
        if data.get_data("text/plain").is_ok_and(|text| !text.is_empty()) {
            return;
        }
        let Some(files) = data.files() else {
            return;
        };
        let Some(file) = (0..files.length())
            .filter_map(|index| files.get(index))
            .find(|file| file.type_().starts_with("image/"))
        else {
            return;
        };
        // Nothing to insert, the element would take nothing anyway.
        event.prevent_default();
        if file.type_() != "image/png" {
            log::warn!(
                "A pasted picture of the type {} is not taken, only png is",
                file.type_()
            );
            return;
        }
        wasm_bindgen_futures::spawn_local(read_pasted(field, file));
    })
}

async fn read_pasted(field: Weak<TextField>, file: File) {
    let bytes = match wasm_bindgen_futures::JsFuture::from(file.array_buffer()).await {
        Ok(buffer) => Uint8Array::new(&buffer).to_vec(),
        Err(error) => {
            log::warn!("Failed to read a pasted picture: {error:?}");
            return;
        }
    };
    match ClipboardImage::from_png(bytes) {
        // The field can be gone by now.
        Ok(image) if field.is_ok() => {
            log::debug!("Pasted a picture of {} by {} pixels", image.width, image.height);
            field.image_pasted.trigger(image);
            request_frame();
        }
        Ok(_) => log::debug!("A pasted picture came after its text field was gone"),
        Err(error) => log::warn!("A pasted png did not read: {error}"),
    }
}

/// A caret moved by the system, like after an undo or a word guess.
fn select_listener(
    field: Weak<TextField>,
    editor: Editor,
    composing: Rc<Cell<bool>>,
) -> Closure<dyn FnMut(Event)> {
    Closure::<dyn FnMut(Event)>::new(move |_| {
        if field.is_null() || composing.get() {
            return;
        }
        let value = Zeroizing::new(editor.value());
        if value.as_str() != field.text() {
            return;
        }
        if let Some((anchor, caret)) = editor.selection() {
            field.system_selection_changed(byte_of_units(&value, anchor), byte_of_units(&value, caret));
            request_frame();
        }
    })
}

pub(super) fn begin(field: Weak<TextField>) {
    watch_keyboard();
    end_listening();

    let Some(editor) = make_editor(field) else {
        return;
    };
    let composing = Rc::new(Cell::new(false));
    let in_event = Rc::new(Cell::new(false));

    let on_input = Closure::<dyn FnMut(Event)>::new({
        let editor = editor.clone();
        let composing = composing.clone();
        let in_event = in_event.clone();
        move |_| {
            if field.is_null() {
                return;
            }
            in_event.set(true);
            take_text(field, &editor);
            in_event.set(false);
            if !composing.get() {
                sync(&field);
            }
            request_frame();
        }
    });

    let on_key = key_listener(field, composing.clone());

    let on_key_up = Closure::<dyn FnMut(KeyboardEvent)>::new(|event: KeyboardEvent| set_modifiers(&event));

    let on_select = select_listener(field, editor.clone(), composing.clone());

    let on_paste = paste_listener(field);

    let on_compose = Closure::<dyn FnMut(Event)>::new({
        let composing = composing.clone();
        move |_| composing.set(true)
    });

    let on_composed = Closure::<dyn FnMut(Event)>::new({
        let editor = editor.clone();
        let composing = composing.clone();
        let in_event = in_event.clone();
        move |_| {
            composing.set(false);
            if field.is_null() {
                return;
            }
            in_event.set(true);
            take_text(field, &editor);
            in_event.set(false);
            sync(&field);
            request_frame();
        }
    });

    // winit focuses the canvas on every press. The focus comes back right in
    // that event, a phone opens its keyboard only inside a user gesture.
    // Firefox names no next element on a blur, so this listens to the
    // canvas.
    let on_canvas_focus = Closure::<dyn FnMut(FocusEvent)>::new({
        let editor = editor.clone();
        move |_: FocusEvent| {
            if EDITED.is_set() {
                focus(editor.html());
            }
        }
    });
    if let Some(canvas) = canvas()
        && let Err(error) =
            canvas.add_event_listener_with_callback("focus", on_canvas_focus.as_ref().unchecked_ref())
    {
        log::error!("Failed to listen to the focus of the canvas: {error:?}");
    }

    let html = editor.html();
    let add = |name: &str, listener: &Function| {
        if let Err(error) = html.add_event_listener_with_callback(name, listener) {
            log::error!("Failed to listen to {name} of a text field in the page: {error:?}");
        }
    };
    add("input", on_input.as_ref().unchecked_ref());
    add("keydown", on_key.as_ref().unchecked_ref());
    add("keyup", on_key_up.as_ref().unchecked_ref());
    add("select", on_select.as_ref().unchecked_ref());
    add("compositionstart", on_compose.as_ref().unchecked_ref());
    add("compositionend", on_composed.as_ref().unchecked_ref());
    add("paste", on_paste.as_ref().unchecked_ref());
    if let Some(document) = window().and_then(|window| window.document())
        && let Err(error) =
            document.add_event_listener_with_callback("selectionchange", on_select.as_ref().unchecked_ref())
    {
        log::error!("Failed to listen to selectionchange of the page: {error:?}");
    }

    EDITED.set(Edited {
        editor,
        composing,
        in_event,
        on_input,
        on_key,
        on_key_up,
        on_select,
        on_compose,
        on_composed,
        on_paste,
        on_canvas_focus,
    });

    follow(&field);
    sync(&field);
    grab_focus();
    log::debug!("A text field is edited through the page");
}

/// Takes every listener off and the element out of the page.
fn end_listening() {
    let Some(edited) = EDITED.try_get_mut() else {
        return;
    };
    let html = edited.editor.html();
    for (name, listener) in [
        ("input", edited.on_input.as_ref().unchecked_ref()),
        ("keydown", edited.on_key.as_ref().unchecked_ref()),
        ("keyup", edited.on_key_up.as_ref().unchecked_ref()),
        ("select", edited.on_select.as_ref().unchecked_ref()),
        ("compositionstart", edited.on_compose.as_ref().unchecked_ref()),
        ("compositionend", edited.on_composed.as_ref().unchecked_ref()),
        ("paste", edited.on_paste.as_ref().unchecked_ref()),
    ] {
        if let Err(error) = html.remove_event_listener_with_callback(name, listener) {
            log::warn!("Failed to stop listening to {name} of a text field in the page: {error:?}");
        }
    }
    if let Some(document) = window().and_then(|window| window.document())
        && let Err(error) = document
            .remove_event_listener_with_callback("selectionchange", edited.on_select.as_ref().unchecked_ref())
    {
        log::warn!("Failed to stop listening to selectionchange of the page: {error:?}");
    }
    if let Some(canvas) = canvas()
        && let Err(error) = canvas
            .remove_event_listener_with_callback("focus", edited.on_canvas_focus.as_ref().unchecked_ref())
    {
        log::warn!("Failed to stop listening to the focus of the canvas: {error:?}");
    }
    html.remove();
    EDITED.clear();
}

pub(super) fn end() {
    end_listening();
    // The keys go back to the engine.
    if let Some(canvas) = canvas() {
        focus(&canvas);
    }
    Input::set_modifiers(Modifiers::empty());
}

/// Puts the focus back on the element. A press on the field ends with it, a
/// phone opens its keyboard only inside a user gesture.
pub(super) fn grab_focus() {
    if let Some(edited) = EDITED.try_get_mut() {
        focus(edited.editor.html());
    }
}

/// Keeps the element over the field, which can move while it is edited.
pub(super) fn follow(field: &TextField) {
    let Some(edited) = EDITED.try_get_mut() else {
        return;
    };
    let frame = field.absolute_frame();
    let (left, top, ratio) = canvas_css_origin();
    let css = f64::from(UIManager::scale()) / ratio;
    let html = edited.editor.html();
    for (name, value) in [
        ("left", left + f64::from(frame.x()) * css),
        ("top", top + f64::from(frame.y()) * css),
        ("width", f64::from(frame.width()) * css),
        ("height", f64::from(frame.height()) * css),
    ] {
        set_style(html, name, &format!("{value:.2}px"));
    }
}

/// Makes the element hold what the engine holds. It writes only what differs,
/// a new value wipes the undo list of the element.
pub(super) fn sync(field: &TextField) {
    let Some(edited) = EDITED.try_get_mut() else {
        return;
    };
    if edited.composing.get() || edited.in_event.get() {
        return;
    }

    let text = field.text();
    let value = Zeroizing::new(edited.editor.value());
    if value.as_str() != text {
        edited.editor.set_value(text);
    }
    let anchor = units_of_byte(text, field.anchor.unwrap_or(field.caret));
    let caret = units_of_byte(text, field.caret);
    if edited.editor.selection() != Some((anchor, caret)) {
        edited.editor.set_selection(anchor, caret);
    }
}
