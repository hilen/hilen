use anyhow::{Result, bail, ensure};
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{
        BLACK, Color, Container, Label, MarkdownView, MathView, Setup, TextAlignment, VerticalAlignment,
        ViewData, ViewFrame, ViewSubviews, ViewTest, WHITE, view,
    },
    ui_test::{check_colors, set_record_probe_count},
};

/// 3 list items with a fraction each, the way a chat explains a
/// derivative, and a paragraph whose fraction stands on a line that
/// wraps. Every fraction is taller than a line of text. Until 2026-11-15
/// the lines kept the height of the text, and each fraction covered the
/// one of the item over it and the words of the line under it.
const TEXT: &str = r"- $\frac{d}{dt}$ is rate.
- $\frac{dx}{dt}$ is rate(position).
- $\frac{d^{2}x}{dt^{2}}$ is rate(rate(position)).

The real $\frac{d}{dt}$ is the same thing with dt shrunk toward 0, the lines under it stay clear of it.
";

const FORMULAS: usize = 4;
const ITEMS: usize = 3;
const LEFT: f32 = 16.0;
const TOP: f32 = 40.0;
const WIDTH: f32 = 388.0;

/// Recorded with `--record-colors`.
const COLORS: &str = r"
             412    4 - #ffffff
              56   20 - #030303
              88   20 - #545454
             104   20 - #313131
             144   20 - #ffffff
             152   20 - #464646
             168   20 - #000000
             180   20 - #757575
             204   20 - #ffffff
             208   20 - #ffffff
             248   20 - #b2b2b2
             264   20 - #1d1d1d
             292   20 - #7e7e7e
              60   52 - #000000
              72   52 - #26282a
             100   52 - #e8f0fe
              56   60 - #e8f0fe
             360   64 - #e8f0fe
              60   76 - #e6eefc
             100   80 - #8f949c
             116   80 - #dfe7f4
             140   80 - #27282b
             144   80 - #000000
             148   80 - #0f1011
             116   84 - #dfe7f4
             256   84 - #e8f0fe
             412   96 - #ffffff
              60  104 - #e8f0fe
              64  108 - #e8f0fe
              80  112 - #9a9fa8
             108  112 - #e8f0fe
             112  112 - #959aa3
             120  112 - #e8f0fe
             136  112 - #e8f0fe
             148  112 - #e8f0fe
             180  112 - #e8f0fe
             188  112 - #e8f0fe
             200  112 - #e8f0fe
              36  144 - #27282b
              60  144 - #000000
              80  144 - #0f0f10
             184  144 - #b1b8c2
             192  144 - #848890
             196  144 - #000000
             268  144 - #000000
             280  144 - #37393d
             284  144 - #b8bec9
             296  144 - #000000
             336  144 - #c9d0dc
             356  144 - #e8f0fe
              36  148 - #27282b
              60  148 - #000000
              80  148 - #0f0f10
             124  148 - #28292c
             128  148 - #191a1c
             184  148 - #b1b8c2
             192  148 - #848890
             196  148 - #010101
             200  148 - #767a81
             268  148 - #010101
             272  148 - #606369
             276  148 - #28292c
             288  148 - #0f1011
             292  148 - #8e939b
             296  148 - #000000
             336  148 - #c9d0dc
             372  148 - #28292c
             376  148 - #191a1c
             152  164 - #c9d0dc
              36  168 - #6c7077
              44  168 - #484a4f
              92  168 - #e8f0fe
              96  168 - #55585d
             104  168 - #9ba0a9
             148  168 - #e8f0fe
             152  168 - #c9d0dc
             160  168 - #e8f0fe
             168  168 - #000000
             172  168 - #6e7279
             188  168 - #141516
             196  168 - #000000
             284  208 - #ffffff
             356  208 - #ffffff
             108  228 - #ffffff
             168  236 - #ffffff
             412  236 - #ffffff
             236  244 - #ffffff
              40  252 - #ffffff
             320  260 - #ffffff
             120  288 - #ffffff
               4  332 - #ffffff
              72  332 - #ffffff
             168  332 - #ffffff
             232  332 - #ffffff
             336  332 - #ffffff
             412  332 - #ffffff
            ";

#[view]
struct MarkdownTallFormula {
    height: f32,

    #[init]
    caption:    Label,
    background: Container,
    text:       MarkdownView,
}

impl Setup for MarkdownTallFormula {
    fn setup(mut self: Weak<Self>) {
        self.set_color(WHITE);
        self.caption
            .set_text_size(13)
            .set_alignment(TextAlignment::Left)
            .set_vertical_alignment(VerticalAlignment::Top);
        self.caption.set_text("fractions in lines, no formula covers another line");
        self.caption.set_text_color(BLACK);
        self.caption.set_frame((LEFT, 12.0, WIDTH, 20.0));

        self.text.set_text_color(BLACK);
        self.text.set_text(TEXT);
        self.height = self.text.height_for_width(WIDTH);
        self.text.set_frame((LEFT, TOP, WIDTH, self.height));
        self.background.set_color(Color::hex("#e8f0fe"));
        self.background.set_frame((LEFT, TOP, WIDTH, self.height));
    }
}

/// A rect in the markdown view, top to bottom and left to right.
#[derive(Clone, Copy, Debug)]
struct Span {
    top:    f32,
    bottom: f32,
    left:   f32,
    right:  f32,
}

impl Span {
    fn overlaps(&self, other: &Span) -> bool {
        self.top < other.bottom - 0.01
            && other.top < self.bottom - 0.01
            && self.left < other.right - 0.01
            && other.left < self.right - 0.01
    }
}

/// A formula view: where it is and its baseline.
#[derive(Debug)]
struct Formula {
    span:     Span,
    baseline: f32,
}

/// 1 line of a label: the label it is in, the top and the bottom of its
/// box, its baseline and the plain line height.
#[derive(Debug)]
struct Line {
    label:      usize,
    box_top:    f32,
    box_bottom: f32,
    baseline:   f32,
    pitch:      f32,
}

/// What the test reads off the screen, in the points of the markdown
/// view.
struct Reading {
    formulas: Vec<Formula>,
    lines:    Vec<Line>,
    /// Every label of a text, with the top of its first line and its text.
    labels:   Vec<(Span, f32, String)>,
    /// The top of every bullet.
    markers:  Vec<f32>,
    height:   f32,
}

fn read(view: Weak<MarkdownTallFormula>) -> Reading {
    from_main(move || {
        let subviews = view.text.subviews();
        let formulas: Vec<Formula> = subviews
            .iter()
            .filter_map(|child| child.downcast_view::<MathView>())
            .map(|math| Formula {
                span:     Span {
                    top:    math.y(),
                    bottom: math.max_y(),
                    left:   math.x(),
                    right:  math.max_x(),
                },
                baseline: math.y() + math.baseline(),
            })
            .collect();
        let all: Vec<Weak<Label>> =
            subviews.iter().filter_map(|child| child.downcast_view::<Label>()).collect();
        let (markers, texts): (Vec<Weak<Label>>, Vec<Weak<Label>>) =
            all.into_iter().partition(|label| label.text() == "•");
        let mut lines = vec![];
        let mut labels = vec![];
        for (index, label) in texts.iter().enumerate() {
            let layout = label.text_layout_for(label.text());
            let frame = *label.frame();
            labels.push((
                Span {
                    top:    frame.origin.y,
                    bottom: frame.origin.y + frame.size.height,
                    left:   frame.origin.x,
                    right:  frame.origin.x + frame.size.width,
                },
                frame.origin.y + layout.line_top(0),
                label.text().to_string(),
            ));
            for line in 0..layout.line_count() {
                let box_top = frame.origin.y + layout.line_box_top(line);
                lines.push(Line {
                    label: index,
                    box_top,
                    box_bottom: box_top + layout.line_box_height(line),
                    baseline: frame.origin.y + layout.line_top(line) + layout.ascent,
                    pitch: layout.line_height,
                });
            }
        }
        let markers: Vec<f32> = markers.iter().map(|marker| marker.y()).collect();
        Reading {
            formulas,
            lines,
            labels,
            markers,
            height: view.height,
        }
    })
}

impl ViewTest for MarkdownTallFormula {
    fn canvas() -> (u32, u32) {
        (420, 340)
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        set_record_probe_count(96);
        check_colors(COLORS)?;

        let Reading {
            formulas,
            lines,
            labels,
            markers,
            height,
        } = read(view);

        ensure!(
            formulas.len() == FORMULAS,
            "{} formula views, not {FORMULAS}",
            formulas.len()
        );

        // No 2 formulas cover each other.
        for (index, one) in formulas.iter().enumerate() {
            for other in &formulas[index + 1..] {
                ensure!(
                    !one.span.overlaps(&other.span),
                    "the formulas at {:?} and {:?} overlap",
                    one.span,
                    other.span
                );
            }
        }

        // No 2 labels cover each other, so a line box is only in 1 label.
        for (index, (one, ..)) in labels.iter().enumerate() {
            for (other, ..) in &labels[index + 1..] {
                ensure!(
                    !one.overlaps(other),
                    "the labels at {one:?} and {other:?} overlap"
                );
            }
        }

        // Every formula stands on the baseline of 1 line and stays inside
        // the box of that line and inside its label.
        for formula in &formulas {
            let Some(line) = lines.iter().find(|line| (line.baseline - formula.baseline).abs() < 0.5) else {
                bail!(
                    "no line has the baseline {} of the formula at {:?}",
                    formula.baseline,
                    formula.span
                );
            };
            ensure!(
                formula.span.top >= line.box_top - 0.01 && formula.span.bottom <= line.box_bottom + 0.01,
                "the formula at {:?} leaves the box of its line, {} to {}",
                formula.span,
                line.box_top,
                line.box_bottom
            );
            let (label, ..) = &labels[line.label];
            ensure!(
                formula.span.top >= label.top - 0.01 && formula.span.bottom <= label.bottom + 0.01,
                "the formula at {:?} leaves its label at {label:?}",
                formula.span
            );
        }

        // A line with no formula keeps the plain line height.
        let plain = lines
            .iter()
            .filter(|line| !formulas.iter().any(|formula| (line.baseline - formula.baseline).abs() < 0.5))
            .collect::<Vec<_>>();
        ensure!(!plain.is_empty(), "every line has a formula, the wrap moved");
        for line in plain {
            let height = line.box_bottom - line.box_top;
            ensure!(
                (height - line.pitch).abs() < 0.01,
                "a line with no formula is {height} high, not {}",
                line.pitch
            );
        }

        // The bullet of an item stands on the first line of its text,
        // which is lower in a line that grew.
        ensure!(markers.len() == ITEMS, "{} bullets, not {ITEMS}", markers.len());
        for (marker, (_, text_top, text)) in
            markers.iter().zip(labels.iter().filter(|label| label.2.contains("rate")))
        {
            ensure!(
                (marker - text_top).abs() < 0.5,
                "the bullet at {marker} is not on the first line of {text:?} at {text_top}"
            );
        }

        // Nothing runs out of the measured height.
        let lowest = formulas.iter().map(|formula| formula.span.bottom).fold(0.0, f32::max);
        let last = labels.iter().map(|(span, ..)| span.bottom).fold(0.0, f32::max);
        ensure!(
            lowest.max(last) <= height + 0.01,
            "a view ends at {}, the measured height is {height}",
            lowest.max(last)
        );

        Ok(())
    }
}
