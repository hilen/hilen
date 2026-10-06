use anyhow::Result;
#[cfg(desktop)]
use hilen::system::Clipboard;
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{
        GRAY, Label, ModifiersState, Setup, TextAlignment, TextSelection, UIManager, VerticalAlignment,
        ViewData, ViewTest, WHITE, view,
    },
    ui_test::{check_colors, inject_keys, inject_modifiers, inject_touches, set_record_probe_count},
};

use crate::text_points::{click, drag, point_after, point_of, shown};

const SINGLE: &str = "The quick brown fox jumps over the lazy dog.";
const FIRST_LINE: &str = "First line of the text.";
const SECOND_LINE: &str =
    "Second line, which is long enough to wrap at the width of this label, more than once.";
const THIRD_LINE: &str = "Third line.";
const PLAIN: &str = "This label is not selectable at all.";

const CHECK_1: &str = r"
     164   24 - #d8d8d8
     272   24 - #e5e5e5
     124   36 - #b8cbd9
     516   36 - #fff3c4
     584   36 - #fff3c4
     196   40 - #fff3c4
     240   44 - #fff3c4
     320   44 - #fff3c4
     404   44 - #c7be99
      80   48 - #000000
      96   48 - #7d8a93
     232   48 - #aba383
     348   48 - #645f4d
     452   48 - #000000
      44   52 - #66614e
     144   52 - #818e98
     180   52 - #4a5157
     272   52 - #fff3c4
     304   52 - #fff3c4
     360   52 - #423f33
     384   52 - #6d6854
     432   52 - #fff3c4
     232   56 - #aba383
     404   56 - #999276
     108   60 - #b8cbd9
     128   60 - #b8cbd9
     160   60 - #b8cbd9
     544   64 - #fff3c4
      12   68 - #fff3c4
     496   68 - #fff3c4
     576   68 - #fff3c4
     144  100 - #cecece
     180  100 - #e9e9e9
      40  104 - #ffffff
     104  104 - #bcbcbc
      68  128 - #7d7d7d
      80  132 - #545454
     152  132 - #000000
     212  132 - #000000
     108  148 - #7a7a7a
     348  148 - #666666
     424  152 - #9a9a9a
      48  156 - #0e0e0e
     176  156 - #424242
     272  156 - #ffffff
     312  156 - #ffffff
     540  156 - #7d7d7d
     576  156 - #181818
     108  160 - #7a7a7a
     392  160 - #000000
     476  160 - #b3b3b3
     520  160 - #2d2d2d
     552  160 - #292929
     576  160 - #181818
     424  164 - #9a9a9a
      88  176 - #4e4e4e
     184  180 - #6d6d6d
      36  184 - #b7b7b7
      72  184 - #000000
      88  184 - #4e4e4e
     112  184 - #7a7a7a
     136  184 - #777777
     216  184 - #1a1a1a
     252  184 - #424242
      76  200 - #5d5d5d
      56  204 - #212121
      92  208 - #3a3a3a
     112  208 - #747474
      40  300 - #bcbcbc
      72  304 - #d8d8d8
     120  324 - #a0a0a0
     176  328 - #a0a0a0
     296  328 - #ffffff
     308  328 - #313131
      56  332 - #212121
     120  332 - #a0a0a0
     160  332 - #a5a5a5
     212  332 - #000000
     260  332 - #000000
     272  332 - #000000
     332  332 - #212121
     336  332 - #2d2d2d
     552  376 - #ffffff
     168  408 - #808080
      80  412 - #ffffff
     128  412 - #ffffff
     192  412 - #808080
     248  412 - #000000
      40  440 - #bebebe
      68  440 - #d9d9d9
     148  440 - #d7d7d7
     424  452 - #ffffff
       4  592 - #ffffff
     208  592 - #ffffff
     372  592 - #ffffff
     592  592 - #ffffff
";

const CHECK_2: &str = r"
      44   20 - #c0c0c0
      96   20 - #cecece
     152   24 - #d1d1d1
     272   24 - #e5e5e5
     196   36 - #b8cbd9
     220   36 - #b8cbd9
     532   36 - #fff3c4
     116   44 - #fff3c4
     172   44 - #fff3c4
     240   44 - #fff3c4
     320   44 - #fff3c4
     364   44 - #fff3c4
     404   44 - #c7be99
      80   48 - #000000
     432   48 - #fff3c4
      44   52 - #66614e
      96   52 - #ada585
     144   52 - #b3ab8a
     264   52 - #4d493b
     360   52 - #423f33
     384   52 - #6d6854
     452   52 - #000000
     232   56 - #aba383
     196   60 - #b8cbd9
     216   60 - #b8cbd9
      12   68 - #fff3c4
     496   68 - #fff3c4
     548   68 - #fff3c4
     584   68 - #fff3c4
     180  100 - #e9e9e9
     120  104 - #fefefe
     224  104 - #d0d0d0
      32  124 - #ffffff
      68  128 - #7d7d7d
      80  132 - #545454
     152  132 - #000000
     160  132 - #8b8b8b
     108  148 - #7a7a7a
     348  148 - #666666
     424  152 - #9a9a9a
     128  156 - #d8d8d8
     176  156 - #424242
     268  156 - #000000
     304  156 - #8f8f8f
     492  156 - #0e0e0e
     540  156 - #7d7d7d
     576  156 - #181818
     108  160 - #7a7a7a
     200  160 - #e6e6e6
     392  160 - #000000
     520  160 - #2d2d2d
     552  160 - #292929
     576  160 - #181818
     424  164 - #9a9a9a
      88  176 - #4e4e4e
     112  176 - #7a7a7a
      36  184 - #b7b7b7
      72  184 - #000000
      88  184 - #4e4e4e
     112  184 - #7a7a7a
     136  184 - #777777
     160  184 - #010101
     192  184 - #1a1a1a
     216  184 - #1a1a1a
     252  184 - #424242
      76  200 - #5d5d5d
      60  208 - #1e1e1e
      92  208 - #3a3a3a
     112  208 - #747474
      52  304 - #fefefe
     104  304 - #c7c7c7
      56  328 - #212121
      84  328 - #ffffff
     176  328 - #a0a0a0
     296  328 - #ffffff
      44  332 - #666666
     120  332 - #a0a0a0
     160  332 - #a5a5a5
     212  332 - #000000
     256  332 - #d8d8d8
     260  332 - #000000
     272  332 - #000000
     332  332 - #212121
     336  332 - #2d2d2d
     548  380 - #ffffff
      56  408 - #181818
      48  412 - #636363
      72  412 - #808080
     140  412 - #9d9d9d
      68  440 - #d9d9d9
     104  440 - #ffffff
     460  508 - #ffffff
     168  588 - #ffffff
       4  592 - #ffffff
     328  592 - #ffffff
     592  592 - #ffffff
";

const CHECK_3: &str = r"
     188   24 - #ffffff
     536   36 - #fff3c4
     108   40 - #fff3c4
     412   44 - #fff3c4
      80   48 - #000000
     456   48 - #fff3c4
      44   52 - #66614e
      96   52 - #ada585
     136   52 - #fff3c4
     180   52 - #66614e
     264   52 - #4d493b
     328   52 - #fff3c4
     360   52 - #423f33
     384   52 - #6d6854
     232   56 - #aba383
      12   68 - #fff3c4
     500   68 - #fff3c4
     584   68 - #fff3c4
     180  100 - #e9e9e9
     120  104 - #fefefe
     224  104 - #d0d0d0
      80  124 - #545454
      80  132 - #545454
     160  132 - #8b8b8b
     264  144 - #b8d2ff
     388  144 - #b8d2ff
     108  148 - #58647a
     200  148 - #a6bde6
     476  148 - #8193b3
     532  148 - #889cbd
      40  152 - #b8d2ff
     108  152 - #58647a
     424  152 - #6f7f9a
      80  156 - #8193b3
     128  156 - #9cb2d8
     176  156 - #303642
     232  156 - #b8d2ff
     364  156 - #98aed3
     528  156 - #b8d2ff
     580  156 - #b8d2ff
     108  160 - #58647a
     200  160 - #a6bde6
     304  160 - #67768f
     348  160 - #4a5466
     476  160 - #8193b3
     552  160 - #1e2229
     424  164 - #6f7f9a
      36  176 - #8497b7
     112  176 - #58647a
     184  180 - #4f5a6d
      36  184 - #8497b7
      72  184 - #000000
      88  184 - #38404e
     112  184 - #58647a
     136  184 - #566277
     192  184 - #13151a
     220  184 - #b8d2ff
     252  184 - #303642
     160  188 - #b8d2ff
     284  188 - #d8e7ff
      44  200 - #666666
      76  200 - #5d5d5d
      60  208 - #1e1e1e
     112  208 - #747474
     592  300 - #ffffff
     120  328 - #a0a0a0
     176  328 - #a0a0a0
     296  328 - #ffffff
     308  328 - #313131
      56  332 - #212121
     120  332 - #a0a0a0
     160  332 - #a5a5a5
     212  332 - #000000
     260  332 - #000000
     272  332 - #000000
     332  332 - #212121
     336  332 - #2d2d2d
     128  408 - #0e0e0e
      40  412 - #000000
      72  412 - #ffffff
     160  412 - #808080
     192  412 - #010101
     224  412 - #808080
     276  416 - #000000
     420  436 - #e4e4e4
     464  436 - #d8d8d8
     172  440 - #bcbcbc
     328  440 - #bcbcbc
     548  440 - #ffffff
     384  444 - #e4e4e4
      40  460 - #c0c0c0
      80  460 - #d7d7d7
     120  460 - #bcbcbc
     128  592 - #ffffff
     328  592 - #ffffff
     592  592 - #ffffff
";

const CHECK_4: &str = r"
     252   20 - #e6e6e6
     584   36 - #fff3c4
      12   40 - #fff3c4
     164   44 - #fff3c4
     204   44 - #fff3c4
     312   44 - #fff3c4
     412   44 - #fff3c4
     484   44 - #fff3c4
      96   48 - #ada585
      44   52 - #66614e
      60   52 - #fff3c4
     144   52 - #b3ab8a
     180   52 - #66614e
     264   52 - #4d493b
     360   52 - #423f33
     384   52 - #6d6854
     452   52 - #000000
     232   56 - #aba383
     536   68 - #fff3c4
      44  100 - #c0c0c0
     180  100 - #e9e9e9
     116  116 - #b8d2ff
     224  116 - #b8d2ff
     188  124 - #b8d2ff
      32  128 - #000000
      68  128 - #7d7d7d
      80  132 - #3d4554
     144  132 - #98aed3
     160  132 - #64728b
     260  144 - #b8d2ff
     340  144 - #b8d2ff
     108  148 - #58647a
     200  148 - #a6bde6
     476  148 - #8193b3
      40  152 - #b8d2ff
     424  152 - #6f7f9a
     516  152 - #010101
      80  156 - #8193b3
     128  156 - #9cb2d8
     176  156 - #303642
     376  156 - #b8d2ff
     532  156 - #889cbd
     552  156 - #1e2229
     108  160 - #58647a
     304  160 - #67768f
     348  160 - #4a5466
     424  160 - #6f7f9a
     476  160 - #8193b3
     580  160 - #b8d2ff
     424  164 - #6f7f9a
      36  176 - #8497b7
     112  176 - #58647a
      32  180 - #92a6ca
     112  180 - #58647a
     184  180 - #4f5a6d
      36  184 - #8497b7
      72  184 - #000000
      88  184 - #38404e
     112  184 - #58647a
     136  184 - #566277
     216  184 - #13151a
     268  188 - #b8d2ff
      76  200 - #434d5d
      56  208 - #181b21
      92  208 - #3a3a3a
     112  208 - #747474
      72  304 - #d8d8d8
     500  304 - #ffffff
      56  328 - #212121
     308  328 - #313131
      44  332 - #666666
     120  332 - #a0a0a0
     160  332 - #a5a5a5
     176  332 - #a0a0a0
     212  332 - #000000
     260  332 - #000000
     272  332 - #000000
     332  332 - #212121
     336  332 - #2d2d2d
      32  408 - #ffffff
      80  412 - #000000
     168  412 - #000000
     204  412 - #545454
     340  436 - #e4e4e4
     388  440 - #bcbcbc
     436  440 - #bcbcbc
     508  440 - #ffffff
     552  440 - #bebebe
     152  456 - #dfdfdf
      48  460 - #ffffff
     108  460 - #ffffff
     208  460 - #c3c3c3
     268  460 - #ffffff
       4  592 - #ffffff
     248  592 - #ffffff
     492  592 - #ffffff
";

const CHECK_5: &str = r"
      44   20 - #c0c0c0
     164   24 - #d8d8d8
     272   24 - #e5e5e5
     124   36 - #b8cbd9
     532   36 - #fff3c4
     568   36 - #fff3c4
     188   40 - #b8cbd9
     240   44 - #fff3c4
     320   44 - #fff3c4
     404   44 - #c7be99
      80   48 - #000000
      96   48 - #7d8a93
     232   48 - #aba383
     432   48 - #fff3c4
      44   52 - #66614e
     144   52 - #818e98
     180   52 - #4a5157
     208   52 - #fff3c4
     272   52 - #fff3c4
     360   52 - #423f33
     384   52 - #6d6854
     456   52 - #fff3c4
     232   56 - #aba383
     404   56 - #999276
     108   60 - #b8cbd9
     160   60 - #b8cbd9
     184   60 - #b8cbd9
      12   68 - #fff3c4
     500   68 - #fff3c4
     548   68 - #fff3c4
     584   68 - #fff3c4
     180  100 - #e9e9e9
     120  104 - #fefefe
      32  124 - #ffffff
      80  132 - #545454
     152  132 - #000000
     212  132 - #000000
     108  148 - #7a7a7a
     424  152 - #9a9a9a
      60  156 - #ffffff
     176  156 - #424242
     268  156 - #000000
     336  156 - #ffffff
     376  156 - #ffffff
     456  156 - #000000
     492  156 - #0e0e0e
     576  156 - #181818
     108  160 - #7a7a7a
     200  160 - #e6e6e6
     304  160 - #8f8f8f
     520  160 - #2d2d2d
     552  160 - #292929
     576  160 - #181818
     424  164 - #9a9a9a
      36  176 - #b7b7b7
     112  180 - #7a7a7a
     184  180 - #6d6d6d
      72  184 - #000000
      88  184 - #4e4e4e
     112  184 - #7a7a7a
     136  184 - #777777
     216  184 - #1a1a1a
     252  184 - #424242
      76  200 - #5d5d5d
      56  208 - #212121
      92  208 - #3a3a3a
      56  300 - #d4d4d4
     104  304 - #c7c7c7
      56  328 - #212121
     120  328 - #a0a0a0
     176  328 - #a0a0a0
     296  328 - #ffffff
     308  328 - #313131
      44  332 - #666666
     120  332 - #a0a0a0
     160  332 - #a5a5a5
     212  332 - #000000
     260  332 - #000000
     272  332 - #000000
     332  332 - #212121
     336  332 - #2d2d2d
     548  380 - #ffffff
     152  408 - #656565
      52  412 - #010101
     108  412 - #969696
     132  412 - #707070
     164  412 - #808080
     204  412 - #383838
      68  440 - #d9d9d9
     148  440 - #d7d7d7
     180  440 - #bcbcbc
     420  456 - #ffffff
       4  592 - #ffffff
     196  592 - #ffffff
     356  592 - #ffffff
     592  592 - #ffffff
";

const CHECK_6: &str = r"
      96   20 - #cecece
      40   24 - #ffffff
     164   24 - #d8d8d8
     208   24 - #c1c1c1
     272   24 - #e5e5e5
     532   36 - #fff3c4
     568   36 - #fff3c4
     120   40 - #fff3c4
     240   44 - #fff3c4
     320   44 - #fff3c4
     404   44 - #c7be99
      80   48 - #000000
     432   48 - #fff3c4
      44   52 - #66614e
      96   52 - #ada585
     144   52 - #b3ab8a
     180   52 - #66614e
     208   52 - #fff3c4
     264   52 - #4d493b
     272   52 - #fff3c4
     360   52 - #423f33
     384   52 - #6d6854
     456   52 - #fff3c4
     232   56 - #aba383
      12   68 - #fff3c4
     500   68 - #fff3c4
     548   68 - #fff3c4
     584   68 - #fff3c4
      44  100 - #c0c0c0
     180  100 - #e9e9e9
     120  104 - #fefefe
      32  124 - #ffffff
     212  124 - #000000
      68  128 - #7d7d7d
      80  132 - #545454
     152  132 - #000000
     108  148 - #7a7a7a
     348  148 - #666666
     424  152 - #9a9a9a
     516  152 - #010101
      56  156 - #ffffff
     176  156 - #424242
     268  156 - #000000
     540  156 - #7d7d7d
     576  156 - #181818
     108  160 - #7a7a7a
     304  160 - #8f8f8f
     392  160 - #000000
     476  160 - #b3b3b3
     552  160 - #292929
     576  160 - #181818
     424  164 - #9a9a9a
      88  176 - #4e4e4e
     112  180 - #7a7a7a
      36  184 - #b7b7b7
      72  184 - #000000
      88  184 - #4e4e4e
     112  184 - #7a7a7a
     152  184 - #ffffff
     192  184 - #1a1a1a
     216  184 - #1a1a1a
     252  184 - #424242
      76  200 - #5d5d5d
      60  208 - #1e1e1e
      92  208 - #3a3a3a
      40  300 - #bcbcbc
      72  304 - #d8d8d8
     120  324 - #a0a0a0
      56  328 - #212121
     296  328 - #ffffff
     308  328 - #313131
      44  332 - #666666
     120  332 - #a0a0a0
     160  332 - #a5a5a5
     176  332 - #a0a0a0
     212  332 - #000000
     260  332 - #000000
     272  332 - #000000
     332  332 - #212121
     336  332 - #2d2d2d
     548  380 - #ffffff
     300  408 - #000000
      84  412 - #808080
     116  412 - #808080
     164  412 - #515151
     188  412 - #707070
     224  412 - #ffffff
     260  412 - #000000
      40  440 - #bebebe
      68  440 - #d9d9d9
     120  440 - #d6d6d6
     432  460 - #ffffff
       4  592 - #ffffff
     212  592 - #ffffff
     388  592 - #ffffff
     592  592 - #ffffff
";

/// Selectable labels select like the text of a text editor: a drag, a
/// double click for a word, a triple click for a line, Shift with a
/// click, and Cmd or Ctrl with C to copy. A label that is not selectable
/// selects nothing, and a click on it drops the selection.
#[view]
struct LabelSelection {
    #[init]
    single_role:  Label,
    single:       Label,
    wrapped_role: Label,
    wrapped:      Label,
    plain_role:   Label,
    plain:        Label,
    step:         Label,
    selected:     Label,
}

impl Setup for LabelSelection {
    fn setup(self: Weak<Self>) {
        self.set_color(WHITE);

        for (role, text, y) in [
            (self.single_role, "selectable label with a background, 1 line", 12),
            (self.wrapped_role, "selectable label, 3 lines that wrap", 92),
            (self.plain_role, "label that is not selectable", 292),
        ] {
            role.set_text(text)
                .set_text_size(14)
                .set_text_color(GRAY)
                .set_alignment(TextAlignment::Left);
            role.place().t(y).lr(12).h(20);
        }

        let wrapped = [FIRST_LINE, SECOND_LINE, THIRD_LINE].join("\n");
        for (label, text) in [
            (self.single, SINGLE),
            (self.wrapped, wrapped.as_str()),
            (self.plain, PLAIN),
        ] {
            label
                .set_text(text)
                .set_text_size(22)
                .set_alignment(TextAlignment::Left)
                .set_vertical_alignment(VerticalAlignment::Top);
        }
        // A background of its own, the selection has to show over it.
        self.single.set_selectable(true).set_color("#FFF3C4");
        self.single.place().t(36).lr(12).h(36);
        self.wrapped.set_multiline(true).set_selectable(true);
        self.wrapped.place().t(116).lr(12).h(160);
        self.plain.place().t(316).lr(12).h(36);

        self.step
            .set_text("nothing done yet")
            .set_text_size(16)
            .set_alignment(TextAlignment::Left);
        self.step.place().t(400).lr(12).h(24);

        self.selected
            .set_text("selected: nothing")
            .set_multiline(true)
            .set_text_size(16)
            .set_text_color(GRAY)
            .set_alignment(TextAlignment::Left)
            .set_vertical_alignment(VerticalAlignment::Top);
        self.selected.place().t(430).lr(12).h(160);
    }
}

impl LabelSelection {
    /// Writes what was done and what is selected now under the labels, so
    /// a human sees both, and gives the selected text back.
    fn show(self: Weak<Self>, step: &'static str) -> String {
        from_main(move || {
            let text = TextSelection::text();
            self.step.set_text(step);
            self.selected.set_text(if text.is_empty() {
                "selected: nothing".to_string()
            } else {
                format!("selected: {}", shown(&text))
            });
            text
        })
    }
}

impl ViewTest for LabelSelection {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        set_record_probe_count(96);

        // A phone scrolls with a drag by default, this test is about the
        // drag that selects.
        from_main(|| UIManager::set_drag_scrolling(false));

        let (quick, brown_end) =
            from_main(move || (point_of(view.single, "quick"), point_after(view.single, "brown")));
        inject_touches(drag(quick, brown_end));
        assert_eq!(view.show("drag from quick to the end of brown"), "quick brown");
        check_colors(CHECK_1)?;

        // The same 2 ends dragged from the right to the left.
        inject_touches(drag(brown_end, quick));
        assert_eq!(view.show("the same drag from right to left"), "quick brown");

        let fox = from_main(move || point_of(view.single, "ox jumps"));
        inject_touches(click(fox));
        inject_touches(click(fox));
        assert_eq!(view.show("double click on fox"), "fox");
        check_colors(CHECK_2)?;

        // A third click right after takes the line the word is on.
        inject_touches(click(fox));
        assert_eq!(view.show("triple click on fox"), SINGLE);

        let wraps = from_main(move || point_of(view.wrapped, "wrap at"));
        inject_touches(click(wraps));
        inject_touches(click(wraps));
        inject_touches(click(wraps));
        assert_eq!(
            view.show("triple click in the second line, it wraps"),
            SECOND_LINE
        );
        check_colors(CHECK_3)?;

        let (line, third) = from_main(move || {
            (
                point_of(view.wrapped, "line of"),
                point_after(view.wrapped, "Third"),
            )
        });
        // Not at the place of the clicks before, this must be a single click.
        inject_touches(click(line));
        assert_eq!(view.show("click in the first line"), "");
        inject_modifiers(ModifiersState::SHIFT);
        inject_touches(click(third));
        inject_modifiers(ModifiersState::empty());
        assert_eq!(
            view.show("Shift and a click in the third line"),
            format!("line of the text.\n{SECOND_LINE}\nThird")
        );
        check_colors(CHECK_4)?;

        copy_and_check("line of the text.");

        // The selection of the wrapped label goes when another label gets one.
        inject_touches(drag(quick, brown_end));
        assert_eq!(view.show("drag in the first label again"), "quick brown");
        check_colors(CHECK_5)?;

        let (not, at) = from_main(move || (point_of(view.plain, "not"), point_after(view.plain, "at all")));
        inject_touches(click(not));
        assert_eq!(view.show("click on the label that is not selectable"), "");
        inject_touches(drag(not, at));
        assert_eq!(view.show("drag over the label that is not selectable"), "");
        check_colors(CHECK_6)?;

        Ok(())
    }
}

/// Cmd or Ctrl with C puts the selected text on the clipboard.
fn copy_and_check(start: &'static str) {
    #[cfg(desktop)]
    from_main(|| Clipboard::set_in_process(true));

    inject_modifiers(ModifiersState::SUPER);
    inject_keys("c");
    inject_modifiers(ModifiersState::empty());

    // Only a desktop has a clipboard a test can keep to itself and read.
    #[cfg(desktop)]
    from_main(move || {
        let copied = Clipboard::get_text().expect("the clipboard has the copy");
        assert_eq!(copied, TextSelection::text());
        assert!(
            copied.starts_with(start),
            "`{copied}` does not start with `{start}`"
        );
    });
    #[cfg(not(desktop))]
    assert!(from_main(TextSelection::text).starts_with(start));
}
