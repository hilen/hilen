use anyhow::Result;
use hilen::{
    AppRunner,
    dispatch::from_main,
    refs::Weak,
    ui::{
        BLACK, Label, RED, Screenshot, Setup, TextAlignment, U8Color, UIColor, ViewData, ViewFrame, ViewTest,
        WHITE, view,
    },
    ui_test::{check_colors, human_checkpoint, set_record_probe_count},
};

const TEXT: &str = "src/ui/settings_sheet/tools_tab.rs";

const LABELS: &str = r"
             516    4 - #597c95
             168   20 - #ffffff
             352   20 - #ffffff
             416   20 - #ffffff
              20   24 - #ffffff
             384   24 - #ffffff
             188   36 - #000000
             236   36 - #000000
             264   36 - #4d4d4d
              44   40 - #ffffff
              60   40 - #ffffff
              84   40 - #2f2f2f
             112   40 - #969696
             120   40 - #787878
             128   40 - #ffffff
             136   40 - #000000
             152   40 - #ffffff
             184   40 - #ffffff
             220   40 - #d8d8d8
             244   40 - #ffffff
             256   40 - #ffffff
             264   40 - #4d4d4d
             292   40 - #ffffff
             308   40 - #ffffff
              48   44 - #000000
              84   44 - #2f2f2f
             132   44 - #010101
             144   44 - #a9a9a9
             188   44 - #010101
             196   44 - #b7b7b7
             224   44 - #ffffff
             236   44 - #000000
             264   44 - #4d4d4d
             272   44 - #ffffff
             324   44 - #e7e7e7
             160   48 - #ffffff
             172   48 - #aeaeae
             280   48 - #aeaeae
             284   48 - #aeaeae
             416   48 - #ffffff
             368   52 - #ffffff
              20   72 - #ffffff
             400   72 - #ffffff
             592   72 - #597c95
             348   76 - #ffffff
             188   84 - #000000
             264   84 - #4d4d4d
              84   88 - #2f2f2f
             132   88 - #010101
             188   88 - #000000
             236   88 - #000000
             264   88 - #4d4d4d
              48   92 - #000000
              60   92 - #ffffff
              84   92 - #2f2f2f
             112   92 - #f0f0f0
             120   92 - #787878
             128   92 - #ffffff
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
             368  100 - #ffffff
             504  100 - #597c95
             344  108 - #ffffff
             412  108 - #ffffff
              20  124 - #ffffff
             176  124 - #ffffff
              48  140 - #000000
              60  140 - #ffffff
              84  140 - #2f2f2f
             112  140 - #969696
             120  140 - #787878
             128  140 - #ffffff
             132  140 - #010101
             136  140 - #000000
             152  140 - #ffffff
              84  144 - #2f2f2f
             132  144 - #010101
             136  144 - #080808
             144  144 - #a9a9a9
              20  172 - #ffffff
             176  172 - #ffffff
             556  176 - #597c95
             264  184 - #597c95
              80  188 - #000000
              84  188 - #2f2f2f
             132  188 - #010101
             384  188 - #597c95
              48  192 - #000000
              60  192 - #ffffff
              80  192 - #000000
              84  192 - #2f2f2f
             112  192 - #f0f0f0
             120  192 - #787878
             132  192 - #010101
             144  192 - #a9a9a9
             152  192 - #ffffff
              20  200 - #ffffff
             176  224 - #ffffff
              20  228 - #ffffff
              64  228 - #ffffff
             104  228 - #ffffff
             140  232 - #ffffff
             460  236 - #597c95
             160  248 - #ffffff
             324  248 - #597c95
              48  264 - #000000
              80  264 - #000000
              84  264 - #2f2f2f
             120  264 - #787878
             132  264 - #010101
              40  268 - #ffffff
              80  268 - #000000
              84  268 - #2f2f2f
             100  268 - #ffffff
             112  268 - #ffffff
             132  268 - #010101
             136  268 - #080808
             144  268 - #a9a9a9
             172  272 - #000000
             592  280 - #597c95
             252  292 - #597c95
              96  300 - #ffffff
             396  300 - #597c95
              20  308 - #ffffff
              64  308 - #ffffff
             124  308 - #ffffff
             164  308 - #ffffff
             480  344 - #597c95
             592  368 - #597c95
             248  376 - #597c95
             380  388 - #597c95
              80  392 - #597c95
               4  436 - #597c95
             540  436 - #597c95
             144  452 - #597c95
             448  460 - #597c95
             308  484 - #597c95
              72  508 - #597c95
             216  512 - #597c95
             516  528 - #597c95
             424  552 - #597c95
               4  568 - #597c95
             132  580 - #597c95
             260  592 - #597c95
             348  592 - #597c95
             592  592 - #597c95
";

const RUNS: &str = r"
             516    4 - #597c95
             168   20 - #ffffff
             352   20 - #ffffff
             416   20 - #ffffff
              20   24 - #ffffff
             384   24 - #ffffff
             188   36 - #000000
             236   36 - #000000
             264   36 - #4d4d4d
              44   40 - #ffffff
              60   40 - #ffffff
              84   40 - #2f2f2f
             112   40 - #969696
             120   40 - #787878
             128   40 - #ffffff
             136   40 - #000000
             152   40 - #ffffff
             184   40 - #ffffff
             220   40 - #d8d8d8
             236   40 - #000000
             244   40 - #ffffff
             256   40 - #ffffff
             264   40 - #4d4d4d
             292   40 - #ffffff
             308   40 - #ffffff
              48   44 - #000000
              84   44 - #2f2f2f
             132   44 - #010101
             144   44 - #a9a9a9
             188   44 - #010101
             196   44 - #b7b7b7
             224   44 - #ffffff
             236   44 - #000000
             264   44 - #4d4d4d
             272   44 - #ffffff
             324   44 - #e7e7e7
             160   48 - #ffffff
             172   48 - #aeaeae
             280   48 - #aeaeae
             284   48 - #aeaeae
             416   48 - #ffffff
             368   52 - #ffffff
              20   72 - #ffffff
             400   72 - #ffffff
             592   72 - #597c95
             348   76 - #ffffff
             188   84 - #000000
             264   84 - #4d4d4d
              84   88 - #2f2f2f
             132   88 - #010101
             188   88 - #000000
             236   88 - #000000
             264   88 - #4d4d4d
              48   92 - #000000
              60   92 - #ffffff
              84   92 - #2f2f2f
             112   92 - #f0f0f0
             120   92 - #787878
             128   92 - #ffffff
             132   92 - #010101
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
             368  100 - #ffffff
             504  100 - #597c95
              20  108 - #ffffff
             344  108 - #ffffff
             412  108 - #ffffff
              76  120 - #ffffff
             116  124 - #ffffff
              56  136 - #ff0000
              40  140 - #ffffff
              56  140 - #ff0000
              92  140 - #000000
              56  144 - #ff0101
              92  144 - #000000
              92  148 - #010101
              20  172 - #ffffff
             176  172 - #ffffff
             556  176 - #597c95
             264  184 - #597c95
              48  188 - #000000
              80  188 - #000000
              84  188 - #2f2f2f
             132  188 - #010101
             384  188 - #597c95
              60  192 - #ffffff
              80  192 - #000000
              84  192 - #2f2f2f
             112  192 - #f0f0f0
             120  192 - #787878
             132  192 - #010101
             144  192 - #a9a9a9
             152  192 - #ffffff
              20  224 - #ffffff
             176  224 - #ffffff
              64  228 - #ffffff
             104  228 - #ffffff
             140  232 - #ffffff
             460  236 - #597c95
             160  248 - #ffffff
             324  248 - #597c95
              20  252 - #ffffff
              48  264 - #000000
              80  264 - #000000
              84  264 - #2f2f2f
             120  264 - #787878
             128  264 - #ffffff
             132  264 - #010101
              40  268 - #ffffff
              48  268 - #000000
              80  268 - #000000
              84  268 - #2f2f2f
             100  268 - #ffffff
             112  268 - #ffffff
             128  268 - #ffffff
             132  268 - #010101
             136  268 - #080808
             144  268 - #a9a9a9
             172  272 - #000000
             592  280 - #597c95
             252  292 - #597c95
              96  300 - #ffffff
             396  300 - #597c95
              20  308 - #ffffff
              64  308 - #ffffff
             124  308 - #ffffff
             164  308 - #ffffff
             480  344 - #597c95
             592  368 - #597c95
             248  376 - #597c95
             380  388 - #597c95
              80  392 - #597c95
               4  436 - #597c95
             540  436 - #597c95
             144  452 - #597c95
             448  460 - #597c95
             308  484 - #597c95
              72  508 - #597c95
             216  512 - #597c95
             516  528 - #597c95
             424  552 - #597c95
               4  568 - #597c95
             132  580 - #597c95
             260  592 - #597c95
             348  592 - #597c95
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
        human_checkpoint("full, fitting, cut and twin labels");

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
