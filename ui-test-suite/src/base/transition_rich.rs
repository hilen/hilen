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
             212   44 - #c0392b
             204   48 - #e09d96
             340   48 - #fffefe
             304   52 - #ffffff
             372   56 - #c75044
             204   60 - #e09d96
             392   60 - #e2a39d
             256   64 - #ffffff
             208  116 - #f5e1df
             208  120 - #f5e1df
             264  120 - #d16f65
             276  120 - #c0392b
             320  120 - #dd958e
             372  120 - #da8b82
             208  124 - #f5e1df
             252  124 - #e5ada7
             312  124 - #c0392b
             352  124 - #d4796f
              36  160 - #464e61
              56  160 - #464e61
             308  160 - #01feff
             344  160 - #01feff
             432  160 - #912e21
             244  164 - #0df2ff
             548  164 - #2f180d
             564  164 - #902d21
             268  168 - #18e7ff
              24  172 - #464e61
             292  172 - #23dcff
             324  172 - #23dcff
             420  176 - #73271b
             456  176 - #3d2f29
             488  176 - #3e2f29
             528  176 - #3f302a
             568  176 - #2e180d
              44  180 - #5894f2
              48  180 - #5894f2
             308  180 - #3ac5ff
             336  180 - #3ac5ff
              44  184 - #5894f2
              48  184 - #5894f2
             252  184 - #45baff
             276  184 - #45baff
             296  184 - #45baff
             352  184 - #45baff
             508  184 - #902d21
              24  192 - #464e61
             436  192 - #544137
             552  192 - #5f493e
              64  196 - #273549
             240  196 - #6798ff
             528  196 - #b6aa9a
             280  200 - #738cff
             312  200 - #738cff
             336  200 - #738cff
             456  200 - #21150a
             256  204 - #7e81ff
             528  204 - #b8ac9c
             296  208 - #8976ff
             536  208 - #797167
             540  208 - #787066
             324  212 - #956aff
             356  212 - #956aff
             432  216 - #544137
             308  220 - #ab54ff
              68  224 - #098d19
             240  224 - #b748ff
             264  224 - #b748ff
             288  224 - #b748ff
             356  232 - #cd32ff
             460  232 - #f7f15f
             560  232 - #372a24
             280  236 - #d926ff
             312  236 - #d926ff
             340  236 - #d926ff
             452  236 - #d4a439
             456  236 - #e0ba3a
             528  236 - #7a7872
              28  240 - #09921a
              52  240 - #09911a
             428  240 - #342821
             456  240 - #cc9c39
             512  240 - #3c3227
             248  244 - #ef10ff
             292  244 - #ef10ff
             524  244 - #7a7872
             272  248 - #fb04ff
             324  248 - #fb04ff
             516  248 - #7a7872
             536  248 - #7a7872
              92  260 - #ffffff
             444  260 - #b27c52
             464  260 - #b27c52
             484  260 - #b27c52
             548  260 - #b27c52
              56  264 - #09901a
             424  268 - #66472f
             460  268 - #8a603f
             500  272 - #8a603f
             520  272 - #865d3d
             540  272 - #8a603f
             436  276 - #8a603f
             564  276 - #8a603f
             452  280 - #8a603f
             472  284 - #66472f
             440  288 - #422e1f
             492  288 - #66472f
             512  288 - #66472f
             552  288 - #422e1f
             532  292 - #442f20
             420  296 - #602117
             456  296 - #422e1f
              24  300 - #0000e7
             472  308 - #1a0f08
             516  308 - #1a0f08
             552  308 - #211109
              76  312 - #0000e7
             128  312 - #0000e7
              32  340 - #0096e6
              64  364 - #fdfeff
             496  364 - #0000e7
               4  376 - #c0392b
              40  388 - #0096e6
              28  420 - #0096e6
              80  420 - #0096e6
             204  456 - #f1c40f
             400  456 - #f1c40f
             592  456 - #c0392b
             332  476 - #645106
             264  480 - #f1c40f
             228  484 - #f1c40f
             332  484 - #645106
             300  488 - #6c5807
             312  488 - #f1c40f
             264  492 - #f1c40f
             276  492 - #000000
             300  492 - #6c5807
             320  492 - #f1c40f
             344  492 - #010100
             288  496 - #000000
             332  496 - #645106
             192  516 - #f1c40f
             240  520 - #81261d
             308  520 - #81261d
             368  520 - #81261d
             408  520 - #87281e
             140  532 - #ffffff
              60  544 - #000000
              88  544 - #000000
              68  552 - #000000
             108  552 - #000000
             456  552 - #d8867d
              80  556 - #acacac
             508  556 - #de968f
              60  560 - #000000
              80  560 - #acacac
              96  564 - #ffffff
              24  572 - #ffffff
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
             444  160 - #fe0100
             492  160 - #fe0100
             520  160 - #fe0100
              32  164 - #273549
             428  168 - #e91600
             456  172 - #df2000
             472  172 - #df2000
             544  172 - #df2000
              24  176 - #464e61
              44  180 - #273549
             112  180 - #0000e7
             500  180 - #cb3400
             444  184 - #c13e00
              64  188 - #273549
             424  188 - #b64900
             476  188 - #b64900
             524  188 - #b64900
              24  192 - #464e61
             460  196 - #a25d00
              44  200 - #273549
             548  200 - #986700
             480  204 - #8e7100
             516  204 - #8e7100
             436  212 - #798600
             500  212 - #798600
             420  216 - #6f9000
             528  216 - #6f9000
              48  224 - #b9b9b9
              76  224 - #d0d0d1
             460  224 - #5ba400
             548  224 - #5ba400
              64  228 - #c7c7c8
             436  232 - #46b900
             484  232 - #46b900
             532  232 - #46b900
             100  236 - #d0d0d1
             420  236 - #3cc300
             512  236 - #3cc300
              72  240 - #c8c8c9
             244  240 - #16a085
              88  244 - #d0d0d1
             500  244 - #28d700
             428  248 - #1de200
             444  248 - #1de200
             536  248 - #1de200
              28  252 - #ffffff
             468  252 - #13ec00
              64  256 - #9c9c9d
             456  256 - #09f600
             492  256 - #09f600
             516  256 - #09f600
              72  260 - #cbcbcc
              92  260 - #d0d0d1
              40  288 - #0000e7
              76  292 - #0000e7
             120  304 - #0096e6
             176  304 - #0096e6
              52  320 - #0000e7
             152  320 - #ffffff
             316  336 - #16a085
             124  344 - #0096e6
             148  364 - #ffffff
             176  368 - #0096e6
             124  380 - #0096e6
             504  436 - #363637
             524  436 - #1a191a
             548  436 - #181b1a
               4  440 - #16a085
             464  440 - #4c371f
             440  444 - #c47436
             560  444 - #935526
             444  448 - #bf7835
             484  448 - #ac7333
             568  448 - #945427
             440  452 - #c47336
             192  456 - #9b59b6
             236  456 - #9b59b6
             448  456 - #b3bbbc
             484  460 - #573b29
             528  460 - #52392c
             388  464 - #9b59b6
             468  468 - #5e3b20
             500  468 - #5c3a23
             512  468 - #724831
             548  468 - #4e3120
             436  472 - #161519
             524  480 - #a66943
             272  484 - #000000
             328  484 - #000000
             336  484 - #000000
             568  484 - #96562c
             216  488 - #9b59b6
             268  488 - #9b59b6
             288  488 - #9b59b6
             292  488 - #8e52a7
             300  488 - #9b59b6
             308  488 - #6b3d7e
             452  488 - #8f5a2e
             508  488 - #c78051
             264  492 - #000000
             288  492 - #9b59b6
             320  492 - #28172f
             328  492 - #714185
             484  492 - #724630
             312  496 - #010001
             328  496 - #714185
             336  496 - #000000
             400  496 - #9b59b6
             188  500 - #127f6a
             288  504 - #000000
             516  504 - #b36e41
             552  504 - #3a291d
             436  508 - #191717
             472  516 - #814621
             496  520 - #8e522b
             208  524 - #127f6a
             248  524 - #127f6a
             348  524 - #127f6a
             380  524 - #127f6a
             552  524 - #3e2b1c
             572  528 - #6b3d22
             528  532 - #714629
             440  536 - #af6836
             460  540 - #c67039
             488  544 - #533019
             544  544 - #72462b
             504  548 - #2d231b
             472  552 - #291d14
             144  556 - #84cdbe
              52  560 - #45b39d
             116  560 - #ffffff
             556  560 - #583821
             436  564 - #151414
             444  564 - #a5602f
             484  564 - #ba6935
             520  568 - #834925
             436  572 - #111615
             464  572 - #7e4724
";
