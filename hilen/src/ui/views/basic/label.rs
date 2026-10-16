mod box_chars;
mod paint;

use std::{fmt::Display, ops::Range, sync::atomic::Ordering};

use atomic_float::AtomicF32;
use ui_proc::view;
use zeroize::{Zeroize, Zeroizing};

use crate::{
    deps::refs::{Weak, weak_from_ref},
    gm::{
        ToF32,
        color::{BLACK, Color},
        flat::Size,
    },
    ui::{
        DynamicColor, ImageView, Setup, Style, ToLabel, UIColor, UIManager, View, ViewCallbacks, ViewFrame,
        text_selection::Lead,
        view::{ViewData, ViewSubviews},
        views::basic::{
            label_runs::StyleRun,
            label_style::{TextOutline, TextShadow},
        },
    },
    window::{BoxChars, Font, Shaping, TextLayout, image::ToImage},
    wipe::joined,
};

static DEFAULT_TEXT_SIZE: AtomicF32 = AtomicF32::new(16.0);

/// Points between the frame and left or right aligned text, see
/// `Label::set_text_inset`. 8 is what apps laid out on a retina screen
/// had when the indent was 16 pixels of the screen.
static DEFAULT_TEXT_INSET: AtomicF32 = AtomicF32::new(8.0);

/// How the chars of a label get their place.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum Placing {
    /// Each char as wide as its glyph.
    #[default]
    Flow,
    /// Each char in whole cells of a grid, see `Label::set_cell_grid`.
    Grid,
}

#[derive(Debug, Default)]
pub enum TextAlignment {
    Left,
    #[default]
    Center,
    Right,
}

impl TextAlignment {
    pub fn center(&self) -> bool {
        matches!(self, Self::Center)
    }
}

/// Where the block of lines sits in the frame. Center is what every
/// label did before this existed. Top is what a text area needs, the
/// first line at the top and the rest following it down.
#[derive(Debug, Default, Clone, Copy)]
pub enum VerticalAlignment {
    Top,
    #[default]
    Center,
}

/// Which end an ellipsized label cuts. Tail keeps the front, the CSS
/// `text-overflow: ellipsis`. Head keeps the end, what a path under
/// `dir="rtl"` shows.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Ellipsize {
    #[default]
    None,
    Tail,
    Head,
}

#[view]
pub struct Label {
    pub alignment: TextAlignment,

    pub vertical_alignment: VerticalAlignment,

    pub text: String,

    /// The text is a secret, see `set_secret`.
    secret: bool,

    /// The text can be selected and copied, see `set_selectable`.
    pub(super) selectable: bool,

    /// What stands between this text and the selectable text before it
    /// in a copy of the cell they are in, a line break when not set.
    pub(crate) selection_lead: Lead,

    multiline: bool,

    /// The most lines a multiline label shows, 0 for no limit. See
    /// `set_max_lines`.
    pub(super) max_lines: usize,

    pub(super) text_outline:     Option<TextOutline>,
    pub(super) text_shadow:      Option<TextShadow>,
    /// Kept apart from the shadow, so the blur can be set before it.
    pub(super) text_shadow_blur: f32,

    ellipsize: Ellipsize,

    /// The truncation computed for the cached width: the width it was
    /// computed at, and the shortened copy, `None` when the full text
    /// fits there. Dropped by every setter that changes what a
    /// truncation depends on. The copy wipes itself when dropped, it is
    /// a part of the text and the text may be a secret.
    pub(super) ellipsized: Option<(f32, Option<Zeroizing<String>>)>,

    #[educe(Default = BLACK)]
    text_color: Color,

    dynamic_text_color: Option<DynamicColor>,

    /// The bottom of the glyph ramp, `None` for flat text. The top is
    /// `text_color`, so a gradient and a plain color cannot both be set.
    text_end_color: Option<Color>,

    dynamic_text_end_color: Option<DynamicColor>,

    /// Byte ranges of the text drawn in their own color, sorted and not
    /// overlapping. Everything outside them keeps `text_color`.
    color_runs: Vec<ColorRun>,

    /// Byte ranges of the text with a color of their own behind them,
    /// sorted and not overlapping. See `set_background_runs`.
    background_runs: Vec<ColorRun>,

    /// Whether the text sits on a cell grid, see `set_cell_grid`.
    placing: Placing,

    /// `Fitted` when box chars fill their cells, see `set_joins_box_chars`.
    box_chars: BoxChars,

    /// Byte ranges drawn in their own font or underlined, sorted and not
    /// overlapping. See `set_font_runs`.
    pub(super) font_runs: Vec<StyleRun>,

    #[educe(Default = DEFAULT_TEXT_SIZE.load(Ordering::Relaxed))]
    text_size: f32,

    /// Points between the frame and left or right aligned text, see
    /// `set_text_inset`.
    #[educe(Default = DEFAULT_TEXT_INSET.load(Ordering::Relaxed))]
    text_inset: f32,

    /// Extra points between glyphs, CoreText style tracking. Negative
    /// tightens. Needed to match fonts whose tracking the platform
    /// applies from the trak table, like SF Pro on macOS.
    letter_spacing: f32,

    /// Points between baselines, the CSS line box. `None` keeps the
    /// font's own line height. Glyphs center in each box with half the
    /// leading above and below, and a multiline measure is boxes times
    /// the box.
    line_height: Option<f32>,

    font: Weak<Font>,
}

impl Label {
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Also drops the color and font runs, they were ranges of the old text.
    pub fn set_text(&self, text: impl ToLabel) -> &Self {
        let mut this = weak_from_ref(self);
        if this.secret {
            this.text.zeroize();
        }
        this.text = text.to_label();
        this.color_runs.clear();
        this.background_runs.clear();
        this.font_runs.clear();
        this.ellipsized = None;
        self
    }

    /// Marks the text of this label as a secret, like a recovery phrase.
    /// Call it before `set_text`.
    ///
    /// A secret label writes zeros over its text before the memory is
    /// freed, when the text is replaced and when the label is dropped.
    /// The copy cut for an ellipsis is wiped the same way. The text stays
    /// out of the shape and measure caches of the font, which keep a copy
    /// of every other text for some seconds, so a secret label is shaped
    /// again on every frame that draws it.
    ///
    /// Give the text as `&str`. A `String` passed by value is the copy of
    /// the caller and is freed as it is. Change the text with `set_text`
    /// only, a write to the `text` field frees the old text as it is.
    ///
    /// What cannot be wiped is what the text became on its way to the
    /// screen. The shaper, the glyph brush and the GPU buffers hold the
    /// glyph numbers and positions of the drawn text until other text
    /// takes their place, and the glyph atlas keeps the picture of every
    /// drawn glyph for the life of the font. That is not the text, but
    /// together with the font it tells what was drawn.
    pub fn set_secret(&self, secret: bool) -> &Self {
        weak_from_ref(self).secret = secret;
        self
    }

    pub fn is_secret(&self) -> bool {
        self.secret
    }

    /// How this label shapes `text`, for the font to measure and lay it
    /// out the way the drawer does.
    pub(super) fn shaping(&self, text: &str) -> Shaping {
        Shaping {
            tracking:    self.letter_spacing,
            line_height: self.line_height(),
            runs:        self.shaping_runs(text),
            secret:      self.secret,
            grid:        self.cell_width(),
        }
    }

    /// Puts the text on a cell grid, like the screen of a terminal.
    /// Every char then takes whole cells, 1 for most, 2 for a wide one
    /// like a CJK letter or an emoji, and none for a mark that joins the
    /// char before it. A glyph that comes from a fallback font, wider or
    /// narrower than the font of the label, moves nothing after it out
    /// of its column. `cell_size` is the size of 1 cell.
    ///
    /// The lines are 1 cell high, unless `set_line_height` says more. A
    /// label 1 cell high per line is so filled by its lines with no gap,
    /// and rows of such labels tile.
    ///
    /// Made for a mono font. With any other font the text is spread
    /// over cells as wide as its space.
    pub fn set_cell_grid(&self, grid: bool) -> &Self {
        let mut this = weak_from_ref(self);
        this.placing = if grid { Placing::Grid } else { Placing::Flow };
        this.ellipsized = None;
        self
    }

    /// The size of 1 cell of the cell grid at the font and the text size
    /// of this label, in points: the advance of a space and the line
    /// pitch of the font, each rounded to whole pixels of the screen. A
    /// cell of half a pixel more would leave seams between the colored
    /// backgrounds of 2 cells. It changes with the scale of the screen.
    pub fn cell_size(&self) -> Size {
        let scale = UIManager::scale();
        let (advance, pitch) = self.font().cell_metrics(self.text_size);
        Size::new((advance * scale).round() / scale, (pitch * scale).round() / scale)
    }

    /// The width of 1 cell when the text sits on a cell grid.
    pub(crate) fn cell_width(&self) -> Option<f32> {
        (self.placing == Placing::Grid).then(|| self.cell_size().width)
    }

    /// Paints byte ranges of the text in their own colors, the rest keeps
    /// the text color. Shaping runs over the whole text once, so kerning
    /// and letter spacing across a run boundary stay as they are. Ranges
    /// are clamped to the text, sorted, and a later range wins an overlap.
    /// A highlighter such as syntect produces exactly this shape.
    pub fn set_color_runs(&self, runs: impl IntoIterator<Item = (Range<usize>, UIColor)>) -> &Self {
        let mut this = weak_from_ref(self);
        this.color_runs = Self::sorted_runs(this.text.len(), runs);
        self
    }

    /// Paints a color behind byte ranges of the text, each as high as
    /// its line, under the glyphs and over the color of the label. The
    /// rest shows the color of the label. Ranges are treated like the
    /// ones of `set_color_runs`. A selection shows over them.
    /// `set_text` clears the runs.
    pub fn set_background_runs(&self, runs: impl IntoIterator<Item = (Range<usize>, UIColor)>) -> &Self {
        let mut this = weak_from_ref(self);
        this.background_runs = Self::sorted_runs(this.text.len(), runs);
        self
    }

    pub fn clear_background_runs(&self) -> &Self {
        weak_from_ref(self).background_runs.clear();
        self
    }

    pub(crate) fn background_runs(&self) -> &[ColorRun] {
        &self.background_runs
    }

    pub fn background_runs_len(&self) -> usize {
        self.background_runs.len()
    }

    /// Ranges clamped to a text of `len` bytes, sorted, with no overlap:
    /// a later range wins.
    fn sorted_runs(len: usize, runs: impl IntoIterator<Item = (Range<usize>, UIColor)>) -> Vec<ColorRun> {
        let mut all: Vec<ColorRun> = runs
            .into_iter()
            .map(|(range, color)| ColorRun {
                range:   range.start.min(len)..range.end.min(len),
                color:   color.resolve(),
                dynamic: color.dynamic(),
            })
            .filter(|run| run.range.start < run.range.end)
            .collect();

        all.sort_by_key(|run| run.range.start);

        // A later range wins, so the earlier one gives up the overlap.
        let mut runs: Vec<ColorRun> = vec![];
        for run in all {
            if let Some(last) = runs.last_mut()
                && last.range.end > run.range.start
            {
                last.range.end = run.range.start;
                if last.range.start >= last.range.end {
                    runs.pop();
                }
            }
            runs.push(run);
        }

        runs
    }

    pub fn clear_color_runs(&self) -> &Self {
        weak_from_ref(self).color_runs.clear();
        self
    }

    pub(crate) fn color_runs(&self) -> &[ColorRun] {
        &self.color_runs
    }

    pub fn color_runs_len(&self) -> usize {
        self.color_runs.len()
    }

    pub fn text_color(&self) -> &Color {
        &self.text_color
    }

    /// The text color as it was set, keeping the theme pair when there is
    /// one. See `ViewData::ui_color` for why restoring a color through
    /// `text_color()` is not enough.
    pub fn ui_text_color(&self) -> UIColor {
        match self.dynamic_text_color {
            Some(dynamic) => UIColor::Dynamic(dynamic),
            None => UIColor::Plain(self.text_color),
        }
    }

    pub fn set_text_color(&self, color: impl Into<UIColor>) -> &Self {
        let mut this = weak_from_ref(self);
        match color.into() {
            UIColor::Plain(color) => {
                this.text_color = color;
                this.dynamic_text_color = None;
            }
            UIColor::Dynamic(color) => {
                this.text_color = color.resolve();
                this.dynamic_text_color = Some(color);
            }
        }
        this.text_end_color = None;
        this.dynamic_text_end_color = None;
        self
    }

    pub(crate) fn text_end_color(&self) -> Option<Color> {
        self.text_end_color
    }

    /// Fades the glyphs from `start` at the top of the label frame to `end` at
    /// its bottom. This is what CSS paints with a linear gradient and
    /// `background-clip: text`. [`Label::set_text_color`] clears it.
    pub fn set_text_gradient(&self, start: impl Into<UIColor>, end: impl Into<UIColor>) -> &Self {
        self.set_text_color(start);

        let mut this = weak_from_ref(self);
        match end.into() {
            UIColor::Plain(color) => {
                this.text_end_color = Some(color);
                this.dynamic_text_end_color = None;
            }
            UIColor::Dynamic(color) => {
                this.text_end_color = Some(color.resolve());
                this.dynamic_text_end_color = Some(color);
            }
        }
        self
    }

    pub(crate) fn text_size(&self) -> f32 {
        self.text_size
    }

    pub fn set_text_size(&self, size: impl ToF32) -> &Self {
        let mut this = weak_from_ref(self);
        this.text_size = size.to_f32();
        this.ellipsized = None;
        self
    }

    pub(crate) fn letter_spacing(&self) -> f32 {
        self.letter_spacing
    }

    pub fn set_letter_spacing(&self, spacing: impl ToF32) -> &Self {
        let mut this = weak_from_ref(self);
        this.letter_spacing = spacing.to_f32();
        this.ellipsized = None;
        self
    }

    /// The line box that was set, else 1 cell for a text on a cell grid.
    pub(crate) fn line_height(&self) -> Option<f32> {
        self.line_height
            .or_else(|| (self.placing == Placing::Grid).then(|| self.cell_size().height))
    }

    /// Points between baselines, the CSS line box, see the field.
    pub fn set_line_height(&self, height: impl ToF32) -> &Self {
        let mut this = weak_from_ref(self);
        this.line_height = Some(height.to_f32());
        this.ellipsized = None;
        self
    }

    pub(crate) fn font(&self) -> Weak<Font> {
        if self.font.is_ok() {
            self.font
        } else {
            Font::default()
        }
    }

    pub fn set_font(&self, font: Weak<Font>) -> &Self {
        let mut this = weak_from_ref(self);
        this.font = font;
        this.ellipsized = None;
        self
    }

    /// Size the label's frame needs to show the current text. Multiline
    /// wraps at the current frame width.
    pub fn content_size(&self) -> Size {
        self.size_for_width(self.width())
    }

    /// Size the label's frame needs at the given frame width. For multiline
    /// this is how auto-height panels measure before layout.
    pub fn size_for_width(&self, width: f32) -> Size {
        let margin = self.alignment_margin();
        let bound = self.multiline.then_some(width - margin);
        // With a line limit the frame is as tall as what is drawn, not as
        // the whole text.
        let text = if self.limits_lines() {
            self.display_text(width)
        } else {
            &self.text
        };
        let measured = self.font().measure(text, self.text_size, bound, self.shaping(text));

        if measured.has_no_area() {
            return measured;
        }

        Size::new(measured.width + margin, measured.height)
    }

    // The drawer indents left and right aligned text by the text inset,
    // so the fitted frame must include it or the text clips.
    fn alignment_margin(&self) -> f32 {
        if self.alignment.center() {
            0.0
        } else {
            self.text_inset
        }
    }
}

impl Label {
    pub fn set_alignment(&self, alignment: TextAlignment) -> &Self {
        weak_from_ref(self).alignment = alignment;
        self
    }

    pub fn set_vertical_alignment(&self, alignment: VerticalAlignment) -> &Self {
        weak_from_ref(self).vertical_alignment = alignment;
        self
    }

    /// Line and caret positions of `text` drawn by this label, in points,
    /// wrapping at the current frame width when multiline. Public so a
    /// view can map a click on drawn text to a byte and back, the way a
    /// diff panel selects code.
    pub fn text_layout_for(&self, text: &str) -> TextLayout {
        let bound = self.multiline.then_some(self.width() - self.alignment_margin());
        self.font().text_layout(text, self.text_size, bound, self.shaping(text))
    }

    /// Where the drawn text starts inside the frame, in points: the inset
    /// for left and right aligned text, 0 for centered text. Public so a
    /// view can lay a selection highlight over the text.
    pub fn text_inset(&self) -> f32 {
        self.alignment_margin()
    }

    /// Points between the frame and the text of a left or a right aligned
    /// label, the same on every screen. 0 puts the text at the edge of the
    /// frame. Centered text ignores it. The default is 8, see
    /// `set_default_text_inset`.
    pub fn set_text_inset(&self, points: impl ToF32) -> &Self {
        let mut this = weak_from_ref(self);
        this.text_inset = points.to_f32();
        this.ellipsized = None;
        self
    }

    pub(crate) fn is_multiline(&self) -> bool {
        self.multiline
    }

    pub fn set_multiline(&self, multiline: bool) -> &Self {
        let mut this = weak_from_ref(self);
        this.multiline = multiline;
        this.ellipsized = None;
        self
    }

    /// A single line label cuts text that does not fit its width to the
    /// longest prefix that does and draws an ellipsis after it, the CSS
    /// `text-overflow: ellipsis`. Off by default, the overflow clips.
    /// Multiline labels wrap instead and ignore this.
    pub fn set_ellipsize(&self, ellipsize: bool) -> &Self {
        let mut this = weak_from_ref(self);
        this.ellipsize = if ellipsize {
            Ellipsize::Tail
        } else {
            Ellipsize::None
        };
        this.ellipsized = None;
        self
    }

    /// Like `set_ellipsize` but cuts the front and keeps the end, with
    /// the ellipsis leading, what a path under `dir="rtl"` with CSS
    /// `text-overflow: ellipsis` shows. Do not combine with color or
    /// font runs, their byte ranges are of the full text and do not
    /// follow the shifted copy.
    pub fn set_ellipsize_head(&self, ellipsize: bool) -> &Self {
        let mut this = weak_from_ref(self);
        this.ellipsize = if ellipsize {
            Ellipsize::Head
        } else {
            Ellipsize::None
        };
        this.ellipsized = None;
        self
    }

    /// The text the drawer paints at the given frame width: the full text
    /// while it fits, the truncated copy with the trailing ellipsis when
    /// it does not. The full text unless `set_ellipsize` opted in, or a
    /// multiline label has `set_max_lines`.
    pub fn display_text(&self, width: f32) -> &str {
        let clamps_lines = self.limits_lines();
        let cuts_line = self.ellipsize != Ellipsize::None && !self.multiline;
        if !(clamps_lines || cuts_line) || self.text.is_empty() {
            return &self.text;
        }

        if self.ellipsized.as_ref().is_none_or(|(w, _)| (*w - width).abs() > f32::EPSILON) {
            let truncated = if clamps_lines {
                self.truncate_to_lines(width)
            } else {
                self.truncate_to(width)
            };
            weak_from_ref(self).ellipsized = Some((width, truncated));
        }

        match &self.ellipsized.as_ref().expect("just computed").1 {
            Some(text) => text.as_str(),
            None => &self.text,
        }
    }

    fn truncate_to(&self, width: f32) -> Option<Zeroizing<String>> {
        const ELLIPSIS: &str = "…";

        let available = width - self.alignment_margin();
        let mut font = self.font();
        let mut fits = |text: &str| {
            let shaping = Shaping {
                line_height: None,
                ..self.shaping(text)
            };
            font.measure(text, self.text_size, None, shaping).width <= available
        };

        if fits(&self.text) {
            return None;
        }

        // Byte boundary of the cut keeping this many characters, prefix
        // ends for the tail cut, suffix starts for the head cut. Keeping
        // every character is out, the full text already does not fit.
        let head = self.ellipsize == Ellipsize::Head;
        let cuts: Vec<usize> = if head {
            let mut starts: Vec<usize> = self.text.char_indices().map(|(index, _)| index).collect();
            starts.reverse();
            starts
        } else {
            self.text
                .char_indices()
                .skip(1)
                .map(|(index, _)| index)
                .chain([self.text.len()])
                .collect()
        };
        let candidate = |kept: usize| {
            if head {
                let start = if kept == 0 {
                    self.text.len()
                } else {
                    cuts[kept - 1]
                };
                joined(&[ELLIPSIS, &self.text[start..]])
            } else {
                let end = if kept == 0 { 0 } else { cuts[kept - 1] };
                joined(&[&self.text[..end], ELLIPSIS])
            }
        };

        // The longest fitting cut by binary search. When even the bare
        // ellipsis does not fit it still draws, like CSS does.
        let mut best = 0;
        let (mut low, mut high) = (0, cuts.len() - 1);
        while low <= high {
            let mid = usize::midpoint(low, high);
            if fits(&candidate(mid)) {
                best = mid;
                low = mid + 1;
            } else if mid == 0 {
                break;
            } else {
                high = mid - 1;
            }
        }

        Some(candidate(best))
    }

    pub fn set_image(&self, image: impl ToImage) -> &Self {
        self.remove_all_subviews();
        let image_view = self.add_view::<ImageView>();
        image_view.place().back();
        image_view.set_image(image);
        image_view.__base_view().z_position = self.z_position();

        self
    }

    pub fn set_resizing_image(&mut self, name: impl Display) -> &mut Self {
        self.remove_all_subviews();
        let mut image_view = self.add_view::<ImageView>();
        image_view.place().back();
        image_view.set_resizing_image(name);
        image_view.__base_view().z_position = self.z_position();
        image_view.subviews_weak().iter_mut().for_each(|v| {
            v.__base_view().z_position = self.z_position();
            v.subviews_weak().iter_mut().for_each(|v| {
                v.__base_view().z_position = self.z_position();
            });
        });

        self
    }
}

impl Label {
    pub fn set_default_text_size(size: impl ToF32) {
        DEFAULT_TEXT_SIZE.store(size.to_f32(), Ordering::Relaxed);
    }

    /// The test harness forces its own size and has to put this back, or every
    /// label in the app keeps the harness size after a run.
    pub(crate) fn default_text_size() -> f32 {
        DEFAULT_TEXT_SIZE.load(Ordering::Relaxed)
    }

    /// The text inset of every label made after this call, in points, see
    /// `set_text_inset`. A label made before keeps its own.
    pub fn set_default_text_inset(points: impl ToF32) {
        DEFAULT_TEXT_INSET.store(points.to_f32(), Ordering::Relaxed);
    }

    pub(crate) fn default_text_inset() -> f32 {
        DEFAULT_TEXT_INSET.load(Ordering::Relaxed)
    }
}

impl Setup for Label {
    fn setup(self: Weak<Self>) {
        Style::apply_global(self);
    }
}

impl Drop for Label {
    fn drop(&mut self) {
        if self.secret {
            self.text.zeroize();
        }
    }
}

impl ViewCallbacks for Label {
    fn theme_changed(&mut self) {
        if let Some(color) = self.dynamic_text_color {
            self.text_color = color.resolve();
        }
        if let Some(color) = self.dynamic_text_end_color {
            self.text_end_color = Some(color.resolve());
        }
        for run in self.color_runs.iter_mut().chain(&mut self.background_runs) {
            if let Some(color) = run.dynamic {
                run.color = color.resolve();
            }
        }
    }
}

/// One colored byte range of a label's text.
pub(crate) struct ColorRun {
    pub range:   Range<usize>,
    pub color:   Color,
    pub dynamic: Option<DynamicColor>,
}

pub trait AddLabel {
    fn add_label(&self, text: impl ToLabel) -> &Self;
}

impl<T: ?Sized + View> AddLabel for T {
    fn add_label(&self, text: impl ToLabel) -> &Self {
        let mut label = self.add_view::<Label>();
        label.place().center().h(20).lr(0);
        label.text = text.to_label();
        self
    }
}

#[cfg(all(test, not_wasm))]
mod tests {
    use serial_test::serial;

    use crate::{
        deps::{hreads::set_current_thread_as_main, refs::Own},
        ui::{Label, Setup, TextAlignment},
        wipe::probe::{Freed, Watch},
    };

    fn left_label() -> Own<Label> {
        let label = Label::new();
        label.set_alignment(TextAlignment::Left);
        label
    }

    // The UI tests run with an inset of their own, so only this test sees
    // the default an app gets.
    #[test]
    #[serial]
    fn a_label_takes_the_default_inset_of_the_time_it_is_made() {
        set_current_thread_as_main();
        let centered = Label::new();
        let before = left_label();
        Label::set_default_text_inset(0);
        let after = left_label();
        Label::set_default_text_inset(8);

        assert!(centered.text_inset().abs() < f32::EPSILON);
        assert!((before.text_inset() - 8.0).abs() < f32::EPSILON);
        assert!(after.text_inset().abs() < f32::EPSILON);

        after.set_text_inset(12);
        assert!((after.text_inset() - 12.0).abs() < f32::EPSILON);
    }

    const FIRST: &str = "correct horse battery staple";
    const SECOND: &str = "another text of the same label";

    #[test]
    #[serial]
    fn a_secret_label_wipes_its_text_when_replaced_and_when_dropped() {
        set_current_thread_as_main();
        let label = Label::new();
        label.set_secret(true);

        label.set_text(FIRST);
        let first = Watch::on(label.text());
        label.set_text(SECOND);

        assert_eq!(first.freed(), Freed::Wiped);

        let second = Watch::on(label.text());
        drop(label);

        assert_eq!(second.freed(), Freed::Wiped);
    }

    // The control of the test above, it shows the probe would see a label
    // that does not wipe.
    #[test]
    #[serial]
    fn a_plain_label_frees_its_text_as_it_is() {
        set_current_thread_as_main();
        let label = Label::new();

        label.set_text(FIRST);
        let first = Watch::on(label.text());
        label.set_text(SECOND);

        assert_eq!(first.freed(), Freed::WithContent);

        let second = Watch::on(label.text());
        drop(label);

        assert_eq!(second.freed(), Freed::WithContent);
    }

    #[test]
    #[serial]
    fn a_label_marked_secret_after_its_text_was_set_still_wipes_it() {
        set_current_thread_as_main();
        let label = Label::new();

        label.set_text(FIRST);
        label.set_secret(true);
        let first = Watch::on(label.text());
        drop(label);

        assert_eq!(first.freed(), Freed::Wiped);
    }
}
