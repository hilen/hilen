use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::{Result, ensure};
use hilen::{
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
             204   40 - #e09d96
             208   44 - #d06c62
             236   48 - #c0392b
             316   48 - #fffefe
             204   52 - #e09d96
             392   56 - #e2a39d
             204   60 - #e09d96
             348   60 - #c0392b
             256   64 - #ffffff
             288   64 - #ffffff
             208  116 - #f5e1df
             208  120 - #f5e1df
             264  120 - #d16f65
             320  120 - #dd958e
             372  120 - #da8b82
             208  124 - #f5e1df
             252  124 - #e5ada7
             304  124 - #fdfafa
             352  124 - #d4796f
              36  160 - #464e61
              56  160 - #464e61
             276  160 - #01feff
             324  160 - #01feff
             352  164 - #0df2ff
             436  164 - #544137
             500  168 - #441a11
              24  172 - #464e61
             244  172 - #23dcff
             480  172 - #59443c
             524  172 - #654d43
             564  172 - #5c463d
              64  176 - #273549
             304  176 - #2fd0ff
             420  176 - #80271d
             460  176 - #3f302a
             560  176 - #3c2e28
              44  180 - #5894f2
              48  180 - #5894f2
             268  180 - #3ac5ff
              44  184 - #5894f2
              48  184 - #5894f2
             292  184 - #45baff
             332  184 - #45baff
             356  184 - #45baff
             240  192 - #5ca3ff
             444  192 - #21150a
             504  192 - #21150a
             540  192 - #b8ac9c
              64  196 - #273549
             284  196 - #6798ff
             528  196 - #b8ac9c
              32  200 - #273549
             256  200 - #738cff
             328  200 - #738cff
             312  204 - #7e81ff
             356  204 - #7e81ff
             528  204 - #b8ac9c
             428  208 - #372a24
             528  208 - #787066
             536  208 - #787066
             540  208 - #787066
             240  212 - #956aff
             340  212 - #956aff
             456  220 - #cf9f3a
              80  224 - #088117
             268  224 - #b748ff
             296  224 - #b748ff
             320  224 - #b748ff
             336  232 - #cd32ff
             456  232 - #f2ea3c
             460  232 - #f7f15e
             524  232 - #c3c0b6
             556  232 - #544137
             240  236 - #d926ff
             356  236 - #d926ff
             428  236 - #372a24
             452  236 - #d4a539
             516  236 - #7a7872
              36  240 - #09921a
              60  240 - #087b16
             456  240 - #cc9c39
             460  240 - #ca9a39
             260  248 - #fb04ff
             284  248 - #fb04ff
             328  248 - #fb04ff
             516  248 - #7a7872
             536  248 - #7a7872
              92  260 - #ffffff
             424  260 - #b27c52
             444  260 - #b27c52
             484  260 - #b27c52
             548  260 - #b27c52
              48  264 - #09921a
              96  264 - #972d22
              76  268 - #922b21
              88  268 - #932c21
             420  268 - #8f2b20
             464  268 - #8a603f
             564  268 - #66472f
             524  272 - #8a603f
             432  276 - #8a603f
             476  276 - #8a603f
             500  276 - #8a603f
             544  276 - #8a603f
             452  280 - #8a603f
             488  284 - #66472f
             464  288 - #66472f
              24  296 - #0000e7
             420  296 - #6b2419
             440  296 - #422e1f
             480  296 - #422e1f
             520  296 - #422e1f
             496  300 - #422e1f
             568  304 - #8e2d20
             432  308 - #1a0f08
             452  308 - #1a0f08
             540  308 - #1a0f08
              68  312 - #0000e7
             128  312 - #0000e7
              32  340 - #0096e6
              64  364 - #ffffff
             504  364 - #0000e7
               4  372 - #c0392b
              36  384 - #0096e6
              28  420 - #0096e6
              80  420 - #0096e6
             592  448 - #c0392b
             204  456 - #f1c40f
             368  456 - #f1c40f
             408  456 - #f1c40f
             332  476 - #645106
             264  480 - #f1c40f
             228  484 - #f1c40f
             300  488 - #6c5807
             312  488 - #f1c40f
             332  488 - #645106
             264  492 - #f1c40f
             276  492 - #000000
             300  492 - #6c5807
             320  492 - #f1c40f
             288  496 - #000000
             332  496 - #645106
             344  496 - #010100
             192  520 - #81261d
             232  520 - #81261d
             368  520 - #81261d
             404  520 - #81261d
             140  532 - #ffffff
              24  536 - #ffffff
              60  544 - #000000
              88  544 - #000000
             108  552 - #000000
              68  556 - #000000
              80  556 - #acacac
             464  556 - #c0392b
             508  556 - #de968f
             532  556 - #c0392b
              80  560 - #acacac
";

const SWITCHED: &str = r"
               4    4 - #16a085
             592    4 - #16a085
             320   36 - #ffffff
             244   44 - #81ccbd
             288   44 - #ffffff
             224   48 - #ffffff
             372   52 - #16a085
             308   56 - #16a085
             332   60 - #feffff
             352   60 - #16a085
             212  112 - #ffffff
             228  120 - #16a085
             248  120 - #b8e2da
             280  120 - #68c2b0
             320  120 - #68c2b0
             328  120 - #40b19b
             388  120 - #16a085
             328  124 - #40b19b
             328  128 - #40b19b
              44  160 - #464e61
              56  160 - #464e61
             136  160 - #0000e7
             460  160 - #fe0100
             480  160 - #fe0100
             504  160 - #fe0100
              32  164 - #273549
             428  168 - #e91600
             492  172 - #df2000
             532  172 - #df2000
              24  176 - #464e61
             512  176 - #d52a00
              44  180 - #273549
             420  184 - #c13e00
              64  188 - #273549
             460  188 - #b64900
              24  192 - #464e61
             548  192 - #ac5300
              44  200 - #273549
             432  200 - #986700
             512  200 - #986700
             480  204 - #8e7100
             592  208 - #16a085
             420  212 - #798600
             456  212 - #798600
             528  212 - #798600
             548  216 - #6f9000
             436  220 - #659a00
             512  220 - #659a00
              48  224 - #b9b9b9
              76  224 - #d0d0d1
             492  224 - #5ba400
              64  228 - #c7c7c8
             532  232 - #46b900
             100  236 - #d0d0d1
             420  236 - #3cc300
             472  236 - #3cc300
              72  240 - #c8c8c9
             512  240 - #32cd00
              88  244 - #d0d0d1
             436  244 - #28d700
             496  244 - #28d700
             452  248 - #1de200
             544  248 - #1de200
              28  252 - #ffffff
             468  252 - #13ec00
              64  256 - #9c9c9d
             432  256 - #09f600
             484  256 - #09f600
             524  256 - #09f600
              72  260 - #cbcbcc
              92  260 - #d0d0d1
              40  288 - #0000e7
              76  292 - #0000e7
             120  304 - #0096e6
             176  304 - #0096e6
              52  320 - #0000e7
             148  320 - #ffffff
             316  332 - #16a085
             176  344 - #0096e6
             148  364 - #ffffff
             120  372 - #0096e6
             176  380 - #0096e6
               4  440 - #16a085
             492  440 - #cc8434
             536  440 - #794520
             464  444 - #b0782c
             552  444 - #bf7237
             560  444 - #92562b
             440  448 - #c67236
             568  448 - #a15c2b
             516  452 - #98602b
             192  456 - #9b59b6
             236  456 - #9b59b6
             384  456 - #9b59b6
             448  456 - #b1b9b9
             460  460 - #54341d
             488  460 - #211b1e
             548  460 - #412c1f
             476  464 - #472e20
             512  464 - #63432e
             500  468 - #52341f
             572  468 - #7b4625
             436  472 - #1b1921
             528  472 - #a47660
             528  476 - #a37766
             272  484 - #000000
             328  484 - #000000
             336  484 - #000000
             448  484 - #4b2811
             488  484 - #b86835
             528  484 - #7d5b47
             188  488 - #127f6a
             268  488 - #9b59b6
             288  488 - #9b59b6
             292  488 - #8e52a7
             300  488 - #9b59b6
             308  488 - #6b3d7e
             464  488 - #b1612b
             264  492 - #000000
             268  492 - #020103
             288  492 - #9b59b6
             320  492 - #28172f
             328  492 - #714185
             372  492 - #9b59b6
             568  492 - #9a572b
             312  496 - #010001
             328  496 - #714185
             336  496 - #000000
             436  496 - #1b191c
             504  496 - #191a1c
             548  496 - #75401b
             288  504 - #000000
             476  512 - #93572b
             528  512 - #84614c
             208  516 - #9b59b6
             528  516 - #845f48
             540  516 - #4f2407
             392  520 - #0f6c5a
             528  520 - #805a41
             248  524 - #127f6a
             356  524 - #127f6a
             444  524 - #b06930
             500  528 - #764723
             556  528 - #25140d
             464  532 - #9d592c
             564  544 - #844a26
             568  544 - #3a2312
             540  548 - #865636
             472  552 - #23150c
             144  556 - #84cdbe
             444  556 - #dbe1e1
             512  556 - #1a1b1a
              52  560 - #45b39d
             116  560 - #ffffff
             548  560 - #764223
             564  560 - #1b1c1c
             492  564 - #b56831
             440  568 - #a65f2e
             452  572 - #985f3a
             472  572 - #5d310c
";
