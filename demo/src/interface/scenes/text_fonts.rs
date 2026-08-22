use hilen::{
    gm::LossyConvert,
    refs::{Weak, manage::DataManager},
    ui::{Font, Label, ScrollView, Setup, ViewData, ViewSubviews, view},
};

use crate::interface::{
    palette::{BG, TEXT, TEXT_DIM},
    scenes::{HEADER_HEIGHT, add_header},
};

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
        self.set_color(BG);

        self.scroll.place().t(HEADER_HEIGHT).lrb(0);

        for (i, (font, sample)) in FONTS.into_iter().enumerate() {
            let y = 16.0 + 40.0 * i.lossy_convert();
            let label = self.scroll.add_view::<Label>();
            label
                .set_text(sample)
                .set_text_color(TEXT)
                .set_text_size(22)
                .set_font(Font::get(font));
            label.place().t(y).l(16).r(16).h(34);
        }

        let spacing = self.scroll.add_view::<Label>();
        spacing
            .set_text("LETTER SPACING")
            .set_text_color(TEXT_DIM)
            .set_text_size(18)
            .set_letter_spacing(6);
        spacing.place().t(226).l(16).r(16).h(30);

        let multiline = self.scroll.add_view::<Label>();
        multiline
            .set_text(
                "This is a multiline label. It wraps across several lines when the text is longer than its width, so paragraphs stay readable.",
            )
            .set_text_color(TEXT)
            .set_text_size(18)
            .set_multiline(true);
        multiline.place().t(266).l(16).r(16).h(180);

        add_header(self, "Text and Fonts");
    }
}
