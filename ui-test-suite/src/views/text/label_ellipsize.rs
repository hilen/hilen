use anyhow::Result;
use hilen::{
    AppRunner,
    dispatch::from_main,
    refs::Weak,
    ui::{
        BLACK, Label, RED, Screenshot, Setup, TextAlignment, U8Color, UIColor, ViewData, ViewFrame, ViewTest,
        WHITE, view,
    },
    ui_test::{check_colors, checkpoint, set_record_probe_count},
};

const TEXT: &str = "src/ui/settings_sheet/tools_tab.rs";

const LABELS: &str = r"
             592    4 - #597c95
             356   20 - #ffffff
             396   20 - #ffffff
             20   24 - #ffffff
             188   36 - #000000
             236   36 - #000000
             264   36 - #4d4d4d
             60   40 - #ffffff
             84   40 - #2f2f2f
             112   40 - #969696
             120   40 - #787878
             136   40 - #000000
             152   40 - #ffffff
             220   40 - #d8d8d8
             244   40 - #ffffff
             264   40 - #4d4d4d
             292   40 - #ffffff
             308   40 - #ffffff
             48   44 - #000000
             84   44 - #2f2f2f
             132   44 - #010101
             144   44 - #a9a9a9
             188   44 - #010101
             196   44 - #b7b7b7
             236   44 - #000000
             264   44 - #4d4d4d
             324   44 - #e7e7e7
             380   44 - #ffffff
             160   48 - #ffffff
             172   48 - #aeaeae
             280   48 - #aeaeae
             284   48 - #aeaeae
             352   52 - #ffffff
             416   56 - #ffffff
             20   72 - #ffffff
             376   72 - #ffffff
             516   72 - #597c95
             348   80 - #ffffff
             404   80 - #ffffff
             188   84 - #000000
             264   84 - #4d4d4d
             84   88 - #2f2f2f
             132   88 - #010101
             236   88 - #000000
             264   88 - #4d4d4d
             48   92 - #000000
             60   92 - #ffffff
             84   92 - #2f2f2f
             120   92 - #787878
             144   92 - #a9a9a9
             188   92 - #010101
             196   92 - #b7b7b7
             204   92 - #f0f0f0
             220   92 - #d8d8d8
             244   92 - #ffffff
             256   92 - #ffffff
             264   92 - #4d4d4d
             292   92 - #ffffff
             308   92 - #ffffff
             324   92 - #e7e7e7
             172   96 - #d6d6d6
             280   96 - #d6d6d6
             284   96 - #d6d6d6
             388  100 - #ffffff
             20  108 - #ffffff
             360  108 - #ffffff
             416  108 - #ffffff
             176  132 - #ffffff
             44  140 - #ffffff
             60  140 - #ffffff
             84  140 - #2f2f2f
             112  140 - #969696
             120  140 - #787878
             136  140 - #000000
             152  140 - #ffffff
             48  144 - #000000
             84  144 - #2f2f2f
             132  144 - #010101
             144  144 - #a9a9a9
             20  156 - #ffffff
             568  160 - #597c95
             104  172 - #ffffff
             176  172 - #ffffff
             480  172 - #597c95
             264  184 - #597c95
             20  188 - #ffffff
             84  188 - #2f2f2f
             132  188 - #010101
             48  192 - #000000
             60  192 - #ffffff
             84  192 - #2f2f2f
             112  192 - #f0f0f0
             120  192 - #787878
             132  192 - #010101
             144  192 - #a9a9a9
             152  192 - #ffffff
             376  200 - #597c95
             108  220 - #ffffff
             176  220 - #ffffff
             20  224 - #ffffff
             140  228 - #ffffff
             48  240 - #000000
             84  240 - #2f2f2f
             60  244 - #ffffff
             84  244 - #2f2f2f
             536  244 - #597c95
             20  252 - #ffffff
             448  256 - #597c95
             128  260 - #333333
             308  260 - #597c95
             60  264 - #3f3f3f
             72  264 - #000000
             92  264 - #ffffff
             164  264 - #ffffff
             40  268 - #ffffff
             52  268 - #ffffff
             76  268 - #414141
             84  268 - #707070
             124  268 - #010101
             128  268 - #333333
             136  268 - #7e7e7e
             144  268 - #ffffff
             156  268 - #ffffff
             112  272 - #000000
             68  284 - #808080
             92  284 - #000000
             108  284 - #292929
             44  288 - #000000
             68  288 - #808080
             92  288 - #000000
             104  288 - #000000
             124  288 - #acacac
             48  292 - #ffffff
             68  292 - #808080
             92  292 - #000000
             104  292 - #000000
             112  292 - #ffffff
             124  292 - #acacac
             84  296 - #3e3e3e
             20  308 - #ffffff
             148  308 - #ffffff
             176  308 - #ffffff
             592  316 - #597c95
             476  360 - #597c95
             264  364 - #597c95
             100  388 - #597c95
             368  388 - #597c95
             12  416 - #597c95
             532  452 - #597c95
             436  456 - #597c95
             148  468 - #597c95
             272  468 - #597c95
             64  484 - #597c95
             332  540 - #597c95
             4  544 - #597c95
             516  544 - #597c95
             424  552 - #597c95
             124  592 - #597c95
             252  592 - #597c95
             592  592 - #597c95
";

const RUNS: &str = r"
             592    4 - #597c95
             348   20 - #ffffff
             384   20 - #ffffff
             416   20 - #ffffff
             20   24 - #ffffff
             188   36 - #000000
             236   36 - #000000
             264   36 - #4d4d4d
             48   40 - #000000
             60   40 - #ffffff
             84   40 - #2f2f2f
             112   40 - #969696
             120   40 - #787878
             136   40 - #000000
             152   40 - #ffffff
             220   40 - #d8d8d8
             244   40 - #ffffff
             264   40 - #4d4d4d
             292   40 - #ffffff
             308   40 - #ffffff
             80   44 - #000000
             84   44 - #2f2f2f
             132   44 - #010101
             144   44 - #a9a9a9
             188   44 - #010101
             196   44 - #b7b7b7
             236   44 - #000000
             264   44 - #4d4d4d
             324   44 - #e7e7e7
             160   48 - #ffffff
             172   48 - #aeaeae
             280   48 - #aeaeae
             284   48 - #aeaeae
             388   48 - #ffffff
             352   52 - #ffffff
             416   52 - #ffffff
             20   72 - #ffffff
             376   72 - #ffffff
             516   72 - #597c95
             348   80 - #ffffff
             408   80 - #ffffff
             188   84 - #000000
             264   84 - #4d4d4d
             84   88 - #2f2f2f
             132   88 - #010101
             236   88 - #000000
             264   88 - #4d4d4d
             48   92 - #000000
             60   92 - #ffffff
             84   92 - #2f2f2f
             120   92 - #787878
             144   92 - #a9a9a9
             152   92 - #ffffff
             188   92 - #010101
             196   92 - #b7b7b7
             204   92 - #f0f0f0
             220   92 - #d8d8d8
             236   92 - #000000
             244   92 - #ffffff
             256   92 - #ffffff
             264   92 - #4d4d4d
             292   92 - #ffffff
             308   92 - #ffffff
             324   92 - #e7e7e7
             172   96 - #d6d6d6
             280   96 - #d6d6d6
             284   96 - #d6d6d6
             388  100 - #ffffff
             20  108 - #ffffff
             360  108 - #ffffff
             416  108 - #ffffff
             76  120 - #ffffff
             116  124 - #ffffff
             56  136 - #ff0000
             40  140 - #ffffff
             56  140 - #ff0000
             92  140 - #000000
             56  144 - #ff0101
             92  148 - #010101
             568  160 - #597c95
             176  172 - #ffffff
             480  172 - #597c95
             20  188 - #ffffff
             48  188 - #000000
             84  188 - #2f2f2f
             132  188 - #010101
             56  192 - #ffffff
             84  192 - #2f2f2f
             112  192 - #f0f0f0
             120  192 - #787878
             132  192 - #010101
             144  192 - #a9a9a9
             152  192 - #ffffff
             388  192 - #597c95
             108  220 - #ffffff
             20  224 - #ffffff
             140  228 - #ffffff
             176  228 - #ffffff
             48  240 - #000000
             84  240 - #2f2f2f
             60  244 - #ffffff
             80  244 - #000000
             84  244 - #2f2f2f
             312  244 - #597c95
             536  244 - #597c95
             20  252 - #ffffff
             448  256 - #597c95
             128  260 - #333333
             60  264 - #3f3f3f
             72  264 - #000000
             92  264 - #ffffff
             40  268 - #ffffff
             52  268 - #ffffff
             76  268 - #414141
             84  268 - #707070
             124  268 - #010101
             128  268 - #333333
             136  268 - #7e7e7e
             144  268 - #ffffff
             156  268 - #ffffff
             164  268 - #ffffff
             112  272 - #000000
             68  284 - #808080
             92  284 - #000000
             108  284 - #292929
             48  288 - #ffffff
             60  288 - #ffffff
             68  288 - #808080
             92  288 - #000000
             104  288 - #000000
             124  288 - #acacac
             68  292 - #808080
             92  292 - #000000
             100  292 - #ffffff
             104  292 - #000000
             112  292 - #ffffff
             124  292 - #acacac
             84  296 - #3e3e3e
             376  300 - #597c95
             20  308 - #ffffff
             148  308 - #ffffff
             176  308 - #ffffff
             592  316 - #597c95
             264  364 - #597c95
             476  364 - #597c95
             112  384 - #597c95
             368  384 - #597c95
             32  428 - #597c95
             444  452 - #597c95
             540  452 - #597c95
             148  468 - #597c95
             276  468 - #597c95
             192  540 - #597c95
             336  540 - #597c95
             520  540 - #597c95
             4  544 - #597c95
             428  544 - #597c95
             124  592 - #597c95
             260  592 - #597c95
             592  592 - #597c95
";

const PLAIN_FRAME: (u32, u32, u32, u32) = (20, 20, 400, 40);
const WIDE_FRAME: (u32, u32, u32, u32) = (20, 70, 400, 40);
const CUT_FRAME: (u32, u32, u32, u32) = (20, 120, 160, 40);
const TWIN_FRAME: (u32, u32, u32, u32) = (20, 170, 160, 40);

#[view]
struct LabelEllipsize {
    #[init]
    plain: Label,
    wide:  Label,
    cut:   Label,
    twin:  Label,
    multi: Label,
}

impl Setup for LabelEllipsize {
    fn setup(self: Weak<Self>) {
        for label in [self.plain, self.wide, self.cut, self.twin] {
            label.set_color(WHITE).set_text_color(BLACK).set_text_size(20);
            label.set_text(TEXT).set_alignment(TextAlignment::Left);
        }

        self.plain.set_frame(PLAIN_FRAME);
        self.wide.set_ellipsize(true).set_frame(WIDE_FRAME);
        self.cut.set_ellipsize(true).set_frame(CUT_FRAME);
        self.twin.set_frame(TWIN_FRAME);

        self.multi.set_color(WHITE).set_text_color(BLACK).set_text_size(20);
        self.multi.set_text(TEXT).set_alignment(TextAlignment::Left);
        self.multi.set_multiline(true).set_ellipsize(true);
        self.multi.set_frame((20, 220, 160, 90));
    }
}

impl ViewTest for LabelEllipsize {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        set_record_probe_count(160);
        // Fitting text is left alone: the ellipsized label reports the
        // full text and renders pixel for pixel like its plain twin.
        let display = from_main(move || view.wide.display_text(view.wide.width()).to_string());
        assert_eq!(display, TEXT, "fitting text was truncated");

        let shot = AppRunner::take_screenshot()?;
        assert!(
            region(&shot, PLAIN_FRAME) == region(&shot, WIDE_FRAME),
            "ellipsize moved glyphs of fitting text"
        );

        // Overflowing text cuts to a prefix with the trailing ellipsis.
        let cut = from_main(move || view.cut.display_text(view.cut.width()).to_string());
        assert!(cut.ends_with('…'), "no trailing ellipsis: {cut:?}");
        assert!(cut.len() < TEXT.len(), "text was not shortened: {cut:?}");
        assert!(
            TEXT.starts_with(cut.trim_end_matches('…')),
            "cut text is not a prefix: {cut:?}"
        );

        // What the drawer paints is exactly that reported copy: a plain
        // label given the truncated string renders identically.
        let twin = cut.clone();
        from_main(move || {
            view.twin.set_text(twin);
        });
        let shot = AppRunner::take_screenshot()?;
        assert!(
            region(&shot, CUT_FRAME) == region(&shot, TWIN_FRAME),
            "drawn pixels do not match the reported truncation"
        );
        check_colors(LABELS)?;
        checkpoint("full, fitting, cut and twin labels")?;

        // A narrower frame recomputes to a shorter prefix.
        let narrower = from_main(move || {
            view.cut.set_frame((20, 120, 100, 40));
            view.cut.display_text(view.cut.width()).to_string()
        });
        assert!(
            narrower.len() < cut.len(),
            "narrower width kept the same text: {narrower:?} vs {cut:?}"
        );
        assert!(narrower.ends_with('…'), "no ellipsis after resize: {narrower:?}");

        // A new text drops the cached truncation.
        let changed = from_main(move || {
            view.cut.set_text("other/path/entirely/too/long/to/fit.rs");
            view.cut.display_text(view.cut.width()).to_string()
        });
        assert!(
            changed.starts_with("other") && changed.ends_with('…'),
            "stale truncation after set_text: {changed:?}"
        );

        // Multiline wraps instead and ignores the flag.
        let multi = from_main(move || view.multi.display_text(view.multi.width()).to_string());
        assert_eq!(multi, TEXT, "multiline text was truncated");

        // Color runs on an ellipsized label clamp to what is drawn, a run
        // past the cut must not slice inside the multi byte ellipsis.
        from_main(move || {
            view.cut
                .set_color_runs(vec![(0..5, UIColor::Plain(RED)), (5..100, UIColor::Plain(BLACK))]);
        });
        check_colors(RUNS)?;

        Ok(())
    }
}

fn region(shot: &Screenshot, frame: (u32, u32, u32, u32)) -> Vec<U8Color> {
    let (x, y, w, h) = frame;
    let mut pixels = Vec::with_capacity((w * h) as usize);

    for row in y..y + h {
        for col in x..x + w {
            pixels.push(shot.get_pixel((col, row)));
        }
    }

    pixels
}
