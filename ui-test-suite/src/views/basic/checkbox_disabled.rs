use anyhow::Result;
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    text::Font,
    ui::{
        CheckBox, CheckBoxMark, Color, DynamicColor, Label, Setup, Theme, ThemeMode, ViewData, ViewTest, view,
    },
    ui_test::{check_colors, inject_touches, set_record_probe_count},
};

const PAGE: DynamicColor = DynamicColor::new(Color::hex("#ffffff"), Color::hex("#16161a"));
const TEXT: DynamicColor = DynamicColor::new(Color::hex("#1c1c1e"), Color::hex("#f2f2f4"));
const BOX_FILL: DynamicColor = DynamicColor::new(Color::hex("#ffffff"), Color::hex("#232326"));
const BOX_BORDER: DynamicColor = DynamicColor::new(Color::hex("#8e8e93"), Color::hex("#7c7c82"));
const MARK: DynamicColor = DynamicColor::new(Color::hex("#2563eb"), Color::hex("#60a5fa"));

/// Big enough that the stroke of the check has pixels away from its edges,
/// the probe picker skips the edges of an svg.
const BOX: f32 = 90.0;
const BOX_TOP: f32 = 60.0;
/// The left edges of the 4 columns.
const COLUMNS: [f32; 4] = [35.0, 175.0, 315.0, 455.0];

/// The opt in check mark and the disabled state of `CheckBox`. The dot box
/// keeps the engine look, so an app that sets nothing looks the same as
/// before. A disabled box is faint and a tap leaves it as it was, enabled
/// again it takes the tap. The themed boxes turn dark with the page.
#[view]
struct CheckBoxDisabled {
    #[init]
    dot:                CheckBox,
    check:              CheckBox,
    disabled_on:        CheckBox,
    disabled_off:       CheckBox,
    dot_label:          Label,
    check_label:        Label,
    disabled_on_label:  Label,
    disabled_off_label: Label,
    status:             Label,
}

impl Setup for CheckBoxDisabled {
    fn setup(mut self: Weak<Self>) {
        self.set_color(PAGE);

        let boxes = [self.dot, self.check, self.disabled_on, self.disabled_off];
        let labels = [
            (self.dot_label, "dot"),
            (self.check_label, "check"),
            (self.disabled_on_label, "disabled on"),
            (self.disabled_off_label, "disabled off"),
        ];
        for ((check_box, (label, text)), left) in boxes.into_iter().zip(labels).zip(COLUMNS) {
            check_box.place().t(BOX_TOP).l(left).size(BOX, BOX);
            label.set_text(text).set_text_size(16).set_text_color(TEXT);
            label.place().t(BOX_TOP + BOX + 10.0).l(left - 20.0).size(BOX + 40.0, 24);
        }

        for check_box in [self.check, self.disabled_on, self.disabled_off] {
            check_box.set_mark(CheckBoxMark::Check);
            check_box.set_colors(BOX_FILL, BOX_BORDER, MARK);
        }
        self.dot.set_on(true);
        self.check.set_on(true);
        self.disabled_on.set_on(true);
        self.disabled_on.set_enabled(false);
        self.disabled_off.set_enabled(false);

        self.status.set_text("light, 2 boxes disabled").set_text_size(18);
        self.status.set_font(Font::bold()).set_text_color(TEXT);
        self.status.place().t(220).lr(20).h(30);
    }
}

impl ViewTest for CheckBoxDisabled {
    fn canvas() -> (u32, u32) {
        (600, 280)
    }

    fn before_start() {
        from_main(|| Theme::set_mode(ThemeMode::Light));
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        set_record_probe_count(64);
        assert!(!view.disabled_on.is_enabled());
        check_colors(LIGHT)?;

        // A tap on a disabled box changes nothing, on or off.
        tap(COLUMNS[2]);
        tap(COLUMNS[3]);
        assert!(view.disabled_on.on());
        assert!(!view.disabled_off.on());

        from_main(move || {
            view.disabled_on.set_enabled(true);
            view.status.set_text("enabled again, a tap turned it off");
        });
        tap(COLUMNS[2]);
        assert!(view.disabled_on.is_enabled());
        assert!(!view.disabled_on.on());
        check_colors(ENABLED_AGAIN)?;

        from_main(move || {
            view.status.set_text("dark, the themed boxes follow");
            Theme::set_mode(ThemeMode::Dark);
        });
        wait_for_next_frame();
        check_colors(DARK)?;

        from_main(|| Theme::set_mode(ThemeMode::System));
        Ok(())
    }
}

/// Taps near the top right corner of a box, inside it but away from the
/// mark, so the marker of human mode does not cover a probe.
fn tap(left: f32) {
    let x = left + BOX - 8.0;
    let y = BOX_TOP + 8.0;
    inject_touches(format!("{x:.0} {y:.0} b\n{x:.0} {y:.0} e"));
    wait_for_next_frame();
}

const LIGHT: &str = r"
    332    4 - #ffffff
    592    4 - #ffffff
    80   60 - #464e61
    188   60 - #8e8e93
    220   60 - #8e8e93
    252   60 - #8e8e93
    392   60 - #d1d1d6
    500   60 - #d1d1d6
    44   64 - #273549
    116   64 - #273549
    124   80 - #464e61
    316   84 - #d1d1d6
    76   88 - #5894f2
    96   92 - #5894f2
    264   92 - #8e8e93
    36   96 - #464e61
    84  100 - #5894f2
    120  100 - #273549
    64  104 - #5894f2
    544  104 - #d1d1d6
    456  108 - #d1d1d6
    92  112 - #5894f2
    176  112 - #8e8e93
    48  120 - #273549
    72  120 - #5894f2
    404  120 - #d1d1d6
    124  124 - #464e61
    264  128 - #8e8e93
    88  132 - #273549
    36  140 - #454d60
    64  144 - #273549
    112  148 - #464e61
    76  168 - #d6d6d6
    208  168 - #787879
    360  168 - #949495
    488  168 - #bababa
    516  168 - #373738
    536  168 - #ffffff
    72  172 - #ffffff
    76  172 - #d6d6d6
    88  172 - #373738
    204  172 - #ffffff
    220  172 - #8e8e8f
    232  172 - #ffffff
    320  172 - #1c1c1e
    352  172 - #ffffff
    360  172 - #949495
    388  172 - #ffffff
    464  172 - #1c1c1e
    488  172 - #bababa
    504  172 - #8e8e8f
    212  232 - #1c1c1e
    312  232 - #1c1c1e
    356  232 - #a0a0a1
    228  236 - #8e8e8f
    284  236 - #ffffff
    356  236 - #a0a0a1
    388  236 - #1c1c1e
    260  240 - #1d1d1f
    340  240 - #1d1d1f
    4  272 - #ffffff
    128  272 - #ffffff
    484  272 - #ffffff
    592  272 - #ffffff
";

const ENABLED_AGAIN: &str = r"
    252    4 - #ffffff
    592    4 - #ffffff
    80   60 - #464e61
    196   60 - #8e8e93
    328   60 - #8e8e93
    392   60 - #8e8e93
    500   60 - #d1d1d6
    44   64 - #273549
    116   64 - #273549
    456   68 - #d1d1d6
    544   72 - #d1d1d6
    76   88 - #5894f2
    96   92 - #5894f2
    36   96 - #464e61
    264   96 - #8e8e93
    64  104 - #5894f2
    120  104 - #273549
    456  108 - #d1d1d6
    96  116 - #5894f2
    316  116 - #8e8e93
    544  116 - #d1d1d6
    48  120 - #273549
    76  120 - #5894f2
    124  132 - #464e61
    84  136 - #273549
    36  140 - #454d60
    60  144 - #273549
    104  148 - #464e61
    76  168 - #d6d6d6
    208  168 - #787879
    360  168 - #949495
    488  168 - #bababa
    516  168 - #373738
    536  168 - #ffffff
    72  172 - #ffffff
    88  172 - #373738
    220  172 - #8e8e8f
    232  172 - #ffffff
    320  172 - #1c1c1e
    352  172 - #ffffff
    360  172 - #949495
    388  172 - #ffffff
    464  172 - #1c1c1e
    488  172 - #bababa
    504  172 - #8e8e8f
    232  232 - #c7c7c7
    308  232 - #1c1c1e
    172  236 - #1c1c1e
    212  236 - #c5c5c6
    256  236 - #787879
    268  236 - #646465
    280  236 - #1d1d1f
    336  236 - #e4e4e5
    344  236 - #282829
    352  236 - #a1a1a1
    364  236 - #d2d2d3
    396  236 - #c5c5c6
    412  236 - #ffffff
    428  236 - #5f5f60
    192  240 - #1d1d1f
    508  264 - #ffffff
    4  272 - #ffffff
    92  272 - #ffffff
    592  272 - #ffffff
";

const DARK: &str = r"
    188    4 - #16161a
    412    4 - #16161a
    252   60 - #7c7c82
    344   60 - #7c7c82
    484   60 - #48484e
    532   60 - #48484e
    44   64 - #273549
    116   64 - #273549
    456   68 - #48484e
    404   72 - #7c7c82
    76   88 - #5894f2
    176   88 - #7c7c82
    96   92 - #5894f2
    456  100 - #48484e
    64  104 - #5894f2
    84  104 - #5894f2
    316  108 - #7c7c82
    544  116 - #48484e
    68  120 - #5894f2
    88  120 - #5894f2
    120  124 - #273549
    264  128 - #7c7c82
    456  136 - #48484e
    40  140 - #273549
    68  140 - #273549
    100  144 - #273549
    76  168 - #3e3e41
    208  168 - #99999c
    360  168 - #7e7e81
    488  168 - #59595d
    516  168 - #d8d8da
    536  168 - #16161a
    72  172 - #16161a
    76  172 - #3e3e41
    88  172 - #d8d8da
    220  172 - #848487
    232  172 - #16161a
    320  172 - #f2f2f4
    352  172 - #16161a
    360  172 - #7e7e81
    388  172 - #16161a
    464  172 - #f2f2f4
    488  172 - #59595d
    504  172 - #848487
    208  232 - #5b5b5e
    260  232 - #525256
    320  232 - #88888b
    392  232 - #4e4e52
    180  236 - #f1f1f3
    224  236 - #46464a
    260  236 - #525256
    268  236 - #e6e6e8
    280  236 - #727275
    320  236 - #88888b
    348  236 - #16161a
    376  236 - #434346
    392  236 - #4e4e52
    400  236 - #f2f2f4
    244  240 - #f2f2f4
    300  240 - #f1f1f3
    4  272 - #16161a
    96  272 - #16161a
    492  272 - #16161a
    592  272 - #16161a
";
