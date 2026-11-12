use anyhow::Result;
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    system::Clipboard,
    ui::{
        Modifiers, NamedKey, Setup, TextAlignment, TextField, ViewData, ViewFrame, ViewTest, ViewTouch, view,
    },
    ui_test::{check_colors, inject_keys, inject_modifiers, inject_named_key, inject_scroll, inject_touches},
};

/// Enough lines to overflow the field, real words so a drag and a double
/// click land on something a human can read.
const STORY: [&str; 19] = [
    "Once upon a time a small fox",
    "lived at the edge of a wood.",
    "Every morning it ran to the",
    "river and every evening it",
    "came back with wet paws.",
    "One day the river was gone.",
    "Only stones and dry mud",
    "lay where the water had been.",
    "The fox sat down and waited.",
    "It waited through the night",
    "and through the next day too.",
    "On the third morning rain came",
    "and the river came back.",
    "The fox drank and drank,",
    "then it splashed in the water",
    "until the sun went down.",
    "Wet and tired and happy",
    "it walked the long way back.",
    "The fox went home.",
];

/// A multiline field is a text area. Enter breaks the line, the caret
/// moves with the arrow keys, Home and End, and a tap, and typing inserts
/// where the caret is. Escape ends editing.
const CHECK_1: &str = r"
             548    4 - #597c95
             392   20 - #ffffff
              40   36 - #bcbcbc
              52   36 - #bcbcbc
             116   36 - #bcbcbc
              76   44 - #ffffff
              96   44 - #ffffff
             164   44 - #ffffff
             100   48 - #bcbcbc
             124   48 - #e4e4e4
             132   48 - #bcbcbc
              44   52 - #c1c1c1
              60   52 - #bcbcbc
              80   52 - #ffffff
             124   52 - #e4e4e4
             140   52 - #ffffff
             144   52 - #ffffff
             160   52 - #bcbcbc
              96   56 - #bcbcbc
             136   56 - #bcbcbc
             168   56 - #bcbcbc
              72   60 - #bcbcbc
             576  156 - #ffffff
             324  304 - #ffffff
               4  320 - #597c95
             592  364 - #597c95
             168  436 - #ffffff
             428  460 - #ffffff
             576  576 - #ffffff
             148  580 - #597c95
               4  592 - #597c95
             292  592 - #597c95
";

const CHECK_2: &str = r"
             548    4 - #597c95
             392   20 - #ffffff
              36   32 - #000000
              40   36 - #bcbcbc
              52   36 - #bcbcbc
             116   36 - #bcbcbc
              36   40 - #000000
              76   44 - #ffffff
              96   44 - #ffffff
             164   44 - #ffffff
              36   48 - #000000
             100   48 - #bcbcbc
             124   48 - #e4e4e4
              44   52 - #c1c1c1
              60   52 - #bcbcbc
              80   52 - #ffffff
             124   52 - #e4e4e4
             144   52 - #ffffff
             160   52 - #bcbcbc
              36   56 - #000000
             136   56 - #bcbcbc
             168   56 - #bcbcbc
              36   60 - #000000
              72   60 - #bcbcbc
             576  156 - #ffffff
             324  304 - #ffffff
               4  324 - #597c95
             592  364 - #597c95
             172  432 - #ffffff
             576  576 - #ffffff
               4  592 - #597c95
             292  592 - #597c95
";

const CHECK_3: &str = r"
             368   20 - #ffffff
              40   40 - #000000
              56   44 - #000000
              68   44 - #000000
              84   44 - #565656
              48   48 - #505050
              84   48 - #565656
              48   52 - #505050
              72   52 - #ffffff
              84   52 - #565656
             140   68 - #404040
             108   80 - #010101
             140   80 - #404040
              40   84 - #000000
              92   84 - #ffffff
              56   88 - #020202
              64   88 - #ffffff
              72   88 - #010101
              80   88 - #ffffff
             132   88 - #ffffff
              48   92 - #010101
             108   92 - #010101
             140   92 - #404040
             140  100 - #404040
             576  132 - #ffffff
             344  308 - #ffffff
               4  336 - #597c95
             592  352 - #597c95
             180  428 - #ffffff
             576  576 - #ffffff
               4  592 - #597c95
             292  592 - #597c95
";

const CHECK_4: &str = r"
             348   20 - #ffffff
             576   20 - #ffffff
              44   40 - #000000
              72   40 - #cbcbcb
              84   44 - #b4b4b4
             100   44 - #ffffff
              84   48 - #b4b4b4
             108   48 - #ffffff
              52   52 - #010101
              68   52 - #000000
              84   52 - #b4b4b4
             100   56 - #000000
             116   56 - #000000
              36   68 - #000000
             136   76 - #000000
              36   84 - #000000
              60   84 - #ffffff
             108   84 - #000000
              76   88 - #ffffff
              92   88 - #ffffff
             132   88 - #ffffff
              48   92 - #010101
              36  100 - #000000
             592  188 - #597c95
             388  284 - #ffffff
               4  344 - #597c95
             592  360 - #597c95
             208  396 - #ffffff
             428  464 - #ffffff
             576  576 - #ffffff
               4  592 - #597c95
             292  592 - #597c95
";

const CHECK_5: &str = r"
             576   20 - #ffffff
             356   24 - #ffffff
             120   40 - #cbcbcb
              76   44 - #dfdfdf
              88   44 - #191919
              44   48 - #ffffff
              60   48 - #000000
              76   48 - #dfdfdf
              88   48 - #191919
             116   48 - #ffffff
              76   52 - #dfdfdf
              88   52 - #191919
              96   52 - #828282
             112   56 - #010101
             136   76 - #000000
             108   80 - #010101
              56   84 - #010101
              76   84 - #ffffff
             144   84 - #000000
              92   88 - #ffffff
             128   88 - #ffffff
              48   92 - #010101
             108   92 - #010101
             136   92 - #000000
             584  192 - #597c95
             396  288 - #ffffff
               4  340 - #597c95
             592  364 - #597c95
             212  396 - #ffffff
             576  576 - #ffffff
               4  592 - #597c95
             292  592 - #597c95
";

const CHECK_6: &str = r"
              44   40 - #000000
             120   40 - #cbcbcb
              88   44 - #191919
              76   52 - #dfdfdf
              96   52 - #828282
             220   76 - #6f6f6f
             108   80 - #010101
             176   80 - #cdcdcd
             248   80 - #000000
             308   80 - #343434
             392   80 - #2c2c2c
             400   80 - #595959
             436   80 - #838383
             308   84 - #343434
             364   84 - #161616
             400   84 - #595959
             468   84 - #ffffff
             520   84 - #ffffff
             176   88 - #cdcdcd
             220   88 - #6f6f6f
             284   88 - #f9f9f9
             308   88 - #343434
              48   92 - #010101
             136   92 - #000000
             176   92 - #cdcdcd
             308   92 - #343434
             364   92 - #161616
             436   92 - #838383
             592  332 - #597c95
             312  420 - #ffffff
              20  576 - #ffffff
             592  592 - #597c95
";

const CHECK_7: &str = r"
             492    4 - #597c95
             128   28 - #cecece
             228   32 - #282828
              44   36 - #ffffff
             356   40 - #dfdfdf
             592  100 - #597c95
             240  120 - #989898
             336  120 - #7a7a7a
             144  152 - #e7e7e7
             204  156 - #e6e6e6
              40  176 - #151515
             396  184 - #5b5b5b
             260  212 - #818181
             132  224 - #b3b3b3
             572  236 - #ffffff
             324  260 - #474747
             296  296 - #000000
             372  296 - #cdcdcd
              60  304 - #343434
             224  304 - #888888
             456  304 - #b5b5b5
             152  340 - #4e4e4e
             344  412 - #010101
             576  412 - #a6a6a6
             224  420 - #808080
              84  448 - #3d3d3d
              84  456 - #3d3d3d
             176  492 - #dbdbdb
             388  532 - #000000
              80  564 - #717171
             272  568 - #ffffff
             572  576 - #ffffff
";

const CHECK_8: &str = r"
             252    4 - #597c95
             576   32 - #a6a6a6
              44   40 - #ffffff
             436   76 - #838383
             176   84 - #cdcdcd
             308   84 - #343434
              96  144 - #a9a9a9
             260  152 - #333333
             192  192 - #1a1a1a
             392  192 - #010101
             156  228 - #6f6f6f
             572  228 - #ffffff
              72  268 - #5f5f5f
             180  272 - #979797
             300  304 - #010101
             408  304 - #717171
             240  344 - #989898
             144  380 - #e7e7e7
             436  380 - #353535
             332  384 - #010101
              44  420 - #141414
             576  440 - #ffffff
             196  452 - #1b1b1b
             288  456 - #000000
             392  460 - #cbcbcb
             100  480 - #565656
             268  524 - #676767
             152  532 - #cecece
             456  532 - #b5b5b5
              44  568 - #ffffff
             364  572 - #e4e4e4
             592  592 - #597c95
";

const CHECK_9: &str = r"
             576   32 - #a6a6a6
              44   40 - #ffffff
             176   72 - #cdcdcd
             324   76 - #ffffff
             436   76 - #838383
              60  116 - #343434
             260  152 - #333333
             236  184 - #8598b8
             156  188 - #b8d3ff
              88  192 - #262b34
             392  192 - #010101
             132  196 - #a2b9e0
             176  196 - #b8d3ff
             228  200 - #b8d3ff
             592  224 - #597c95
             180  272 - #979797
             300  304 - #010101
             408  304 - #717171
              60  308 - #343434
             144  380 - #e7e7e7
             444  380 - #4b4b4b
             396  420 - #5b5b5b
             260  452 - #818181
              40  460 - #000000
             188  460 - #cbcbcb
             324  480 - #474747
             120  492 - #181818
             372  528 - #cdcdcd
             456  532 - #b5b5b5
             132  564 - #b3b3b3
             256  572 - #131313
             576  576 - #ffffff
";

const CHECK_10: &str = r"
             576   32 - #a6a6a6
              44   40 - #ffffff
             176   72 - #cdcdcd
             436   76 - #838383
             308   80 - #343434
              96  140 - #a9a9a9
             248  184 - #9ab1d6
             192  192 - #1a1a1a
             368  192 - #7d8fad
             236  196 - #8598b8
             312  196 - #a5bde4
             112  208 - #b8d3ff
              40  216 - #b8d3ff
             592  224 - #597c95
              60  268 - #b8d3ff
             228  268 - #282828
             300  304 - #010101
             408  304 - #717171
             192  340 - #000000
             444  380 - #4b4b4b
             104  416 - #e2e2e2
             308  420 - #676767
             392  452 - #cbcbcb
             236  460 - #000000
             296  460 - #818181
             112  480 - #686868
              60  524 - #343434
             468  524 - #ffffff
             372  528 - #cdcdcd
             136  564 - #b3b3b3
             256  572 - #131313
             576  576 - #ffffff
";

const CHECK_11: &str = r"
              44   32 - #ffffff
             288   32 - #cbcbcb
             168   36 - #cdcdcd
             440   36 - #010101
             576   44 - #ffffff
              60  120 - #343434
             292  152 - #343434
             368  152 - #000000
             144  156 - #e7e7e7
             444  160 - #4b4b4b
             232  192 - #b8d3ff
              60  196 - #95aace
             176  204 - #b8d3ff
             196  224 - #1b1b1b
             392  232 - #cbcbcb
             120  264 - #181818
             300  296 - #010101
              60  304 - #343434
             224  308 - #888888
             456  308 - #b5b5b5
             152  336 - #4e4e4e
             364  368 - #9a9a9a
              48  412 - #b4b4b4
             256  416 - #767676
             576  424 - #a6a6a6
             108  452 - #898989
             368  460 - #181818
             176  492 - #dbdbdb
             276  524 - #ffffff
             104  572 - #e2e2e2
             196  572 - #686868
             592  592 - #597c95
";

const CHECK_12: &str = r"
             572   20 - #ffffff
              44   32 - #b8d3ff
             168   32 - #94aacd
             428   32 - #7f92b0
             272   40 - #b8d3ff
             396   76 - #363e4b
             240  112 - #6e7e98
              60  116 - #262b34
             144  144 - #a7bfe7
             184  152 - #8fa4c6
             304  152 - #010101
             444  152 - #363e4b
             592  160 - #597c95
              36  208 - #b8d3ff
             392  224 - #92a8cb
             196  232 - #13161b
             584  280 - #597c95
             296  296 - #000000
             432  304 - #697992
             152  332 - #38414e
             364  344 - #a5bde4
              60  364 - #95aace
             576  400 - #a6a6a6
             188  408 - #010101
             292  412 - #13161a
             368  452 - #111418
              84  456 - #2c323d
             264  524 - #262b34
             432  540 - #b8d3ff
              80  564 - #525e71
             180  564 - #525e71
             572  576 - #ffffff
";

const CHECK_13: &str = r"
             592    4 - #597c95
             280   28 - #e4e4e4
              44   32 - #ffffff
             492   40 - #ffffff
             168   44 - #cdcdcd
             388   80 - #353535
             144  144 - #e7e7e7
             276  148 - #000000
             436  152 - #353535
             576  176 - #a6a6a6
             152  212 - #7a8ca9
             320  224 - #b3b3b3
             392  228 - #cbcbcb
              64  232 - #b8d3ff
             196  232 - #1b1b1b
             112  240 - #b8d3ff
             296  296 - #000000
             224  304 - #888888
             456  304 - #b5b5b5
             152  332 - #4e4e4e
              60  364 - #cecece
             364  364 - #9a9a9a
             212  412 - #000000
             292  420 - #1a1a1a
              84  456 - #3d3d3d
             368  456 - #181818
             176  488 - #dbdbdb
             308  492 - #4d4d4d
             232  552 - #4f4f4f
              80  564 - #717171
             304  564 - #717171
             572  576 - #ffffff
";

const CHECK_14: &str = r"
             492    4 - #597c95
             128   28 - #cecece
             228   32 - #282828
              44   36 - #ffffff
             356   40 - #dfdfdf
             592  100 - #597c95
             240  120 - #989898
             336  120 - #7a7a7a
             144  152 - #e7e7e7
             204  156 - #e6e6e6
              40  176 - #151515
             396  184 - #5b5b5b
             260  212 - #818181
             132  224 - #b3b3b3
             576  236 - #ffffff
             324  260 - #474747
             296  296 - #000000
             372  296 - #cdcdcd
              60  304 - #343434
             224  304 - #888888
             456  304 - #b5b5b5
             152  340 - #4e4e4e
             344  412 - #010101
             576  412 - #a6a6a6
             224  420 - #808080
              84  448 - #3d3d3d
              84  456 - #3d3d3d
             176  492 - #dbdbdb
             388  532 - #000000
              80  564 - #717171
             272  568 - #ffffff
             576  576 - #ffffff
";

const CHECK_15: &str = r"
             228   20 - #ffffff
             316   20 - #ffffff
             108   36 - #ffffff
              60   40 - #ffffff
              48   44 - #989898
              84   44 - #7a7a7a
             152   68 - #cecece
              60   84 - #343434
              96   84 - #4e4e4e
             196   84 - #000000
             116  112 - #dbdbdb
             272  116 - #222222
             280  116 - #636363
              40  120 - #000000
              84  120 - #818181
             120  120 - #cdcdcd
             164  120 - #676767
             192  120 - #3d3d3d
             280  120 - #636363
             100  152 - #565656
             152  152 - #4e4e4e
             192  152 - #343434
             112  156 - #686868
             276  156 - #e2e2e2
              68  160 - #1a1a1a
             244  160 - #010101
             316  180 - #a6a6a6
             316  188 - #a6a6a6
              40  216 - #151515
             216  216 - #010101
             128  592 - #597c95
             592  592 - #597c95
";

const CHECK_16: &str = r"
             228   20 - #ffffff
             316   20 - #ffffff
             108   36 - #ffffff
              60   40 - #ffffff
              48   44 - #989898
              84   44 - #7a7a7a
             152   68 - #cecece
              60   84 - #343434
              96   84 - #4e4e4e
             196   84 - #000000
             116  112 - #dbdbdb
             272  116 - #222222
             280  116 - #636363
              40  120 - #000000
              84  120 - #818181
             120  120 - #cdcdcd
             164  120 - #676767
             192  120 - #3d3d3d
             280  120 - #636363
             100  152 - #565656
             152  152 - #4e4e4e
             192  152 - #343434
             112  156 - #686868
             276  156 - #e2e2e2
              68  160 - #1a1a1a
             244  160 - #010101
             316  180 - #a6a6a6
             316  188 - #a6a6a6
              40  216 - #151515
             216  216 - #010101
             128  592 - #597c95
             592  592 - #597c95
";

#[view]
struct MultilineTextField {
    #[init]
    field: TextField,
}

impl Setup for MultilineTextField {
    fn setup(self: Weak<Self>) {
        self.field.set_multiline(true).set_text_size(32);
        self.field.set_alignment(TextAlignment::Left);
        self.field.set_placeholder("Type here");
        self.field.place().tl(20).br(20);
    }
}

fn check_typing_and_caret(view: Weak<MultilineTextField>) -> Result<()> {
    check_colors(CHECK_1)?;

    // Tap into the empty field, the caret sits at the top left.
    inject_touches("200 200 b\n200 200 e");

    from_main(move || {
        assert!(view.field.is_selected());
        assert!(view.field.is_placeholding());
    });

    check_colors(CHECK_2)?;

    inject_keys("first");
    inject_named_key(NamedKey::Enter);
    inject_keys("second");

    from_main(move || {
        assert!(view.field.is_selected(), "Enter must not end editing");
        assert_eq!(view.field.text(), "first\nsecond");
    });

    check_colors(CHECK_3)?;

    // Up keeps the column, Home goes to the line start, typing inserts there.
    inject_named_key(NamedKey::ArrowUp);
    inject_named_key(NamedKey::Home);
    inject_keys("A ");

    from_main(move || {
        assert_eq!(view.field.text(), "A first\nsecond");
    });

    // End of the first line, then Right crosses the line break.
    inject_named_key(NamedKey::End);
    inject_named_key(NamedKey::ArrowRight);
    inject_keys("B");

    from_main(move || {
        assert_eq!(view.field.text(), "A first\nBsecond");
    });

    // Up and Down keep the column, so the caret is back at the second
    // line start. Right steps over B, backspace removes it.
    inject_named_key(NamedKey::ArrowUp);
    inject_named_key(NamedKey::ArrowDown);
    inject_named_key(NamedKey::Home);
    inject_named_key(NamedKey::ArrowRight);
    inject_keys("\u{8}");

    from_main(move || {
        assert_eq!(view.field.text(), "A first\nsecond");
    });

    check_colors(CHECK_4)?;

    // A tap after the end of the second line appends there.
    inject_touches("300 80 b\n300 80 e");
    inject_keys("!");

    from_main(move || {
        assert_eq!(view.field.text(), "A first\nsecond!");
    });

    // A tap between the first two glyphs of the first line inserts there.
    inject_touches("50 40 b\n50 40 e");
    inject_keys("-");

    from_main(move || {
        assert_eq!(view.field.text(), "A- first\nsecond!");
    });

    check_colors(CHECK_5)?;

    Ok(())
}

fn check_wrap_and_scroll(view: Weak<MultilineTextField>) -> Result<()> {
    // Long text wraps at the field width and the caret follows.
    inject_named_key(NamedKey::ArrowDown);
    inject_named_key(NamedKey::End);
    inject_keys(" one two three four five six");

    from_main(move || {
        assert_eq!(view.field.text(), "A- first\nsecond! one two three four five six");
    });

    check_colors(CHECK_6)?;

    // More lines than fit. The content grows past the field and the
    // scroll follows the caret to the bottom, then Up all the way
    // brings the top back.
    for line in STORY {
        inject_named_key(NamedKey::Enter);
        inject_keys(line);
    }

    from_main(move || {
        assert!(view.field.scrolled_to_caret(), "the caret must stay visible");
        assert!(view.field.content_height() > view.field.height());
    });

    check_colors(CHECK_7)?;

    for _ in 0..STORY.len() + 2 {
        inject_named_key(NamedKey::ArrowUp);
    }

    from_main(move || {
        assert!(view.field.scrolled_to_caret(), "the caret must stay visible");
    });

    check_colors(CHECK_8)?;

    Ok(())
}

fn check_selection(view: Weak<MultilineTextField>) -> Result<()> {
    // A drag inside a middle line selects part of it.
    inject_touches("80 200 b\n140 200 m\n230 200 m\n230 200 e");

    from_main(move || {
        let selected = view.field.selected_text();
        assert!(!selected.is_empty(), "a drag must select");
        assert!(view.field.text().contains(&selected));
        assert!(!selected.contains('\n'), "the drag stayed on one line");
    });

    check_colors(CHECK_9)?;

    // A drag down across lines selects across them, and typing
    // replaces the whole span with one char.
    inject_touches("200 200 b\n230 240 m\n120 280 m\n120 280 e");

    let (selected, before) = from_main(move || {
        let selected = view.field.selected_text();
        assert!(
            selected.contains('\n'),
            "the drag must cross lines, got {selected:?}"
        );
        (selected, view.field.text().to_string())
    });

    check_colors(CHECK_10)?;

    inject_keys("X");

    from_main(move || {
        assert_eq!(view.field.text().len(), before.len() - selected.len() + 1);
        assert_eq!(view.field.selected_text(), "");
    });

    // A drag from the end of one line back up to the start of it.
    inject_touches("60 200 b\n120 200 m\n230 200 m\n230 200 e");

    from_main(move || {
        let selected = view.field.selected_text();
        assert!(!selected.is_empty(), "a drag must select");
        assert!(!selected.contains('\n'), "the drag stayed on one line");
    });

    check_colors(CHECK_11)?;

    let before = from_main(move || view.field.text().to_string());
    inject_keys("X");

    from_main(move || {
        assert!(
            view.field.text().len() < before.len(),
            "typing must replace the selection"
        );
        assert_eq!(view.field.selected_text(), "");
    });

    // Shift plus arrows extend, a plain arrow collapses.
    inject_modifiers(Modifiers::SHIFT);
    inject_named_key(NamedKey::ArrowLeft);
    inject_modifiers(Modifiers::empty());

    from_main(move || {
        assert_eq!(view.field.selected_text(), "X");
    });

    inject_named_key(NamedKey::ArrowRight);

    from_main(move || {
        assert_eq!(view.field.selected_text(), "");
    });

    // Select all, cut, paste. The clipboard part needs a display server,
    // a headless CI box has none, so it is checked only where it works.
    inject_modifiers(Modifiers::META);
    inject_keys("a");
    inject_modifiers(Modifiers::empty());

    let whole = from_main(move || {
        let text = view.field.text().to_string();
        assert_eq!(view.field.selected_text(), text);
        text
    });

    check_colors(CHECK_12)?;

    if from_main(|| Clipboard::set_text("probe")).is_ok() {
        inject_modifiers(Modifiers::META);
        inject_keys("x");
        inject_modifiers(Modifiers::empty());

        from_main(move || {
            assert!(view.field.is_placeholding(), "cut must empty the field");
        });

        inject_modifiers(Modifiers::META);
        inject_keys("v");
        inject_modifiers(Modifiers::empty());

        let whole = whole.clone();
        from_main(move || {
            assert_eq!(view.field.text(), whole);
        });
    }

    // A double click selects the word under it, backspace removes it.
    inject_touches("100 240 b\n100 240 e\n100 240 b\n100 240 e");

    let (word, before) = from_main(move || {
        let word = view.field.selected_text();
        assert!(
            !word.is_empty() && word.chars().all(char::is_alphanumeric),
            "got {word:?}"
        );
        (word, view.field.text().to_string())
    });

    check_colors(CHECK_13)?;

    inject_keys("\u{8}");

    from_main(move || {
        assert_eq!(view.field.text().len(), before.len() - word.len());
        assert_eq!(view.field.selected_text(), "");
    });

    Ok(())
}

fn check_shift_tap_and_copy(view: Weak<MultilineTextField>) {
    // Shift plus a tap extends from the caret to the tap.
    inject_keys("hello world");
    inject_named_key(NamedKey::Home);
    inject_modifiers(Modifiers::SHIFT);
    inject_touches("160 240 b\n160 240 e");
    inject_modifiers(Modifiers::empty());

    from_main(move || {
        let selected = view.field.selected_text();
        assert!(!selected.is_empty(), "shift tap must select");
        assert!(view.field.text().contains(&selected));
    });

    // Copy leaves the text alone, paste at the end doubles the selection.
    if from_main(|| Clipboard::set_text("probe")).is_ok() {
        let selected = from_main(move || view.field.selected_text());

        inject_modifiers(Modifiers::META);
        inject_keys("c");
        inject_modifiers(Modifiers::empty());

        let text = from_main(move || view.field.text().to_string());

        inject_named_key(NamedKey::ArrowRight);
        inject_named_key(NamedKey::End);

        inject_modifiers(Modifiers::META);
        inject_keys("v");
        inject_modifiers(Modifiers::empty());

        from_main(move || {
            assert_eq!(view.field.text().len(), text.len() + selected.len());
            let pasted_at_line_end = view
                .field
                .text()
                .lines()
                .any(|line| line.contains("hello world") && line.ends_with(&selected));
            assert!(pasted_at_line_end, "got {:?}", view.field.text());
        });
    }
}

fn check_column_wheel_resize(view: Weak<MultilineTextField>) -> Result<()> {
    // Down then Up keeps the column on the way back.
    inject_named_key(NamedKey::Home);
    inject_named_key(NamedKey::ArrowRight);
    inject_named_key(NamedKey::ArrowRight);
    inject_keys("^");
    inject_named_key(NamedKey::ArrowDown);
    inject_named_key(NamedKey::ArrowUp);
    inject_keys("|");

    from_main(move || {
        let text = view.field.text();
        assert!(text.contains("^|"), "got {text:?}");
    });

    // The wheel scrolls the content and the bar shows, the field is
    // taller than its view again once it has many lines.
    inject_named_key(NamedKey::End);
    for line in STORY {
        inject_named_key(NamedKey::Enter);
        inject_keys(line);
    }
    inject_scroll(300);

    from_main(move || {
        assert!(view.field.content_height() > view.field.height());
        assert!(view.field.scroll_offset() < 0.0, "the wheel must scroll");
    });

    check_colors(CHECK_14)?;

    // Resizing the field refits the lines and the caret.
    from_main(move || {
        view.field.place().tl(20).size(300, 200);
    });
    wait_for_next_frame();
    wait_for_next_frame();
    inject_keys(".");

    from_main(move || {
        assert!(
            view.field.scrolled_to_caret(),
            "the caret must stay visible after a resize"
        );
    });

    check_colors(CHECK_15)?;

    inject_named_key(NamedKey::Escape);

    from_main(move || {
        assert!(!view.field.is_selected(), "Escape must end editing");
        assert_ne!(view.field.text(), "");
    });

    check_colors(CHECK_16)?;

    Ok(())
}

impl ViewTest for MultilineTextField {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        check_typing_and_caret(view)?;
        check_wrap_and_scroll(view)?;
        check_selection(view)?;
        check_shift_tap_and_copy(view);
        check_column_wheel_resize(view)?;

        Ok(())
    }
}
