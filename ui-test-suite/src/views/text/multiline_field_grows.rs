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
    fn setup(self: Weak<Self>) {
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
              428    4 - #597c95
              592    4 - #597c95
              132   68 - #000000
               40   72 - #597c95
               52   72 - #0b0f12
               60   72 - #597c95
              128   72 - #597c95
              152   72 - #000000
              240   72 - #000000
              308   72 - #000001
               40   76 - #597c95
               92   76 - #000000
              132   76 - #000001
              144   76 - #597c95
              216   76 - #000001
              252   76 - #0c1114
              304   76 - #10171b
              308   76 - #000001
              328   76 - #000001
              336   76 - #597c95
              184   92 - #ffffff
              400   92 - #ffffff
              456   92 - #ffffff
              504   92 - #ffffff
              544   92 - #ffffff
              364   96 - #ffffff
              284  100 - #ffffff
              228  104 - #ffffff
              428  104 - #ffffff
              144  108 - #ffffff
               92  112 - #ffffff
              256  112 - #ffffff
              200  116 - #ffffff
              576  116 - #a6a6a6
               40  120 - #ffffff
               52  120 - #ffffff
               60  120 - #ffffff
              324  120 - #ffffff
              576  120 - #a6a6a6
              172  124 - #ffffff
              576  124 - #a6a6a6
               56  128 - #ffffff
              400  128 - #ffffff
              576  128 - #a6a6a6
              120  132 - #ffffff
              224  132 - #ffffff
              284  132 - #ffffff
              368  132 - #ffffff
              436  132 - #ffffff
              480  132 - #ffffff
              528  132 - #ffffff
              568  232 - #597c95
                4  252 - #597c95
              176  252 - #597c95
              332  252 - #597c95
              468  268 - #597c95
               92  288 - #597c95
              256  292 - #597c95
              592  336 - #597c95
              212  376 - #0f151a
              312  376 - #000000
              336  376 - #000000
               48  380 - #597c95
               56  380 - #597c95
               76  380 - #000001
               92  380 - #597c95
              164  380 - #0e1317
              212  380 - #0f151a
              220  380 - #597c95
              252  380 - #597c95
              288  380 - #000000
              312  380 - #000000
              332  380 - #597c95
              336  380 - #000000
              388  380 - #597c95
              392  380 - #090c0f
               64  384 - #000001
              140  384 - #10171b
              212  384 - #0f151a
              288  384 - #000000
              312  384 - #000000
              360  384 - #10171b
              392  384 - #090c0f
              500  388 - #597c95
              180  464 - #597c95
              576  464 - #597c95
              368  484 - #597c95
               68  496 - #597c95
              268  496 - #597c95
              460  520 - #597c95
              168  548 - #597c95
                4  592 - #597c95
               92  592 - #597c95
              240  592 - #597c95
              328  592 - #597c95
              592  592 - #597c95
            ";

const CHECK_2: &str = r"
              460    4 - #597c95
              592    4 - #597c95
              132   68 - #000000
               40   72 - #597c95
               60   72 - #597c95
              128   72 - #597c95
              152   72 - #000000
              240   72 - #000000
               40   76 - #597c95
               96   76 - #040506
              144   76 - #597c95
              216   76 - #000001
              252   76 - #0c1114
              304   76 - #10171b
              328   76 - #000001
              336   76 - #597c95
               44   92 - #ffffff
               52   92 - #ffffff
               64   92 - #ffffff
              524   92 - #ffffff
               64   96 - #ffffff
              464  116 - #ffffff
               56  120 - #000000
               68  120 - #ffffff
               80  120 - #ffffff
              392  128 - #ffffff
              576  128 - #a6a6a6
              164  132 - #ffffff
              276  132 - #ffffff
              576  140 - #a6a6a6
               64  148 - #000000
              512  148 - #ffffff
               44  152 - #ffffff
               48  152 - #ffffff
              220  152 - #ffffff
              576  152 - #a6a6a6
              328  164 - #ffffff
              576  164 - #a6a6a6
              440  168 - #ffffff
              576  168 - #a6a6a6
              576  172 - #a6a6a6
              576  176 - #a6a6a6
              576  180 - #a6a6a6
              576  184 - #a6a6a6
              576  188 - #a6a6a6
              156  192 - #ffffff
              576  192 - #a6a6a6
               52  196 - #ffffff
              576  196 - #a6a6a6
              488  200 - #ffffff
              576  200 - #a6a6a6
              576  204 - #a6a6a6
               52  208 - #ffffff
               60  208 - #ffffff
              576  208 - #a6a6a6
               56  212 - #ffffff
              404  212 - #ffffff
              576  212 - #a6a6a6
              204  216 - #ffffff
              256  216 - #ffffff
              348  216 - #ffffff
              572  216 - #ffffff
              124  284 - #597c95
              332  284 - #597c95
              460  288 - #597c95
                4  292 - #597c95
              592  336 - #597c95
              236  376 - #000000
               48  380 - #597c95
               56  380 - #597c95
               64  380 - #000000
               92  380 - #597c95
              160  380 - #597c95
              188  380 - #000000
              284  380 - #597c95
              300  380 - #597c95
              376  380 - #050708
              408  380 - #597c95
              428  380 - #597c95
              432  380 - #050708
              440  380 - #597c95
              480  380 - #597c95
               76  384 - #000001
              140  384 - #10171b
              212  384 - #000000
              240  384 - #07090b
              592  456 - #597c95
              372  480 - #597c95
              272  492 - #597c95
               64  496 - #597c95
              460  528 - #597c95
              168  548 - #597c95
                4  592 - #597c95
               92  592 - #597c95
              328  592 - #597c95
              592  592 - #597c95
            ";

const CHECK_3: &str = r"
              372    4 - #597c95
              456    4 - #597c95
              552    4 - #597c95
               40   72 - #597c95
               60   72 - #597c95
              128   72 - #597c95
              152   72 - #000000
              240   72 - #000000
               40   76 - #597c95
               96   76 - #040506
              132   76 - #000001
              144   76 - #597c95
              216   76 - #000001
              252   76 - #0c1114
              304   76 - #10171b
              328   76 - #000001
              336   76 - #597c95
              592   84 - #597c95
              416   92 - #ffffff
               36  112 - #ffffff
               40  112 - #ffffff
               56  112 - #010101
               40  116 - #ffffff
               36  136 - #000000
              520  136 - #ffffff
               64  140 - #ffffff
               64  144 - #ffffff
              244  148 - #ffffff
              380  148 - #ffffff
              452  152 - #ffffff
              168  160 - #ffffff
               36  164 - #000000
               68  168 - #ffffff
               80  168 - #ffffff
               68  172 - #ffffff
              316  172 - #ffffff
              576  184 - #ffffff
               36  192 - #000000
               48  196 - #ffffff
               64  196 - #000000
               48  200 - #ffffff
               56  200 - #000000
              492  208 - #ffffff
              408  212 - #ffffff
              132  220 - #ffffff
              232  220 - #ffffff
               64  224 - #ffffff
               52  228 - #000000
               40  252 - #ffffff
               60  252 - #ffffff
               60  256 - #ffffff
              576  260 - #ffffff
              324  276 - #ffffff
              156  284 - #ffffff
              476  284 - #ffffff
              240  300 - #ffffff
              400  300 - #ffffff
               96  312 - #ffffff
               20  316 - #ffffff
              556  348 - #597c95
              268  376 - #000000
               48  380 - #597c95
               56  380 - #597c95
               64  380 - #000000
               92  380 - #597c95
              200  380 - #597c95
              308  380 - #597c95
              316  380 - #597c95
              344  380 - #597c95
              352  380 - #0b0f12
              372  380 - #000000
              384  380 - #597c95
              420  380 - #597c95
              428  380 - #597c95
              444  380 - #597c95
              472  380 - #597c95
               76  384 - #000001
              164  384 - #10171b
              280  384 - #10171b
              332  384 - #030405
              364  384 - #070a0c
              592  432 - #597c95
              396  472 - #597c95
                4  480 - #597c95
              208  480 - #597c95
              304  504 - #597c95
              508  508 - #597c95
              116  512 - #597c95
              588  512 - #597c95
              408  564 - #597c95
                4  592 - #597c95
               88  592 - #597c95
              224  592 - #597c95
              320  592 - #597c95
              492  592 - #597c95
              592  592 - #597c95
            ";
