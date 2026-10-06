//! Where a byte of a drawn text is on the screen, for the text selection
//! tests. A test points at a byte, not at a pixel it once measured, so
//! it holds on a platform that shapes the text a little wider.

use hilen::{
    refs::Weak,
    ui::{Label, ViewData, ViewFrame, ViewSubviews, WeakView},
};

/// The point on the screen at the caret place of `byte`, in the middle of
/// its line. For a label with its text at the top and at the left. Call
/// it on the main thread.
pub(crate) fn text_point(label: Weak<Label>, byte: usize) -> (f32, f32) {
    let layout = label.text_layout_for(label.text());
    let line = layout.line_of(byte);
    let frame = label.absolute_frame();
    (
        frame.x() + label.text_inset() + layout.x_on_line(line, byte),
        frame.y() + layout.line_top(line) + layout.line_height / 2.0,
    )
}

/// The point of the first byte of `part` in the text of `label`.
pub(crate) fn point_of(label: Weak<Label>, part: &str) -> (f32, f32) {
    text_point(label, byte_of(label, part))
}

/// The point right after `part` in the text of `label`.
pub(crate) fn point_after(label: Weak<Label>, part: &str) -> (f32, f32) {
    text_point(label, byte_of(label, part) + part.len())
}

fn byte_of(label: Weak<Label>, part: &str) -> usize {
    label
        .text()
        .find(part)
        .unwrap_or_else(|| panic!("the label has no `{part}` in `{}`", label.text()))
}

/// The first label under `root` whose text has `part` in it, hidden views
/// left out. Call it on the main thread.
pub(crate) fn label_with(root: WeakView, part: &str) -> Weak<Label> {
    find(root, part).unwrap_or_else(|| panic!("no label shows `{part}`"))
}

fn find(view: WeakView, part: &str) -> Option<Weak<Label>> {
    if view.is_null() || view.is_hidden() {
        return None;
    }
    if let Some(label) = view.downcast_view::<Label>()
        && label.text().contains(part)
    {
        return Some(label);
    }
    view.subviews().iter().find_map(|sub| find(sub.weak(), part))
}

/// A press at `from`, a move over the middle to `to` and a release there,
/// as the lines `inject_touches` reads.
pub(crate) fn drag(from: (f32, f32), to: (f32, f32)) -> String {
    let middle = (f32::midpoint(from.0, to.0), f32::midpoint(from.1, to.1));
    format!(
        "{} {} b\n{} {} m\n{} {} m\n{} {} e",
        from.0, from.1, middle.0, middle.1, to.0, to.1, to.0, to.1
    )
}

/// A press and a release at one point.
pub(crate) fn click(at: (f32, f32)) -> String {
    format!("{} {} b\n{} {} e", at.0, at.1, at.0, at.1)
}

/// A selected text for the status label of a fixture: line breaks and
/// tabs written out, and only the start of a long text, so it fits the
/// label and a human sees where the breaks are.
pub(crate) fn shown(text: &str) -> String {
    const MOST: usize = 300;

    let written = text.replace('\n', "\\n").replace('\t', "\\t");
    if written.chars().count() <= MOST {
        return written;
    }
    let start: String = written.chars().take(MOST).collect();
    format!("{start} ...")
}
