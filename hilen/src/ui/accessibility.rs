//! Screen reader support. The UI is drawn on a canvas, which a screen reader
//! can not read, so the visible views a user reads or acts on are listed as
//! nodes with a role and a label. In a browser the nodes become a hidden DOM
//! layer over the canvas, kept in step 3 times a second: a button, a text
//! box, a check box, a switch, a slider or a text, each at the place of its
//! view. A screen reader reads them in the order of the view tree, and its
//! click on 1 of them taps the view. `Accessibility::announce` speaks a
//! message through an `aria-live` region, like a new alert. On other
//! platforms the nodes are only there for tests and tools, a native screen
//! reader is not wired yet.

use crate::{
    gm::flat::Rect,
    ui::{
        Button, CheckBox, Label, Slider, Switch, TextField, ToolButton, UIManager, View, ViewData, ViewFrame,
        ViewSubviews, WeakView,
    },
};

/// What a node is to a screen reader.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessibleRole {
    Button,
    TextBox,
    CheckBox,
    Switch,
    Slider,
    Text,
}

impl AccessibleRole {
    /// The ARIA role of the DOM element in a browser.
    pub fn aria_role(self) -> &'static str {
        match self {
            Self::Button => "button",
            Self::TextBox => "textbox",
            Self::CheckBox => "checkbox",
            Self::Switch => "switch",
            Self::Slider => "slider",
            Self::Text => "note",
        }
    }
}

/// 1 view as a screen reader gets it.
#[derive(Clone, PartialEq)]
pub struct AccessibleNode {
    pub role:        AccessibleRole,
    pub label:       String,
    /// The text of a text box or the value of a slider.
    pub value:       Option<String>,
    /// On or off for a check box, a switch or a toggle button.
    pub checked:     Option<bool>,
    /// In window points.
    pub frame:       Rect,
    pub(crate) view: WeakView,
}

impl std::fmt::Debug for AccessibleNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AccessibleNode")
            .field("role", &self.role)
            .field("label", &self.label)
            .field("value", &self.value)
            .field("checked", &self.checked)
            .field("frame", &self.frame)
            .finish_non_exhaustive()
    }
}

/// The label a screen reader says for a view, for a view whose meaning is
/// not in a text, like a button with only an icon, or a view of the app
/// that takes taps. A view with a label and no role of its own is read as a
/// button.
pub trait ViewAccessibility {
    fn set_accessibility_label(&self, label: impl ToString) -> &Self;
    fn accessibility_label(&self) -> Option<&str>;
}

impl<T: ?Sized + View> ViewAccessibility for T {
    fn set_accessibility_label(&self, label: impl ToString) -> &Self {
        self.__base_view().accessibility_label = Some(label.to_string());
        self
    }

    fn accessibility_label(&self) -> Option<&str> {
        self.__base_view().accessibility_label.as_deref()
    }
}

pub struct Accessibility;

impl Accessibility {
    /// The nodes of the views on screen now, in reading order.
    pub fn nodes() -> Vec<AccessibleNode> {
        let mut nodes = vec![];
        collect(UIManager::root_view().weak_view(), &mut nodes);
        nodes
    }

    /// Says `text` once through the screen reader, like a new alert.
    pub fn announce(text: impl ToString) {
        let text = text.to_string();
        log::debug!("announce: {text}");
        #[cfg(wasm)]
        web::announce(&text);
    }

    /// Keeps the DOM layer of a browser in step with the views.
    #[cfg(feature = "gpu")]
    pub(crate) fn tick() {
        #[cfg(wasm)]
        web::tick();
    }
}

fn collect(view: WeakView, nodes: &mut Vec<AccessibleNode>) {
    if view.is_null() || view.is_hidden() {
        return;
    }
    if let Some(node) = node_of(view) {
        let leaf = node.role != AccessibleRole::Text || view.subviews().is_empty();
        if view.is_visible_on_screen() && node.frame.size.width > 0.0 && !node.label.is_empty()
            || node.role == AccessibleRole::TextBox
        {
            nodes.push(node);
        }
        if leaf {
            return;
        }
    }
    for child in view.subviews() {
        collect(child.weak_view(), nodes);
    }
}

fn node_of(view: WeakView) -> Option<AccessibleNode> {
    let own = view.accessibility_label().map(str::to_string);
    let frame = *view.absolute_frame();
    let any = view.as_any();
    let node = |role, label: String, value, checked| AccessibleNode {
        role,
        label: own.clone().unwrap_or(label),
        value,
        checked,
        frame,
        view,
    };
    if let Some(button) = any.downcast_ref::<Button>() {
        return Some(node(
            AccessibleRole::Button,
            button.text().to_string(),
            None,
            None,
        ));
    }
    if let Some(button) = any.downcast_ref::<ToolButton>() {
        let pressed = button.is_on().then_some(true);
        return Some(node(
            AccessibleRole::Button,
            button.text().to_string(),
            None,
            pressed,
        ));
    }
    if let Some(field) = any.downcast_ref::<TextField>() {
        let value = (!field.is_secure()).then(|| field.text().to_string());
        return Some(node(
            AccessibleRole::TextBox,
            field.placeholder().to_string(),
            value,
            None,
        ));
    }
    if let Some(check) = any.downcast_ref::<CheckBox>() {
        return Some(node(
            AccessibleRole::CheckBox,
            String::new(),
            None,
            Some(check.on()),
        ));
    }
    if let Some(switch) = any.downcast_ref::<Switch>() {
        return Some(node(
            AccessibleRole::Switch,
            String::new(),
            None,
            Some(switch.on()),
        ));
    }
    if let Some(slider) = any.downcast_ref::<Slider>() {
        let value = format!("{:.0}", slider.value());
        return Some(node(AccessibleRole::Slider, String::new(), Some(value), None));
    }
    if let Some(label) = any.downcast_ref::<Label>() {
        return Some(node(AccessibleRole::Text, label.text().to_string(), None, None));
    }
    own.is_some().then(|| node(AccessibleRole::Button, String::new(), None, None))
}

impl AccessibleNode {
    /// Taps the view at its middle, what the click of a screen reader on
    /// the node does.
    pub fn activate(&self) {
        activate(self.view);
    }
}

fn activate(view: WeakView) {
    use crate::ui::input::{Input, Touch};
    if view.is_null() || view.is_hidden() {
        return;
    }
    let center = view.absolute_frame().center();
    for phase in ["b", "e"] {
        let touch = Touch::from(format!("{} {} {phase}", center.x, center.y).as_str());
        Input::process_touch_event(touch);
    }
}

#[cfg(wasm)]
mod web {
    use std::{
        cell::RefCell,
        collections::{HashMap, hash_map::Entry},
    };

    use web_sys::{
        HtmlElement,
        wasm_bindgen::{JsCast, closure::Closure},
        window,
    };

    use super::{Accessibility, AccessibleNode, AccessibleRole, activate};
    use crate::{gm::ToF32, ui::UIManager};

    const EVERY_MS: f64 = 300.0;
    const EVERY_MS_INT: i32 = 300;
    const HIDDEN: &str = "position:fixed;opacity:0;pointer-events:none;margin:0;padding:0;overflow:hidden;";

    struct Mirror {
        element: HtmlElement,
        shown:   String,
        _click:  Closure<dyn FnMut()>,
    }

    #[derive(Default)]
    struct State {
        layer:   Option<HtmlElement>,
        live:    Option<HtmlElement>,
        nodes:   HashMap<(usize, u64), Mirror>,
        order:   Vec<(usize, u64)>,
        last:    f64,
        /// A frame is asked for, see `frame_later`.
        pending: bool,
    }

    thread_local! {
        static STATE: RefCell<State> = RefCell::new(State::default());
    }

    fn create(tag: &str) -> Option<HtmlElement> {
        let document = window()?.document()?;
        document.create_element(tag).ok()?.dyn_into::<HtmlElement>().ok()
    }

    fn layer(state: &mut State) -> Option<HtmlElement> {
        if state.layer.is_none() {
            let body = window()?.document()?.body()?;
            let layer = create("div")?;
            layer.set_id("hilen-accessibility");
            layer
                .style()
                .set_css_text("position:fixed;left:0;top:0;width:0;height:0;z-index:1;");
            let live = create("div")?;
            live.set_attribute("aria-live", "polite").ok()?;
            live.style()
                .set_css_text(&format!("{HIDDEN}left:0;top:0;width:1px;height:1px;"));
            body.append_child(&layer).ok()?;
            body.append_child(&live).ok()?;
            state.layer = Some(layer);
            state.live = Some(live);
        }
        state.layer.clone()
    }

    pub(super) fn announce(text: &str) {
        STATE.with_borrow_mut(|state| {
            if layer(state).is_none() {
                return;
            }
            if let Some(live) = &state.live
                && let Some(line) = create("p")
            {
                line.set_text_content(Some(text));
                let _ = live.append_child(&line);
                while live.child_element_count() > 3 {
                    if let Some(first) = live.first_element_child() {
                        first.remove();
                    }
                }
            }
        });
    }

    pub(super) fn tick() {
        let now = window()
            .and_then(|window| window.performance())
            .map_or(0.0, |clock| clock.now());
        let due = STATE.with_borrow(|state| now - state.last >= EVERY_MS);
        if !due {
            // A page draws only when something changed, so the last change
            // may come before the layer is due. 1 more frame then catches it.
            frame_later();
            return;
        }
        let nodes = Accessibility::nodes();
        let ratio = window().map_or(1.0, |window| window.device_pixel_ratio()).to_f32();
        let scale = UIManager::scale() / ratio;
        STATE.with_borrow_mut(|state| {
            state.last = now;
            let Some(layer) = layer(state) else {
                return;
            };
            let mut order = Vec::with_capacity(nodes.len());
            for node in &nodes {
                let raw = node.view.raw();
                let id = (raw.addr(), raw.stamp());
                order.push(id);
                let mirror = match state.nodes.entry(id) {
                    Entry::Occupied(entry) => entry.into_mut(),
                    Entry::Vacant(entry) => {
                        let Some(mirror) = new_mirror(node) else {
                            continue;
                        };
                        entry.insert(mirror)
                    }
                };
                show(mirror, node, scale);
            }
            state.nodes.retain(|id, mirror| {
                let keep = order.contains(id);
                if !keep {
                    mirror.element.remove();
                }
                keep
            });
            if order != state.order {
                for id in &order {
                    if let Some(mirror) = state.nodes.get(id) {
                        let _ = layer.append_child(&mirror.element);
                    }
                }
                state.order = order;
            }
        });
    }

    fn frame_later() {
        let pending = STATE.with_borrow(|state| state.pending);
        let Some(window) = window() else {
            return;
        };
        if pending {
            return;
        }
        let wake = Closure::once_into_js(|| {
            STATE.with_borrow_mut(|state| state.pending = false);
            crate::window::request_frame();
        });
        if window
            .set_timeout_with_callback_and_timeout_and_arguments_0(wake.unchecked_ref(), EVERY_MS_INT)
            .is_ok()
        {
            STATE.with_borrow_mut(|state| state.pending = true);
        }
    }

    fn new_mirror(node: &AccessibleNode) -> Option<Mirror> {
        let element = create("div")?;
        element.set_attribute("tabindex", "-1").ok()?;
        let view = node.view;
        let click = Closure::<dyn FnMut()>::new(move || activate(view));
        element.set_onclick(Some(click.as_ref().unchecked_ref()));
        Some(Mirror {
            element,
            shown: String::new(),
            _click: click,
        })
    }

    fn show(mirror: &mut Mirror, node: &AccessibleNode, scale: f32) {
        let frame = node.frame;
        let key = format!("{node:?}{scale}");
        if key == mirror.shown {
            return;
        }
        mirror.shown = key;
        let element = &mirror.element;
        let _ = element.set_attribute("role", node.role.aria_role());
        let _ = element.set_attribute("aria-label", &node.label);
        if node.role == AccessibleRole::Text {
            element.set_text_content(Some(&node.label));
        }
        match node.checked {
            Some(on) if node.role == AccessibleRole::Button => {
                let _ = element.set_attribute("aria-pressed", if on { "true" } else { "false" });
            }
            Some(on) => {
                let _ = element.set_attribute("aria-checked", if on { "true" } else { "false" });
            }
            None => {}
        }
        if let Some(value) = &node.value {
            let name = if node.role == AccessibleRole::Slider {
                "aria-valuenow"
            } else {
                "aria-valuetext"
            };
            let _ = element.set_attribute(name, value);
        }
        element.style().set_css_text(&format!(
            "{HIDDEN}left:{}px;top:{}px;width:{}px;height:{}px;",
            frame.origin.x * scale,
            frame.origin.y * scale,
            frame.size.width * scale,
            frame.size.height * scale
        ));
    }
}
