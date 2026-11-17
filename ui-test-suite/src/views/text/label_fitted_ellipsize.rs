use anyhow::{Result, bail};
use hilen::{
    AppRunner,
    dispatch::from_main,
    refs::Weak,
    ui::{
        BLACK, Font, Label, Screenshot, Setup, TextAlignment, U8Color, ViewData, ViewFrame, ViewTest, WHITE,
        view,
    },
    ui_test::{check_colors, checkpoint},
};

const WORDS: [&str; 16] = [
    "Nexus",
    "Hexium",
    "Thunderstore",
    "Join the Discord",
    "Sign out",
    "Report a bug",
    "Go to the forum",
    "What we keep about you",
    "Settings",
    "Newer",
    "Older",
    "Like",
    "Pick",
    "All feedback",
    "Whole forum",
    "carl",
];
const SIZES: [f32; 5] = [12.0, 13.0, 14.0, 15.0, 17.0];
/// The tops of the 3 rows, in points.
const ROWS: [u32; 3] = [48, 82, 116];
/// The left edges of the 2 columns. A region of 240 by 28 at the same top
/// holds 1 label and the ground around it in each.
const FITTED_X: u32 = 20;
const TWIN_X: u32 = 300;

const FITTED: &str = r"
              68   24 - #809bae
              40   28 - #a5b8c5
              80   28 - #597c95
              92   28 - #d3dce3
             108   28 - #597c95
             304   28 - #597c95
             336   28 - #7693a8
              28   64 - #181818
             308   64 - #181818
              72   92 - #ffffff
             128   92 - #141414
             348   92 - #ffffff
             408   92 - #141414
              56   96 - #a8a8a8
             116   96 - #d1d1d1
             396   96 - #d1d1d1
              32  100 - #010101
             312  100 - #010101
             392  128 - #000000
              32  132 - #656565
             112  132 - #000000
             160  132 - #e6e7e8
             300  132 - #ffffff
             312  132 - #656565
             420  132 - #ffffff
             440  132 - #e6e7e8
              72  140 - #ffffff
             364  140 - #ffffff
             440  140 - #f9fafb
             332  440 - #597c95
              72  592 - #597c95
             592  592 - #597c95
";

fn fonts() -> [Weak<Font>; 4] {
    [Font::roboto(), Font::medium(), Font::bold(), Font::mono()]
}

/// Lays a label out as wide as its measured text, the way a button sizes
/// itself to its title.
fn fit(label: Weak<Label>, x: u32, y: u32) {
    let width = label.size_for_width(1000.0).width;
    label.set_frame((x, y, width, 28.0));
}

#[view]
struct LabelFittedEllipsize {
    #[init]
    fitted: Label,
    plain:  Label,
    probe:  Label,
    cut_1:  Label,
    cut_2:  Label,
    cut_3:  Label,
    twin_1: Label,
    twin_2: Label,
    twin_3: Label,
}

/// A fitted label with ellipsis on, its plain twin, the text, the face and
/// the size.
type Row = (Weak<Label>, Weak<Label>, &'static str, Weak<Font>, f32);

impl LabelFittedEllipsize {
    fn rows(self: Weak<Self>) -> [Row; 3] {
        [
            (self.cut_1, self.twin_1, "Nexus", Font::medium(), 14.0),
            (self.cut_2, self.twin_2, "Join the Discord", Font::bold(), 15.0),
            (
                self.cut_3,
                self.twin_3,
                "What we keep about you",
                Font::roboto(),
                13.0,
            ),
        ]
    }
}

impl Setup for LabelFittedEllipsize {
    fn setup(self: Weak<Self>) {
        self.probe.set_hidden(true);
        for (caption, text, x) in [
            (self.fitted, "fitted, ellipsis on", FITTED_X),
            (self.plain, "plain twin", TWIN_X),
        ] {
            caption.set_text(text).set_text_size(14).set_text_color(WHITE);
            caption.set_alignment(TextAlignment::Left).set_text_inset(0.0);
            caption.set_frame((x, 16, 240, 22));
        }
        for ((cut, twin, text, font, size), y) in self.rows().into_iter().zip(ROWS) {
            for label in [cut, twin] {
                label.set_color(WHITE).set_text_color(BLACK).set_text_size(size).set_font(font);
                label.set_alignment(TextAlignment::Left).set_text_inset(0.0);
                label.set_text(text);
            }
            cut.set_ellipsize(true);
            fit(cut, FITTED_X, y);
            fit(twin, TWIN_X, y);
        }
    }
}

impl ViewTest for LabelFittedEllipsize {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        // Every word in every face and size keeps its whole text in a
        // frame exactly as wide as its measure, with and without the
        // default inset.
        let cut: Vec<String> = from_main(move || {
            let mut cut = Vec::new();
            for font in fonts() {
                for size in SIZES {
                    for inset in [0.0, 8.0] {
                        for word in WORDS {
                            let label = view.probe;
                            label.set_font(font).set_text_size(size).set_text_inset(inset);
                            label.set_alignment(TextAlignment::Left).set_ellipsize(true);
                            label.set_text(word);
                            fit(label, 0, 0);
                            let drawn = label.display_text(label.width()).to_string();
                            if drawn != word {
                                cut.push(format!("{word:?} at {size} inset {inset} drew {drawn:?}"));
                            }
                        }
                    }
                }
            }
            cut
        });
        if !cut.is_empty() {
            bail!("fitted labels were cut:\n{}", cut.join("\n"));
        }

        // What the drawer paints matches a plain label with the same text.
        let shot = AppRunner::take_screenshot()?;
        for y in ROWS {
            if !regions_match(&shot, (FITTED_X, y, 240, 28), (TWIN_X, y, 240, 28)) {
                bail!("the fitted label at the top {y} draws other pixels than its plain twin");
            }
        }
        check_colors(FITTED)?;
        checkpoint("fitted labels with ellipsis on the left, plain twins on the right, all whole")?;

        Ok(())
    }
}

/// Pixel equality with one level of slack per channel, a software
/// rasterizer rounds an anti aliased edge one level differently at
/// another place.
fn regions_match(shot: &Screenshot, a: (u32, u32, u32, u32), b: (u32, u32, u32, u32)) -> bool {
    region(shot, a)
        .into_iter()
        .zip(region(shot, b))
        .all(|(x, y)| channel_gap(x, y) <= 1)
}

fn channel_gap(a: U8Color, b: U8Color) -> u8 {
    [
        a.r.abs_diff(b.r),
        a.g.abs_diff(b.g),
        a.b.abs_diff(b.b),
        a.a.abs_diff(b.a),
    ]
    .into_iter()
    .fold(0, u8::max)
}

fn region(shot: &Screenshot, frame: (u32, u32, u32, u32)) -> Vec<U8Color> {
    let (x, y, w, h) = frame;
    let mut pixels = Vec::with_capacity((w * h) as usize);

    for row in y..y + h {
        for col in x..x + w {
            pixels.push(shot.get_pixel((col, row)));
        }
    }

    pixels
}
