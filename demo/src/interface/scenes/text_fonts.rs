use std::ops::Range;

use hilen::{
    gm::{Animation, LossyConvert},
    refs::{Weak, manage::DataManager},
    ui::{
        Font, Label, RunStyle, ScrollView, Setup, UIAnimation, VerticalAlignment, ViewData, ViewFrame,
        ViewSubviews, view,
    },
};

use crate::interface::{
    palette::{BORDER, TEXT, TEXT_DIM},
    scenes::{HEADER_HEIGHT, add_title},
};

// Each script past Latin draws in its own font through a font run.
const JAPANESE: &str = "日本語の文字も同じ行の中で折り返します。";
const THAI: &str = "ข้อความภาษาไทยก็ตัดบรรทัดได้เช่นกัน ";
const KOREAN: &str = "한국어 문장도 단어 단위로 줄이 바뀝니다.";

const WRAPPING_TEXT: &str = concat!(
    "This label wraps its text to the width it has, so long paragraphs stay readable. ",
    "日本語の文字も同じ行の中で折り返します。",
    "ข้อความภาษาไทยก็ตัดบรรทัดได้เช่นกัน ",
    "한국어 문장도 단어 단위로 줄이 바뀝니다.",
);

const WRAPPING_TOP: f32 = 266.0;
const WRAPPING_HEIGHT: f32 = 620.0;
const WRAPPING_MARGIN: f32 = 10.0;
const WRAPPING_MIN_WIDTH: f32 = 100.0;
const WRAPPING_HALF_CYCLE: f32 = 20.0;

/// Byte range of one script in `WRAPPING_TEXT`.
fn run(part: &str) -> Range<usize> {
    let start = WRAPPING_TEXT.find(part).expect("every part is in the wrapping text");
    start..start + part.len()
}

// Font file and a short sample. Each renders in its own typeface to show
// the shaping pipeline across very different scripts.
const FONTS: [(&str, &str); 5] = [
    ("RussoOne-Regular.ttf", "Russo One heading"),
    ("OpenSans.ttf", "Open Sans stays clean"),
    ("Monoton-Regular.ttf", "MONOTON NEON 88"),
    ("Neucha.ttf", "Neucha handwriting"),
    ("SpecialElite-Regular.ttf", "Special Elite keys"),
];

/// A scrollable stack of labels, each in a different font, plus a letter
/// spacing row and a multiline row. Sizes are picked to fit a 320 point
/// wide phone screen.
#[view]
pub struct TextFonts {
    #[init]
    scroll: ScrollView,
}

impl Setup for TextFonts {
    fn setup(self: Weak<Self>) {
        self.scroll.place().t(HEADER_HEIGHT).lrb(0);

        for (i, (font, sample)) in FONTS.into_iter().enumerate() {
            let y = 16.0 + 40.0 * i.lossy_convert();
            let label = self.scroll.add_view::<Label>();
            label
                .set_text(sample)
                .set_text_color(TEXT)
                .set_text_size(22)
                .set_font(Font::get(font));
            label.place().t(y).l(28).r(28).h(34);
        }

        let spacing = self.scroll.add_view::<Label>();
        spacing
            .set_text("LETTER SPACING")
            .set_text_color(TEXT_DIM)
            .set_text_size(18)
            .set_letter_spacing(6);
        spacing.place().t(226).l(28).r(28).h(30);

        self.add_wrapping_label();

        add_title(self, "Fonts", "");
    }
}

impl TextFonts {
    /// The label breathes between the full panel width and a narrow
    /// column, so the text rewraps in front of the viewer. The frame is
    /// set on every tick from the live parent width, a window resize
    /// moves the ends with it.
    fn add_wrapping_label(mut self: Weak<Self>) {
        let label = self.scroll.add_view::<Label>();
        label
            .set_text(WRAPPING_TEXT)
            .set_text_color(TEXT)
            .set_text_size(18)
            .set_border_width(1)
            .set_border_color(BORDER)
            .set_multiline(true)
            .set_vertical_alignment(VerticalAlignment::Top);
        label.set_font_runs([
            (run(JAPANESE), RunStyle::font(Font::get("NotoSansJP-Regular.ttf"))),
            (run(THAI), RunStyle::font(Font::get("NotoSansThai.ttf"))),
            (run(KOREAN), RunStyle::font(Font::get("NotoSansKR-Regular.ttf"))),
        ]);
        self.scroll.set_content_height(WRAPPING_TOP + WRAPPING_HEIGHT + 20.0);

        let scroll = self.scroll;
        let anim = UIAnimation::new(move |label, shrink| {
            let full = scroll.width() - WRAPPING_MARGIN * 2.0;
            let width = full + (WRAPPING_MIN_WIDTH - full) * shrink;
            let x = (scroll.width() - width) / 2.0;
            label.set_frame((x, WRAPPING_TOP, width, WRAPPING_HEIGHT));
        })
        .animation(Animation::new(0.0, 1.0, WRAPPING_HALF_CYCLE))
        .repeat();
        label.add_animation(anim);
    }
}
