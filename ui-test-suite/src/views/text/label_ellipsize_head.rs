use anyhow::Result;
use hilen::{
    AppRunner,
    dispatch::from_main,
    refs::Weak,
    ui::{
        BLACK, Label, Screenshot, Setup, TextAlignment, U8Color, ViewData, ViewFrame, ViewTest, WHITE, view,
    },
    ui_test::{check_colors, human_checkpoint, set_record_probe_count},
};

const TEXT: &str = "src/ui/settings_sheet/tools_tab.rs";

const PROBES: &str = r"
             524    4 - #597c95
             356   20 - #ffffff
             416   20 - #ffffff
              20   24 - #ffffff
             384   28 - #ffffff
             188   36 - #000000
             236   36 - #000000
             264   36 - #505050
              48   40 - #000000
              60   40 - #ffffff
              84   40 - #2f2f2f
             108   40 - #000000
             112   40 - #969696
             120   40 - #787878
             128   40 - #ffffff
             132   40 - #010101
             136   40 - #000000
             152   40 - #ffffff
             188   40 - #000000
             220   40 - #cfcfcf
             224   40 - #ffffff
             236   40 - #020202
             244   40 - #ffffff
             256   40 - #ffffff
             264   40 - #505050
             292   40 - #ffffff
             308   40 - #ffffff
              80   44 - #000000
              84   44 - #2f2f2f
             132   44 - #010101
             136   44 - #080808
             144   44 - #a9a9a9
             188   44 - #010101
             196   44 - #b7b7b7
             236   44 - #010101
             264   44 - #505050
             272   44 - #ffffff
             324   44 - #e4e4e4
             160   48 - #ffffff
             172   48 - #aeaeae
             280   48 - #aeaeae
             284   48 - #aeaeae
              20   56 - #ffffff
             348   56 - #ffffff
             376   56 - #ffffff
             412   56 - #ffffff
             148   84 - #3f3f3f
             148   88 - #000000
             164   88 - #000000
             592   88 - #597c95
              56   92 - #f0f0f0
              64   92 - #ffffff
              76   92 - #cdcdcd
              88   92 - #ffffff
              96   92 - #ffffff
             144   92 - #e5e5e5
             148   92 - #000000
             152   92 - #ffffff
             164   92 - #010101
              20   96 - #ffffff
             124   96 - #d6d6d6
             112  120 - #ffffff
              36  124 - #ffffff
              64  140 - #ffffff
              76  140 - #cdcdcd
              88  140 - #ffffff
              96  140 - #ffffff
             136  140 - #ffffff
             148  140 - #000000
             152  140 - #ffffff
             164  140 - #000000
             176  140 - #ffffff
             148  144 - #000000
             164  144 - #010101
             124  148 - #aeaeae
              20  156 - #ffffff
              56  176 - #ffffff
             304  176 - #597c95
             100  180 - #ffffff
             140  180 - #ffffff
             176  184 - #ffffff
             536  184 - #597c95
              20  192 - #ffffff
              48  216 - #000000
              60  216 - #ffffff
              80  216 - #000000
              84  216 - #2f2f2f
             112  216 - #010101
             120  216 - #787878
             128  216 - #ffffff
             132  216 - #010101
             144  216 - #a9a9a9
             152  216 - #ffffff
             428  220 - #597c95
              20  224 - #ffffff
             176  228 - #ffffff
             140  252 - #ffffff
              20  256 - #ffffff
              64  256 - #ffffff
             104  256 - #ffffff
             176  256 - #ffffff
             592  280 - #597c95
             284  292 - #597c95
             464  332 - #597c95
              88  372 - #597c95
             592  376 - #597c95
             356  380 - #597c95
             192  424 - #597c95
             520  436 - #597c95
               4  456 - #597c95
             420  464 - #597c95
             108  492 - #597c95
             296  496 - #597c95
             592  496 - #597c95
             500  540 - #597c95
             400  568 - #597c95
              60  592 - #597c95
             208  592 - #597c95
             304  592 - #597c95
             592  592 - #597c95
            ";

const WIDE_FRAME: (u32, u32, u32, u32) = (20, 20, 400, 40);
const CUT_FRAME: (u32, u32, u32, u32) = (20, 70, 160, 40);
const TWIN_FRAME: (u32, u32, u32, u32) = (20, 120, 160, 40);

#[view]
struct LabelEllipsizeHead {
    #[init]
    wide:  Label,
    cut:   Label,
    twin:  Label,
    multi: Label,
}

impl Setup for LabelEllipsizeHead {
    fn setup(self: Weak<Self>) {
        for label in [self.wide, self.cut, self.twin] {
            label.set_color(WHITE).set_text_color(BLACK).set_text_size(20);
            label.set_text(TEXT).set_alignment(TextAlignment::Left);
        }

        self.wide.set_ellipsize_head(true).set_frame(WIDE_FRAME);
        self.cut.set_ellipsize_head(true).set_frame(CUT_FRAME);
        self.twin.set_frame(TWIN_FRAME);

        self.multi.set_color(WHITE).set_text_color(BLACK).set_text_size(20);
        self.multi.set_text(TEXT).set_alignment(TextAlignment::Left);
        self.multi.set_multiline(true).set_ellipsize_head(true);
        self.multi.set_frame((20, 170, 160, 90));
    }
}

impl ViewTest for LabelEllipsizeHead {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        set_record_probe_count(120);

        // Fitting text is left alone.
        let display = from_main(move || view.wide.display_text(view.wide.width()).to_string());
        assert_eq!(display, TEXT, "fitting text was truncated");

        // Overflowing text keeps a suffix with the leading ellipsis.
        let cut = from_main(move || view.cut.display_text(view.cut.width()).to_string());
        assert!(cut.starts_with('…'), "no leading ellipsis: {cut:?}");
        assert!(cut.len() < TEXT.len(), "text was not shortened: {cut:?}");
        assert!(
            TEXT.ends_with(cut.trim_start_matches('…')),
            "cut text is not a suffix: {cut:?}"
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
        check_colors(PROBES)?;
        human_checkpoint("wide, cut and twin labels, the multiline one wrapping");

        // A narrower frame recomputes to a shorter suffix.
        let narrower = from_main(move || {
            view.cut.set_frame((20, 70, 100, 40));
            view.cut.display_text(view.cut.width()).to_string()
        });
        assert!(
            narrower.len() < cut.len(),
            "narrower width kept the same text: {narrower:?} vs {cut:?}"
        );
        assert!(
            narrower.starts_with('…'),
            "no ellipsis after resize: {narrower:?}"
        );

        // A new text drops the cached truncation.
        let changed = from_main(move || {
            view.cut.set_text("other/path/entirely/too/long/to/fit.rs");
            view.cut.display_text(view.cut.width()).to_string()
        });
        assert!(
            changed.starts_with('…') && changed.ends_with("fit.rs"),
            "stale truncation after set_text: {changed:?}"
        );

        // Multiline wraps instead and ignores the flag.
        let multi = from_main(move || view.multi.display_text(view.multi.width()).to_string());
        assert_eq!(multi, TEXT, "multiline text was truncated");

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
