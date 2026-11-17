use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{
        Container, Label, RED, Setup, TextAlignment, UIManager, ViewData, ViewFrame, ViewTest, WHITE, view,
    },
    ui_test::{check_colors, set_record_probe_count},
};

/// The text of a left or a right aligned label stands the same number of
/// points from the edge of the frame at every UI scale. Every red strip is
/// 8 points wide, so the text of each row starts or ends where its strip
/// does, at scale 1, 2 and 3 alike. The middle row has an inset of 0, its
/// text starts at the edge of its card, right after a strip outside it.
#[view]
struct LabelTextInset {
    #[init]
    title: Label,

    left_card:  Container,
    left_strip: Container,
    left:       Label,

    zero_card:  Container,
    zero_strip: Container,
    zero:       Label,

    right_card:  Container,
    right_strip: Container,
    right:       Label,

    status: Label,
}

/// The fixture is 200 points wide, the canvas at scale 3.
const CARD_X: f32 = 10.0;
const CARD_WIDTH: f32 = 180.0;
const ROW_HEIGHT: f32 = 36.0;
const STRIP: f32 = 8.0;

impl Setup for LabelTextInset {
    fn setup(self: Weak<Self>) {
        self.title.set_text("text inset in points").set_text_size(14);
        self.title.set_frame((0.0, 4.0, 200.0, 20.0));

        Self::row(self.left_card, self.left, 30.0, "Left 8");
        self.left.set_alignment(TextAlignment::Left).set_text_inset(STRIP);
        self.left_strip.set_color(RED).set_frame((CARD_X, 30.0, STRIP, ROW_HEIGHT));

        Self::row(self.zero_card, self.zero, 76.0, "Left 0");
        self.zero.set_alignment(TextAlignment::Left).set_text_inset(0);
        self.zero_strip
            .set_color(RED)
            .set_frame((CARD_X - STRIP, 76.0, STRIP, ROW_HEIGHT));

        Self::row(self.right_card, self.right, 122.0, "Right 8");
        self.right.set_alignment(TextAlignment::Right).set_text_inset(STRIP);
        self.right_strip
            .set_color(RED)
            .set_frame((CARD_X + CARD_WIDTH - STRIP, 122.0, STRIP, ROW_HEIGHT));

        self.status.set_text("scale 1").set_text_size(14);
        self.status.set_frame((0.0, 168.0, 200.0, 20.0));
    }
}

impl LabelTextInset {
    fn row(card: Weak<Container>, label: Weak<Label>, y: f32, text: &str) {
        card.set_color(WHITE).set_frame((CARD_X, y, CARD_WIDTH, ROW_HEIGHT));
        label.set_text(text).set_text_size(20);
        label.set_frame((CARD_X, y, CARD_WIDTH, ROW_HEIGHT));
    }

    fn insets(self: Weak<Self>) -> [f32; 3] {
        [
            self.left.text_inset(),
            self.zero.text_inset(),
            self.right.text_inset(),
        ]
    }
}

impl ViewTest for LabelTextInset {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        // At scale 1 the fixture covers a ninth of the canvas, the default
        // count puts few probes on its glyphs.
        set_record_probe_count(160);

        check_colors(SCALE_1)?;

        for (scale, expected) in [(2.0, SCALE_2), (3.0, SCALE_3)] {
            let insets = from_main(move || {
                UIManager::override_scale(scale);
                view.status.set_text(format!("scale {scale}"));
                view.insets()
            });
            let wanted = [STRIP, 0.0, STRIP];
            let same = insets
                .iter()
                .zip(wanted)
                .all(|(inset, want)| (inset - want).abs() < f32::EPSILON);
            ensure!(same, "insets {insets:?} at scale {scale}, wanted {wanted:?}");
            check_colors(expected)?;
        }

        from_main(|| UIManager::override_scale(1.0));

        Ok(())
    }
}

const SCALE_1: &str = r"
     332    4 - #597c95
     592    4 - #597c95
     464   12 - #597c95
      52   16 - #597c95
      72   16 - #445f72
      84   16 - #597c95
      92   16 - #597c95
     104   16 - #445f73
     108   16 - #06080a
     112   16 - #364c5b
     136   16 - #293945
     144   16 - #374d5c
      12   32 - #ff0000
     164   32 - #ffffff
     188   32 - #ffffff
      16   36 - #ff0000
      80   36 - #ffffff
      12   40 - #ff0000
      32   40 - #ffffff
     100   40 - #ffffff
     144   40 - #ffffff
      20   44 - #000000
      60   44 - #000000
      64   44 - #ffffff
      12   48 - #ff0000
      20   48 - #000000
      48   48 - #525252
     124   48 - #ffffff
      20   52 - #000000
     160   52 - #ffffff
     396   52 - #597c95
      16   56 - #ff0000
     108   56 - #ffffff
     184   56 - #ffffff
      12   60 - #ff0000
     140   60 - #ffffff
      16   64 - #ff0000
      64   64 - #ffffff
      88   64 - #ffffff
       4   76 - #ff0000
      36   76 - #ffffff
     112   76 - #ffffff
     164   76 - #ffffff
     188   76 - #ffffff
     280   76 - #597c95
       8   80 - #ff0000
      76   80 - #ffffff
     136   80 - #ffffff
       4   84 - #ff0000
       8   84 - #ff0000
      92   84 - #ffffff
       4   88 - #ff0000
       8   88 - #ff0000
      12   88 - #000000
     176   88 - #ffffff
       4   92 - #ff0000
       8   92 - #ff0000
      12   92 - #000000
      56   92 - #ffffff
     124   92 - #ffffff
       4   96 - #ff0000
       8   96 - #ff0000
      12   96 - #000000
      28   96 - #f9f9f9
      40   96 - #525252
      56   96 - #ffffff
     104   96 - #ffffff
     144   96 - #ffffff
     488   96 - #597c95
       4  100 - #ff0000
       8  100 - #ff0000
      16  100 - #343434
      32  100 - #ffffff
      36  100 - #ffffff
      80  100 - #ffffff
     164  100 - #ffffff
     188  100 - #ffffff
       4  104 - #ff0000
       8  104 - #ff0000
       4  108 - #ff0000
       8  108 - #ff0000
     132  108 - #ffffff
      20  124 - #ffffff
      64  124 - #ffffff
      84  124 - #ffffff
     160  124 - #ffffff
     184  124 - #ff0000
     188  124 - #ff0000
      44  128 - #ffffff
     104  128 - #ffffff
     184  128 - #ff0000
     188  128 - #ff0000
     408  128 - #597c95
     124  132 - #959595
     140  132 - #ffffff
     144  132 - #ffffff
     180  132 - #ffffff
     184  132 - #ff0000
     188  132 - #ff0000
     124  136 - #ffffff
     176  136 - #ffffff
     184  136 - #ff0000
     188  136 - #ff0000
      68  140 - #ffffff
     124  140 - #000000
     144  140 - #ffffff
     184  140 - #ff0000
     188  140 - #ff0000
      96  144 - #ffffff
     156  144 - #131313
     176  144 - #ffffff
     184  144 - #ff0000
     188  144 - #ff0000
      12  148 - #ffffff
     184  148 - #ff0000
     188  148 - #ff0000
     556  148 - #597c95
      32  152 - #ffffff
     184  152 - #ff0000
     188  152 - #ff0000
      52  156 - #ffffff
      84  156 - #ffffff
     108  156 - #ffffff
     136  156 - #ffffff
     184  156 - #ff0000
     188  156 - #ff0000
     300  164 - #597c95
     108  180 - #597c95
     376  208 - #597c95
     592  224 - #597c95
     460  228 - #597c95
     216  256 - #597c95
       8  260 - #597c95
     104  276 - #597c95
     316  276 - #597c95
     552  296 - #597c95
     428  340 - #597c95
     168  348 - #597c95
     264  356 - #597c95
     508  368 - #597c95
       4  372 - #597c95
     348  376 - #597c95
      92  388 - #597c95
     592  400 - #597c95
     424  416 - #597c95
     164  428 - #597c95
     300  448 - #597c95
     496  452 - #597c95
     104  476 - #597c95
       4  488 - #597c95
     396  488 - #597c95
     208  492 - #597c95
     584  496 - #597c95
     500  552 - #597c95
     296  560 - #597c95
      96  572 - #597c95
       4  592 - #597c95
     188  592 - #597c95
     408  592 - #597c95
     592  592 - #597c95
";
const SCALE_2: &str = r"
     592    4 - #597c95
     144   24 - #2f414e
     148   24 - #2f424f
     216   24 - #151d22
      88   28 - #384e5e
     100   28 - #4c6a7f
     116   28 - #020304
     144   28 - #2f414e
     148   28 - #2f424f
     172   28 - #597c95
     184   28 - #4c6a7f
     196   28 - #1e2a33
     216   28 - #151d22
     244   28 - #597c95
     260   28 - #597c95
      88   32 - #384e5e
     128   32 - #000000
     144   32 - #2f414e
     148   32 - #2f424f
     164   32 - #597c95
     168   32 - #597c95
     184   32 - #597c95
     196   32 - #1e2a33
     216   32 - #151d22
     244   32 - #597c95
     260   32 - #597c95
     272   32 - #000001
     288   32 - #151e24
     304   32 - #597c95
     104   36 - #000001
     296   36 - #000000
      20   64 - #ff0000
     376   68 - #ffffff
      24   80 - #ff0000
     496   80 - #597c95
      40   84 - #000000
     132   84 - #010101
      88   88 - #919191
     124   88 - #ffffff
     252   88 - #ffffff
      68   92 - #ffffff
      20   96 - #ff0000
      60   96 - #010101
      96   96 - #bebebe
     100   96 - #a5a5a5
      68  100 - #d9d9d9
      72  100 - #d9d9d9
      96  100 - #bebebe
     100  100 - #a5a5a5
     124  100 - #ffffff
     196  100 - #ffffff
     592  100 - #597c95
      40  104 - #000000
      84  104 - #010101
      52  108 - #000000
     128  108 - #000000
      24  112 - #ff0000
     304  112 - #ffffff
      20  128 - #ff0000
     236  152 - #ffffff
     376  152 - #ffffff
       4  156 - #ff0000
     176  156 - #ffffff
      16  168 - #ff0000
       4  176 - #ff0000
     116  176 - #000000
      24  180 - #000000
      72  180 - #919191
     104  180 - #000000
      52  184 - #ffffff
      44  188 - #010101
      80  188 - #bebebe
      84  188 - #a5a5a5
     108  188 - #ffffff
     116  188 - #484848
     556  188 - #597c95
      24  192 - #000000
      52  192 - #d9d9d9
      56  192 - #d9d9d9
      80  192 - #bebebe
      84  192 - #a5a5a5
     304  192 - #ffffff
       4  196 - #ff0000
      68  196 - #010101
     104  196 - #000000
      32  200 - #000000
     112  200 - #000000
     352  200 - #ffffff
      16  208 - #ff0000
     248  208 - #ffffff
     188  216 - #ffffff
       4  220 - #ff0000
     144  244 - #ffffff
     376  244 - #ff0000
     364  256 - #ff0000
      52  260 - #ffffff
     248  268 - #a7a7a7
     300  268 - #000000
     376  268 - #ff0000
     256  272 - #ffffff
     320  272 - #919191
     328  272 - #919191
     352  272 - #ffffff
     248  276 - #ffffff
     268  276 - #010101
     312  276 - #010101
     284  280 - #ffffff
     288  280 - #ffffff
     112  284 - #ffffff
     180  284 - #ffffff
     260  284 - #ffffff
     280  284 - #ffffff
     284  284 - #ffffff
     288  284 - #ffffff
     312  284 - #3e3e3e
     324  284 - #000000
     348  284 - #ffffff
     356  284 - #ffffff
     268  288 - #010101
     300  288 - #000000
     312  288 - #3e3e3e
     352  288 - #ffffff
     244  292 - #010101
     292  292 - #010101
     328  292 - #010101
     352  292 - #010101
     376  292 - #ff0000
     284  296 - #ffffff
     496  296 - #597c95
     284  300 - #010101
     364  300 - #ff0000
      20  312 - #ffffff
      72  312 - #ffffff
     376  312 - #ff0000
     204  348 - #1d2830
     204  352 - #1d2830
     240  352 - #000000
     164  356 - #597c95
     180  356 - #597c95
     204  356 - #1d2830
     212  356 - #4c6a7f
     216  356 - #4c6a7f
     176  360 - #597c95
     180  360 - #597c95
     192  360 - #597c95
     204  360 - #1d2830
     164  364 - #000001
     232  364 - #000000
     592  368 - #597c95
     444  384 - #597c95
      48  456 - #597c95
     328  456 - #597c95
     232  464 - #597c95
     460  480 - #597c95
     576  480 - #597c95
     168  536 - #597c95
       4  592 - #597c95
     328  592 - #597c95
     444  592 - #597c95
     592  592 - #597c95
";
const SCALE_3: &str = r"
       4    4 - #597c95
     548    4 - #597c95
     408   28 - #000000
     136   32 - #202d36
     292   32 - #000000
     188   36 - #000000
     216   36 - #1a242b
     324   36 - #1a242c
     424   36 - #000001
     136   40 - #202d36
     156   40 - #597c95
     172   40 - #000000
     216   40 - #1a242b
     260   40 - #182228
     276   40 - #597c95
     324   40 - #1a242c
     460   40 - #597c95
     136   44 - #202d36
     216   44 - #1a242b
     236   44 - #263540
     324   44 - #1a242c
     364   44 - #597c95
     136   48 - #202d36
     216   48 - #1a242b
     236   48 - #263540
     240   48 - #435e71
     324   48 - #1a242c
     340   48 - #000001
     392   48 - #597c95
     440   48 - #0d1216
     216   52 - #1a242b
     236   52 - #263540
     276   52 - #597c95
     324   52 - #1a242c
     416   52 - #000000
     168   56 - #354a59
     224   56 - #354a59
     316   56 - #354a59
     592   88 - #597c95
      32   92 - #ff0000
      64  124 - #656565
     188  128 - #ffffff
     132  132 - #cdcdcd
     152  132 - #cdcdcd
      32  136 - #ff0000
      64  136 - #656565
     376  136 - #ffffff
     100  140 - #ffffff
     124  140 - #e5e5e5
      64  144 - #656565
     124  152 - #e5e5e5
     292  152 - #ffffff
     540  152 - #ffffff
      64  156 - #656565
     108  156 - #ffffff
     148  156 - #000000
     192  156 - #ffffff
      96  160 - #000000
      60  164 - #808080
      68  164 - #808080
      72  164 - #808080
      76  164 - #808080
     128  164 - #808080
     460  164 - #ffffff
      32  168 - #ff0000
      36  196 - #ff0000
     248  224 - #597c95
       8  228 - #ff0000
     352  228 - #ffffff
     568  228 - #ffffff
     168  260 - #000000
      40  264 - #656565
     100  268 - #e5e5e5
       8  272 - #ff0000
      40  272 - #656565
     132  272 - #999999
      80  276 - #ffffff
     100  276 - #e5e5e5
      40  280 - #656565
     152  280 - #dedede
     100  284 - #e5e5e5
     464  284 - #ffffff
      80  288 - #d3d3d3
      88  288 - #d3d3d3
     176  288 - #000000
      40  292 - #656565
     100  292 - #e5e5e5
      76  296 - #ffffff
      12  300 - #ff0000
      60  300 - #2e2e2e
     100  300 - #e5e5e5
     132  300 - #777777
     160  300 - #000000
     280  304 - #ffffff
     380  312 - #ffffff
       8  332 - #ff0000
     568  368 - #ff0000
     156  380 - #ffffff
      80  392 - #ffffff
     232  396 - #ffffff
     364  400 - #747474
     376  400 - #000000
     448  400 - #d4d4d4
     524  400 - #000000
     364  404 - #747474
     568  404 - #ff0000
     364  408 - #747474
     404  408 - #cdcdcd
     480  408 - #cdcdcd
     492  408 - #cdcdcd
     364  412 - #747474
     380  412 - #ffffff
     448  412 - #d4d4d4
     532  412 - #ffffff
     364  416 - #747474
     440  416 - #696969
     364  420 - #747474
     364  424 - #747474
     392  424 - #ffffff
     424  424 - #ffffff
     448  424 - #d4d4d4
     528  424 - #ffffff
     364  428 - #747474
     440  428 - #696969
     468  428 - #505050
     516  428 - #010101
     540  428 - #010101
     364  432 - #747474
     364  436 - #747474
     440  436 - #696969
     448  436 - #d4d4d4
     468  436 - #505050
     488  436 - #000000
     368  440 - #808080
     392  440 - #808080
     404  440 - #808080
     452  440 - #808080
     568  440 - #ff0000
     428  444 - #ffffff
       4  448 - #597c95
     568  472 - #ff0000
     116  476 - #597c95
     500  516 - #597c95
      44  520 - #597c95
     304  520 - #000000
     360  524 - #000000
     240  528 - #000001
     268  528 - #000000
     284  532 - #597c95
     320  532 - #597c95
     272  540 - #597c95
     304  540 - #000000
     260  544 - #000000
     288  544 - #597c95
     324  544 - #597c95
     352  544 - #597c95
     468  588 - #597c95
       4  592 - #597c95
     140  592 - #597c95
     592  592 - #597c95
";
