use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::{Result, ensure};
use test_engine::{
    refs::Weak,
    ui::{
        Button, CLEAR, CheckBox, CircleView, Container, GREEN, ImageView, Label, NumberView, PURPLE,
        ProgressView, RED, Setup, Shadow, Slider, Switch, TURQUOISE, ViewData, ViewTest, WHITE, view,
    },
    ui_test::{check_colors, inject_touches, set_record_probe_count},
};

static FIRST_SETUPS: AtomicUsize = AtomicUsize::new(0);

#[view]
struct TransitionRich {
    #[init]
    title:    Label,
    subtitle: Label,
    shop:     ImageView,
    check:    CheckBox,
    toggle:   Switch,
    progress: ProgressView,
    number:   NumberView,
    card:     Container,
    circle:   CircleView,
    slider:   Slider,
    hint:     Label,
    to_next:  Button,
    info:     Button,
}

impl Setup for TransitionRich {
    fn setup(mut self: Weak<Self>) {
        FIRST_SETUPS.fetch_add(1, Ordering::Relaxed);

        self.set_color("#C0392B");

        self.title.set_text("Red screen");
        self.title.set_text_size(40);
        self.title.set_text_color(WHITE);
        self.title.set_color(CLEAR);
        self.title.place().t(24).center_x().size(360, 56);

        self.subtitle.set_text("Instant transition test");
        self.subtitle.set_text_size(20);
        self.subtitle.set_text_color(WHITE);
        self.subtitle.set_color(CLEAR);
        self.subtitle.place().below(self.title, 12);

        self.shop.set_image("shop.png");
        self.shop.place().size(150, 150).t(160).l(420);

        self.check.set_on(true);
        self.check.place().size(44, 44).t(160).l(24);

        self.toggle.set_on(true);
        self.toggle.place().size(80, 44).t(224).l(24);

        self.progress.set_progress(0.7);
        self.progress.place().size(150, 24).t(292).l(24);

        self.number.set_value(42);
        self.number.place().size(60, 84).t(340).l(24);

        self.card.set_gradient(TURQUOISE, PURPLE);
        self.card.set_corner_radius(12);
        self.card.place().size(120, 90).t(160).l(240);

        self.circle.set_radius(32);
        self.circle.set_color(PURPLE);
        self.circle.place().t(280).l(268);

        self.slider.set_range(0, 100).set_value(30);
        self.slider.place().size(44, 160).t(330).l(480);

        self.hint.set_text("loop demo");
        self.hint.set_text_size(18);
        self.hint.set_text_color(WHITE);
        self.hint.set_color(CLEAR);
        self.hint.place().r(24).b(24).size(160, 40);

        self.to_next.set_text("Switch");
        self.to_next.set_color("#F1C40F");
        self.to_next.set_shadow(Shadow::default());
        self.to_next.place().center_x().b(80).size(220, 64);
        self.to_next.add_transition::<TransitionRich, SwitchedRich>();

        self.info.set_text("Info");
        self.info.place().l(24).b(24).size(120, 44);
    }
}

#[view]
struct SwitchedRich {
    #[init]
    title:     Label,
    body:      Label,
    crate_box: ImageView,
    check:     CheckBox,
    toggle:    Switch,
    progress:  ProgressView,
    number:    NumberView,
    card:      Container,
    circle:    CircleView,
    slider:    Slider,
    again:     Button,
    footer:    Label,
}

impl Setup for SwitchedRich {
    fn setup(mut self: Weak<Self>) {
        self.set_color("#16A085");

        self.title.set_text("Switched");
        self.title.set_text_size(40);
        self.title.set_text_color(WHITE);
        self.title.set_color(CLEAR);
        self.title.place().t(24).center_x().size(360, 56);

        self.body.set_text("Root view replaced");
        self.body.set_text_size(22);
        self.body.set_text_color(WHITE);
        self.body.set_color(CLEAR);
        self.body.place().below(self.title, 12);

        self.crate_box.set_image("crate_box.png");
        self.crate_box.place().br(24).size(140, 140);

        self.check.place().size(44, 44).t(160).l(24);

        self.toggle.place().size(80, 44).t(224).l(24);

        self.slider.set_range(0, 10).set_value(8);
        self.slider.place().size(44, 160).t(280).l(36);

        self.progress.set_progress(0.25);
        self.progress.place().size(150, 24).t(160).l(110);

        self.circle.set_radius(36);
        self.circle.set_color(RED);
        self.circle.place().t(210).l(140);

        self.number.set_value(7);
        self.number.place().size(60, 84).t(300).l(120);

        self.card.set_gradient(RED, GREEN);
        self.card.set_corner_radius(20);
        self.card.place().size(130, 100).t(160).l(420);

        self.again.set_text("Again");
        self.again.set_color("#9B59B6");
        self.again.set_shadow(Shadow::default());
        self.again.place().center_x().b(80).size(220, 64);
        self.again.add_transition::<SwitchedRich, TransitionRich>();

        self.footer.set_text("root swapped");
        self.footer.set_text_size(18);
        self.footer.set_text_color(WHITE);
        self.footer.set_color(CLEAR);
        self.footer.place().l(24).b(24).size(160, 40);
    }
}

impl ViewTest for TransitionRich {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        set_record_probe_count(160);

        // The counter survives suite cycles, so compare against the start value.
        let setups = FIRST_SETUPS.load(Ordering::Relaxed);

        check_colors(FIRST)?;

        // Switch and Again sit at the same spot, each lap swaps there and back.
        for _ in 0..4 {
            inject_touches(
                "
                300 488 b
                300 488 e
            ",
            );

            check_colors(SWITCHED)?;

            inject_touches(
                "
                300 488 b
                300 488 e
            ",
            );

            check_colors(FIRST)?;
        }

        ensure!(
            view.is_null(),
            "the old view must be deallocated after an instant transition"
        );

        ensure!(
            FIRST_SETUPS.load(Ordering::Relaxed) == setups + 4,
            "every lap must create a fresh first view"
        );

        Ok(())
    }
}

const FIRST: &str = r"
               4    4 - #c0392b
             592    4 - #c0392b
             268   36 - #df9992
             208   40 - #c0392b
             308   44 - #ffffff
             396   44 - #ffffff
             268   48 - #df9992
             328   52 - #da8a81
             236   56 - #c0392b
             344   56 - #c0392b
             372   56 - #c0392b
             268   60 - #df9992
             328   60 - #da8a81
             252  116 - #e7b4ae
             280  116 - #f1d3d0
             320  116 - #f6e4e2
             344  116 - #d67f76
             236  120 - #fcf5f4
             280  120 - #f1d3d0
             304  120 - #d3766c
             372  120 - #eec9c6
             388  120 - #e9b8b3
              36  160 - #464e61
              56  160 - #464e61
             272  160 - #01feff
             244  164 - #0df2ff
             352  164 - #0df2ff
             440  164 - #21150a
             500  168 - #441a11
              24  172 - #464e61
             296  172 - #23dcff
             480  172 - #59443c
             524  172 - #654d43
             564  172 - #5c463d
              64  176 - #273549
             312  176 - #2fd0ff
             420  176 - #80271d
             460  176 - #3f302a
             560  176 - #3c2e28
              44  180 - #5894f2
              48  180 - #5894f2
             284  180 - #3ac5ff
             328  180 - #3ac5ff
              44  184 - #5894f2
              48  184 - #5894f2
             248  184 - #45baff
             268  184 - #45baff
             348  184 - #45baff
             492  188 - #21150a
             540  192 - #b8ac9c
              64  196 - #273549
             320  196 - #6798ff
             528  196 - #b8ac9c
             532  196 - #b8ac9c
              32  200 - #273549
             240  200 - #738cff
             284  200 - #738cff
             304  200 - #738cff
             536  200 - #b8ac9c
             560  200 - #372a24
             356  204 - #7e81ff
             432  204 - #544137
             528  204 - #b8ac9c
             532  204 - #b8ac9c
             536  204 - #b8ac9c
             528  208 - #787066
             536  208 - #787066
             540  208 - #787066
             264  212 - #956aff
             296  212 - #956aff
             332  212 - #956aff
             240  220 - #ab54ff
             456  220 - #cf9f3a
              80  224 - #088117
             284  224 - #b748ff
             308  224 - #b748ff
             352  224 - #b748ff
             528  224 - #562015
             296  232 - #cd32ff
             456  232 - #f2ea3c
             460  232 - #f7f15e
              28  236 - #09921a
              60  236 - #088217
             248  236 - #d926ff
             268  236 - #d926ff
             332  236 - #d926ff
             452  236 - #d4a539
             532  236 - #7a7872
             560  236 - #372a24
             428  240 - #372a24
             456  240 - #cc9c39
             460  240 - #ca9a39
             516  240 - #7a7872
             356  244 - #ef10ff
             296  248 - #fb04ff
             316  248 - #fb04ff
             516  256 - #1a0f08
             444  260 - #b27c52
             464  260 - #b27c52
             484  260 - #b27c52
             548  260 - #b27c52
              36  264 - #09921a
              96  264 - #972d22
             420  264 - #7f271d
              76  268 - #922b21
              88  268 - #932c21
             488  268 - #66472f
             504  268 - #66472f
             540  272 - #8a603f
             564  272 - #8a603f
             436  276 - #8a603f
             472  276 - #8a603f
             452  280 - #8a603f
             424  284 - #66472f
             496  284 - #8a603f
             516  284 - #8a603f
             124  292 - #0000e7
             464  292 - #66472f
              24  296 - #0000e7
             420  296 - #6b2419
             480  296 - #422e1f
             528  296 - #3e2c1d
             560  296 - #422e1f
             496  300 - #422e1f
             432  308 - #1a0f08
              48  312 - #0000e7
              88  312 - #0000e7
              24  344 - #0096e6
              60  360 - #ffffff
               4  380 - #c0392b
              80  384 - #0096e6
              56  404 - #ffffff
              32  420 - #0096e6
              80  420 - #0096e6
             592  436 - #c0392b
             204  456 - #f1c40f
             368  456 - #f1c40f
             408  456 - #f1c40f
             332  476 - #000000
             264  480 - #f1c40f
             300  484 - #000000
             316  484 - #010100
             288  488 - #000000
             328  488 - #f1c40f
             384  488 - #f1c40f
             260  492 - #f1c40f
             320  492 - #f1c40f
             344  492 - #000000
             192  520 - #81261d
             232  520 - #81261d
             364  520 - #81261d
             400  520 - #81261d
             140  532 - #ffffff
              60  544 - #747474
              88  544 - #000000
             456  552 - #efcecb
             100  556 - #ffffff
             508  556 - #eabbb7
              80  560 - #000000
              24  572 - #ffffff
";

const SWITCHED: &str = r"
              64    4 - #16a085
             592    4 - #16a085
             320   36 - #8cd0c3
             376   36 - #fefefe
             224   40 - #feffff
             280   44 - #9ad6ca
             252   56 - #ffffff
             280   56 - #9ad6ca
             372   56 - #16a085
             280   60 - #9ad6ca
             300   60 - #feffff
             320   60 - #8cd0c3
             352   60 - #feffff
             208  112 - #a5dad0
             344  112 - #69c2b1
             344  116 - #69c2b1
             208  120 - #a5dad0
             228  120 - #16a085
             248  120 - #70c5b4
             280  120 - #d1ece7
             312  120 - #99d5c9
             324  120 - #d1ece7
             344  120 - #69c2b1
             356  120 - #62bfad
             376  120 - #d1ece7
             388  120 - #16a085
             344  124 - #69c2b1
              44  160 - #464e61
              56  160 - #464e61
             120  160 - #0000e7
             144  160 - #0000e7
             464  160 - #fe0100
             496  160 - #fe0100
              32  164 - #273549
             540  164 - #f40b00
             512  168 - #e91600
             444  172 - #df2000
             484  172 - #df2000
              24  176 - #464e61
              44  180 - #273549
             112  180 - #0000e7
             136  180 - #0000e7
             424  180 - #cb3400
             464  184 - #c13e00
              64  188 - #273549
             548  188 - #b64900
              24  192 - #464e61
             444  196 - #a25d00
              44  200 - #273549
             420  200 - #986700
             508  200 - #986700
             484  204 - #8e7100
             532  204 - #8e7100
             444  212 - #798600
             464  212 - #798600
             428  220 - #659a00
             504  220 - #659a00
              48  224 - #b9b9b9
              76  224 - #d0d0d1
             488  224 - #5ba400
             548  224 - #5ba400
              64  228 - #c7c7c8
             444  228 - #50af00
             520  228 - #50af00
             100  236 - #d0d0d1
             420  236 - #3cc300
             460  236 - #3cc300
              72  240 - #c8c8c9
              48  244 - #ffffff
              88  244 - #d0d0d1
             444  244 - #28d700
             476  244 - #28d700
             500  248 - #1de200
             536  248 - #1de200
              28  252 - #ffffff
              64  256 - #9c9c9d
             432  256 - #09f600
             456  256 - #09f600
             484  256 - #09f600
             516  256 - #09f600
              72  260 - #cbcbcc
              92  260 - #d0d0d1
             120  304 - #0096e6
             148  312 - #ffffff
             176  324 - #0096e6
             316  336 - #16a085
             124  340 - #0096e6
             148  364 - #ffffff
             176  380 - #0096e6
               4  412 - #16a085
             456  436 - #151c1b
             524  436 - #1b181b
             476  444 - #c69041
             552  444 - #bf7237
             512  448 - #4b2715
             568  448 - #a15c2b
             492  452 - #98672b
             192  456 - #9b59b6
             236  456 - #9b59b6
             448  456 - #b1b9b9
             388  460 - #9b59b6
             468  460 - #533420
             520  460 - #5a3e2a
             488  464 - #211c1b
             508  464 - #67462f
             548  464 - #432d20
             448  468 - #694525
             528  472 - #a47660
             528  476 - #a37766
             288  480 - #000000
             332  480 - #000000
             464  480 - #ac5f32
             492  480 - #b76937
             308  484 - #9b59b6
             528  484 - #7d5b47
             264  488 - #000000
             268  488 - #000000
             292  488 - #9b59b6
             320  488 - #0a060b
             508  488 - #c97945
             288  492 - #9b59b6
             304  492 - #9b59b6
             336  492 - #010001
             436  492 - #1a1821
             568  492 - #9a572b
             540  496 - #562b11
             288  500 - #9b59b6
             472  500 - #522808
             500  504 - #874d28
             528  512 - #84614c
             208  516 - #9b59b6
             528  516 - #845f48
             400  520 - #0f6c5a
             436  520 - #211a14
             472  520 - #b77143
             528  520 - #805a41
             248  524 - #127f6a
             316  524 - #127f6a
             364  524 - #127f6a
             536  524 - #b36531
             556  528 - #25140d
             500  532 - #744426
             540  536 - #b86d3c
             444  540 - #af652f
             564  544 - #844a26
             464  548 - #a05d30
             540  548 - #865636
             156  552 - #d9efeb
             472  552 - #23150c
             516  552 - #80471d
              60  556 - #16a085
             124  556 - #16a085
             144  556 - #b0dfd5
             444  556 - #dbe1e1
             548  560 - #764223
             564  560 - #1b1c1c
             436  564 - #1c1b1b
             452  572 - #985f3a
             476  572 - #62300d
             504  572 - #5f3714
";
