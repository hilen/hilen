use anyhow::{Result, anyhow, ensure};

use crate::{
    self as hilen,
    deps::{hreads::from_main, refs::Weak},
    gm::color::{BLACK, Color, WHITE},
    ui::{Font, Label, Setup, TextAlignment, ViewData, ViewFrame, ViewTest, view},
    ui_test::checkpoint,
};

/// From the Thunderstore description of the Jotunn package, as blackforge
/// shows it. The 4 IPA chars are in no bundled font, Roboto alone draws
/// them as notdef boxes.
const TEXT: &str = "Jötunn /ˈjɔːtʊn/";
const IPA: [char; 4] = ['ˈ', 'ɔ', 'ː', 'ʊ'];

const PAGE: Color = Color::hex("#e8ebef");
const MUTED: Color = Color::hex("#4a515c");

#[view]
struct SystemFontFallback {
    #[init]
    state:    Label,
    text:     Label,
    drawn_by: Label,
}

impl Setup for SystemFontFallback {
    fn setup(self: Weak<Self>) {
        self.set_color(PAGE);

        for label in [self.state, self.drawn_by] {
            label.set_text_size(22).set_text_color(MUTED).set_alignment(TextAlignment::Left);
        }

        self.state.set_frame((20, 20, 560, 30));
        self.state.set_text("Roboto label, system fallback on, the default");

        self.text.set_frame((20, 60, 560, 110));
        self.text.set_color(WHITE).set_text_color(BLACK).set_text_size(64);
        self.text.set_text(TEXT);

        let mut families: Vec<String> = ipa_fonts(&self.text)
            .iter()
            .flatten()
            .map(|font| font.family_name().unwrap_or_else(|| font.name.clone()))
            .collect();
        families.dedup();

        self.drawn_by.set_frame((20, 180, 560, 30));
        self.drawn_by.set_text(format!("IPA chars drawn by {}", families.join(", ")));
    }
}

/// The font each IPA char of the label shapes with, `None` for the
/// label's own font.
fn ipa_fonts(label: &Label) -> Vec<Option<Weak<Font>>> {
    let runs = label.shaping_runs(TEXT);
    TEXT.char_indices()
        .filter(|(_, char)| IPA.contains(char))
        .map(|(byte, _)| runs.iter().find(|run| run.range.contains(&byte)).map(|run| run.font))
        .collect()
}

impl ViewTest for SystemFontFallback {
    /// The runner turns the system fallback off for every test, the
    /// system fonts differ per machine. On before the view is built, as
    /// in an app.
    fn before_start() {
        Font::set_system_fallback(true);
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        let covered = from_main(|| IPA.iter().any(|char| Font::default().has_glyph(*char)));
        ensure!(
            !covered,
            "the default font covers an IPA char, the test shows nothing"
        );

        let fonts = from_main(move || ipa_fonts(&view.text));
        for (char, font) in IPA.iter().zip(&fonts) {
            let font = font.ok_or_else(|| anyhow!("no system font picked for {char}"))?;
            let (name, has) = from_main(move || (font.name.clone(), font.has_glyph(*char)));
            ensure!(
                name.starts_with("system "),
                "{char} went to {name}, not a system font"
            );
            ensure!(has, "{char} went to {name}, which has no glyph for it");
        }

        // Every other char stays with the label font.
        let other_runs = from_main(move || {
            let runs = view.text.shaping_runs(TEXT);
            TEXT.char_indices()
                .filter(|(_, char)| !IPA.contains(char))
                .any(|(byte, _)| runs.iter().any(|run| run.range.contains(&byte)))
        });
        ensure!(!other_runs, "a char the label font covers left it");

        // Off, the chars stay with Roboto. Switched back on in the same
        // main thread turn, no frame draws the boxes.
        let off = from_main(move || {
            Font::set_system_fallback(false);
            let off = ipa_fonts(&view.text);
            Font::set_system_fallback(true);
            off
        });
        ensure!(
            off.iter().all(Option::is_none),
            "the IPA chars left Roboto with the fallback off"
        );

        checkpoint("the 4 IPA chars draw with the named system font, no boxes")?;

        Ok(())
    }
}
