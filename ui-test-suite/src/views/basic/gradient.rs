use anyhow::Result;
use hilen::{
    AppRunner,
    dispatch::from_main,
    refs::Weak,
    ui::{
        self, Anchor::Left, BLACK, BLUE, Button, CLEAR, Container, GREEN, Label, PURPLE, RED, Screenshot,
        Setup, TURQUOISE, U8Color, ViewData, ViewFrame, ViewSubviews, ViewTest, WHITE, view,
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

/// The bordered box sits in the empty strip right of the button, so it adds no
/// pixels where the recorded block already probes.
const BORDERED_FRAME: (u32, u32) = (240, 344);
/// Screen pixels, so the band can be probed without a float cast.
const BORDER: u32 = 6;

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

    angled:   Container,
    bordered: Container,

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

        // A gradient fills the box, so the border has to be drawn on top of the
        // ramp. It used to be dropped entirely, which left every gradient view
        // with no outline at all.
        self.bordered.set_gradient(RED, BLUE).set_corner_radius(12);
        self.bordered.set_border_color(GREEN).set_border_width(BORDER);
        self.bordered.place().l(BORDERED_FRAME.0).t(BORDERED_FRAME.1).size(120, 60);

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
        self.radial_center
            .place()
            .t(424)
            .size(170, 170)
            .anchor(Left, self.radial_top, 20);
        self.radial_corner
            .place()
            .t(424)
            .size(170, 170)
            .anchor(Left, self.radial_center, 20);
    }
}

impl ViewTest for Gradient {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        set_record_probe_count(PROBES);

        check_text_gradient(view)?;
        check_gradient_border(view)?;

        check_colors(COLORS)?;

        Ok(())
    }
}

const COLORS: &str = r"
             108   20 - #000000
             536   20 - #000000
             392   44 - #ff0000
             224   52 - #a10000
             228   52 - #a10000
             224   56 - #530000
             272   60 - #000000
             320   68 - #860000
             360   68 - #560000
              20  104 - #000000
             304  108 - #ff0000
             208  116 - #fe0000
             384  120 - #e70000
             336  128 - #790000
             268  132 - #000000
             360  136 - #560000
             228  140 - #fe0000
             592  152 - #597c95
             304  184 - #bf003a
             388  188 - #b00048
             340  192 - #a00055
             208  196 - #900063
             244  196 - #910064
             276  196 - #900063
             320  196 - #4c0034
             224  200 - #520048
             228  200 - #520048
             224  204 - #25002a
             292  208 - #000000
             320  208 - #34004a
             360  208 - #210030
             372  208 - #2e0043
             260  212 - #53009c
             320  216 - #240059
             340  216 - #4300a9
             372  216 - #200050
             392  216 - #4300a9
              20  256 - #f60900
             124  256 - #09f6ff
             288  256 - #f6f6f6
             356  256 - #dc001f
             168  260 - #18e7ff
             256  260 - #e7e7e7
             400  260 - #85006f
              44  264 - #d92600
              64  264 - #d92600
             152  268 - #35caff
             316  268 - #cacaca
              88  272 - #bc4300
             132  272 - #43bcff
             224  272 - #bcbcbc
             376  272 - #9b005b
              28  276 - #ad5200
             176  276 - #52adff
             204  276 - #adadad
             268  276 - #adadad
             244  280 - #9e9e9e
              48  284 - #906f00
              84  284 - #906f00
             160  284 - #6f90ff
             284  284 - #909090
             304  284 - #909090
              72  288 - #817e00
             120  288 - #7e81ff
             144  288 - #7e81ff
             216  288 - #818181
             232  288 - #818181
             252  292 - #737373
             360  292 - #940061
              20  296 - #649b00
              36  296 - #649b00
             276  296 - #646464
              48  300 - #56a900
             132  300 - #a956ff
             156  300 - #a956ff
             172  300 - #a956ff
             292  300 - #565656
             312  300 - #565656
              60  304 - #47b800
             208  304 - #474747
             264  304 - #474747
             408  304 - #2600c4
              80  308 - #38c700
             232  308 - #383838
             116  312 - #d52aff
             140  312 - #d52aff
             164  312 - #d52aff
             252  316 - #1b1b1b
             276  316 - #1b1b1b
             300  316 - #1b1b1b
              24  320 - #0df200
              48  320 - #0df200
              68  320 - #0df200
             340  320 - #85006f
             376  320 - #4300aa
             396  320 - #1f00cb
             164  344 - #fffdfd
             336  348 - #00ff00
             144  352 - #ffdbdb
             212  352 - #ffdbdb
             256  352 - #db0021
             308  352 - #db0021
              24  356 - #ffcaca
              60  356 - #ffcaca
             104  356 - #ffcaca
             592  360 - #597c95
              40  364 - #ffa8a8
              80  364 - #583a3a
             120  364 - #ffa8a8
             168  364 - #ffa8a8
             188  364 - #ffa8a8
              92  368 - #ff9797
             132  368 - #ff9797
             284  368 - #97005e
              64  372 - #ff8686
             108  372 - #a65757
             116  372 - #5c3030
             124  372 - #3f2121
             148  372 - #ff8686
             160  372 - #fe8686
             208  372 - #ff8686
             352  372 - #86006e
              48  376 - #ff7575
              80  376 - #ff7575
             108  376 - #a64c4c
             116  376 - #5c2a2a
             124  376 - #3f1d1d
             108  380 - #a64141
             180  380 - #ff6464
              24  384 - #ff5353
             132  384 - #ff5353
             148  384 - #ff5353
             160  384 - #ff5353
             200  384 - #ff5353
             216  388 - #ff4242
             276  388 - #4200ab
              40  392 - #ff3131
              64  392 - #ff3131
              92  396 - #ff2020
             116  396 - #ff2020
             188  396 - #ff2020
             256  396 - #2000ca
             296  400 - #00ff00
             332  400 - #00ff00
              80  424 - #c72adb
             124  448 - #ca28dd
              60  464 - #8e54b7
              96  468 - #c829dc
             440  480 - #7468a6
             336  484 - #9053b8
             276  500 - #d023e1
             312  512 - #d61fe5
             100  520 - #8f53b8
             500  528 - #7567a7
             296  544 - #ad3dcb
             344  544 - #7369a5
             412  544 - #c42cd9
             400  592 - #fd01fe
             464  592 - #b537d0
             592  592 - #597c95
        ";

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

/// The border is checked by relation for the same reason the glyph ramp is.
/// Probing it would mean re-recording the whole block, since the recorder picks
/// its probes across the canvas and one new box moves them everywhere.
fn check_gradient_border(view: Weak<Gradient>) -> Result<()> {
    // Half the band in from the left edge at mid height, so no corner and no
    // antialiased boundary is involved.
    let band = (BORDERED_FRAME.0 + BORDER / 2, BORDERED_FRAME.1 + 30);
    let inside = (BORDERED_FRAME.0 + 60, BORDERED_FRAME.1 + 30);

    let shot = AppRunner::take_screenshot()?;

    let bordered = shot.get_pixel(band);
    let ramp = shot.get_pixel(inside);

    assert!(
        bordered.g > bordered.r && bordered.g > bordered.b,
        "the border band is not the border color, got {bordered:?}"
    );
    assert!(
        ramp.g < ramp.r || ramp.g < ramp.b,
        "the box interior took the border color instead of the ramp, got {ramp:?}"
    );

    from_main(move || {
        view.bordered.set_border_width(0);
    });

    let borderless = AppRunner::take_screenshot()?.get_pixel(band);

    assert!(
        borderless.g < bordered.g,
        "clearing the border width left the band painted, got {borderless:?}"
    );

    from_main(move || {
        view.bordered.set_border_width(BORDER);
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
