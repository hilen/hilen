//! How the selected part of a label reaches the screen, one more part of
//! `ui_drawer.rs` beside `label_drawer.rs`.

use crate::{
    gm::{
        color::{CLEAR, Color},
        flat::{CornerRadii, Rect},
    },
    pipelines::Pipelines,
    render::data::UIRectInstance,
    ui::{Label, TextSelection, View, ViewData, ViewFrame, ui_drawer::UIDrawer},
};

/// The color over selected text, of a label and of a text field.
pub(crate) const SELECTION_COLOR: Color = Color::rgba(0.2, 0.5, 1.0, 0.35);

/// `top` drawn over `bottom` as 1 color, the "over" of alpha blending.
fn over(top: Color, bottom: Color) -> Color {
    let alpha = top.a + bottom.a * (1.0 - top.a);
    if alpha <= 0.0 {
        return CLEAR;
    }
    let mix =
        |top_part: f32, bottom_part: f32| (top_part * top.a + bottom_part * bottom.a * (1.0 - top.a)) / alpha;
    Color::rgba(
        mix(top.r, bottom.r),
        mix(top.g, bottom.g),
        mix(top.b, bottom.b),
        alpha,
    )
}

impl UIDrawer {
    /// One rect per selected line of a label, under its glyphs.
    ///
    /// The rects sit at the depth of the label itself and are queued
    /// before its background. Of 2 rects at one depth the first one
    /// wins, so they show in place of the background, and their color is
    /// the selection color already laid over the background color. A
    /// depth of their own between the background and the glyphs does not
    /// exist: the glyphs are less than 2 steps of a 24 bit depth buffer
    /// in front of the label, and a rect that lands on their step would
    /// hide them.
    pub(super) fn draw_selection(view: &dyn View, frame: &Rect, scale: f32, opacity: f32) {
        if TextSelection::is_empty() {
            return;
        }
        let Some(label) = view.as_any().downcast_ref::<Label>() else {
            return;
        };
        if !label.is_selectable() {
            return;
        }
        let Some(highlight) = TextSelection::highlight(label) else {
            return;
        };

        let color = over(SELECTION_COLOR.faded(opacity), label.color().faded(opacity));

        for rect in label.selection_rects(highlight.start..highlight.end, highlight.goes_on) {
            Pipelines::rect().add(UIRectInstance::new(
                Rect::new(
                    frame.x() + rect.x(),
                    frame.y() + rect.y(),
                    rect.width(),
                    rect.height(),
                ),
                color,
                CLEAR,
                0.0,
                CornerRadii::default(),
                label.z_position(),
                scale,
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::over;
    use crate::gm::color::{CLEAR, Color};

    fn close(a: Color, b: Color) -> bool {
        [(a.r, b.r), (a.g, b.g), (a.b, b.b), (a.a, b.a)]
            .into_iter()
            .all(|(a, b)| (a - b).abs() < 1e-5)
    }

    #[test]
    fn over_nothing_a_color_stays_as_it_is() {
        let top = Color::rgba(0.2, 0.5, 1.0, 0.35);
        assert!(close(over(top, CLEAR), top));
    }

    #[test]
    fn over_a_solid_color_the_result_is_solid_and_mixed() {
        let mixed = over(Color::rgba(1.0, 0.0, 0.0, 0.5), Color::rgba(0.0, 0.0, 1.0, 1.0));
        assert!(close(mixed, Color::rgba(0.5, 0.0, 0.5, 1.0)));
    }
}
