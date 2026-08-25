use anyhow::Result;
use hilen::{
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
              20   48 - #000001
             224   52 - #24323c
             296   52 - #23303a
             376   52 - #344856
             336   56 - #597c95
             124   60 - #597c95
             184   60 - #000000
             276   68 - #161f25
             288   68 - #384e5e
             224   72 - #24323c
             376   72 - #344856
              76   76 - #000000
             296  152 - #23303a
             376  152 - #344856
              92  156 - #000000
             184  156 - #000000
             336  156 - #597c95
             124  160 - #597c95
             232  160 - #597c95
             276  160 - #161f25
             276  168 - #161f25
             288  168 - #384e5e
              20  172 - #000000
             224  172 - #24323c
             376  172 - #344856
             308  176 - #000001
             412  176 - #000001
             148  184 - #000000
             592  344 - #597c95
             300  488 - #597c95
               4  592 - #597c95
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
             224   52 - #24323c
             296   52 - #23303a
             376   52 - #344856
             336   56 - #597c95
             124   60 - #597c95
             248   64 - #597c95
             276   64 - #161f25
             376   64 - #344856
             276   68 - #161f25
             288   68 - #384e5e
              20   72 - #000000
             188   72 - #000000
             288   72 - #384e5e
             376   72 - #344856
              76   76 - #000000
             336  144 - #223039
             336  148 - #223039
              96  152 - #000000
             228  156 - #283843
              64  160 - #2b3c48
             124  168 - #597c95
             192  168 - #000000
             156  172 - #000001
             360  172 - #597c95
              36  176 - #000000
             228  176 - #283843
             276  176 - #000001
             416  176 - #0b0f12
             592  348 - #597c95
             352  452 - #597c95
             112  592 - #597c95
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
             592    4 - #597c95
             124   52 - #000000
             236   52 - #000000
             360   52 - #000000
              80   60 - #597c95
             320   60 - #597c95
             196   64 - #597c95
             268   68 - #597c95
              24   76 - #000000
             416   76 - #000001
             104   88 - #000001
             336  144 - #223039
             336  148 - #223039
              96  152 - #000000
              56  156 - #1e2932
             160  156 - #000000
             228  156 - #283843
              64  164 - #2b3c48
             128  164 - #597c95
              56  172 - #1e2932
             360  172 - #597c95
              64  176 - #2b3c48
             192  176 - #000000
             228  176 - #283843
             276  176 - #000001
             416  176 - #0b0f12
             116  188 - #000000
             592  348 - #597c95
             104  404 - #597c95
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
             224   52 - #24323c
             296   52 - #23303a
             376   52 - #344856
             336   56 - #597c95
             124   60 - #597c95
             248   64 - #597c95
             276   64 - #161f25
             376   64 - #344856
             276   68 - #161f25
             288   68 - #384e5e
              20   72 - #000000
             188   72 - #000000
             288   72 - #384e5e
             376   72 - #344856
              76   76 - #000000
             336  144 - #223039
             336  148 - #223039
              96  152 - #000000
             228  156 - #283843
              64  160 - #2b3c48
             124  168 - #597c95
             192  168 - #000000
             156  172 - #000001
             360  172 - #597c95
              36  176 - #000000
             228  176 - #283843
             276  176 - #000001
             416  176 - #0b0f12
             592  348 - #597c95
             352  452 - #597c95
             112  592 - #597c95
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
