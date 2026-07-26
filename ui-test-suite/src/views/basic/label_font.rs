use anyhow::Result;
use test_engine::{
    AppRunner,
    dispatch::from_main,
    refs::{Weak, manage::DataManager},
    ui::{Font, Label, Screenshot, Setup, U8Color, ViewFrame, ViewTest, view},
    ui_test::check_colors,
};

const TEXT: &str = "Grumpy wizards 123";

const DEFAULT_FRAME: (u32, u32, u32, u32) = (20, 20, 400, 80);
const CUSTOM_FRAME: (u32, u32, u32, u32) = (20, 120, 400, 80);

#[view]
struct LabelFont {
    #[init]
    default_label: Label,
    custom_label:  Label,
}

impl Setup for LabelFont {
    fn setup(self: Weak<Self>) {
        self.default_label.set_frame(DEFAULT_FRAME);
        self.default_label.set_text(TEXT).set_text_size(50);

        self.custom_label.set_frame(CUSTOM_FRAME);
        self.custom_label.set_text(TEXT).set_text_size(50);
    }
}

impl ViewTest for LabelFont {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        default_font_colors()?;

        let original = AppRunner::take_screenshot()?;

        let custom_font = set_font_changes_one_label(view, &original)?;
        set_default_changes_default_label(&custom_font)?;
        reset_default_restores_original(&original)?;

        Ok(())
    }
}

fn default_font_colors() -> Result<()> {
    check_colors(
        r"
             592    4 - #597c95
             380   44 - #000000
              88   48 - #000000
             260   52 - #000000
             416   52 - #000000
             128   60 - #597c95
             196   60 - #000000
             232   60 - #597c95
             316   64 - #597c95
              52   68 - #000000
             288   68 - #000000
             340   68 - #597c95
             380   68 - #000000
             380  144 - #000000
              88  148 - #000000
             416  148 - #000000
             224  152 - #000000
             132  160 - #597c95
             188  160 - #000000
             252  160 - #597c95
             268  164 - #597c95
              52  168 - #000000
             340  168 - #597c95
             380  168 - #000000
             312  172 - #000000
             592  336 - #597c95
             412  344 - #597c95
              96  388 - #597c95
             300  484 - #597c95
               4  592 - #597c95
             172  592 - #597c95
             592  592 - #597c95
            ",
    )
}

fn set_font_changes_one_label(view: Weak<LabelFont>, original: &Screenshot) -> Result<Screenshot> {
    from_main(move || {
        view.custom_label.set_font(Font::get("OpenSans.ttf"));
    });

    check_colors(
        r"
             224   40 - #000000
             380   44 - #000000
              88   48 - #000000
             288   52 - #000000
             416   52 - #000000
             128   60 - #597c95
             188   60 - #000000
             252   60 - #597c95
             316   64 - #597c95
              52   68 - #000000
             340   68 - #597c95
             156   72 - #000000
             592  132 - #597c95
              80  156 - #000000
             300  156 - #000000
             216  160 - #000000
             124  164 - #597c95
             260  164 - #597c95
             332  164 - #597c95
             156  172 - #000000
             360  172 - #597c95
             404  172 - #000000
              40  176 - #000000
             244  176 - #000000
             276  176 - #000000
             592  324 - #597c95
             192  364 - #597c95
               4  412 - #597c95
             376  436 - #597c95
             160  592 - #597c95
             324  592 - #597c95
             592  592 - #597c95
            ",
    )?;

    let custom_font = AppRunner::take_screenshot()?;

    assert!(
        region(&custom_font, CUSTOM_FRAME) != region(original, CUSTOM_FRAME),
        "set_font did not change the label rendering"
    );
    assert!(
        region(&custom_font, DEFAULT_FRAME) == region(original, DEFAULT_FRAME),
        "set_font on one label changed another label"
    );

    Ok(custom_font)
}

fn set_default_changes_default_label(custom_font: &Screenshot) -> Result<()> {
    from_main(|| {
        Font::set_default(Font::get("DroidSansMono.ttf"));
    });

    check_colors(
        r"
             416   44 - #000000
             124   52 - #000000
             204   52 - #000000
             288   52 - #000000
             360   52 - #000000
              80   60 - #597c95
             164   64 - #000000
             244   64 - #597c95
             328   64 - #597c95
             268   68 - #597c95
              44   72 - #000000
             592  124 - #597c95
             340  144 - #000000
              24  156 - #000000
             164  156 - #000000
             244  160 - #597c95
             124  164 - #597c95
             328  168 - #597c95
             196  172 - #000000
             360  172 - #597c95
             404  172 - #000000
              44  176 - #000000
              84  176 - #000000
             276  176 - #000000
             148  188 - #000000
             592  340 - #597c95
             416  352 - #597c95
             180  360 - #597c95
               4  376 - #597c95
             300  488 - #597c95
               4  592 - #597c95
             592  592 - #597c95
            ",
    )?;

    let custom_default = AppRunner::take_screenshot()?;

    assert!(
        region(&custom_default, DEFAULT_FRAME) != region(custom_font, DEFAULT_FRAME),
        "set_default did not change the default label rendering"
    );
    assert!(
        region(&custom_default, CUSTOM_FRAME) == region(custom_font, CUSTOM_FRAME),
        "set_default changed a label with its own font"
    );

    Ok(())
}

fn reset_default_restores_original(original: &Screenshot) -> Result<()> {
    from_main(Font::reset_default);

    check_colors(
        r"
             224   40 - #000000
             380   44 - #000000
              88   48 - #000000
             288   52 - #000000
             416   52 - #000000
             128   60 - #597c95
             188   60 - #000000
             252   60 - #597c95
             316   64 - #597c95
              52   68 - #000000
             340   68 - #597c95
             156   72 - #000000
             592  132 - #597c95
              80  156 - #000000
             300  156 - #000000
             216  160 - #000000
             124  164 - #597c95
             260  164 - #597c95
             332  164 - #597c95
             156  172 - #000000
             360  172 - #597c95
             404  172 - #000000
              40  176 - #000000
             244  176 - #000000
             276  176 - #000000
             592  324 - #597c95
             192  364 - #597c95
               4  412 - #597c95
             376  436 - #597c95
             160  592 - #597c95
             324  592 - #597c95
             592  592 - #597c95
            ",
    )?;

    let restored = AppRunner::take_screenshot()?;

    assert!(
        region(&restored, DEFAULT_FRAME) == region(original, DEFAULT_FRAME),
        "reset_default did not restore the original rendering"
    );

    Ok(())
}

fn region(shot: &Screenshot, frame: (u32, u32, u32, u32)) -> Vec<U8Color> {
    let (x, y, width, height) = frame;
    let mut pixels = Vec::with_capacity((width * height) as usize);

    for py in y..y + height {
        for px in x..x + width {
            pixels.push(shot.get_pixel((px, py)));
        }
    }

    pixels
}
