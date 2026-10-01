use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    gm::{Animation, Clock},
    refs::Weak,
    ui::{
        BLACK, BLUE, Color, Container, DrawingView, GREEN, ImageView, Label, RED, Setup, Shadow, UIAnimation,
        ViewData, ViewFrame, ViewTest, WHITE, YELLOW, view,
    },
    ui_test::{check_colors, step_frames},
};

const CHECK_1: &str = r"
    12   76 - #466275
    248   88 - #565e65
    536   88 - #ffffff
    204   92 - #acbdca
    124  116 - #d8abab
    64  124 - #bf9789
    268  136 - #aa9c99
    452  136 - #000000
    224  152 - #b4a056
    108  168 - #896e59
    540  176 - #a68772
    104  180 - #856956
    40  184 - #dda5a4
    176  184 - #efd394
    484  184 - #e6c3bb
    284  188 - #ffff80
    72  200 - #e2001a
    380  208 - #ffff00
    516  208 - #af0048
    564  216 - #7c0076
    124  224 - #4900a5
    468  232 - #1600d3
    288  256 - #2d3e4b
    184  260 - #acbdca
    576  288 - #ffffff
    104  300 - #00ff00
    240  304 - #56de65
    16  316 - #344857
    292  360 - #000001
    484  364 - #597c95
    220  368 - #121a1f
    52  592 - #597c95
";

const CHECK_2: &str = r"
    200   88 - #565e65
    56   92 - #ffffff
    496   92 - #acbdca
    124  116 - #d8abab
    32  136 - #e9c0c2
    552  140 - #a3928f
    76  144 - #c2a48e
    236  148 - #747775
    288  152 - #808000
    516  152 - #b4a056
    108  168 - #896e59
    104  180 - #856956
    40  184 - #dda5a4
    176  184 - #efd394
    564  184 - #dbcd82
    456  196 - #ffff7f
    272  204 - #e48059
    376  208 - #ffff00
    524  220 - #875fac
    196  232 - #615fce
    12  248 - #466175
    80  252 - #00ff00
    236  264 - #56de65
    576  268 - #acbdca
    452  272 - #2d3e4b
    132  292 - #ffffff
    16  316 - #344857
    388  352 - #000000
    292  360 - #000001
    484  364 - #597c95
    220  368 - #121a1f
    52  592 - #597c95
";

const CHECK_3: &str = r"
    592    4 - #597c95
    12   76 - #466275
    100   88 - #ffffff
    248   88 - #565e65
    204   92 - #acbdca
    128  120 - #ddb1b4
    64  124 - #bf9789
    268  136 - #aa9c99
    236  148 - #747775
    224  152 - #b4a056
    448  156 - #ffff00
    68  160 - #cea58f
    108  168 - #896e59
    248  176 - #d3c379
    104  180 - #856956
    284  180 - #ffff80
    40  184 - #dda5a4
    176  184 - #efd394
    72  200 - #e2001a
    236  204 - #e48059
    196  220 - #875fac
    256  232 - #615fce
    288  240 - #2d3e4b
    76  252 - #00ff00
    184  260 - #acbdca
    248  300 - #56de65
    108  312 - #ffffff
    16  316 - #344857
    388  352 - #000000
    292  360 - #000001
    484  364 - #597c95
    220  368 - #121a1f
";

/// One second of fade at the 60 fps of the stepped timeline.
const FADE_SECONDS: f32 = 1.0;
const FADE_FRAMES: u32 = 60;

/// One of everything a view can draw: a flat color with a border and a
/// shadow, text, an image, a gradient and a vector path.
#[view]
struct OpacityCard {
    #[init]
    title:    Label,
    image:    ImageView,
    gradient: Container,
    path:     DrawingView,
}

impl Setup for OpacityCard {
    fn setup(mut self: Weak<Self>) {
        self.set_color(WHITE)
            .set_border_color(BLACK)
            .set_border_width(4)
            .set_corner_radius(12)
            .set_shadow(Shadow {
                offset: (0, 6).into(),
                radius: 8.0,
                color:  Color::rgb(0.0, 0.0, 0.0).with_alpha(0.6),
            });

        self.title.set_frame((8, 8, 114, 40));
        self.title.set_text("card").set_text_color(BLACK).set_text_size(28);
        self.image.set_frame((15, 56, 100, 70));
        self.image.set_image("cat.png");
        self.gradient.set_frame((15, 136, 100, 40));
        self.gradient.set_gradient(RED, BLUE);
        self.path.set_frame((15, 186, 100, 60));
        self.path.add_path([(0, 60), (50, 0), (100, 60)], GREEN);
    }
}

/// Four copies of the same card over a yellow band. Proves `set_opacity`
/// fades a view with everything in it, the color, the border, the shadow,
/// the text, the image, the gradient and the path, that 0 draws nothing at
/// all, and that a `UIAnimation` drives it frame by frame to the end.
#[view]
struct ViewOpacity {
    #[init]
    band:     Container,
    full:     OpacityCard,
    half:     OpacityCard,
    none:     OpacityCard,
    animated: OpacityCard,
    names:    Label,
}

impl Setup for ViewOpacity {
    fn setup(self: Weak<Self>) {
        self.band.set_frame((0, 150, 600, 60));
        self.band.set_color(YELLOW);

        self.full.set_frame((14, 60, 130, 260));
        self.half.set_frame((160, 60, 130, 260));
        self.half.set_opacity(0.5);
        self.none.set_frame((306, 60, 130, 260));
        self.none.set_opacity(0);
        self.animated.set_frame((452, 60, 130, 260));

        self.names.set_frame((0, 340, 600, 40));
        self.names.set_text("1, 0.5, 0 and one that fades out");
    }
}

impl ViewTest for ViewOpacity {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        from_main(Clock::enter_stepped);
        step_frames(1);
        check_colors(CHECK_1)?;

        from_main(move || {
            let fade = UIAnimation::new(|card, opacity| {
                card.set_opacity(opacity);
            })
            .animation(Animation::new(1.0, 0.0, FADE_SECONDS));
            view.animated.add_animation(fade);
        });

        step_frames(FADE_FRAMES / 2);
        let half_way = from_main(move || view.animated.opacity());
        ensure!(
            (half_way - 0.5).abs() < 0.02,
            "half of the frames is half of the fade, got {half_way}"
        );
        check_colors(CHECK_2)?;

        step_frames(FADE_FRAMES / 2 + 2);
        let end = from_main(move || view.animated.opacity());
        ensure!(end < 0.02, "the fade ends at nothing, got {end}");
        check_colors(CHECK_3)?;

        from_main(Clock::exit_stepped);
        Ok(())
    }
}
