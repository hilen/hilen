use anyhow::Result;
use test_engine::{
    refs::Weak,
    ui::{
        BLUE, BROWN, Container, CornerRadii, GRAY, GREEN, ImageView, ORANGE, PURPLE, RED, Setup, TURQUOISE,
        ViewData, ViewTest, YELLOW, view,
    },
    ui_test::{check_colors, set_record_probe_count},
};

// Each rounded view sits on a backdrop of a unique color, so the cut
// corners expose that color and the recorder pins probes there. The
// backdrop fields come first, so they stay behind their views.
#[view]
struct CornerRadiiTest {
    #[init]
    under_top:    Container,
    under_mixed:  Container,
    under_border: Container,
    under_image:  Container,
    under_grad:   Container,
    top_round:    Container,
    mixed:        Container,
    bordered:     Container,
    image:        ImageView,
    grad:         Container,
}

impl Setup for CornerRadiiTest {
    fn setup(self: Weak<Self>) {
        for (backdrop, color, x, y) in [
            (self.under_top, ORANGE, 20, 20),
            (self.under_mixed, PURPLE, 180, 20),
            (self.under_border, GRAY, 340, 20),
            (self.under_image, TURQUOISE, 20, 180),
            (self.under_grad, BROWN, 180, 180),
        ] {
            backdrop.set_color(color);
            backdrop.place().t(y).l(x).size(140, 140);
        }

        self.top_round.set_color(BLUE);
        self.top_round.place().tl(20).size(140, 140);
        self.top_round.set_corner_radii(CornerRadii::top(60));

        self.mixed.set_color(YELLOW);
        self.mixed.place().t(20).l(180).size(140, 140);
        self.mixed.set_corner_radii(CornerRadii {
            top_left: 60.0,
            bottom_right: 60.0,
            ..CornerRadii::default()
        });

        self.bordered.set_color(GREEN).set_border_color(RED).set_border_width(6);
        self.bordered.place().t(20).l(340).size(140, 140);
        self.bordered.set_corner_radii(CornerRadii::bottom(50));

        self.image.set_image("cat.png");
        self.image.place().t(180).l(20).size(140, 140);
        self.image.set_corner_radii(CornerRadii::top(60));

        self.grad.set_gradient(RED, BLUE);
        self.grad.place().t(180).l(180).size(140, 140);
        self.grad.set_corner_radii(CornerRadii::bottom(60));
    }
}

impl ViewTest for CornerRadiiTest {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        set_record_probe_count(96);

        rounded_corners_colors()
    }
}

fn rounded_corners_colors() -> Result<()> {
    check_colors(
        r"
              64   20 - #ffcb00
             156   20 - #ffcb00
             220   20 - #ff00ff
             284   20 - #ffff00
             340   24 - #ff0000
             472   28 - #00ff00
             188   32 - #ff00ff
             424   32 - #00ff00
             376   36 - #00ff00
             112   40 - #0000e7
             180   64 - #ff00ff
             340   76 - #ff0000
             476   80 - #ff0000
              92   84 - #0000e7
             428   84 - #00ff00
             252   88 - #ffff00
              20   92 - #0000e7
             384  100 - #00ff00
             148  128 - #0000e7
             208  128 - #ffff00
             348  128 - #ff0000
             472  128 - #ff0000
              60  136 - #0000e7
             476  136 - #bcbcbc
             356  144 - #ff0000
             300  148 - #ff00ff
             348  148 - #bcbcbc
             472  148 - #bcbcbc
             372  152 - #ff0000
             448  152 - #ff0000
             468  152 - #bcbcbc
              24  180 - #00ffff
              52  180 - #00ffff
              96  180 - #deb6b7
             184  180 - #fe0001
             156  184 - #00ffff
             128  188 - #dcaeb0
             236  188 - #f0000e
              68  200 - #c6978d
             296  200 - #da0022
              36  204 - #e8c0c1
             204  204 - #d20028
              60  208 - #cd8576
             272  212 - #c40036
             152  216 - #cd9c98
             316  216 - #bd003c
              20  224 - #00ffff
             104  224 - #c8ae97
             224  228 - #a70050
             252  228 - #a70050
             180  232 - #9f0057
              44  236 - #efd2c7
              72  236 - #2a2519
              92  236 - #b7987b
             284  236 - #98005d
              84  240 - #d0b49c
             108  244 - #0e0403
             204  244 - #8a006a
             148  252 - #ca9698
             228  256 - #74007e
             256  256 - #74007e
             300  256 - #74007e
             124  260 - #c19f86
             184  260 - #6c0085
              40  264 - #f3d8cd
              80  264 - #dbb3ad
             104  272 - #c6a38f
             280  272 - #570099
             208  276 - #4f009f
             232  276 - #4f009f
             252  280 - #4800a6
             128  284 - #7a614e
             156  284 - #967b69
             292  284 - #4100ac
             312  284 - #4100ac
              40  288 - #ecd0c5
             184  292 - #daaa7c
             124  296 - #9d7e69
             144  296 - #9c7e66
             276  296 - #2b00c0
              20  300 - #e3b0af
             212  300 - #2400c7
             236  300 - #2400c7
             256  300 - #2400c7
             296  300 - #2400c7
              80  304 - #ca9c89
             152  308 - #b89a82
             124  312 - #896d57
              48  316 - #e8c5bd
             196  316 - #daaa7c
             316  316 - #daaa7c
             588  356 - #597c95
             432  468 - #597c95
               4  592 - #597c95
             284  592 - #597c95
             592  592 - #597c95
            ",
    )
}
