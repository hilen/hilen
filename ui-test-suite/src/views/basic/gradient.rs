use anyhow::Result;
use test_engine::{
    AppRunner,
    dispatch::from_main,
    refs::Weak,
    ui::{
        self,
        Anchor::Left,
        BLACK, BLUE, Button, CLEAR, Container, GREEN, Label, PURPLE, RED, Screenshot, Setup, TURQUOISE,
        U8Color, ViewData, ViewFrame, ViewSubviews, ViewTest, WHITE, view,
    },
    ui_test::{check_colors, set_record_probe_count},
};

const TEXT: &str = "Gradient";
const TEXT_SIZE: f32 = 52.0;

const PLAIN_FRAME: (u32, u32, u32, u32) = (20, 20, 560, 66);
const EQUAL_FRAME: (u32, u32, u32, u32) = (20, 94, 560, 66);
const RAMP_FRAME: (u32, u32, u32, u32) = (20, 168, 560, 66);

/// Five times the default density, since most of this canvas is text ink and
/// soft gradient falloff, where a sparse grid pins very little.
const PROBES: usize = 160;

#[view]
struct Gradient {
    button: Weak<Button>,

    #[init]
    plain_text: Label,
    equal_text: Label,
    ramp_text:  Label,

    grad_1: Container,
    grad_2: Container,
    grad_3: Container,

    button_container: Container,

    angled: Container,

    radial_top:    Container,
    radial_center: Container,
    radial_corner: Container,
}

impl Setup for Gradient {
    fn setup(mut self: Weak<Self>) {
        // Gradient text is the subject of this test, so it takes the top of the
        // canvas at a size where the ramp is readable across the glyphs. The
        // three frames match exactly, which is what lets the checks compare
        // their pixel blocks against each other.
        for (label, frame) in [
            (self.plain_text, PLAIN_FRAME),
            (self.equal_text, EQUAL_FRAME),
            (self.ramp_text, RAMP_FRAME),
        ] {
            label.set_color(BLACK);
            label.set_frame(frame);
            label.set_text(TEXT).set_text_size(TEXT_SIZE);
        }

        self.plain_text.set_text_color(RED);
        self.equal_text.set_text_gradient(RED, RED);
        // CSS `linear-gradient(red, blue)` with `background-clip: text`.
        self.ramp_text.set_text_gradient(RED, BLUE);

        self.grad_1.set_gradient(RED, GREEN).place().l(20).t(254).size(70, 70);

        self.grad_2.set_gradient(TURQUOISE, PURPLE).set_corner_radius(20);
        self.grad_2.place().t(254).size(70, 70).anchor(Left, self.grad_1, 20);

        self.grad_3.set_gradient(WHITE, BLACK).set_corner_radius(16);
        self.grad_3.place().t(254).size(120, 70).anchor(Left, self.grad_2, 20);

        // CSS `linear-gradient(135deg, red, blue)`, red in the top left
        // corner and blue in the bottom right one.
        self.angled.apply_gradient(ui::Gradient::linear(135, RED, BLUE));
        self.angled.place().t(254).size(70, 70).anchor(Left, self.grad_3, 20);

        self.button_container.place().l(20).t(344).size(200, 60);

        self.button = self.button_container.add_view();

        self.button.place().back();
        self.button.set_text("Button").set_gradient(WHITE, RED).set_corner_radius(24);

        // The same radial in three places, so the shape is checked against its
        // center rather than only against one lucky position. Top is the CSS
        // `radial-gradient(ellipse at top, purple, transparent 60%)`, the other
        // two move the center without changing anything else.
        for (view, center) in [
            (self.radial_top, (0.5, 0.0)),
            (self.radial_center, (0.5, 0.5)),
            (self.radial_corner, (0.0, 1.0)),
        ] {
            view.apply_gradient(ui::Gradient::radial_at(center, PURPLE, CLEAR).with_end_stop(0.6));
        }

        // Square, so the ending shape reads as the CSS ellipse it is. In a
        // wider box the same gradient stretches to the box aspect, which looks
        // like a bug next to nothing that explains it.
        self.radial_top.place().l(20).t(424).size(170, 170);
        self.radial_center.place().t(424).size(170, 170).anchor(Left, self.radial_top, 20);
        self.radial_corner.place().t(424).size(170, 170).anchor(Left, self.radial_center, 20);
    }
}

impl ViewTest for Gradient {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        set_record_probe_count(PROBES);

        check_text_gradient(view)?;

        check_colors(
            r"
              20   24 - #000000
             212   32 - #fe0000
             312   32 - #7a0000
             392   32 - #bd0000
             312   56 - #7a0000
             364   56 - #460000
             268   60 - #000000
             364   60 - #460000
             220   64 - #ff0000
             312  108 - #7a0000
             392  108 - #ff0000
             592  108 - #597c95
             208  112 - #ff0000
             248  116 - #8b0000
             340  120 - #000000
             272  132 - #000000
             364  132 - #460000
              80  136 - #000000
             224  140 - #ff0000
             300  140 - #fe0000
             396  140 - #ff0000
             220  180 - #ce002c
             392  180 - #990020
             232  184 - #bf003a
             312  188 - #540022
             340  188 - #af0047
             372  188 - #b00048
             244  192 - #7f0044
             280  192 - #a00056
             292  192 - #a00055
             308  192 - #a00055
             312  196 - #450030
             204  200 - #810072
             264  204 - #720080
             312  204 - #37003d
             364  204 - #1f0023
             312  208 - #2f0044
             332  208 - #62008d
             364  208 - #1b0027
             392  208 - #63008e
             236  212 - #480086
             280  212 - #53009c
             296  212 - #53009b
             312  212 - #28004b
             364  212 - #17002b
             536  232 - #000000
             132  256 - #09f6ff
             156  256 - #09f6ff
             392  256 - #9b005b
             172  260 - #18e7ff
             232  260 - #e7e7e7
             360  260 - #ce002d
              24  264 - #d92600
              48  264 - #d92600
              80  264 - #d92600
             212  264 - #d9d9d9
             120  268 - #35caff
             144  268 - #35caff
             308  268 - #cacaca
             272  272 - #bcbcbc
             160  276 - #52adff
             224  276 - #adadad
             252  276 - #adadad
             356  276 - #b80040
              68  280 - #9e6100
             132  280 - #619eff
             176  280 - #619eff
             340  280 - #ce002d
              20  284 - #906f00
              88  284 - #906f00
             204  284 - #909090
             264  284 - #909090
             288  284 - #909090
             408  284 - #4b00a3
             116  288 - #7e81ff
             316  288 - #818181
             384  288 - #6f0082
             148  292 - #8c73ff
             236  292 - #737373
             276  292 - #737373
              36  296 - #649b00
              56  296 - #649b00
              76  296 - #649b00
             132  296 - #9b64ff
             252  296 - #646464
             364  296 - #85006f
             168  300 - #a956ff
             220  300 - #565656
             300  300 - #565656
              20  304 - #47b800
             404  304 - #2e00be
              48  308 - #38c700
              88  308 - #38c700
             112  308 - #c738ff
             264  308 - #383838
             284  308 - #383838
             316  308 - #383838
             136  312 - #d52aff
             156  312 - #d52aff
             396  312 - #2e00be
             212  316 - #1b1b1b
             240  316 - #1b1b1b
             300  316 - #1b1b1b
              28  320 - #0df200
              64  320 - #0df200
              80  320 - #0df200
             340  320 - #85006f
             364  320 - #590096
             388  320 - #2e00be
             408  320 - #0900df
             144  344 - #fffdfd
             172  344 - #fffdfd
              52  352 - #ffdbdb
              24  356 - #ffcaca
             176  360 - #ffb9b9
             216  360 - #ffb9b9
              40  364 - #ffa8a8
              80  364 - #ffa8a8
             104  364 - #ffa8a8
             120  364 - #ffa8a8
             140  364 - #ffa8a8
             192  364 - #ffa8a8
              60  372 - #ff8686
              96  372 - #ff8686
             124  372 - #663636
             160  372 - #ff8686
             212  372 - #ff8686
              20  376 - #ff7575
             124  376 - #662f2f
             136  376 - #ff7575
              80  380 - #010000
             112  380 - #ff6464
             148  380 - #ff6464
             164  380 - #d65454
             184  380 - #ff6464
             200  380 - #ff6464
              44  384 - #ff5353
             216  388 - #ff4242
              64  392 - #ff3131
             164  392 - #ff3131
              96  396 - #ff2020
             132  396 - #ff2020
             184  396 - #ff2020
             592  396 - #597c95
              80  424 - #c72adb
             124  452 - #c72adc
              60  464 - #8e54b7
             440  480 - #7468a6
             336  484 - #9053b8
             276  500 - #d023e1
             312  512 - #d61fe5
              96  520 - #8e54b7
             500  528 - #7567a7
             296  544 - #ad3dcb
             344  544 - #7369a5
             412  544 - #c42cd9
             440  564 - #c52bda
             400  592 - #fd01fe
             464  592 - #b537d0
             592  592 - #597c95
        ",
        )?;

        Ok(())
    }
}

/// Glyph gradients are checked by relation instead of by recorded probes. The
/// text is antialiased and its ink covers a small part of each frame, so a
/// probe grid is a poor spec for it, while these relations hold on any
/// platform and any font.
fn check_text_gradient(view: Weak<Gradient>) -> Result<()> {
    let shot = AppRunner::take_screenshot()?;

    // Every frame holds the same text at the same size, so the pixel blocks
    // line up index by index and compare directly.
    assert!(
        region(&shot, EQUAL_FRAME) == region(&shot, PLAIN_FRAME),
        "a gradient with both ends equal must render exactly like flat text"
    );

    assert!(
        region(&shot, RAMP_FRAME) != region(&shot, PLAIN_FRAME),
        "a two color gradient rendered the same as flat text"
    );

    let (top_red, top_blue) = channel_sums(&shot, half(RAMP_FRAME, 0));
    let (bottom_red, bottom_blue) = channel_sums(&shot, half(RAMP_FRAME, 1));

    assert!(
        top_red > top_blue,
        "the top of the ramp is not dominated by its start color, red {top_red} blue {top_blue}"
    );
    assert!(
        bottom_blue > bottom_red,
        "the bottom of the ramp is not dominated by its end color, red {bottom_red} blue {bottom_blue}"
    );

    from_main(move || {
        view.ramp_text.set_text_color(RED);
    });

    let flattened = AppRunner::take_screenshot()?;

    assert!(
        region(&flattened, RAMP_FRAME) == region(&flattened, PLAIN_FRAME),
        "setting a plain text color did not clear the gradient"
    );

    from_main(move || {
        view.ramp_text.set_text_gradient(RED, BLUE);
    });

    Ok(())
}

/// The two halves of a frame, so both bands are guaranteed to hold glyph ink.
fn half(frame: (u32, u32, u32, u32), index: u32) -> (u32, u32, u32, u32) {
    let (x, y, width, height) = frame;
    (x, y + index * height / 2, width, height / 2)
}

/// Summed red and blue over a band. The label background is black, so it adds
/// the same amount to both and drops out of the comparison. Both bands hold the
/// same number of pixels, so sums rank exactly like means would.
fn channel_sums(shot: &Screenshot, frame: (u32, u32, u32, u32)) -> (u64, u64) {
    let pixels = region(shot, frame);

    let red = pixels.iter().map(|pixel| u64::from(pixel.r)).sum();
    let blue = pixels.iter().map(|pixel| u64::from(pixel.b)).sum();

    (red, blue)
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
