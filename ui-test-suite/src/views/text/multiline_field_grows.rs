use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{Label, Setup, TextAlignment, TextField, ViewData, ViewFrame, ViewTest, view},
    ui_test::{
        check_colors, set_record_probe_count,
        system_input::{press_return, tap, type_text, wait_until},
    },
};

const FIELD_TOP: f32 = 90.0;
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
        check_colors(CHECK_1)?;
        from_main(move || {
            ensure!(
                view.field.scroll_offset() < -100.0,
                "the low field did not scroll to its last line, offset {}",
                view.field.scroll_offset()
            );
            Ok(())
        })?;

        from_main(move || {
            view.set_field_height(MIDDLE);
            view.say("room for 4 lines: the last lines show, six at the bottom edge");
        });
        wait_until("the field is higher", move || {
            (view.field.height() - MIDDLE).abs() < 0.5
        })?;
        check_colors(CHECK_2)?;
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

        from_main(move || {
            view.set_field_height(HIGH);
            view.say("room for all 6 lines: one is at the top, nothing is scrolled");
        });
        wait_until("the field is at its full height", move || {
            (view.field.height() - HIGH).abs() < 0.5
        })?;
        check_colors(CHECK_3)?;
        from_main(move || {
            ensure!(
                view.field.scroll_offset().abs() < 0.5,
                "a field that fits its text is scrolled by {}",
                view.field.scroll_offset()
            );
            Ok(())
        })
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
              40   92 - #565656
              68   92 - #5e5e5e
             396   92 - #bcbcbc
             456   92 - #bcbcbc
             504   92 - #bcbcbc
             544   92 - #bcbcbc
              40   96 - #565656
             268  108 - #bcbcbc
             196  116 - #bcbcbc
             576  116 - #7a7a7a
              44  120 - #bcbcbc
              52  120 - #707070
              56  120 - #bcbcbc
              64  120 - #bcbcbc
             576  120 - #7a7a7a
              52  124 - #707070
             576  124 - #7a7a7a
             576  128 - #7a7a7a
             108  132 - #bcbcbc
             156  132 - #bcbcbc
             236  132 - #bcbcbc
             296  132 - #bcbcbc
             356  132 - #bcbcbc
             396  132 - #bcbcbc
             436  132 - #bcbcbc
             480  132 - #bcbcbc
             528  132 - #bcbcbc
             568  232 - #597c95
               4  252 - #597c95
             192  252 - #597c95
             344  256 - #597c95
             468  268 - #597c95
             592  336 - #597c95
             496  372 - #597c95
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
             544  464 - #597c95
               4  480 - #597c95
             332  496 - #597c95
             140  532 - #597c95
             432  544 - #597c95
               4  592 - #597c95
             276  592 - #597c95
             592  592 - #597c95
            ";

const CHECK_2: &str = r"
             592    4 - #597c95
              40   72 - #597c95
              60   72 - #597c95
              76   72 - #25343f
             128   72 - #597c95
             168   72 - #435e71
             240   72 - #000000
              96   76 - #040506
             144   76 - #597c95
             216   76 - #000001
             252   76 - #0c1114
             304   76 - #10171b
             336   76 - #597c95
              40   92 - #4d4d4d
              48   92 - #bcbcbc
              56   92 - #bcbcbc
             412   92 - #bcbcbc
             500   92 - #bcbcbc
              40   96 - #4d4d4d
              68   96 - #bcbcbc
              40  120 - #4d4d4d
              72  120 - #bcbcbc
              84  120 - #bcbcbc
              40  124 - #4d4d4d
              60  124 - #0f0f0f
              72  124 - #afafaf
             576  128 - #7a7a7a
             352  140 - #bcbcbc
             576  140 - #7a7a7a
             224  144 - #bcbcbc
              40  148 - #151515
              68  148 - #000000
              40  152 - #151515
              48  152 - #bcbcbc
              52  152 - #bcbcbc
             576  152 - #7a7a7a
             160  160 - #bcbcbc
             288  160 - #bcbcbc
             412  164 - #bcbcbc
             576  164 - #7a7a7a
             576  172 - #7a7a7a
             484  176 - #bcbcbc
              40  180 - #565656
              68  180 - #8a8a8a
              40  184 - #565656
             576  184 - #7a7a7a
             576  192 - #7a7a7a
              52  204 - #707070
             576  204 - #7a7a7a
              52  208 - #707070
              56  208 - #bcbcbc
              64  208 - #bcbcbc
             576  208 - #7a7a7a
              52  212 - #707070
              60  212 - #bcbcbc
             356  212 - #bcbcbc
             576  212 - #7a7a7a
             224  216 - #bcbcbc
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
             404  380 - #2f424f
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
             592  464 - #597c95
               4  480 - #597c95
             304  504 - #597c95
             116  516 - #597c95
             408  568 - #597c95
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
             480   92 - #bcbcbc
             556   92 - #bcbcbc
              56  108 - #000000
              40  112 - #bcbcbc
              44  112 - #bcbcbc
              68  112 - #979797
              44  116 - #bcbcbc
             400  128 - #bcbcbc
              40  140 - #4d4d4d
              68  140 - #bcbcbc
              68  144 - #bcbcbc
             592  156 - #597c95
             172  160 - #bcbcbc
              40  168 - #4d4d4d
              72  168 - #bcbcbc
              84  168 - #bcbcbc
              60  172 - #0f0f0f
              72  172 - #bcbcbc
             324  176 - #bcbcbc
              40  196 - #151515
              52  196 - #bcbcbc
              68  196 - #000000
             512  196 - #bcbcbc
              40  200 - #151515
              52  200 - #bcbcbc
             424  208 - #bcbcbc
             236  220 - #bcbcbc
              40  224 - #565656
              68  224 - #bcbcbc
              40  228 - #565656
              56  228 - #000000
              44  252 - #bcbcbc
              52  252 - #707070
              64  252 - #bcbcbc
              52  256 - #707070
              64  256 - #bcbcbc
             336  272 - #bcbcbc
             160  280 - #bcbcbc
             480  284 - #bcbcbc
             576  292 - #bcbcbc
             240  300 - #bcbcbc
             404  304 - #bcbcbc
             148  376 - #456073
             268  376 - #000000
              48  380 - #597c95
              84  380 - #425c6e
              92  380 - #597c95
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
