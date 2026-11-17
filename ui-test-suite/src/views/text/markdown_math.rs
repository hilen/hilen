use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{
        BLACK, Color, Container, Label, MarkdownView, MathView, Setup, TextAlignment, VerticalAlignment,
        ViewData, ViewFrame, ViewSubviews, ViewTest, WHITE, view,
    },
    ui_test::{check_colors, set_record_probe_count},
};

/// Formulas in a line, 2 display formulas on their own lines and 1 that
/// does not parse, which stays as code.
const TEXT: &str = r"## Exponents

Cardano solved $x^{3} = 15x + 4$ in 1545. De Moivre found $(\cos a + i\sin a)^{n} = \cos(na) + i\sin(na)$.

$$x = \frac{-b \pm \sqrt{b^{2} - 4ac}}{2a}$$

Wallis wrote $x^{-n} = \frac{1}{x^{n}}$ and $x^{1/n} = \sqrt[n]{x}$, and $\frac{a}$ is broken.

$$\sum_{n=1}^{\infty} \frac{1}{n^{2}} = \frac{\pi^{2}}{6} \qquad \int_{0}^{1} x^{2}\,dx = \frac{1}{3}$$
";

/// The inline formulas of `TEXT`, in reading order.
const INLINE: [&str; 4] = [
    r"$x^{3} = 15x + 4$",
    r"$(\cos a + i\sin a)^{n} = \cos(na) + i\sin(na)$",
    r"$x^{-n} = \frac{1}{x^{n}}$",
    r"$x^{1/n} = \sqrt[n]{x}$",
];
const BROKEN: &str = r"$\frac{a}$";
const DISPLAY: usize = 2;

/// Recorded with `--record-colors`.
const COLORS: &str = r"
             468    4 - #ffffff
             632    4 - #ffffff
              44   16 - #878787
             140   16 - #7f7f7f
             160   16 - #3b3b3b
             240   16 - #939393
              44   20 - #878787
              68   20 - #5b5b5b
             152   20 - #000000
             160   20 - #3b3b3b
             180   20 - #414141
             212   20 - #d2d2d2
             240   20 - #939393
             260   20 - #ffffff
             296   20 - #ffffff
             364   20 - #ffffff
             372   20 - #ffffff
             552   20 - #ffffff
             128   24 - #e6e6e6
             152   24 - #313131
              40   44 - #161718
              68   48 - #000000
              80   48 - #010101
             108   48 - #000000
             116   48 - #000000
              48   52 - #010101
              56   52 - #cfd7e3
              60   52 - #010101
              64   52 - #000000
              76   52 - #000000
              80   52 - #b2b8c3
              88   52 - #010101
              92   52 - #010101
              96   52 - #010101
             104   52 - #969ba4
             108   52 - #000000
             116   52 - #000000
              36   56 - #000000
              40   56 - #000000
              68   56 - #000000
             116   56 - #010101
             436   56 - #e8f0fe
             580   76 - #e8f0fe
              60   80 - #bbc1cc
             104   80 - #56585e
             176   80 - #242628
             208   80 - #d9e0ed
             260   80 - #e8f0fe
             268   80 - #9398a1
             316   80 - #c4cad6
              36   84 - #e8f0fe
              60   84 - #bbc1cc
              80   84 - #e8f0fe
             104   84 - #56585e
             124   84 - #e8f0fe
             176   84 - #242628
             184   84 - #e8f0fe
             212   84 - #aeb4be
             232   84 - #c9d0dc
             240   84 - #a5aab4
             256   84 - #e8f0fe
             272   84 - #e8f0fe
             288   84 - #e8f0fe
             316   84 - #c4cad6
             336   84 - #6c7077
             344   84 - #222325
             352   84 - #9fa5ae
             364   84 - #c3c9d5
             372   84 - #83888f
             384   84 - #e8f0fe
              96   88 - #e8f0fe
             108   88 - #e8f0fe
             120   88 - #e8f0fe
             204   88 - #e8f0fe
             264   88 - #e8f0fe
             292   88 - #e8f0fe
              88   96 - #e8f0fe
             104   96 - #e8f0fe
             260   96 - #e8f0fe
             276   96 - #e8f0fe
             520   96 - #e8f0fe
             112  100 - #010101
             160  100 - #606369
             164  100 - #606369
             236  100 - #e8f0fe
              84  104 - #888c94
             112  104 - #020202
             240  104 - #595c62
             632  108 - #ffffff
             448  116 - #e8f0fe
             344  132 - #e8f0fe
             308  136 - #c2c8d4
             376  136 - #e8f0fe
             308  140 - #c2c8d4
             312  140 - #e8f0fe
             332  140 - #e8f0fe
               4  144 - #ffffff
             308  144 - #c7ceda
             312  144 - #c7ceda
             260  148 - #494b50
             264  148 - #494b50
             568  152 - #e8f0fe
             496  156 - #e8f0fe
             336  160 - #e8f0fe
             280  184 - #000000
              52  188 - #c9d0dc
              56  188 - #3e4044
             124  188 - #b1b8c2
             204  188 - #d2d9e6
             220  188 - #dbe3f0
              52  192 - #c9d0dc
              56  192 - #3e4044
              60  192 - #0f0f10
             204  192 - #d2d9e6
             276  192 - #e8f0fe
             300  192 - #9ba0a9
             364  192 - #e8f0fe
             408  192 - #55585d
             424  192 - #e8f0fe
             436  192 - #e8f0fe
             268  196 - #e8f0fe
             616  208 - #e8f0fe
             524  212 - #e8f0fe
             284  228 - #47494e
             160  240 - #e8f0fe
             264  244 - #4d5055
             268  244 - #4d5055
             368  244 - #e8f0fe
             384  244 - #e8f0fe
             396  244 - #e8f0fe
             408  244 - #4d5055
             412  244 - #4d5055
               4  252 - #ffffff
             100  252 - #e8f0fe
             428  256 - #e8f0fe
             224  260 - #676a71
             344  260 - #e8f0fe
             488  260 - #e8f0fe
             216  264 - #a2a8b1
             580  264 - #e8f0fe
             144  304 - #ffffff
             396  304 - #ffffff
             528  304 - #ffffff
              60  308 - #ffffff
             632  308 - #ffffff
             300  324 - #ffffff
             452  332 - #ffffff
             580  340 - #ffffff
               4  344 - #ffffff
             228  348 - #ffffff
             372  356 - #ffffff
              72  380 - #ffffff
             144  388 - #ffffff
             544  392 - #ffffff
               4  412 - #ffffff
             208  412 - #ffffff
             284  412 - #ffffff
             348  412 - #ffffff
             456  412 - #ffffff
             632  412 - #ffffff
            ";

const LEFT: f32 = 16.0;
const TOP: f32 = 40.0;
const WIDTH: f32 = 608.0;

/// A markdown text with LaTeX math in it, on a tinted box as high as
/// `height_for_width` said, so math that runs out of the measured
/// height shows.
#[view]
struct MarkdownMath {
    height: f32,

    #[init]
    caption:    Label,
    background: Container,
    text:       MarkdownView,
}

impl Setup for MarkdownMath {
    fn setup(mut self: Weak<Self>) {
        self.set_color(WHITE);
        self.caption
            .set_text_size(13)
            .set_alignment(TextAlignment::Left)
            .set_vertical_alignment(VerticalAlignment::Top);
        self.caption
            .set_text("inline formulas, 2 display formulas, 1 broken formula as code");
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

/// Where a formula view is in its markdown view: its left edge, its
/// baseline, its right edge and its bottom.
struct Placed {
    x:        f32,
    baseline: f32,
    right:    f32,
    bottom:   f32,
}

/// Where the room of `formula` is in the label that holds it: the left
/// edge and the baseline of its line, in the markdown view.
fn room_of(label: Weak<Label>, formula: &str) -> Option<(f32, f32)> {
    let start = label.text().find(formula)?;
    let layout = label.text_layout_for(label.text());
    let line = layout.line_of(start);
    let frame = label.frame();
    let x = frame.origin.x + label.text_inset() + layout.x_on_line(line, start);
    let baseline = frame.origin.y + layout.line_top(line) + layout.ascent;
    Some((x, baseline))
}

impl ViewTest for MarkdownMath {
    fn canvas() -> (u32, u32) {
        (640, 420)
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        set_record_probe_count(160);
        check_colors(COLORS)?;

        let (maths, rooms, broken, bottom) = from_main(move || {
            let subviews = view.text.subviews();
            let maths: Vec<Placed> = subviews
                .iter()
                .filter_map(|child| child.downcast_view::<MathView>())
                .map(|math| Placed {
                    x:        math.x(),
                    baseline: math.y() + math.baseline(),
                    right:    math.max_x(),
                    bottom:   math.max_y(),
                })
                .collect();
            let labels: Vec<Weak<Label>> =
                subviews.iter().filter_map(|child| child.downcast_view::<Label>()).collect();
            let rooms: Vec<Option<(f32, f32)>> = INLINE
                .iter()
                .map(|formula| labels.iter().find_map(|label| room_of(*label, formula)))
                .collect();
            let broken = labels.iter().any(|label| label.text().contains(BROKEN));
            let bottom = subviews.iter().map(|child| child.max_y()).fold(0.0, f32::max);
            (maths, rooms, broken, bottom)
        });

        ensure!(
            maths.len() == INLINE.len() + DISPLAY,
            "{} formula views, not {}",
            maths.len(),
            INLINE.len() + DISPLAY
        );
        ensure!(broken, "the broken formula is not in a label as its text");

        // Every inline formula starts where its room starts, on the
        // baseline of its line.
        for (formula, room) in INLINE.iter().zip(rooms) {
            let Some((x, baseline)) = room else {
                anyhow::bail!("no label holds the room of {formula}");
            };
            let placed = maths
                .iter()
                .find(|math| (math.x - x).abs() < 0.5 && (math.baseline - baseline).abs() < 0.5);
            ensure!(
                placed.is_some(),
                "no formula view at {x}, {baseline}, the room of {formula}"
            );
        }

        // A display formula is centered in the column.
        let center = WIDTH / 2.0;
        let centered = maths
            .iter()
            .filter(|math| (f32::midpoint(math.x, math.right) - center).abs() < 1.0)
            .count();
        ensure!(
            centered >= DISPLAY,
            "{centered} formulas are centered, not {DISPLAY}"
        );

        // Nothing runs out of the measured height.
        let lowest = maths.iter().map(|math| math.bottom).fold(bottom, f32::max);
        let height = from_main(move || view.height);
        ensure!(
            lowest <= height + 0.01,
            "a view ends at {lowest}, the measured height is {height}"
        );

        Ok(())
    }
}
