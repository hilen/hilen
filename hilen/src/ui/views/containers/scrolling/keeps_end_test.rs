use anyhow::{Result, ensure};

use crate::{
    self as hilen,
    deps::{
        hreads::{from_main, wait_for_next_frame},
        refs::Weak,
    },
    gm::{LossyConvert, color::Color},
    ui::{
        Label, ScrollView, Setup, TextAlignment, ViewCallbacks, ViewData, ViewFrame, ViewSubviews, ViewTest,
        view,
    },
    ui_test::{check_colors, inject_scroll, inject_touches, set_record_probe_count},
};

const TOP: f32 = 80.0;
const TALL: f32 = 480.0;
const SHORT: f32 = 300.0;
const ROW: f32 = 40.0;
const ROWS: usize = 20;
const SCROLLED_UP: f32 = 150.0;

const BACK: Color = Color::rgba(0.35, 0.38, 0.42, 1.0);
const EVEN: Color = Color::rgba(0.87, 0.91, 0.94, 1.0);
const ODD: Color = Color::rgba(0.76, 0.82, 0.88, 1.0);

/// How much of the content is under the bottom edge of the view, 0 for a
/// view at its end.
fn below_end(scroll: &ScrollView) -> f32 {
    scroll.content_height() + scroll.get_scroll_content_offset() - scroll.height()
}

fn add_row(scroll: Weak<ScrollView>, row: usize) {
    let y: f32 = row.lossy_convert();
    let label = scroll.add_view::<Label>();
    label.set_text(format!("row {row}")).set_text_size(18);
    label.set_color(if row.is_multiple_of(2) { EVEN } else { ODD });
    label.place().t(y * ROW).lr(0).h(ROW);
}

/// Looks at a scroll view once a frame, after it got its frame and before
/// the frame is drawn. A view that gets its end back one frame late is
/// caught here, a check after the change would see only the frame after
/// it.
#[view]
struct ScrollEndWatch {
    scroll:  Weak<ScrollView>,
    /// The view has to be at its end in every frame now.
    expect:  bool,
    /// Frames that drew the view off its end while it had to be there.
    off_end: usize,
}

impl ViewCallbacks for ScrollEndWatch {
    fn update(&mut self) {
        if self.expect && self.scroll.is_ok() && below_end(&self.scroll).abs() > 0.5 {
            self.off_end += 1;
        }
    }
}

/// 2 scroll views with the same rows and the automatic content height.
/// The left one has `set_keeps_end`, the right one is plain, so every
/// picture shows what the rule changes. Both get shorter and taller and
/// get new rows. Then the left one is scrolled up and must keep its
/// offset.
#[view]
struct ScrollKeepsEnd {
    rows: usize,

    #[init]
    status:      Label,
    kept_title:  Label,
    plain_title: Label,
    kept:        ScrollView,
    plain:       ScrollView,
    // After the scroll views, they read the frames of this frame.
    kept_watch:  ScrollEndWatch,
    plain_watch: ScrollEndWatch,
}

impl ScrollKeepsEnd {
    fn set_scroll_height(self: Weak<Self>, height: f32) {
        self.kept.place().clear().t(TOP).l(20).w(270).h(height);
        self.plain.place().clear().t(TOP).l(310).w(270).h(height);
    }

    fn say(self: Weak<Self>, text: &str) {
        self.status.set_text(text);
    }

    fn add_rows(mut self: Weak<Self>, count: usize) {
        for row in self.rows..self.rows + count {
            add_row(self.kept, row);
            add_row(self.plain, row);
        }
        self.rows += count;
    }
}

impl Setup for ScrollKeepsEnd {
    fn setup(mut self: Weak<Self>) {
        self.status.set_text_size(16);
        self.status.set_alignment(TextAlignment::Left);
        self.status.place().t(10).lr(20).h(24);

        self.kept_title.set_text("set_keeps_end(true)").set_text_size(16);
        self.kept_title.place().t(44).l(20).w(270).h(24);
        self.plain_title.set_text("a plain scroll view").set_text_size(16);
        self.plain_title.place().t(44).l(310).w(270).h(24);

        self.kept.set_color(BACK);
        self.kept.set_keeps_end(true);
        self.plain.set_color(BACK);
        self.set_scroll_height(TALL);
        self.add_rows(ROWS);

        self.kept_watch.scroll = self.kept;
        self.plain_watch.scroll = self.plain;
    }
}

/// The automatic content height is known one frame after the rows have
/// their frames.
fn settle_content() {
    wait_for_next_frame();
    wait_for_next_frame();
}

fn expect_below(view: Weak<ScrollKeepsEnd>, step: &'static str, kept: f32, plain: f32) -> Result<()> {
    from_main(move || {
        let (kept_now, plain_now) = (below_end(&view.kept), below_end(&view.plain));
        ensure!(
            (kept_now - kept).abs() < 0.5,
            "{step}: the view with the rule has {kept_now} under its edge, not {kept}"
        );
        ensure!(
            (plain_now - plain).abs() < 0.5,
            "{step}: the plain view has {plain_now} under its edge, not {plain}"
        );
        ensure!(
            view.kept_watch.off_end == 0,
            "{step}: {} frames drew the view with the rule off its end",
            view.kept_watch.off_end
        );
        Ok(())
    })
}

fn expect_offset(view: Weak<ScrollKeepsEnd>, step: &'static str, offset: f32) -> Result<()> {
    from_main(move || {
        let now = view.kept.get_scroll_content_offset();
        ensure!(
            now.to_bits() == offset.to_bits(),
            "{step}: the view scrolled up moved from {offset} to {now}"
        );
        Ok(())
    })
}

impl ViewTest for ScrollKeepsEnd {
    fn perform_test(mut view: Weak<Self>) -> Result<()> {
        set_record_probe_count(64);

        settle_content();
        from_main(move || {
            ensure!(
                view.plain.content_height() > view.plain.height(),
                "the content has no height yet"
            );
            view.plain.set_content_offset(f32::MIN);
            view.kept_watch.expect = true;
            view.plain_watch.expect = true;
            view.say("both at their end, row 19 is the last");
            Ok(())
        })?;
        check_colors(AT_END)?;
        expect_below(view, "at the start", 0.0, 0.0)?;

        from_main(move || {
            view.set_scroll_height(SHORT);
            view.say("shorter: left still ends with row 19, right lost its end");
        });
        check_colors(SHORTER)?;
        expect_below(view, "shorter", 0.0, TALL - SHORT)?;
        // The watch is only worth something when it sees a lost end.
        let seen = from_main(move || view.plain_watch.off_end);
        ensure!(seen > 0, "the watch did not see the plain view lose its end");

        from_main(move || {
            view.plain_watch.expect = false;
            view.set_scroll_height(TALL);
            view.say("as tall as before: both end with row 19 again");
        });
        check_colors(TALL_AGAIN)?;
        expect_below(view, "taller", 0.0, 0.0)?;

        from_main(move || {
            view.add_rows(3);
            view.say("3 new rows: left ends with row 22, right stays at row 19");
        });
        settle_content();
        check_colors(NEW_ROWS)?;
        expect_below(view, "new rows", 0.0, ROW * 3.0)?;

        from_main(move || view.kept_watch.expect = false);
        inject_touches("150 300 m");
        inject_scroll(SCROLLED_UP);
        let scrolled = from_main(move || view.kept.get_scroll_content_offset());
        from_main(move || {
            ensure!(
                (below_end(&view.kept) - SCROLLED_UP).abs() < 0.5,
                "the wheel left {} under the edge",
                below_end(&view.kept)
            );
            view.set_scroll_height(SHORT);
            view.say("left scrolled up by 150, then shorter: its top rows stay");
            Ok(())
        })?;
        check_colors(SCROLLED_SHORTER)?;
        expect_offset(view, "scrolled up and shorter", scrolled)?;

        from_main(move || {
            view.add_rows(2);
            view.set_scroll_height(TALL);
            view.say("2 new rows and taller: the top rows of left still stay");
        });
        settle_content();
        check_colors(SCROLLED_TALLER)?;
        expect_offset(view, "scrolled up, new rows and taller", scrolled)?;

        inject_scroll(-5000);
        from_main(move || {
            view.kept_watch.expect = true;
            view.set_scroll_height(SHORT);
            view.say("left scrolled back to its end, then shorter: ends with row 24");
        });
        check_colors(BACK_AT_END)?;
        expect_below(view, "back at the end", 0.0, ROW * 5.0 + TALL - SHORT)
    }
}

const AT_END: &str = r"
             592    4 - #597c95
              56   20 - #000000
             160   20 - #24323c
             212   20 - #49667a
              92   24 - #000000
             148   24 - #597c95
             240   24 - #161f25
             124   56 - #2d3e4b
             144   56 - #000000
             196   56 - #304351
             396   56 - #000000
             444   56 - #597c95
             484   56 - #40596c
             144  100 - #dee8f0
             432  100 - #dee8f0
             288  116 - #dee8f0
              20  128 - #c2d1e0
             172  136 - #c2d1e0
             424  140 - #1b1e20
             572  160 - #dee8f0
             168  176 - #848a8f
             456  180 - #aeb6bc
             420  184 - #4b4e51
             468  216 - #60676f
             140  220 - #c2d1e0
             468  220 - #60676f
             420  224 - #41464c
             468  224 - #60676f
             288  236 - #c2d1e0
              20  244 - #dee8f0
             168  256 - #848a8f
             456  264 - #aeb6bc
             576  276 - #90979c
             168  300 - #747d86
             420  304 - #41464c
              52  328 - #dee8f0
             168  336 - #848a8f
             140  340 - #dee8f0
             284  340 - #90979c
             456  340 - #aeb6bc
             420  344 - #4b4e51
             168  376 - #747d86
             428  380 - #c2d1e0
               4  404 - #597c95
             572  404 - #dee8f0
             168  416 - #848a8f
             140  420 - #dee8f0
             456  424 - #aeb6bc
             288  444 - #c2d1e0
              68  456 - #c2d1e0
             168  456 - #747d86
             420  464 - #41464c
             168  496 - #848a8f
             464  496 - #000000
             428  500 - #dee8f0
             284  516 - #90979c
             576  516 - #90979c
              20  524 - #c2d1e0
             176  536 - #c2d1e0
             140  540 - #c2d1e0
             456  544 - #98a4b0
             220  588 - #597c95
             312  592 - #597c95
             532  592 - #597c95
";
const SHORTER: &str = r"
              96   20 - #141b21
             140   20 - #40596c
             280   20 - #24323c
             296   20 - #2d3f4c
             392   20 - #25343f
              64   24 - #172026
              84   24 - #435e71
             104   24 - #597c95
             172   24 - #597c95
             204   24 - #263540
             212   24 - #090d0f
             280   24 - #24323c
             304   24 - #000000
             316   24 - #49667a
             352   24 - #030506
             124   56 - #2d3e4b
             144   56 - #000000
             184   56 - #597c95
             196   56 - #304351
             456   56 - #000000
             484   56 - #40596c
             396   60 - #000000
             168   84 - #848a8f
             424  104 - #1f2122
             316  112 - #dee8f0
              20  116 - #c2d1e0
             176  116 - #c2d1e0
             140  120 - #c2d1e0
             576  132 - #c2d1e0
             424  144 - #1b1e20
             168  164 - #848a8f
             428  180 - #dee8f0
             468  180 - #dee8f0
             456  184 - #aeb6bc
             140  200 - #c2d1e0
             288  200 - #c2d1e0
              24  220 - #dee8f0
             456  220 - #98a4b0
             420  224 - #41464c
             468  224 - #60676f
             168  236 - #848a8f
             576  240 - #90979c
             420  264 - #4b4e51
             456  264 - #aeb6bc
             576  276 - #90979c
             140  280 - #c2d1e0
             168  284 - #747d86
              56  300 - #dee8f0
             428  300 - #c2d1e0
             284  304 - #90979c
             456  304 - #98a4b0
             168  316 - #848a8f
             140  320 - #dee8f0
             576  320 - #dee8f0
             168  324 - #848a8f
             284  336 - #90979c
             428  340 - #dee8f0
             456  344 - #aeb6bc
             168  364 - #747d86
              20  376 - #c2d1e0
             168  540 - #597c95
               4  592 - #597c95
             332  592 - #597c95
             592  592 - #597c95
";
const TALL_AGAIN: &str = r"
             592    4 - #597c95
              72   20 - #0b0f12
             136   20 - #486478
             292   20 - #1f2c35
              40   24 - #597c95
             136   24 - #486478
             252   24 - #1f2c35
             292   24 - #1f2c35
             144   56 - #000000
             200   56 - #000000
             400   56 - #597c95
             444   56 - #597c95
             484   56 - #40596c
             172   96 - #dee8f0
             424  100 - #1f2122
               4  124 - #597c95
             144  140 - #c2d1e0
             424  144 - #1b1e20
             288  148 - #c2d1e0
             576  160 - #dee8f0
             168  176 - #848a8f
             468  180 - #dee8f0
             420  184 - #4b4e51
             340  212 - #c2d1e0
             468  216 - #60676f
             468  220 - #60676f
             168  224 - #747d86
             420  224 - #41464c
             468  224 - #60676f
              20  244 - #dee8f0
             140  260 - #dee8f0
             428  260 - #dee8f0
             168  264 - #848a8f
             468  264 - #c0c9d0
             284  276 - #90979c
             576  276 - #90979c
             168  300 - #747d86
             456  304 - #98a4b0
              72  316 - #c2d1e0
             576  332 - #90979c
             168  344 - #848a8f
             420  344 - #4b4e51
             252  356 - #dee8f0
             140  380 - #c2d1e0
             456  380 - #98a4b0
             572  388 - #c2d1e0
               4  404 - #597c95
             168  416 - #848a8f
             316  416 - #dee8f0
             140  420 - #dee8f0
             456  420 - #aeb6bc
             168  456 - #747d86
             464  456 - #c2d1e0
             420  464 - #41464c
             284  484 - #90979c
              48  496 - #dee8f0
             456  500 - #aeb6bc
             168  504 - #848a8f
             576  512 - #90979c
             140  540 - #c2d1e0
             456  544 - #98a4b0
             312  556 - #c2d1e0
               4  592 - #597c95
             220  592 - #597c95
";
const NEW_ROWS: &str = r"
             592    4 - #597c95
             212   20 - #3e5769
             420   20 - #597c95
              56   24 - #161f25
              84   24 - #0e1317
             300   24 - #172026
             304   24 - #2c3e4a
             124   56 - #2d3e4b
             132   56 - #2d3e4b
             180   56 - #000000
             396   56 - #000000
             484   56 - #40596c
             140  100 - #c2d1e0
             176  100 - #abb8c6
             424  100 - #1f2122
             320  124 - #c2d1e0
             168  136 - #848a8f
             424  140 - #1b1e20
             432  140 - #c2d1e0
             424  144 - #1b1e20
             576  160 - #dee8f0
             468  180 - #dee8f0
             168  184 - #747d86
             420  184 - #4b4e51
               4  200 - #597c95
             428  220 - #c2d1e0
             168  224 - #848a8f
             456  224 - #98a4b0
             312  228 - #c2d1e0
             576  252 - #90979c
             168  264 - #747d86
             456  264 - #aeb6bc
             168  296 - #848a8f
             140  300 - #dee8f0
             456  300 - #98a4b0
             168  304 - #848a8f
             420  304 - #41464c
             284  316 - #90979c
             572  324 - #dee8f0
             168  336 - #747d86
             456  340 - #aeb6bc
             168  376 - #848a8f
             456  384 - #98a4b0
              20  388 - #dee8f0
             576  404 - #90979c
             276  416 - #c2d1e0
             464  420 - #000000
             168  424 - #747d86
             420  424 - #4b4e51
             140  460 - #dee8f0
             180  460 - #010101
             456  464 - #98a4b0
             576  488 - #90979c
             140  500 - #c2d1e0
             176  500 - #abb8c6
             332  500 - #dee8f0
             456  504 - #aeb6bc
              20  512 - #c2d1e0
             176  536 - #dee8f0
             140  540 - #dee8f0
             428  540 - #c2d1e0
             456  544 - #98a4b0
             292  592 - #597c95
             532  592 - #597c95
";
const SCROLLED_SHORTER: &str = r"
             180   16 - #0b1013
             124   20 - #2a3a46
              56   24 - #030506
              96   24 - #0c1013
             228   24 - #354a59
             276   24 - #597c95
             300   24 - #030506
             324   24 - #597c95
             372   24 - #597c95
             404   24 - #597c95
             132   56 - #2d3e4b
             200   56 - #000000
             400   56 - #597c95
             456   56 - #000000
             484   56 - #40596c
             144   92 - #c2d1e0
             432  100 - #dee8f0
              20  108 - #c2d1e0
             576  116 - #dee8f0
             320  124 - #c2d1e0
             144  132 - #dee8f0
             172  132 - #dee8f0
             432  140 - #c2d1e0
             172  168 - #c2d1e0
             144  172 - #c2d1e0
             420  184 - #4b4e51
             456  184 - #aeb6bc
              56  192 - #dee8f0
             284  192 - #90979c
             576  196 - #90979c
             168  212 - #848a8f
             456  220 - #98a4b0
             420  224 - #41464c
             468  224 - #60676f
             284  228 - #90979c
             576  244 - #90979c
             176  252 - #abb8c6
             456  260 - #aeb6bc
             420  264 - #4b4e51
             468  264 - #c0c9d0
             576  268 - #90979c
              20  272 - #dee8f0
             168  288 - #848a8f
             340  288 - #c2d1e0
             576  288 - #c2d1e0
             168  292 - #848a8f
             456  300 - #98a4b0
             420  304 - #41464c
             168  328 - #747d86
             428  340 - #dee8f0
             420  344 - #4b4e51
             456  344 - #aeb6bc
             288  352 - #dee8f0
             168  368 - #848a8f
             168  372 - #848a8f
              24  376 - #dee8f0
             132  376 - #dee8f0
             156  376 - #dee8f0
             524  376 - #c2d1e0
             456  516 - #597c95
             164  540 - #597c95
               4  592 - #597c95
             320  592 - #597c95
             592  592 - #597c95
";
const SCROLLED_TALLER: &str = r"
             536    4 - #597c95
             308   20 - #263540
             372   20 - #000000
              56   24 - #161f25
              92   24 - #597c95
             136   24 - #000000
             180   24 - #597c95
             240   24 - #597c95
             124   56 - #2d3e4b
             200   56 - #000000
             456   56 - #000000
             396   60 - #000000
             172   84 - #33373b
             144   92 - #c2d1e0
             576   92 - #dee8f0
             432  100 - #dee8f0
             424  104 - #1f2122
             316  128 - #c2d1e0
             172  132 - #dee8f0
               4  136 - #597c95
             424  140 - #1b1e20
             424  144 - #1b1e20
             144  172 - #c2d1e0
             468  180 - #dee8f0
             420  184 - #4b4e51
             168  208 - #848a8f
             592  216 - #597c95
             468  220 - #60676f
             420  224 - #41464c
             284  228 - #90979c
             168  252 - #747d86
              20  256 - #c2d1e0
             428  260 - #dee8f0
             468  264 - #c0c9d0
             168  288 - #848a8f
             284  288 - #90979c
             176  296 - #b2bac0
             428  300 - #c2d1e0
             576  320 - #90979c
             324  328 - #dee8f0
              80  332 - #c2d1e0
             456  344 - #aeb6bc
             168  368 - #848a8f
             284  372 - #90979c
             456  380 - #98a4b0
             420  384 - #41464c
              20  412 - #c2d1e0
             168  412 - #747d86
             456  420 - #aeb6bc
             572  428 - #dee8f0
             284  432 - #90979c
             168  448 - #848a8f
             428  460 - #c2d1e0
             176  484 - #33373b
             456  500 - #aeb6bc
             544  516 - #dee8f0
             168  528 - #848a8f
             176  532 - #dee8f0
             456  540 - #98a4b0
             420  544 - #41464c
              40  548 - #dee8f0
             288  556 - #c2d1e0
             112  592 - #597c95
             592  592 - #597c95
";
const BACK_AT_END: &str = r"
             124   20 - #2c3d4a
             316   20 - #3e5769
             380   20 - #40596c
              44   24 - #597c95
              96   24 - #0c1013
             208   24 - #597c95
             280   24 - #202c35
             336   24 - #597c95
             396   24 - #3a5161
             444   24 - #0d1215
             140   56 - #597c95
             180   56 - #000000
             200   56 - #000000
             448   56 - #597c95
             484   56 - #40596c
             396   60 - #000000
             252  104 - #dee8f0
             424  104 - #1f2122
             168  116 - #848a8f
             576  116 - #dee8f0
             168  120 - #848a8f
             168  124 - #848a8f
              56  140 - #c2d1e0
             432  140 - #c2d1e0
             168  164 - #747d86
             312  168 - #dee8f0
             468  180 - #dee8f0
             576  180 - #90979c
             420  184 - #4b4e51
             140  200 - #dee8f0
             168  204 - #c0c9d0
             468  216 - #60676f
             252  220 - #c2d1e0
             428  220 - #c2d1e0
             456  220 - #98a4b0
             468  224 - #60676f
             140  240 - #c2d1e0
             576  240 - #90979c
              20  244 - #c2d1e0
             176  244 - #abb8c6
             348  244 - #dee8f0
             428  260 - #dee8f0
             456  260 - #aeb6bc
             468  264 - #c0c9d0
             176  276 - #dee8f0
             140  280 - #dee8f0
             284  296 - #90979c
             456  300 - #98a4b0
             420  304 - #41464c
             168  324 - #a8b5c2
             284  344 - #90979c
             420  344 - #4b4e51
             456  344 - #aeb6bc
             140  360 - #dee8f0
             168  364 - #c0c9d0
             284  372 - #90979c
              20  376 - #dee8f0
             372  376 - #c2d1e0
             576  376 - #c2d1e0
             452  516 - #597c95
             160  524 - #597c95
               4  592 - #597c95
             312  592 - #597c95
             592  592 - #597c95
";
