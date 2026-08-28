use anyhow::Result;
use hilen::{
    refs::Weak,
    ui::{BLUE, Container, GREEN, ImageView, Label, RED, Setup, ViewData, ViewTest, WHITE, view},
    ui_test::{check_colors, set_record_probe_count},
};

/// A card that clips its subtree to its rounded outline, the way a CSS
/// `overflow: hidden` card does. Every child pokes past a corner.
#[view]
struct Card {
    #[init]
    stripe:   Container,
    gradient: Container,
    image:    ImageView,
    label:    Label,
}

impl Setup for Card {
    fn setup(self: Weak<Self>) {
        self.set_color(WHITE)
            .set_border_color(BLUE)
            .set_border_width(6)
            .set_corner_radius(60);

        self.stripe.set_color(RED).place().tl(0).b(0).w(30);
        self.gradient.set_gradient(GREEN, BLUE).place().b(0).r(0).size(160, 60);
        self.image.set_image("cat.png").place().t(0).r(0).size(80, 120);
        self.label.set_text("Clipped").set_text_size(40).place().bl(0).size(200, 50);
    }

    fn clips_to_bounds(&self) -> bool {
        true
    }
}

#[view]
struct RoundedClip {
    #[init]
    card: Card,
}

impl Setup for RoundedClip {
    fn setup(self: Weak<Self>) {
        self.card.place().size(400, 300).tl(100);
    }
}

impl ViewTest for RoundedClip {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        set_record_probe_count(256);

        // The corners outside the arcs stay the clear color, the stripe
        // and the gradient are cut at the corners, the stripe is full
        // height along the straight edge.
        check_colors(
            r"
             102  102 - #597c95
             498  102 - #597c95
             102  398 - #597c95
             498  398 - #597c95
             108  108 - #597c95
             492  108 - #597c95
             108  392 - #597c95
             492  392 - #597c95
             115  250 - #ff0000
             115  180 - #ff0000
             300  250 - #ffffff
        ",
        )?;

        check_colors(PROBES)
    }
}

/// The recorded look of the card, every child cut at its corner arc.
const PROBES: &str = r"
               4    4 - #597c95
             112    4 - #597c95
             484    4 - #597c95
             592    4 - #597c95
             180  100 - #0000e7
             224  100 - #0000e7
             260  100 - #0000e7
             320  100 - #0000e7
             372  100 - #0000e7
             292  104 - #0000e7
             344  104 - #0000e7
             140  108 - #0000e7
             420  108 - #ebc6cd
             440  108 - #e9c2c3
             460  108 - #0000e7
             468  116 - #e0b2b4
             428  120 - #e5bdbf
             456  120 - #e3bab9
             444  124 - #c9846f
             128  128 - #ff0000
             208  128 - #ffffff
             424  132 - #e9c1c2
             108  136 - #0000e7
             440  136 - #e0c4af
             484  136 - #bd947e
             492  136 - #0000e7
             456  140 - #d0b6a3
             472  140 - #b39980
             112  144 - #ff0000
             488  144 - #9a6451
             440  148 - #ecd0c5
             480  148 - #a88871
             420  152 - #e4b9bb
             460  152 - #bb9f84
             484  152 - #a07a60
             476  160 - #b8987b
             128  164 - #ff0000
             452  164 - #cfa48c
             484  164 - #b4917b
             264  168 - #ffffff
             432  168 - #f2dcd1
             460  168 - #bb8f76
             472  168 - #c09278
             176  172 - #ffffff
             420  172 - #e3abad
             468  172 - #bd9079
             484  172 - #b2917d
             108  176 - #ff0000
             344  176 - #ffffff
             444  176 - #d9b6a3
             492  176 - #aa8e78
             424  184 - #dfa1a3
             456  184 - #cca993
             492  184 - #a48570
             440  188 - #debeaa
             468  188 - #ceae99
             484  188 - #8f735d
             492  192 - #9a7e68
             128  196 - #ff0000
             480  196 - #a88c78
             488  196 - #a4866f
             420  200 - #dda0a2
             464  200 - #ceac98
             484  200 - #927660
             432  204 - #edccc5
             448  204 - #d6b1a0
             476  204 - #af8f7b
             484  204 - #a0836b
             492  204 - #a4856c
             484  208 - #a78971
             108  212 - #ff0000
             240  212 - #ffffff
             476  212 - #a68671
             480  212 - #896d57
             484  212 - #a5866f
             488  212 - #aa8a72
             372  216 - #ffffff
             420  216 - #dba2a3
             436  216 - #e6c1ba
             448  216 - #dbb2a6
             460  216 - #d3ae9b
             476  216 - #a2836c
             480  216 - #93755f
             484  216 - #a4846d
             492  216 - #b5967f
             192  224 - #ffffff
             128  228 - #ff0000
             288  232 - #ffffff
             100  240 - #0000e7
             496  248 - #0000e7
             128  264 - #ff0000
             360  264 - #ffffff
             432  280 - #ffffff
             496  280 - #0000e7
             108  284 - #ff0000
             236  288 - #ffffff
             300  296 - #ffffff
             128  300 - #ff0000
             184  308 - #ffffff
             108  312 - #ff0000
             496  316 - #0000e7
               4  320 - #597c95
             124  332 - #ff0000
             100  336 - #0000e7
             352  340 - #00fd02
             364  340 - #00fd02
             384  340 - #00fd02
             404  340 - #00fd02
             412  340 - #00fd02
             420  340 - #00fd02
             436  340 - #00fd02
             452  340 - #00fd02
             472  340 - #00fd02
             484  340 - #00fd02
             340  344 - #00ec11
             376  344 - #00ec11
             396  344 - #00ec11
             360  348 - #00db21
             368  348 - #00db21
             388  348 - #00db21
             408  348 - #00db21
             432  348 - #00db21
             444  348 - #00db21
             460  348 - #00db21
             492  348 - #00db21
             128  352 - #ff0000
             352  352 - #00ca30
             380  352 - #00ca30
             400  352 - #00ca30
             424  352 - #00ca30
             480  352 - #00ca30
             340  356 - #00b940
             372  356 - #00b940
             392  356 - #00b940
             416  356 - #00b940
             440  356 - #00b940
             452  356 - #00b940
             492  356 - #0000e7
             108  360 - #0000e7
             164  360 - #000000
             260  360 - #9b9b9b
             360  360 - #00a84f
             384  360 - #00a84f
             404  360 - #00a84f
             424  360 - #00a84f
             432  360 - #00a84f
             468  360 - #00a84f
             260  364 - #9b9b9b
             352  364 - #00975e
             396  364 - #00975e
             448  364 - #00975e
             460  364 - #00975e
             480  364 - #00975e
             492  364 - #0000e7
             112  368 - #0000e7
             164  368 - #000000
             188  368 - #000000
             212  368 - #010101
             232  368 - #000000
             236  368 - #000000
             256  368 - #000000
             340  368 - #00866e
             372  368 - #00866e
             388  368 - #00866e
             408  368 - #00866e
             424  368 - #00866e
             440  368 - #00866e
             472  368 - #00866e
             136  372 - #000000
             172  372 - #000000
             196  372 - #010101
             204  372 - #010101
             228  372 - #010101
             240  372 - #010101
             348  372 - #00757d
             360  372 - #00757d
             400  372 - #00757d
             432  372 - #00757d
             456  372 - #00757d
             164  376 - #000000
             184  376 - #3c3c3c
             216  376 - #ffffff
             232  376 - #2c2c2c
             236  376 - #2c2c2c
             240  376 - #010101
             248  376 - #000000
             252  376 - #ffffff
             256  376 - #ffffff
             260  376 - #9b9b9b
             340  376 - #00648d
             380  376 - #00648d
             392  376 - #00648d
             416  376 - #00648d
             448  376 - #00648d
             464  376 - #00648d
             476  376 - #00648d
             192  380 - #ffffff
             208  380 - #ffffff
             220  380 - #010101
             228  380 - #030303
             248  380 - #000000
             256  380 - #ffffff
             260  380 - #9b9b9b
             352  380 - #00539c
             368  380 - #00539c
             436  380 - #00539c
             164  384 - #000000
             172  384 - #000000
             204  384 - #010101
             228  384 - #010101
             236  384 - #ffffff
             260  384 - #000000
             340  384 - #0042ab
             360  384 - #0042ab
             376  384 - #0042ab
             384  384 - #0042ab
             396  384 - #0042ab
             408  384 - #0042ab
             428  384 - #0042ab
             444  384 - #0042ab
             460  384 - #0042ab
             132  388 - #0000e7
             156  388 - #ffffff
             184  388 - #3c3c3c
             196  388 - #ffffff
             220  388 - #ffffff
             228  388 - #ffffff
             248  388 - #ffffff
             348  388 - #0031bb
             468  388 - #0000e7
             136  392 - #0000e7
             184  392 - #3c3c3c
             192  392 - #ffffff
             204  392 - #010101
             340  392 - #0020ca
             356  392 - #0020ca
             364  392 - #0020ca
             372  392 - #0020ca
             380  392 - #0020ca
             388  392 - #0020ca
             400  392 - #0020ca
             408  392 - #0020ca
             416  392 - #0020ca
             424  392 - #0020ca
             436  392 - #0020ca
             456  392 - #0000e7
             464  392 - #0000e7
             300  396 - #0000e7
               4  452 - #597c95
             592  456 - #597c95
             244  496 - #597c95
             148  544 - #597c95
             440  548 - #597c95
               4  592 - #597c95
             292  592 - #597c95
             592  592 - #597c95

";
