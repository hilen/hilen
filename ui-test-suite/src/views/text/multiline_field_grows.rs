use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{Label, Setup, TextAlignment, TextField, ViewData, ViewFrame, ViewTest, view},
    ui_test::{
        check_colors, set_record_probe_count,
        system_input::{press_return, screen_ink, system_input, system_request, tap, type_text, wait_until},
    },
};

const FIELD_TOP: f32 = 90.0;
const OUTSIDE: (f32, f32) = (300.0, 540.0);
/// Room for 1 line of the text.
const LOW: f32 = 44.0;
/// Room for about 4 lines.
const MIDDLE: f32 = 130.0;
/// Room for all 6 lines.
const HIGH: f32 = 230.0;
const LINES: [&str; 6] = ["one", "two", "three", "four", "five", "six"];

/// A text area that grows with its text, like the box of a chat. While it
/// is 1 line high and gets 6 lines, it scrolls to the last one. When its
/// owner then makes it higher, it has to show as many lines as fit, with
/// no empty room under the last line.
#[view]
struct MultilineFieldGrows {
    #[init]
    title:  Label,
    field:  TextField,
    status: Label,
}

impl Setup for MultilineFieldGrows {
    fn setup(mut self: Weak<Self>) {
        self.title.set_text("a text area that its owner makes higher").set_text_size(18);
        self.title.set_alignment(TextAlignment::Left);
        self.title.place().t(60).lr(20).h(26);

        self.field.set_text_size(24).set_multiline(true);
        self.field.set_alignment(TextAlignment::Left);
        self.set_field_height(LOW);

        self.status.set_text_size(18).set_multiline(true);
        self.status.set_alignment(TextAlignment::Left);
        self.status.place().t(340).lr(20).h(80);
        self.say("room for 1 line, empty");
    }
}

impl MultilineFieldGrows {
    fn set_field_height(self: Weak<Self>, height: f32) {
        self.field.place().clear().t(FIELD_TOP).lr(20).h(height);
    }

    fn say(self: Weak<Self>, step: &str) {
        self.status.set_text(step);
    }

    /// The look of the field while it is edited. On a phone the system text
    /// view draws the text then, and a capture of the engine frame holds no
    /// system view, so there the check reads the screen: the text has to
    /// show, inside the field.
    fn check_field(self: Weak<Self>, colors: &str) -> Result<()> {
        if !system_input() {
            return check_colors(colors);
        }
        let frame = from_main(move || *self.field.absolute_frame());
        let ink = screen_ink(frame.x(), frame.y(), frame.width(), frame.height())?;
        ensure!(
            ink.bottom > ink.top,
            "the edited field shows no text on the screen"
        );
        // The box of the ink counts from the top left of the area read.
        ensure!(
            ink.top >= 0.0 && ink.bottom <= frame.height(),
            "the text of the edited field runs out of it, {} to {} in a field {} high",
            ink.top,
            ink.bottom,
            frame.height()
        );
        Ok(())
    }

    /// How far the field could still scroll down, 0 when the last line
    /// sits at its bottom edge.
    fn room_under_the_text(&self) -> f32 {
        self.field.height() - self.field.content_height() - self.field.scroll_offset()
    }
}

impl ViewTest for MultilineFieldGrows {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        set_record_probe_count(96);

        tap(300.0, FIELD_TOP + LOW / 2.0)?;
        wait_until("the field is edited", move || view.field.is_editing())?;
        for (index, line) in LINES.iter().enumerate() {
            if index > 0 {
                press_return()?;
            }
            type_text(line)?;
        }
        wait_until("the field holds 6 lines", move || {
            view.field.text() == LINES.join("\n")
        })?;
        from_main(move || view.say("room for 1 line: the field shows its last line, six"));
        view.check_field(CHECK_1)?;
        // The system text view of a phone scrolls itself while it edits, the
        // scroll of the engine field stays where it was, see `check_field`.
        let engine_scrolls = !system_input();
        if engine_scrolls {
            from_main(move || {
                ensure!(
                    view.field.scroll_offset() < -100.0,
                    "the low field did not scroll to its last line, offset {}",
                    view.field.scroll_offset()
                );
                Ok(())
            })?;
        }

        from_main(move || {
            view.set_field_height(MIDDLE);
            view.say("room for 4 lines: the last lines show, six at the bottom edge");
        });
        wait_until("the field is higher", move || {
            (view.field.height() - MIDDLE).abs() < 0.5
        })?;
        view.check_field(CHECK_2)?;
        if engine_scrolls {
            from_main(move || {
                ensure!(
                    view.room_under_the_text().abs() < 0.5,
                    "the higher field has {} points of empty room under its text",
                    view.room_under_the_text()
                );
                ensure!(
                    view.field.scrolled_to_caret(),
                    "the caret left the view when the field grew"
                );
                Ok(())
            })?;
        }

        from_main(move || {
            view.set_field_height(HIGH);
            view.say("room for all 6 lines: one is at the top, nothing is scrolled");
        });
        wait_until("the field is at its full height", move || {
            (view.field.height() - HIGH).abs() < 0.5
        })?;
        view.check_field(CHECK_3)?;
        if engine_scrolls {
            from_main(move || {
                ensure!(
                    view.field.scroll_offset().abs() < 0.5,
                    "a field that fits its text is scrolled by {}",
                    view.field.scroll_offset()
                );
                Ok(())
            })?;
        }

        // The keyboard goes down before the next test, which taps its own
        // field.
        tap(OUTSIDE.0, OUTSIDE.1)?;
        wait_until("the field is not edited any more", move || {
            !view.field.is_editing()
        })?;
        if system_input() {
            system_request("keyboard 0")?;
        }
        Ok(())
    }
}

const CHECK_1: &str = r"
             592    4 - #597c95
              40   72 - #597c95
              52   72 - #0b0f12
              60   72 - #597c95
              76   72 - #25343f
             128   72 - #597c95
             152   72 - #000000
             168   72 - #435e71
             240   72 - #000000
             308   72 - #000001
              40   76 - #597c95
              96   76 - #040506
             132   76 - #000001
             144   76 - #597c95
             216   76 - #000001
             252   76 - #0c1114
             304   76 - #10171b
             328   76 - #000001
             336   76 - #597c95
              40   92 - #747474
              68   92 - #808080
             396   92 - #ffffff
             456   92 - #ffffff
             504   92 - #ffffff
             544   92 - #ffffff
              40   96 - #747474
             268  108 - #ffffff
             196  116 - #ffffff
             576  116 - #a6a6a6
              44  120 - #ffffff
              52  120 - #989898
              56  120 - #ffffff
              64  120 - #ffffff
             576  120 - #a6a6a6
              52  124 - #989898
             576  124 - #a6a6a6
              60  128 - #ffffff
             576  128 - #a6a6a6
             108  132 - #ffffff
             156  132 - #ffffff
             236  132 - #ffffff
             296  132 - #ffffff
             356  132 - #ffffff
             396  132 - #ffffff
             436  132 - #ffffff
             480  132 - #ffffff
             528  132 - #ffffff
             568  232 - #597c95
               4  252 - #597c95
             192  252 - #597c95
             344  256 - #597c95
             468  268 - #597c95
             592  336 - #597c95
             116  376 - #476377
             168  376 - #304351
             212  376 - #0f151a
             240  376 - #1e2a33
             312  376 - #000000
              48  380 - #597c95
              56  380 - #597c95
              76  380 - #000001
              84  380 - #425c6e
              92  380 - #597c95
             100  380 - #273742
             116  380 - #476377
             148  380 - #334756
             168  380 - #304351
             212  380 - #0f151a
             220  380 - #597c95
             240  380 - #1c272f
             252  380 - #597c95
             292  380 - #40596c
             332  380 - #597c95
             336  380 - #000000
             368  380 - #334756
             388  380 - #597c95
              64  384 - #000001
              84  384 - #425c6e
             100  384 - #273742
             116  384 - #476377
             140  384 - #10171b
             168  384 - #304351
             212  384 - #0f151a
             240  384 - #1e2a33
             288  384 - #000000
             312  384 - #000000
             360  384 - #10171b
             392  384 - #090c0f
             532  464 - #597c95
               4  480 - #597c95
             324  500 - #597c95
             128  524 - #597c95
             424  556 - #597c95
               4  592 - #597c95
             256  592 - #597c95
             592  592 - #597c95
            ";

const CHECK_2: &str = r"
             592    4 - #597c95
              40   72 - #597c95
              60   72 - #597c95
              76   72 - #25343f
             128   72 - #597c95
             152   72 - #000000
             168   72 - #435e71
             240   72 - #000000
              96   76 - #040506
             144   76 - #597c95
             216   76 - #000001
             252   76 - #0c1114
             304   76 - #10171b
             336   76 - #597c95
              48   92 - #ffffff
              56   92 - #ffffff
             412   92 - #ffffff
             500   92 - #ffffff
              40   96 - #686868
              68   96 - #ffffff
              40  120 - #686868
              72  120 - #ffffff
              84  120 - #ffffff
              60  124 - #151515
              72  124 - #eeeeee
             576  128 - #a6a6a6
             352  140 - #ffffff
             576  140 - #a6a6a6
             224  144 - #ffffff
              40  148 - #1d1d1d
              68  148 - #000000
              40  152 - #1d1d1d
              52  152 - #ffffff
             576  152 - #a6a6a6
             160  160 - #ffffff
             288  160 - #ffffff
             412  164 - #ffffff
             576  164 - #a6a6a6
             576  172 - #a6a6a6
              40  176 - #747474
             484  176 - #ffffff
              40  180 - #747474
              68  180 - #bbbbbb
              40  184 - #747474
             576  184 - #a6a6a6
             576  192 - #a6a6a6
              56  196 - #ffffff
              52  204 - #989898
             576  204 - #a6a6a6
              52  208 - #989898
              56  208 - #ffffff
              64  208 - #ffffff
             576  208 - #a6a6a6
              52  212 - #989898
              60  212 - #ffffff
             356  212 - #ffffff
             576  212 - #a6a6a6
             224  216 - #ffffff
             592  320 - #597c95
             236  376 - #000000
             380  376 - #395060
             404  376 - #2f424f
              48  380 - #597c95
              84  380 - #425c6e
             100  380 - #273742
             148  380 - #334756
             160  380 - #597c95
             172  380 - #2e414e
             188  380 - #000000
             244  380 - #1c272f
             300  380 - #597c95
             376  380 - #050708
             380  380 - #395060
             428  380 - #597c95
             440  380 - #597c95
             480  380 - #597c95
             500  380 - #334756
              64  384 - #000001
              84  384 - #425c6e
             140  384 - #10171b
             212  384 - #000000
             244  384 - #1e2a33
             288  384 - #304350
             292  384 - #3d5566
             380  384 - #395060
             404  384 - #2f424f
             448  384 - #3a5060
             452  384 - #334756
             592  464 - #597c95
               4  480 - #597c95
             304  504 - #597c95
             116  516 - #597c95
             408  564 - #597c95
               4  592 - #597c95
             224  592 - #597c95
             592  592 - #597c95
            ";

const CHECK_3: &str = r"
             416    4 - #597c95
             504    4 - #597c95
             592    4 - #597c95
              40   72 - #597c95
              60   72 - #597c95
              76   72 - #25343f
             128   72 - #597c95
             152   72 - #000000
             168   72 - #435e71
             240   72 - #000000
              96   76 - #040506
             144   76 - #597c95
             216   76 - #000001
             252   76 - #0c1114
             304   76 - #10171b
             336   76 - #597c95
             480   92 - #ffffff
             556   92 - #ffffff
              56  108 - #000000
              40  112 - #ffffff
              44  112 - #ffffff
              68  112 - #cdcdcd
              44  116 - #ffffff
              60  116 - #010101
             400  128 - #ffffff
              40  140 - #686868
              68  140 - #ffffff
              68  144 - #ffffff
             592  156 - #597c95
             168  160 - #ffffff
              40  168 - #686868
              72  168 - #ffffff
              84  168 - #ffffff
              60  172 - #151515
              72  172 - #ffffff
             316  172 - #ffffff
              40  196 - #1d1d1d
              52  196 - #ffffff
              68  196 - #000000
             512  196 - #ffffff
              40  200 - #1d1d1d
              52  200 - #ffffff
             424  208 - #ffffff
             228  220 - #ffffff
              40  224 - #747474
              68  224 - #ffffff
              40  228 - #747474
              56  228 - #000000
              44  252 - #ffffff
              52  252 - #989898
              64  252 - #ffffff
              52  256 - #989898
              64  256 - #ffffff
             328  268 - #ffffff
             156  280 - #ffffff
             480  284 - #ffffff
             576  292 - #ffffff
             240  300 - #ffffff
             400  300 - #ffffff
             148  376 - #456073
             268  376 - #000000
              48  380 - #597c95
              84  380 - #425c6e
             100  380 - #273742
             148  380 - #456073
             200  380 - #597c95
             308  380 - #597c95
             316  380 - #597c95
             332  380 - #000000
             348  380 - #000001
             372  380 - #000000
             384  380 - #597c95
             396  380 - #212e38
             420  380 - #597c95
             428  380 - #597c95
             444  380 - #597c95
             472  380 - #597c95
              64  384 - #000001
              84  384 - #425c6e
             100  384 - #273742
             148  384 - #456073
             164  384 - #10171b
             212  384 - #19232a
             280  384 - #10171b
             364  384 - #070a0c
             396  384 - #212e38
             592  452 - #597c95
             500  468 - #597c95
               4  480 - #597c95
             232  484 - #597c95
             336  492 - #597c95
             144  536 - #597c95
             440  540 - #597c95
               4  592 - #597c95
             284  592 - #597c95
             592  592 - #597c95
            ";
