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
              540    4 - #597c95
              388   20 - #ffffff
               36   36 - #bcbcbc
               44   36 - #bcbcbc
              112   36 - #bcbcbc
               72   44 - #ffffff
               92   44 - #ffffff
              112   44 - #bcbcbc
              160   44 - #ffffff
              128   48 - #bcbcbc
               56   52 - #bcbcbc
               68   52 - #bcbcbc
               72   52 - #ffffff
               76   52 - #ffffff
              112   52 - #bcbcbc
              136   52 - #ffffff
              140   52 - #ffffff
              156   52 - #bcbcbc
               92   56 - #bcbcbc
              132   56 - #bcbcbc
              164   56 - #bcbcbc
               68   60 - #bcbcbc
              576  152 - #ffffff
              324  300 - #ffffff
               16  324 - #597c95
              592  364 - #597c95
              180  432 - #ffffff
              428  460 - #ffffff
              576  576 - #ffffff
              148  580 - #597c95
                4  592 - #597c95
              292  592 - #597c95
";

const CHECK_2: &str = r"
              540    4 - #597c95
              388   20 - #ffffff
               32   32 - #000000
               44   36 - #bcbcbc
              112   36 - #bcbcbc
               32   44 - #000000
               72   44 - #ffffff
               92   44 - #ffffff
              112   44 - #bcbcbc
              160   44 - #ffffff
              128   48 - #bcbcbc
               32   52 - #000000
               56   52 - #bcbcbc
               76   52 - #ffffff
              112   52 - #bcbcbc
              136   52 - #ffffff
              140   52 - #ffffff
              156   52 - #bcbcbc
               32   56 - #000000
               92   56 - #bcbcbc
              132   56 - #bcbcbc
              164   56 - #bcbcbc
               32   60 - #000000
               68   60 - #bcbcbc
              576  152 - #ffffff
              324  300 - #ffffff
                4  324 - #597c95
              592  364 - #597c95
              172  432 - #ffffff
              576  576 - #ffffff
                4  592 - #597c95
              292  592 - #597c95
";

const CHECK_3: &str = r"
              576   20 - #ffffff
              352   32 - #ffffff
               36   40 - #000000
               68   44 - #ffffff
               52   48 - #010101
               36   52 - #010101
               68   56 - #010101
               84   56 - #000000
              132   76 - #000000
              104   80 - #010101
               36   84 - #000000
               44   84 - #ffffff
               68   84 - #010101
               88   84 - #ffffff
               96   84 - #000000
              124   84 - #ffffff
               60   88 - #ffffff
               76   88 - #ffffff
              128   88 - #ffffff
               44   92 - #010101
              104   92 - #010101
              132   92 - #000000
              592  188 - #597c95
              388  284 - #ffffff
                4  336 - #597c95
              592  360 - #597c95
              208  396 - #ffffff
              428  464 - #ffffff
              576  576 - #ffffff
              148  580 - #597c95
                4  592 - #597c95
              292  592 - #597c95
";

const CHECK_4: &str = r"
              576   20 - #ffffff
              348   28 - #ffffff
               40   40 - #000000
               96   44 - #ffffff
               64   48 - #000000
              104   48 - #ffffff
               48   52 - #010101
               96   56 - #000000
              112   56 - #000000
               32   68 - #000000
              132   76 - #000000
              104   80 - #010101
               32   84 - #000000
               44   84 - #ffffff
               68   84 - #010101
               56   88 - #ffffff
               76   88 - #ffffff
               92   88 - #ffffff
              128   88 - #ffffff
               44   92 - #010101
              104   92 - #010101
               32  100 - #000000
              592  188 - #597c95
              388  284 - #ffffff
                4  344 - #597c95
              592  360 - #597c95
              208  396 - #ffffff
              428  464 - #ffffff
              576  576 - #ffffff
              148  580 - #597c95
                4  592 - #597c95
              292  592 - #597c95
";

const CHECK_5: &str = r"
              352   20 - #ffffff
              576   20 - #ffffff
               92   44 - #000000
               40   48 - #ffffff
               56   48 - #000000
              112   48 - #ffffff
              104   56 - #010101
              132   76 - #000000
              104   80 - #010101
               36   84 - #000000
               44   84 - #ffffff
               72   84 - #ffffff
               96   84 - #000000
              132   84 - #000000
              140   84 - #000000
               60   88 - #ffffff
               88   88 - #ffffff
              124   88 - #ffffff
              132   88 - #000000
               44   92 - #010101
              104   92 - #010101
              132   92 - #000000
              592  188 - #597c95
              392  284 - #ffffff
                4  336 - #597c95
              592  360 - #597c95
              208  396 - #ffffff
              428  464 - #ffffff
              576  576 - #ffffff
              148  580 - #597c95
                4  592 - #597c95
              292  592 - #597c95
";

const CHECK_6: &str = r"
                40   40 - #000000
                92   44 - #000000
                56   48 - #000000
               108   56 - #010101
               132   76 - #000000
               288   76 - #000000
                72   84 - #ffffff
               160   84 - #ffffff
               256   84 - #ffffff
               424   84 - #000000
               464   84 - #ffffff
               516   84 - #ffffff
               228   88 - #010101
               320   88 - #ffffff
               372   88 - #ffffff
                44   92 - #010101
               104   92 - #010101
               184   92 - #000000
               288   92 - #000000
               404   92 - #000000
               592  204 - #597c95
               368  264 - #ffffff
               188  268 - #ffffff
                16  316 - #597c95
               528  332 - #ffffff
               284  416 - #ffffff
               592  444 - #597c95
                16  456 - #597c95
               188  576 - #ffffff
               576  576 - #ffffff
                 4  592 - #597c95
               376  592 - #597c95
";

const CHECK_7: &str = r"
              448    4 - #597c95
              592    4 - #597c95
              308   28 - #000000
               40   36 - #ffffff
              168   40 - #ffffff
              584   96 - #597c95
              108  112 - #010101
              420  148 - #ffffff
              300  156 - #010101
               20  172 - #ffffff
              176  184 - #010101
              576  188 - #a6a6a6
               92  228 - #ffffff
              372  232 - #010101
              576  232 - #a6a6a6
              572  272 - #ffffff
              440  296 - #000000
              308  300 - #010101
              164  304 - #010101
               40  308 - #010101
              576  348 - #a6a6a6
              592  420 - #597c95
              288  424 - #ffffff
              164  444 - #010101
              416  444 - #ffffff
               36  448 - #000000
              576  496 - #a6a6a6
              156  572 - #ffffff
              292  572 - #ffffff
              576  572 - #a6a6a6
               20  576 - #ffffff
              432  592 - #597c95
";

const CHECK_8: &str = r"
              172    4 - #597c95
              576   20 - #ffffff
              308   32 - #ffffff
               40   40 - #ffffff
              444   40 - #ffffff
              228   80 - #010101
              576   96 - #a6a6a6
              296  152 - #000000
              176  160 - #000000
              592  168 - #597c95
              416  172 - #ffffff
               40  184 - #ffffff
              576  240 - #a6a6a6
              168  292 - #000000
              452  292 - #ffffff
               40  300 - #ffffff
              296  304 - #000000
              576  312 - #a6a6a6
              576  364 - #a6a6a6
              108  368 - #000000
              416  412 - #ffffff
              576  416 - #a6a6a6
                4  436 - #597c95
              176  440 - #000000
              304  452 - #000000
              100  492 - #ffffff
              584  504 - #597c95
              464  536 - #010101
              192  568 - #ffffff
              324  568 - #000000
               52  572 - #000000
              592  592 - #597c95
";

const CHECK_9: &str = r"
              204    4 - #597c95
              372   20 - #ffffff
              576   20 - #ffffff
               40   40 - #ffffff
              476   92 - #ffffff
              576   96 - #a6a6a6
              576  168 - #a6a6a6
              116  172 - #b8d2ff
              184  172 - #b8d2ff
              232  172 - #b8d2ff
              152  188 - #b8d2ff
              400  188 - #010101
               88  196 - #b8d2ff
              124  196 - #b8d2ff
              196  204 - #b8d2ff
              232  204 - #b8d2ff
              576  240 - #a6a6a6
                4  272 - #597c95
              408  300 - #ffffff
              296  304 - #010101
              576  312 - #a6a6a6
              168  340 - #000000
               44  376 - #ffffff
              436  420 - #000000
              208  452 - #ffffff
              576  452 - #ffffff
              336  460 - #010101
              468  528 - #010101
              324  568 - #010101
               52  572 - #000000
              188  592 - #597c95
              592  592 - #597c95
";

const CHECK_10: &str = r"
              140    4 - #597c95
              376    4 - #597c95
              576   20 - #ffffff
               40   40 - #ffffff
              220   76 - #ffffff
              456   80 - #010101
              576  104 - #a6a6a6
               88  152 - #ffffff
              212  172 - #b8d2ff
              400  172 - #b8d2ff
              284  176 - #b8d2ff
              576  184 - #a6a6a6
              108  232 - #b8d2ff
              184  236 - #b8d2ff
              340  236 - #b8d2ff
              256  244 - #b8d2ff
              576  264 - #a6a6a6
               56  268 - #b8d2ff
              296  304 - #010101
              408  308 - #ffffff
              124  332 - #000000
              576  344 - #a6a6a6
              164  416 - #ffffff
              436  420 - #000000
               20  436 - #ffffff
              312  448 - #000000
              552  472 - #ffffff
              200  560 - #000000
               80  572 - #000000
              332  572 - #020202
              460  592 - #597c95
              592  592 - #597c95
";

const CHECK_11: &str = r"
               592    4 - #597c95
               208   28 - #ffffff
                40   32 - #ffffff
               396   44 - #ffffff
               544  140 - #ffffff
               328  148 - #010101
                72  172 - #b8d2ff
               152  172 - #b8d2ff
               232  172 - #b8d2ff
               208  188 - #b8d2ff
               124  192 - #b8d2ff
               228  192 - #b8d2ff
               408  192 - #000000
               180  196 - #b8d2ff
                88  204 - #b8d2ff
               576  208 - #a6a6a6
               576  284 - #a6a6a6
               296  296 - #010101
               460  300 - #ffffff
               144  308 - #ffffff
                 4  352 - #597c95
               576  424 - #a6a6a6
               148  428 - #000000
               440  432 - #ffffff
               304  440 - #000000
                 4  456 - #597c95
               576  500 - #a6a6a6
                36  552 - #000000
               160  564 - #000000
               284  572 - #010101
               576  576 - #ffffff
               428  592 - #597c95
";

const CHECK_12: &str = r"
              188    4 - #597c95
              452   20 - #b8d2ff
              576   20 - #ffffff
               40   32 - #b8d2ff
              332   44 - #000000
              124   80 - #b8d2ff
              432  148 - #000000
              320  156 - #b8d2ff
              180  160 - #b8d2ff
               20  168 - #ffffff
              576  172 - #a6a6a6
               96  228 - #000000
              576  248 - #a6a6a6
              444  284 - #b8d2ff
              164  296 - #010101
              308  300 - #010101
               36  304 - #000000
              576  320 - #a6a6a6
              396  376 - #ffffff
               88  388 - #b8d2ff
              184  420 - #010101
                4  440 - #597c95
              320  444 - #010101
              576  448 - #a6a6a6
              452  464 - #ffffff
              108  484 - #010101
              296  568 - #b8d2ff
              168  572 - #b8d2ff
               32  576 - #b8d2ff
              416  576 - #ffffff
              504  584 - #597c95
              592  592 - #597c95
";

const CHECK_13: &str = r"
              576   20 - #ffffff
               40   32 - #ffffff
              460   32 - #000000
              332   36 - #000000
              192   68 - #000000
                4  136 - #597c95
              300  156 - #010101
              592  164 - #597c95
              452  180 - #ffffff
               72  208 - #b8d2ff
              100  208 - #b8d2ff
              152  208 - #b8d2ff
              144  224 - #b8d2ff
               60  232 - #b8d2ff
               80  232 - #b8d2ff
              120  232 - #b8d2ff
              152  240 - #b8d2ff
              576  240 - #a6a6a6
              308  300 - #010101
              576  320 - #a6a6a6
              160  344 - #000000
              436  388 - #ffffff
              576  388 - #a6a6a6
                4  396 - #597c95
              308  416 - #ffffff
              192  444 - #010101
              576  456 - #a6a6a6
              364  528 - #ffffff
              484  536 - #ffffff
               64  568 - #000000
              232  568 - #ffffff
              592  592 - #597c95
";

const CHECK_14: &str = r"
              404    4 - #597c95
              592    4 - #597c95
              308   28 - #000000
               40   36 - #ffffff
              176   48 - #ffffff
              456   88 - #ffffff
              572  120 - #ffffff
              328  156 - #000000
                4  172 - #597c95
              176  184 - #010101
               92  228 - #ffffff
              532  232 - #ffffff
              412  260 - #ffffff
              576  276 - #a6a6a6
              308  300 - #010101
              164  304 - #010101
               40  308 - #010101
              576  308 - #a6a6a6
              576  340 - #a6a6a6
              248  376 - #000000
              576  400 - #a6a6a6
              416  416 - #ffffff
              164  444 - #010101
               36  448 - #000000
              304  452 - #000000
              576  460 - #a6a6a6
              384  532 - #000000
              156  572 - #ffffff
              292  572 - #ffffff
               20  576 - #ffffff
              592  588 - #597c95
              464  592 - #597c95
";

const CHECK_15: &str = r"
              592    4 - #597c95
              164   20 - #ffffff
              240   20 - #ffffff
              316   20 - #ffffff
              104   36 - #ffffff
               56   40 - #ffffff
              116   76 - #ffffff
              192   76 - #000000
               68   84 - #010101
              300   96 - #ffffff
              244  112 - #ffffff
              144  116 - #ffffff
               36  120 - #000000
              196  120 - #000000
              108  148 - #000000
              240  148 - #ffffff
              280  152 - #000000
               52  160 - #010101
              172  160 - #000000
              316  172 - #a6a6a6
              316  176 - #a6a6a6
              316  180 - #a6a6a6
              316  184 - #a6a6a6
              316  188 - #a6a6a6
               88  196 - #010101
               20  216 - #ffffff
              160  216 - #000000
              212  216 - #010101
              592  296 - #597c95
              356  456 - #597c95
              116  592 - #597c95
              592  592 - #597c95
";

const CHECK_16: &str = r"
              592    4 - #597c95
              164   20 - #ffffff
              240   20 - #ffffff
              316   20 - #ffffff
              104   36 - #ffffff
               56   40 - #ffffff
              116   76 - #ffffff
              192   76 - #000000
               68   84 - #010101
              300   96 - #ffffff
              244  112 - #ffffff
              144  116 - #ffffff
               36  120 - #010101
              196  120 - #000000
              108  148 - #000000
              240  148 - #ffffff
              280  152 - #010101
               52  160 - #010101
              172  160 - #000000
              316  172 - #a6a6a6
              316  176 - #a6a6a6
              316  180 - #a6a6a6
              316  184 - #a6a6a6
              316  188 - #a6a6a6
               88  196 - #010101
               20  216 - #ffffff
              160  216 - #000000
              212  216 - #010101
              592  296 - #597c95
              356  456 - #597c95
              116  592 - #597c95
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
