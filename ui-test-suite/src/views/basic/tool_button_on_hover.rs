use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::color::Color,
    refs::Weak,
    ui::{
        Anchor::Left, Container, DynamicColor, Label, Setup, TextAlignment, Theme, ThemeMode, ToolButton,
        ViewData, ViewFrame, ViewTest, ViewTouch, view,
    },
    ui_test::{check_colors, inject_touches},
};

const PAGE: DynamicColor = DynamicColor::new(Color::hex("#F4F5F7"), Color::hex("#1B1D21"));
const INK: DynamicColor = DynamicColor::new(Color::hex("#20232A"), Color::hex("#E8EAEE"));
const TRACK: DynamicColor = DynamicColor::new(Color::hex("#E6E7EA"), Color::hex("#262A34"));
const ON: DynamicColor = DynamicColor::new(Color::hex("#FFFFFF"), Color::hex("#4A5163"));
const ON_HOVER: DynamicColor = DynamicColor::new(Color::hex("#F0F1F4"), Color::hex("#555D71"));
const POINTER: DynamicColor = DynamicColor::new(Color::hex("#E11D48"), Color::hex("#FB7185"));

const LIGHT_AWAY: &str = r"
    392    4 - #f4f5f7
    272    8 - #f4f5f7
    48   20 - #d8d9dc
    56   20 - #cccdd0
    72   20 - #20232a
    36   52 - #94959a
    64   52 - #21242b
    88   52 - #f4f5f7
    104   52 - #c2c4c7
    136   52 - #373a41
    160   52 - #c0c1c4
    36   84 - #0a6cff
    56   84 - #66a3ff
    108   84 - #e3e4e6
    76  132 - #989a9e
    276  132 - #c3c4c8
    128  136 - #62646a
    144  136 - #c4c5c8
    148  136 - #4f5157
    196  136 - #f4f5f7
    212  136 - #989a9e
    236  136 - #62646a
    244  136 - #6a6c72
    248  136 - #6a6c72
    48  140 - #818388
    128  168 - #ffffff
    168  168 - #5b5d62
    56  172 - #24272e
    68  172 - #d7d7d9
    92  172 - #2f3238
    160  172 - #a3a4a7
    392  232 - #f4f5f7
";
const LIGHT_OFF_HOVERED: &str = r"
    268   16 - #adafb3
    60   20 - #787a7f
    72   20 - #20232a
    152   20 - #5d5f65
    212   20 - #cccdd0
    244   20 - #94959a
    36   52 - #94959a
    64   52 - #21242b
    104   52 - #c2c4c7
    136   52 - #373a41
    160   52 - #c0c1c4
    392   64 - #f4f5f7
    36   84 - #0a6cff
    56   84 - #66a3ff
    100   84 - #ececf0
    276  132 - #c3c4c8
    80  136 - #f4f5f7
    128  136 - #62646a
    144  136 - #c4c5c8
    196  136 - #f4f5f7
    236  136 - #62646a
    248  136 - #6a6c72
    276  136 - #c3c4c8
    40  140 - #97999d
    48  140 - #818388
    52  172 - #a0a1a4
    68  172 - #d7d7d9
    92  172 - #2f3238
    128  172 - #ffffff
    164  172 - #20232a
    292  232 - #f4f5f7
    392  232 - #f4f5f7
";
const LIGHT_ON_HOVERED: &str = r"
    100   16 - #3b3d44
    160   16 - #d8d9dc
    248   16 - #999a9f
    56   20 - #cccdd0
    108   20 - #595b61
    208   20 - #d9dbdd
    312   20 - #83858a
    36   52 - #94959a
    64   52 - #21242b
    104   52 - #c2c4c7
    136   52 - #373a41
    76   72 - #d2e2ff
    36   84 - #0a6cff
    56   84 - #5f9eff
    108   84 - #e3e4e6
    16   92 - #d2e2ff
    76   92 - #d2e2ff
    392  112 - #f4f5f7
    76  132 - #989a9e
    124  136 - #21242b
    148  136 - #4f5157
    196  136 - #f4f5f7
    232  136 - #21242b
    236  136 - #62646a
    248  136 - #6a6c72
    264  136 - #f4f5f7
    48  140 - #818388
    68  172 - #d7d7d9
    128  172 - #ffffff
    168  172 - #5b5d62
    292  232 - #f4f5f7
    392  232 - #f4f5f7
";
const LIGHT_APP_ON_HOVERED: &str = r"
    232   16 - #d8d9dc
    48   20 - #d8d9dc
    60   20 - #787a7f
    72   20 - #20232a
    132   20 - #aaacaf
    176   20 - #f4f5f7
    228   20 - #b8babd
    256   20 - #20232a
    288   20 - #f4f5f7
    36   52 - #94959a
    104   52 - #c2c4c7
    136   52 - #373a41
    36   84 - #0a6cff
    56   84 - #66a3ff
    108   84 - #e3e4e6
    392  100 - #f4f5f7
    76  132 - #989a9e
    276  132 - #c3c4c8
    128  136 - #62646a
    148  136 - #4f5157
    196  136 - #f4f5f7
    212  136 - #989a9e
    236  136 - #62646a
    248  136 - #6a6c72
    48  140 - #818388
    52  172 - #a0a1a4
    68  172 - #d7d7d9
    92  172 - #2f3238
    128  172 - #ffffff
    164  172 - #20232a
    288  232 - #f4f5f7
    392  232 - #f4f5f7
";
const DARK_AWAY: &str = r"
    392    4 - #1b1d21
    48   20 - #36383c
    68   20 - #787a7e
    56   52 - #dadce0
    104   52 - #4b4d51
    136   52 - #d1d3d7
    160   52 - #4e5054
    76   72 - #1f3352
    36   84 - #5aa5ff
    56   84 - #4174b5
    108   84 - #2c2e32
    16   92 - #1f3352
    76  132 - #74767a
    276  132 - #4a4c50
    128  136 - #a8aaae
    144  136 - #4a4c50
    148  136 - #bbbdc1
    196  136 - #1b1d21
    232  136 - #e7e9ed
    236  136 - #a8aaae
    248  136 - #a0a2a6
    264  136 - #1b1d21
    40  140 - #75777b
    48  140 - #8a8c90
    128  168 - #4a5163
    168  168 - #bec2c9
    56  172 - #e5e7eb
    156  172 - #8b909c
    168  172 - #bec2c9
    20  176 - #4a5163
    104  180 - #4a5163
    392  232 - #1b1d21
";
const DARK_OFF_HOVERED: &str = r"
    268   16 - #5f6165
    48   20 - #36383c
    92   20 - #56585c
    124   20 - #787a7e
    148   20 - #b5b7bb
    172   20 - #e7e9ed
    224   20 - #e6e8ec
    36   52 - #787a7e
    64   52 - #e7e9ed
    136   52 - #d1d3d7
    92   68 - #323236
    392   80 - #1b1d21
    36   84 - #5aa5ff
    56   84 - #4174b5
    128   88 - #323236
    104   92 - #323236
    76  136 - #74767a
    128  136 - #a8aaae
    144  136 - #4a4c50
    164  136 - #1b1d21
    196  136 - #1b1d21
    236  136 - #a8aaae
    264  136 - #1b1d21
    48  140 - #8a8c90
    96  156 - #4a5163
    36  168 - #4a5163
    120  168 - #4a5163
    52  172 - #8e929e
    156  172 - #8b909c
    92  184 - #4a5163
    292  232 - #1b1d21
    392  232 - #1b1d21
";
const DARK_ON_HOVERED: &str = r"
    60   20 - #939599
    108   20 - #b1b3b7
    148   20 - #c6c8cc
    208   20 - #35373b
    276   20 - #1b1d21
    312   20 - #888a8e
    180   24 - #616367
    36   52 - #787a7e
    64   52 - #e7e9ed
    100   52 - #1b1d21
    160   52 - #4e5054
    36   84 - #5aa5ff
    56   84 - #457abe
    64   84 - #284068
    108   84 - #2c2e32
    60   88 - #284068
    392  108 - #1b1d21
    276  132 - #4a4c50
    76  136 - #74767a
    124  136 - #e7e9ed
    148  136 - #bbbdc1
    196  136 - #1b1d21
    236  136 - #a8aaae
    48  140 - #8a8c90
    172  156 - #4a5163
    36  168 - #4a5163
    144  168 - #505668
    56  172 - #e5e7eb
    92  172 - #dde0e5
    144  172 - #505668
    160  184 - #4a5163
    392  232 - #1b1d21
";
const DARK_APP_ON_HOVERED: &str = r"
    152   16 - #e8eaee
    232   16 - #36383c
    48   20 - #36383c
    72   20 - #e8eaee
    124   20 - #e6e8ec
    188   20 - #7d7f83
    288   20 - #1b1d21
    392   36 - #1b1d21
    104   52 - #4b4d51
    136   52 - #d1d3d7
    160   52 - #4e5054
    76   72 - #1f3352
    36   84 - #5aa5ff
    56   84 - #4174b5
    108   84 - #2c2e32
    16   92 - #1f3352
    76  132 - #74767a
    128  136 - #a8aaae
    148  136 - #bbbdc1
    196  136 - #1b1d21
    236  136 - #a8aaae
    264  136 - #1b1d21
    48  140 - #8a8c90
    20  164 - #4a5163
    168  168 - #bec2c9
    52  172 - #8e929e
    68  172 - #676d7c
    88  172 - #4a5163
    156  172 - #8b909c
    168  172 - #bec2c9
    116  180 - #4a5163
    392  232 - #1b1d21
";

/// A point off every button, where the pointer rests between the steps.
const AWAY: &str = "390 230 m";

/// Buttons that are on keep an on fill while the pointer is over them: the
/// engine on fill gets a step darker, an app's own on fill stays, and an
/// app's own on hover fill is used when it gives one. An off button still
/// takes the plain hover fill.
#[view]
struct ToolButtonOnHover {
    #[init]
    page:         Container,
    status:       Label,
    engine_title: Label,
    engine_on:    ToolButton,
    engine_off:   ToolButton,
    app_title:    Label,
    track:        Container,
    app_on_hover: ToolButton,
    app_on:       ToolButton,
    pointer:      Container,
}

impl Setup for ToolButtonOnHover {
    fn setup(self: Weak<Self>) {
        self.page.set_color(PAGE);
        self.page.place().back();

        for (label, text, top) in [
            (self.status, "pointer away", 8),
            (self.engine_title, "engine colors: on, off", 40),
            (self.app_title, "app colors: own on hover, own on fill only", 124),
        ] {
            label
                .set_text(text)
                .set_text_size(14)
                .set_alignment(TextAlignment::Left)
                .set_text_color(INK);
            label.place().t(top).lr(16).h(22);
        }

        self.engine_on.set_text("Picked").set_on(true);
        self.engine_off.set_text("Other");
        self.engine_on.place().t(68).l(16);
        self.engine_off.place().t(68).anchor(Left, self.engine_on, 8);

        self.track.set_color(TRACK).set_corner_radius(8);
        self.track.place().t(150).l(16).size(260, 40);
        for (button, text) in [(self.app_on_hover, "Own hover"), (self.app_on, "Own fill")] {
            button.set_text(text).set_on(true);
            button.set_tint(INK).set_on_tint(INK).set_on_color(ON);
        }
        self.app_on_hover.set_on_hover_color(ON_HOVER);
        self.app_on_hover.place().t(154).l(20);
        self.app_on.place().t(154).anchor(Left, self.app_on_hover, 8);

        self.pointer.set_color(POINTER).set_corner_radius(4);
        self.pointer.set_frame((0, 0, 8, 8)).set_hidden(true);
    }
}

impl ToolButtonOnHover {
    /// Moves the pointer onto the middle of `button`, draws its marker and
    /// says what should be seen.
    fn hover(
        view: Weak<Self>,
        button: impl Fn(Weak<Self>) -> Weak<ToolButton> + Send + 'static,
        status: &str,
    ) {
        let status = status.to_string();
        let (x, y) = from_main(move || {
            let center = button(view).absolute_frame().center();
            view.pointer.set_frame((center.x - 4.0, center.y - 4.0, 8.0, 8.0));
            view.pointer.set_hidden(false);
            view.status.set_text(status);
            (center.x, center.y)
        });
        inject_touches(format!("{x} {y} m"));
        wait_for_next_frame();
    }

    fn away(view: Weak<Self>) {
        from_main(move || {
            view.pointer.set_hidden(true);
            view.status.set_text("pointer away");
        });
        inject_touches(AWAY);
        wait_for_next_frame();
    }

    fn check_theme(view: Weak<Self>, mode: ThemeMode, blocks: [&str; 4]) -> Result<()> {
        from_main(move || Theme::set_mode(mode));
        Self::away(view);
        let resting = from_main(move || *view.engine_on.color());
        check_colors(blocks[0])?;

        Self::hover(
            view,
            |v| v.engine_off,
            "pointer on Other, off, the plain hover fill",
        );
        let off_hovered = from_main(move || *view.engine_off.color());
        check_colors(blocks[1])?;

        Self::hover(
            view,
            |v| v.engine_on,
            "pointer on Picked, it keeps an on fill a step darker",
        );
        from_main(move || {
            ensure!(view.engine_on.is_hovered(), "Picked took no hover");
            let fill = *view.engine_on.color();
            ensure!(
                fill != off_hovered,
                "a hovered on button has the hover fill of an off one"
            );
            ensure!(fill != resting, "a hovered on button shows no hover at all");
            ensure!(fill.a > 0.0, "a hovered on button lost its fill");
            Ok(())
        })?;
        check_colors(blocks[2])?;

        Self::hover(
            view,
            |v| v.app_on_hover,
            "pointer on Own hover, the on hover fill of the app",
        );
        from_main(move || {
            ensure!(
                *view.app_on_hover.color() == ON_HOVER.resolve(),
                "the on hover fill of the app is not used"
            );
            Ok(())
        })?;
        Self::hover(
            view,
            |v| v.app_on,
            "pointer on Own fill, the on fill of the app stays",
        );
        from_main(move || {
            ensure!(view.app_on.is_hovered(), "Own fill took no hover");
            ensure!(
                *view.app_on.color() == ON.resolve(),
                "the on fill of the app is lost under the pointer"
            );
            Ok(())
        })?;
        check_colors(blocks[3])?;
        Ok(())
    }
}

impl ViewTest for ToolButtonOnHover {
    fn canvas() -> (u32, u32) {
        (400, 240)
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        Self::check_theme(
            view,
            ThemeMode::Light,
            [
                LIGHT_AWAY,
                LIGHT_OFF_HOVERED,
                LIGHT_ON_HOVERED,
                LIGHT_APP_ON_HOVERED,
            ],
        )?;
        Self::check_theme(
            view,
            ThemeMode::Dark,
            [DARK_AWAY, DARK_OFF_HOVERED, DARK_ON_HOVERED, DARK_APP_ON_HOVERED],
        )?;
        from_main(|| Theme::set_mode(ThemeMode::System));
        Ok(())
    }
}
