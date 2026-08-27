use anyhow::Result;
use hilen::{
    AppRunner,
    refs::{Weak, manage::DataManager},
    ui::{Font, Screenshot, Setup, TextField, U8Color, ViewFrame, ViewTest, view},
    ui_test::{check_colors, set_record_probe_count},
};

const FIELDS: &str = r"
             440    4 - #597c95
             568    4 - #597c95
              24   20 - #ffffff
              80   20 - #ffffff
             132   20 - #ffffff
             180   20 - #ffffff
             228   20 - #ffffff
             284   20 - #ffffff
             316   20 - #ffffff
              56   24 - #ffffff
             208   28 - #000000
             104   32 - #000000
             244   32 - #e6e6e6
             256   32 - #ffffff
             300   32 - #ffffff
              88   36 - #000000
             148   36 - #000000
             156   36 - #000000
             168   36 - #aaaaaa
             192   36 - #000000
             240   36 - #ffffff
             244   36 - #e6e6e6
             252   36 - #ffffff
             256   36 - #ffffff
              84   40 - #ffffff
              96   40 - #000000
             104   40 - #000000
             128   40 - #ffffff
             140   40 - #ffffff
             168   40 - #aaaaaa
             184   40 - #000000
             200   40 - #ffffff
             208   40 - #000000
             240   40 - #ffffff
             244   40 - #e6e6e6
             252   40 - #ffffff
             280   40 - #ffffff
              76   44 - #010101
             144   44 - #ffffff
             156   44 - #000000
             168   44 - #aaaaaa
             180   44 - #ffffff
             192   44 - #ffffff
             196   44 - #ffffff
             200   44 - #000000
             232   44 - #010101
             236   44 - #010101
             240   44 - #010101
             316   44 - #ffffff
              20   48 - #ffffff
              88   48 - #000000
              96   48 - #000000
             104   48 - #000000
             136   48 - #ffffff
             168   48 - #aaaaaa
             184   48 - #000000
             208   48 - #000000
             212   48 - #ffffff
             216   48 - #ffffff
             244   48 - #e6e6e6
             252   48 - #010101
             256   48 - #909090
             260   48 - #909090
             148   52 - #010101
             296   52 - #ffffff
              44   56 - #ffffff
              84   56 - #000000
              44   80 - #ffffff
              84   80 - #ffffff
             120   80 - #ffffff
             144   80 - #ffffff
             192   80 - #ffffff
             248   80 - #ffffff
             316   80 - #ffffff
              20   88 - #ffffff
             220   88 - #828282
             220   92 - #828282
             284   92 - #ffffff
             156   96 - #000000
             164   96 - #000000
             176   96 - #000000
             204   96 - #000000
             220   96 - #828282
             232   96 - #000000
             268   96 - #000000
             304   96 - #ffffff
              56  100 - #ffffff
              72  100 - #d6d6d6
              92  100 - #3a3a3a
             144  100 - #000000
             148  100 - #ffffff
             152  100 - #ffffff
             164  100 - #000000
             176  100 - #000000
             212  100 - #ffffff
             220  100 - #828282
             232  100 - #000000
             264  100 - #ffffff
             268  100 - #000000
             280  100 - #ffffff
              72  104 - #d6d6d6
              92  104 - #3a3a3a
             144  104 - #010101
             148  104 - #ffffff
             152  104 - #ffffff
             156  104 - #010101
             164  104 - #000000
             176  104 - #000000
             188  104 - #ffffff
             212  104 - #010101
             220  104 - #828282
             232  104 - #000000
             260  104 - #000000
             264  104 - #000000
             268  104 - #000000
             272  104 - #000000
             156  108 - #010101
             176  108 - #000000
             212  108 - #010101
             220  108 - #828282
             224  108 - #ffffff
             232  108 - #000000
             268  108 - #000000
             280  108 - #010101
             284  108 - #7c7c7c
             288  108 - #7c7c7c
              48  112 - #000000
              60  112 - #000000
             156  112 - #010101
              20  116 - #ffffff
             120  116 - #ffffff
             156  116 - #010101
             248  116 - #ffffff
             316  116 - #ffffff
             592  116 - #597c95
             480  136 - #597c95
             308  220 - #597c95
             120  232 - #597c95
               4  244 - #597c95
             592  252 - #597c95
             396  272 - #597c95
             224  280 - #597c95
             500  312 - #597c95
             312  332 - #597c95
              68  356 - #597c95
             592  360 - #597c95
             400  388 - #597c95
             508  424 - #597c95
             300  440 - #597c95
             164  456 - #597c95
               4  464 - #597c95
             592  488 - #597c95
             388  492 - #597c95
             496  532 - #597c95
              96  540 - #597c95
             300  552 - #597c95
               4  592 - #597c95
             192  592 - #597c95
             404  592 - #597c95
             592  592 - #597c95
";

const DEFAULT_FRAME: (u32, u32, u32, u32) = (20, 20, 300, 40);
const MONO_FRAME: (u32, u32, u32, u32) = (20, 80, 300, 40);

/// `TextField::set_font` renders the field's text in the given face, the
/// squash textarea of a port is mono like its original.
#[view]
struct TextFieldFont {
    #[init]
    default_font: TextField,
    mono:         TextField,
}

impl Setup for TextFieldFont {
    fn setup(self: Weak<Self>) {
        self.default_font.set_frame(DEFAULT_FRAME);
        self.mono.set_frame(MONO_FRAME);
        self.mono.set_font(Font::get("DroidSansMono.ttf"));
        for field in [self.default_font, self.mono] {
            field.set_text("git squash 42");
        }
    }
}

impl ViewTest for TextFieldFont {
    fn perform_test(_: Weak<Self>) -> Result<()> {
        set_record_probe_count(160);
        // Same text, same frame size, different face: the mono field must
        // not render like the default one.
        let shot = AppRunner::take_screenshot()?;
        assert!(
            region(&shot, DEFAULT_FRAME) != region(&shot, MONO_FRAME),
            "mono field rendered identically to the default font"
        );

        // Both drew something at all.
        let pixels = region(&shot, MONO_FRAME);
        let first = pixels[0];
        assert!(
            pixels.iter().any(|p| *p != first),
            "mono field rendered flat, no glyphs drawn"
        );

        check_colors(FIELDS)?;

        Ok(())
    }
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
