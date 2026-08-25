use anyhow::Result;
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    system::Clipboard,
    ui::{ModifiersState, Setup, TextAlignment, TextField, ViewData, ViewFrame, ViewTest, ViewTouch, view},
    ui_test::{check_colors, inject_keys, inject_modifiers, inject_named_key, inject_scroll, inject_touches},
    window::NamedKey,
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
             124   48 - #e3e3e3
             132   48 - #bcbcbc
              44   52 - #c1c1c1
              60   52 - #bcbcbc
              80   52 - #ffffff
             124   52 - #e3e3e3
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
               4    4 - #597c95
             168    4 - #597c95
             304    4 - #597c95
             440   12 - #597c95
             576   20 - #bcbcbc
              36   32 - #000000
              36   36 - #000000
              36   40 - #000000
              36   44 - #000000
              36   48 - #000000
              36   52 - #000000
              36   56 - #000000
              36   60 - #000000
             192  152 - #bcbcbc
             432  156 - #bcbcbc
             592  160 - #597c95
               4  196 - #597c95
             164  284 - #bcbcbc
             300  300 - #bcbcbc
             576  300 - #bcbcbc
             440  312 - #bcbcbc
              36  328 - #bcbcbc
               4  400 - #597c95
             180  416 - #bcbcbc
             592  436 - #597c95
             428  448 - #bcbcbc
              52  464 - #bcbcbc
             576  576 - #bcbcbc
             148  580 - #597c95
               4  592 - #597c95
             292  592 - #597c95
             436  592 - #597c95
";

const CHECK_3: &str = r"
             368   20 - #bcbcbc
              40   40 - #000000
              56   44 - #000000
              68   44 - #000000
              84   44 - #3e3e3e
              48   48 - #3b3b3b
              84   48 - #3e3e3e
              48   52 - #3b3b3b
              72   52 - #bcbcbc
              84   52 - #3e3e3e
             140   68 - #2f2f2f
             108   80 - #000000
             140   80 - #2f2f2f
              40   84 - #010101
              48   84 - #bcbcbc
              92   84 - #bcbcbc
              56   88 - #010101
              64   88 - #bcbcbc
              72   88 - #010101
             132   88 - #bcbcbc
              48   92 - #010101
             108   92 - #000000
             140   92 - #2f2f2f
             140  100 - #2f2f2f
             576  132 - #bcbcbc
             344  308 - #bcbcbc
               4  336 - #597c95
             592  352 - #597c95
             180  428 - #bcbcbc
             576  576 - #bcbcbc
               4  592 - #597c95
             292  592 - #597c95
";

const CHECK_4: &str = r"
             348   20 - #bcbcbc
             576   20 - #bcbcbc
              44   40 - #000000
              72   40 - #939393
              84   44 - #8e8e8e
             100   44 - #bcbcbc
              84   48 - #8e8e8e
              52   52 - #010101
              68   52 - #000000
              84   52 - #8e8e8e
             100   56 - #010101
             116   56 - #010101
              36   68 - #000000
             136   76 - #000000
              36   84 - #000000
              48   84 - #bcbcbc
              60   84 - #bcbcbc
             108   84 - #000000
              76   88 - #bcbcbc
              92   88 - #bcbcbc
             132   88 - #bcbcbc
              48   92 - #010101
              36  100 - #000000
             592  188 - #597c95
             388  284 - #bcbcbc
               4  344 - #597c95
             592  360 - #597c95
             208  396 - #bcbcbc
             428  464 - #bcbcbc
             576  576 - #bcbcbc
               4  592 - #597c95
             292  592 - #597c95
";

const CHECK_5: &str = r"
             576   20 - #bcbcbc
             356   24 - #bcbcbc
             120   40 - #939393
              76   44 - #9c9c9c
              88   44 - #1b1b1b
             124   44 - #181818
              44   48 - #bcbcbc
              60   48 - #000000
              76   48 - #9c9c9c
              88   48 - #1b1b1b
             124   48 - #181818
              76   52 - #9c9c9c
              88   52 - #1b1b1b
              96   52 - #616161
             112   56 - #010101
             136   76 - #000000
             108   80 - #000000
              56   84 - #000000
              76   84 - #bcbcbc
             144   84 - #000000
              64   88 - #bcbcbc
              92   88 - #bcbcbc
             128   88 - #bcbcbc
              48   92 - #010101
             108   92 - #000000
             136   92 - #000000
             396  288 - #bcbcbc
               4  340 - #597c95
             592  364 - #597c95
             576  576 - #bcbcbc
               4  592 - #597c95
             292  592 - #597c95
";

const CHECK_6: &str = r"
             120   40 - #939393
              76   44 - #9c9c9c
             124   44 - #181818
              44   48 - #bcbcbc
              76   52 - #9c9c9c
              88   52 - #1b1b1b
             136   76 - #000000
             220   76 - #565656
             176   80 - #9f9f9f
             248   80 - #000000
             308   80 - #292929
             400   80 - #444444
             436   80 - #676767
              76   84 - #bcbcbc
             400   84 - #444444
             436   84 - #676767
             176   88 - #9f9f9f
             220   88 - #565656
             308   88 - #292929
             320   88 - #000000
             520   88 - #bcbcbc
              48   92 - #010101
             108   92 - #000000
             292   92 - #000000
             364   92 - #090909
             436   92 - #676767
              80  336 - #bcbcbc
             584  336 - #597c95
             312  420 - #bcbcbc
              20  576 - #bcbcbc
             216  592 - #597c95
             592  592 - #597c95
";

const CHECK_7: &str = r"
             564   20 - #bcbcbc
             228   32 - #252525
             328   32 - #a5a5a5
              44   40 - #bcbcbc
             396   76 - #393939
             144   80 - #000000
              60  120 - #292929
             240  120 - #6e6e6e
             104  184 - #a2a2a2
             396  184 - #474747
             592  192 - #597c95
             260  212 - #595959
             152  288 - #999999
             296  296 - #000000
             468  296 - #bcbcbc
              60  304 - #282828
             576  308 - #7a7a7a
             160  372 - #bcbcbc
             364  372 - #707070
             260  380 - #606060
              72  412 - #232323
             404  420 - #010101
             572  420 - #bcbcbc
             188  444 - #000000
             280  476 - #060606
             348  500 - #454545
             116  512 - #5b5b5b
             304  564 - #565656
              60  568 - #9a9a9a
             196  568 - #525252
             576  572 - #7a7a7a
             440  592 - #597c95
";

const CHECK_8: &str = r"
             592    4 - #597c95
              44   40 - #bcbcbc
             392   72 - #1d1d1d
             308   84 - #292929
             228  116 - #7e7e7e
              96  140 - #7c7c7c
             576  140 - #7a7a7a
             412  152 - #6d6d6d
             332  184 - #000000
              88  192 - #2f2f2f
             168  196 - #939393
             248  196 - #a5a5a5
              72  268 - #3e3e3e
             572  280 - #bcbcbc
             412  300 - #bcbcbc
             184  304 - #1b1b1b
             300  304 - #010101
             576  364 - #7a7a7a
             188  408 - #000000
             308  412 - #474747
              44  420 - #0f0f0f
             396  420 - #474747
             260  448 - #595959
             576  448 - #bcbcbc
             324  480 - #2f2f2f
             220  496 - #000000
              96  516 - #373737
             432  532 - #6a6a6a
             132  564 - #828282
             152  572 - #323232
             276  572 - #a3a3a3
             592  592 - #597c95
";

const CHECK_9: &str = r"
             592    4 - #597c95
              44   40 - #bcbcbc
             176   72 - #9f9f9f
             392   72 - #1d1d1d
             228  116 - #7e7e7e
             308  116 - #6a6a6a
              96  140 - #7c7c7c
             576  140 - #7a7a7a
             132  192 - #748baf
             392  192 - #000000
              92  196 - #8ca7d3
             132  196 - #748baf
             156  196 - #8ca7d3
             228  200 - #8ca7d3
              72  268 - #3e3e3e
             572  280 - #bcbcbc
             184  304 - #1b1b1b
             300  304 - #010101
             412  308 - #bcbcbc
             112  368 - #000000
             188  408 - #000000
              44  420 - #0f0f0f
             396  420 - #474747
             260  440 - #595959
             576  448 - #bcbcbc
             112  480 - #474747
             324  480 - #2f2f2f
             324  492 - #2f2f2f
             432  532 - #6a6a6a
             132  564 - #828282
             276  572 - #a3a3a3
             592  592 - #597c95
";

const CHECK_10: &str = r"
             576   32 - #7a7a7a
              44   40 - #bcbcbc
             176   72 - #9f9f9f
             308   72 - #292929
             436   76 - #676767
              60  116 - #282828
             208  152 - #232323
              88  188 - #2f2f2f
             312  192 - #778db3
             248  196 - #7b93b9
             388  196 - #7187aa
             592  220 - #597c95
             336  236 - #8ca7d3
             172  244 - #8ca7d3
             108  264 - #617392
              60  268 - #8ca7d3
             300  304 - #010101
             360  304 - #444444
             412  304 - #565656
             528  320 - #bcbcbc
             184  368 - #8e8e8e
              52  380 - #bcbcbc
             412  416 - #000000
             308  420 - #474747
             576  428 - #bcbcbc
             152  440 - #797979
             324  480 - #2f2f2f
              96  516 - #373737
             296  532 - #323232
             468  536 - #000000
             132  564 - #828282
             592  592 - #597c95
";

const CHECK_11: &str = r"
              44   32 - #bcbcbc
             168   36 - #939393
             464   36 - #000000
             328   40 - #4a4a4a
             592   88 - #597c95
             364  112 - #5b5b5b
             156  120 - #535353
             148  172 - #8ca7d3
             236  172 - #8ca7d3
             396  184 - #474747
             104  188 - #7990b6
              64  196 - #8ca7d3
             104  196 - #7990b6
             184  196 - #8ca7d3
             152  216 - #797979
             572  240 - #bcbcbc
             120  272 - #181818
             300  296 - #000000
             432  304 - #6a6a6a
             212  308 - #000000
             120  340 - #181818
             360  380 - #939393
              60  384 - #9a9a9a
             576  400 - #7a7a7a
             236  412 - #bcbcbc
             140  452 - #232323
             348  484 - #414141
             300  564 - #828282
              44  572 - #0f0f0f
             232  572 - #3e3e3e
             576  576 - #bcbcbc
             436  592 - #597c95
";

const CHECK_12: &str = r"
             392   20 - #8ca7d3
             508   20 - #8ca7d3
              44   32 - #8ca7d3
             200   32 - #51617a
             328   32 - #374254
             280   44 - #778eb4
              60  116 - #1f252e
             144  144 - #7b93b9
             404  144 - #000000
             576  176 - #7a7a7a
             232  184 - #323c4c
             104  192 - #637796
             352  224 - #617392
             152  288 - #7288ac
             296  296 - #000000
             576  296 - #7a7a7a
             456  300 - #637796
              60  304 - #1e232d
             104  372 - #7990b6
             308  404 - #000000
             572  412 - #bcbcbc
             400  416 - #617493
             196  444 - #000000
              80  448 - #8ca7d3
             348  500 - #333d4d
             116  512 - #445167
             228  512 - #323c4b
              80  564 - #404c60
             184  564 - #404c60
             572  576 - #bcbcbc
             288  592 - #597c95
             440  592 - #597c95
";

const CHECK_13: &str = r"
             576   20 - #bcbcbc
              44   32 - #bcbcbc
             428   32 - #7f7f7f
             160   40 - #bcbcbc
             280   44 - #a0a0a0
              60  108 - #292929
             120  156 - #808080
             268  156 - #000000
             444  156 - #3b3b3b
             576  180 - #7a7a7a
             124  208 - #8ca7d3
              64  216 - #8ca7d3
             352  224 - #828282
              96  228 - #8ca7d3
             148  228 - #8ca7d3
             296  296 - #000000
             464  300 - #010101
              60  304 - #282828
             592  316 - #597c95
             152  336 - #323232
             288  372 - #202020
             400  416 - #838383
             172  420 - #3b3b3b
              48  444 - #676767
             576  444 - #7a7a7a
             264  452 - #989898
             348  500 - #454545
             176  524 - #707070
              80  564 - #565656
             276  568 - #060606
             572  576 - #bcbcbc
             440  592 - #597c95
";

const CHECK_14: &str = r"
             572    4 - #597c95
             228   32 - #252525
             328   32 - #a5a5a5
              44   40 - #bcbcbc
             136   44 - #000000
             396   76 - #393939
              60  116 - #292929
             240  120 - #6e6e6e
             104  184 - #a2a2a2
             396  184 - #474747
             592  192 - #597c95
             260  212 - #595959
             352  252 - #262626
             152  288 - #999999
             296  296 - #000000
             432  296 - #6b6b6b
              60  304 - #282828
             576  304 - #7a7a7a
              40  364 - #707070
             128  376 - #bcbcbc
             260  380 - #606060
             364  416 - #010101
             572  420 - #bcbcbc
             188  444 - #000000
              84  452 - #323232
             116  512 - #5b5b5b
             228  512 - #434343
             412  512 - #050505
             304  564 - #565656
              60  568 - #9a9a9a
             184  568 - #bcbcbc
             484  576 - #bcbcbc
";

const CHECK_15: &str = r"
             228   20 - #bcbcbc
             316   20 - #bcbcbc
              60   40 - #bcbcbc
              48   44 - #737373
              84   44 - #565656
              96   68 - #373737
             152   68 - #9a9a9a
              60   80 - #292929
             116  112 - #9a9a9a
             164  112 - #484848
             108  116 - #404040
             236  116 - #bcbcbc
             280  116 - #464646
              48  120 - #6f6f6f
              84  120 - #646464
             120  120 - #9a9a9a
             192  120 - #333333
             120  152 - #1a1a1a
             276  152 - #a3a3a3
             100  156 - #404040
             164  160 - #030303
             244  160 - #000000
             316  172 - #7a7a7a
             316  188 - #7a7a7a
              84  192 - #bcbcbc
              40  216 - #121212
              60  216 - #9a9a9a
             164  216 - #000000
             216  216 - #060606
             288  216 - #202020
             128  592 - #597c95
             592  592 - #597c95
";

const CHECK_16: &str = r"
             228   20 - #ffffff
             316   20 - #ffffff
             108   36 - #ffffff
              60   40 - #ffffff
              48   44 - #9c9c9c
              84   44 - #747474
             152   68 - #d1d1d1
             196   80 - #000000
              96   84 - #4b4b4b
             120  112 - #d1d1d1
             152  116 - #e9e9e9
             164  116 - #616161
             272  116 - #2a2a2a
              40  120 - #080808
              84  120 - #888888
             116  120 - #d1d1d1
             152  120 - #e9e9e9
             164  120 - #616161
             112  140 - #5f5f5f
             100  156 - #565656
             204  156 - #000000
              68  160 - #181818
             152  160 - #444444
             276  160 - #dddddd
             316  172 - #a6a6a6
             316  180 - #a6a6a6
             316  188 - #a6a6a6
              40  216 - #181818
             216  216 - #080808
             288  216 - #2b2b2b
             128  592 - #597c95
             592  592 - #597c95
";

#[view]
struct MultilineTextField {
    #[init]
    field: TextField,
}

impl Setup for MultilineTextField {
    fn setup(mut self: Weak<Self>) {
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
        assert!(view.field.selected_text().is_empty());
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
        assert!(view.field.selected_text().is_empty());
    });

    // Shift plus arrows extend, a plain arrow collapses.
    inject_modifiers(ModifiersState::SHIFT);
    inject_named_key(NamedKey::ArrowLeft);
    inject_modifiers(ModifiersState::empty());

    from_main(move || {
        assert_eq!(view.field.selected_text(), "X");
    });

    inject_named_key(NamedKey::ArrowRight);

    from_main(move || {
        assert!(view.field.selected_text().is_empty());
    });

    // Select all, cut, paste. The clipboard part needs a display server,
    // a headless CI box has none, so it is checked only where it works.
    inject_modifiers(ModifiersState::SUPER);
    inject_keys("a");
    inject_modifiers(ModifiersState::empty());

    let whole = from_main(move || {
        let text = view.field.text().to_string();
        assert_eq!(view.field.selected_text(), text);
        text
    });

    check_colors(CHECK_12)?;

    if from_main(|| Clipboard::set_text("probe")).is_ok() {
        inject_modifiers(ModifiersState::SUPER);
        inject_keys("x");
        inject_modifiers(ModifiersState::empty());

        from_main(move || {
            assert!(view.field.is_placeholding(), "cut must empty the field");
        });

        inject_modifiers(ModifiersState::SUPER);
        inject_keys("v");
        inject_modifiers(ModifiersState::empty());

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
        assert!(view.field.selected_text().is_empty());
    });

    Ok(())
}

fn check_shift_tap_and_copy(view: Weak<MultilineTextField>) {
    // Shift plus a tap extends from the caret to the tap.
    inject_keys("hello world");
    inject_named_key(NamedKey::Home);
    inject_modifiers(ModifiersState::SHIFT);
    inject_touches("160 240 b\n160 240 e");
    inject_modifiers(ModifiersState::empty());

    from_main(move || {
        let selected = view.field.selected_text();
        assert!(!selected.is_empty(), "shift tap must select");
        assert!(view.field.text().contains(&selected));
    });

    // Copy leaves the text alone, paste at the end doubles the selection.
    if from_main(|| Clipboard::set_text("probe")).is_ok() {
        let selected = from_main(move || view.field.selected_text());

        inject_modifiers(ModifiersState::SUPER);
        inject_keys("c");
        inject_modifiers(ModifiersState::empty());

        let text = from_main(move || view.field.text().to_string());

        inject_named_key(NamedKey::ArrowRight);
        inject_named_key(NamedKey::End);

        inject_modifiers(ModifiersState::SUPER);
        inject_keys("v");
        inject_modifiers(ModifiersState::empty());

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
        assert!(!view.field.text().is_empty());
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
