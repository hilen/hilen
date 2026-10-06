use std::sync::mpsc::channel;

use anyhow::Result;
#[cfg(desktop)]
use hilen::system::Clipboard;
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{
        ContextMenu, GRAY, Label, MarkdownView, ModifiersState, Setup, TextAlignment, TextSelection,
        UIManager, VerticalAlignment, View, ViewData, ViewFrame, ViewTest, WHITE, view,
    },
    ui_test::{
        check_colors, inject_keys, inject_modifiers, inject_right_click, inject_touches,
        set_record_probe_count,
    },
};

use crate::text_points::{click, drag, label_with, point_after, point_of, shown};

const TEXT: &str = r"# A heading

Plain text with **bold words** and a [link to a page](https://example.com/page) in it.

- First bullet
- Second bullet

1. Step one
2. Step two

```rust
let answer = 42;
```

| Name | Value |
|------|-------|
| Rows | 2     |
";

/// What a copy of all of `TEXT` gives: the text as it is drawn, with no
/// markdown marks.
fn drawn() -> String {
    [
        "A heading",
        "",
        "Plain text with bold words and a link to a page in it.",
        "",
        "• First bullet",
        "• Second bullet",
        "",
        "1. Step one",
        "2. Step two",
        "",
        "let answer = 42;",
        "",
        "Name\tValue",
        "Rows\t2",
    ]
    .join("\n")
}

const CHECK_1: &str = r"
     528    4 - #ffffff
      40   20 - #ffffff
     104   20 - #ffffff
     172   20 - #c7c7c7
      84   32 - #b8d2ff
     116   40 - #b8d2ff
      32   44 - #1b1e25
      52   48 - #7686a3
      56   48 - #454e5f
      80   48 - #1a1d24
     108   52 - #b8d2ff
      64   56 - #b8d2ff
      92   56 - #b8d2ff
     132   56 - #b8d2ff
     172   68 - #b8d2ff
     200   68 - #b8d2ff
     144   72 - #1a1d24
     180   72 - #8699ba
     236   72 - #b8d2ff
      48   76 - #b3ccf8
      64   76 - #1a1d24
      80   76 - #b8d2ff
     104   76 - #9bb1d7
     128   76 - #1a1d24
     160   76 - #b8d2ff
     180   76 - #8699ba
     216   76 - #b8d2ff
     256   76 - #317ef6
     276   76 - #2f7df6
     300   76 - #b8d2ff
     308   76 - #6aa1fa
     332   76 - #343b48
     336   76 - #5d6a81
      28   80 - #b8d2ff
      40   96 - #b8d2ff
      72   96 - #b8d2ff
     100  100 - #9b9da0
      56  104 - #93a8cc
     100  104 - #9b9da0
     112  124 - #525459
      80  128 - #1a1d24
      84  128 - #8f9094
     468  132 - #ffffff
      52  152 - #ffffff
      88  156 - #9b9da0
     592  200 - #ffffff
     144  216 - #9369e2
     160  216 - #b9621f
     168  220 - #e5d8d0
      28  252 - #dce0e6
      60  256 - #dfe2e7
      72  260 - #3b3e45
      88  260 - #1b1e25
     160  260 - #1a1d24
     408  260 - #ffffff
      32  276 - #e3e6eb
      80  276 - #e3e6eb
     100  276 - #e3e6eb
     132  276 - #e3e6eb
     152  276 - #e3e6eb
     168  276 - #e3e6eb
      56  288 - #929497
      28  300 - #dce0e6
     592  360 - #ffffff
     108  428 - #1a1a1a
      32  432 - #ffffff
      52  432 - #010101
      80  432 - #ffffff
     104  432 - #3d3d3d
     120  432 - #808080
     164  432 - #858585
     204  432 - #3d3d3d
     248  432 - #000000
     252  432 - #757575
     256  432 - #4d4d4d
     280  432 - #000000
     308  432 - #808080
      88  452 - #e2e2e2
     392  452 - #bcbcbc
      44  456 - #c0c0c0
      76  456 - #ffffff
      88  456 - #e2e2e2
     120  456 - #dbdbdb
     128  456 - #e6e6e6
     148  456 - #dedede
     188  456 - #d4d4d4
     272  456 - #cecece
     300  456 - #ffffff
     336  456 - #cecece
     360  456 - #bcbcbc
     412  456 - #bcbcbc
     436  456 - #ffffff
     468  456 - #cecece
       4  592 - #ffffff
     232  592 - #ffffff
     592  592 - #ffffff
";

const CHECK_2: &str = r"
     452    4 - #ffffff
     588    4 - #ffffff
      44   16 - #c0c0c0
      40   20 - #ffffff
      88   20 - #ffffff
      96   20 - #c6c6c6
     116   20 - #c0c0c0
     172   20 - #c7c7c7
      32   44 - #1b1e25
      80   44 - #1a1d24
     124   44 - #1a1d24
      52   48 - #9fa0a3
      56   48 - #585a5f
      92   48 - #1a1d24
     112   48 - #1a1d24
     120   72 - #1a1d24
     136   72 - #1a1d24
     180   72 - #b7b8ba
      64   76 - #1a1d24
      80   76 - #ffffff
     104   76 - #d5d5d7
     128   76 - #1a1d24
     144   76 - #1a1d24
     164   76 - #ffffff
     180   76 - #b7b8ba
     256   76 - #327ff6
     276   76 - #2f7df6
     308   76 - #88b5fa
     332   76 - #404248
     336   76 - #7b7d81
     240   80 - #ffffff
     100  100 - #9b9da0
      56  104 - #cacbcc
      60  104 - #292c33
     100  104 - #9b9da0
     108  124 - #d1d2d3
     112  124 - #525459
      84  128 - #8f9094
     488  136 - #ffffff
      52  152 - #ffffff
      72  156 - #ffffff
      88  156 - #9b9da0
     292  200 - #eef1f5
      92  216 - #adc9f8
     100  216 - #adc9f8
     108  216 - #adc9f8
     124  216 - #adc9f8
     144  216 - #9369e2
     160  216 - #b9621f
      96  220 - #adc9f8
     112  220 - #adc9f8
     128  220 - #adc9f8
     168  220 - #e5d8d0
      60  256 - #dfe2e7
      72  260 - #3b3e45
      88  260 - #1b1e25
     148  260 - #afb2b7
     160  260 - #1a1d24
      28  264 - #dce0e6
     592  268 - #ffffff
     392  272 - #ffffff
      44  276 - #e3e6eb
      68  276 - #e3e6eb
      92  276 - #e3e6eb
     104  276 - #e3e6eb
     116  276 - #e3e6eb
     128  276 - #e3e6eb
     140  276 - #e3e6eb
     156  276 - #e3e6eb
     168  276 - #e3e6eb
      28  284 - #dce0e6
      56  288 - #929497
      28  300 - #dce0e6
     380  392 - #ffffff
      32  432 - #ffffff
      48  432 - #636363
      56  432 - #000000
      72  432 - #808080
     100  432 - #ffffff
     120  432 - #ffffff
     148  432 - #9a9a9a
     180  432 - #808080
     196  432 - #000000
     212  432 - #d1d1d1
     244  432 - #ffffff
     268  432 - #808080
     492  432 - #ffffff
      44  452 - #c0c0c0
      44  456 - #c0c0c0
      76  456 - #ffffff
      96  456 - #c7c7c7
     124  456 - #ffffff
       4  592 - #ffffff
     224  592 - #ffffff
     388  592 - #ffffff
     592  592 - #ffffff
";

const CHECK_3: &str = r"
     592    4 - #ffffff
      44   16 - #c0c0c0
      40   20 - #ffffff
      88   20 - #ffffff
      96   20 - #c6c6c6
     116   20 - #c0c0c0
     172   20 - #c7c7c7
      32   44 - #1b1e25
     124   44 - #1a1d24
      52   48 - #9fa0a3
      56   48 - #585a5f
      92   48 - #1a1d24
     112   48 - #1a1d24
     120   72 - #1a1d24
      64   76 - #1a1d24
     104   76 - #d5d5d7
     144   76 - #1a1d24
     164   76 - #ffffff
     180   76 - #b7b8ba
     256   76 - #327ff6
     276   76 - #2f7df6
     308   76 - #88b5fa
     332   76 - #404248
     336   76 - #7b7d81
     240   80 - #ffffff
     100  100 - #9b9da0
      60  104 - #292c33
     100  104 - #9b9da0
     108  104 - #ffffff
     112  124 - #525459
      84  128 - #8f9094
     492  136 - #ffffff
      52  156 - #ffffff
      88  156 - #9b9da0
     144  216 - #9369e2
     160  216 - #b9621f
     168  220 - #e5d8d0
      28  252 - #dce0e6
      56  252 - #aac7f6
      72  252 - #aac7f6
      80  252 - #aac7f6
      92  252 - #aac7f6
     136  252 - #aac7f6
     148  252 - #aac7f6
     156  252 - #aac7f6
      64  256 - #aac7f6
     144  256 - #aac7f6
     152  256 - #aac7f6
      72  260 - #313846
      92  260 - #aac7f6
     148  260 - #8197bb
     160  260 - #1a1d24
      84  264 - #aac7f6
     128  264 - #aac7f6
     136  264 - #aac7f6
     144  264 - #aac7f6
     156  264 - #aac7f6
     392  268 - #ffffff
     592  272 - #ffffff
      44  276 - #e3e6eb
      68  276 - #e3e6eb
      92  276 - #e3e6eb
     108  276 - #e3e6eb
     148  276 - #e3e6eb
     168  276 - #e3e6eb
      28  280 - #dce0e6
      56  288 - #929497
      28  300 - #dce0e6
     100  428 - #000000
     200  428 - #d4d4d4
     284  428 - #a1a1a1
     488  428 - #ffffff
      32  432 - #ffffff
      52  432 - #010101
      84  432 - #808080
     100  432 - #000000
     116  432 - #808080
     148  432 - #000000
     180  432 - #ffffff
     196  432 - #bababa
     224  432 - #9d9d9d
     244  432 - #000000
     260  432 - #6b6b6b
     284  432 - #a1a1a1
      96  452 - #e4e4e4
     132  452 - #bcbcbc
     152  452 - #dbdbdb
      44  456 - #c0c0c0
      76  456 - #ffffff
      96  456 - #e4e4e4
     116  456 - #dcdcdc
     152  456 - #dbdbdb
       4  592 - #ffffff
     236  592 - #ffffff
     396  592 - #ffffff
     592  592 - #ffffff
";

const CHECK_4: &str = r"
     520    4 - #ffffff
      40   20 - #ffffff
      96   20 - #c6c6c6
     172   20 - #c7c7c7
      32   44 - #1b1e25
      80   44 - #1a1d24
     124   44 - #1a1d24
      52   48 - #7686a3
      56   48 - #454e5f
      92   48 - #1a1d24
     112   48 - #1a1d24
     200   68 - #b8d2ff
     120   72 - #1a1d24
     180   72 - #8699ba
      28   76 - #b8d2ff
      64   76 - #1a1d24
     104   76 - #9bb1d7
     144   76 - #1a1d24
     180   76 - #8699ba
     228   76 - #b8d2ff
     256   76 - #317ef6
     276   76 - #2f7df6
     308   76 - #6aa1fa
     332   76 - #343b48
     336   76 - #5d6a81
     100  100 - #7383a0
      56  104 - #93a8cc
     100  104 - #7383a0
      44  124 - #b8d2ff
     108  124 - #98aed3
     112  124 - #404959
      84  128 - #6b7994
     136  128 - #b8d2ff
     464  132 - #ffffff
      48  148 - #b8d2ff
      68  152 - #b8d2ff
      88  156 - #7383a0
     104  156 - #b8d2ff
      32  184 - #b8d2ff
      72  184 - #b8d2ff
     104  184 - #b8d2ff
     592  188 - #ffffff
     296  200 - #eef1f5
      56  216 - #adc9f8
      92  216 - #adc9f8
     144  216 - #8963e3
     160  216 - #b35e1f
     120  224 - #adc9f8
     180  224 - #adc9f8
      28  252 - #dce0e6
      60  256 - #a3beeb
      72  260 - #313846
      88  260 - #1b1e25
     148  260 - #8197bb
     164  260 - #aac7f6
     404  268 - #ffffff
      32  276 - #e3e6eb
      68  276 - #e3e6eb
      92  276 - #e3e6eb
     120  276 - #e3e6eb
     140  276 - #e3e6eb
     156  276 - #e3e6eb
      56  288 - #6d7c97
      80  296 - #b8d2ff
     140  296 - #d5e5ff
      28  300 - #dce0e6
     280  324 - #ffffff
     592  332 - #ffffff
      32  432 - #ffffff
      76  432 - #000000
      96  432 - #050505
     132  432 - #ffffff
     328  452 - #d7d7d7
     380  452 - #e2e2e2
     484  452 - #bcbcbc
      52  456 - #ffffff
      92  456 - #e5e5e5
     132  456 - #bcbcbc
     164  456 - #ffffff
     220  456 - #ffffff
     252  456 - #ffffff
     516  456 - #d2d2d2
     200  468 - #d8d8d8
     460  468 - #d9d9d9
     276  472 - #e5e5e5
     304  472 - #c4c4c4
     360  472 - #bcbcbc
     404  472 - #e9e9e9
     436  472 - #bcbcbc
      40  488 - #dfdfdf
      88  488 - #e1e1e1
     128  488 - #bcbcbc
     164  488 - #ffffff
     196  488 - #ffffff
     440  592 - #ffffff
     592  592 - #ffffff
";

const CHECK_5: &str = r"
     544    4 - #ffffff
      44   16 - #c0c0c0
     172   20 - #c7c7c7
     132   32 - #b8d2ff
      36   40 - #1b1e25
      64   44 - #9eb4da
      84   44 - #9eb4da
      96   44 - #9eb4da
     108   44 - #9eb4da
     124   44 - #9eb4da
     160   44 - #dadada
     192   44 - #dadada
      44   52 - #a0b7de
      44   72 - #9eb4da
     208   72 - #8da1c3
     212   72 - #a3bae1
     208   76 - #8da1c3
     212   76 - #a3bae1
     256   76 - #317ef6
     276   76 - #2f7df6
     308   76 - #6aa1fa
     332   76 - #343b48
     336   76 - #5d6a81
      44   80 - #9eb4da
     208   80 - #8da1c3
     132   88 - #2a2a2a
      88   92 - #8b8b8b
      96   92 - #000000
     100   92 - #8b8b8b
     112   92 - #6c6c6c
      44  100 - #9eb4da
     208  100 - #c3c3c3
      56  112 - #c3c3c3
      92  112 - #c3c3c3
     128  112 - #c3c3c3
     172  112 - #c3c3c3
     196  112 - #c3c3c3
     152  116 - #e1e1e1
     108  124 - #98aed3
     112  124 - #404959
      40  128 - #b8d2ff
      80  128 - #1a1d24
      84  128 - #6b7994
     472  132 - #ffffff
      48  148 - #b8d2ff
      88  156 - #7383a0
      68  180 - #b8d2ff
     104  180 - #b8d2ff
      36  184 - #b8d2ff
      92  208 - #c6d9f7
     144  216 - #8963e3
     160  216 - #b35e1f
     592  216 - #ffffff
      56  224 - #adc9f8
     116  224 - #adc9f8
     180  224 - #adc9f8
      28  256 - #dce0e6
      56  256 - #1a1d24
      72  260 - #313846
      88  260 - #1b1e25
     148  260 - #8197bb
     164  260 - #aac7f6
     404  260 - #ffffff
     116  276 - #e3e6eb
      56  288 - #6d7c97
      92  292 - #b8d2ff
     132  296 - #b8d2ff
      28  300 - #dce0e6
     592  348 - #ffffff
      48  428 - #282828
     176  428 - #000000
     132  432 - #000000
     148  432 - #808080
     164  432 - #969696
     176  432 - #000000
     288  432 - #808080
     316  432 - #3d3d3d
      44  436 - #000000
     236  436 - #000000
     380  452 - #e2e2e2
      76  456 - #ffffff
     136  456 - #e2e2e2
     480  456 - #c0c0c0
     516  456 - #d2d2d2
     448  468 - #ededed
     160  472 - #fbfbfb
     276  472 - #e5e5e5
     312  472 - #fbfbfb
     344  472 - #ffffff
     404  472 - #e9e9e9
     232  476 - #bcbcbc
      40  488 - #dfdfdf
      88  488 - #e1e1e1
     120  488 - #bcbcbc
     196  488 - #ffffff
     592  592 - #ffffff
";

const CHECK_6: &str = r"
     452    4 - #ffffff
     588    4 - #ffffff
      44   16 - #c0c0c0
      40   20 - #ffffff
      88   20 - #ffffff
      96   20 - #c6c6c6
     116   20 - #c0c0c0
     172   20 - #c7c7c7
      32   44 - #1b1e25
      80   44 - #1a1d24
     124   44 - #1a1d24
      52   48 - #9fa0a3
      56   48 - #585a5f
      92   48 - #1a1d24
     112   48 - #1a1d24
     252   68 - #b8d2ff
     260   68 - #b8d2ff
     276   68 - #b8d2ff
     120   72 - #1a1d24
     180   72 - #b7b8ba
     244   72 - #bcd5ff
     248   72 - #b8d2ff
     252   72 - #b8d2ff
     264   72 - #b8d2ff
     268   72 - #b8d2ff
     272   72 - #b8d2ff
     276   72 - #b8d2ff
      64   76 - #1a1d24
      80   76 - #ffffff
     104   76 - #d5d5d7
     144   76 - #1a1d24
     164   76 - #ffffff
     244   76 - #bcd5ff
     252   76 - #b8d2ff
     256   76 - #317ef6
     268   76 - #b8d2ff
     276   76 - #2f7df6
     308   76 - #88b5fa
     332   76 - #404248
     336   76 - #7b7d81
     244   80 - #bcd5ff
     252   80 - #b8d2ff
     268   80 - #b8d2ff
      52  100 - #ffffff
     100  100 - #9b9da0
      56  104 - #cacbcc
     100  104 - #9b9da0
     112  124 - #525459
      84  128 - #8f9094
     488  140 - #ffffff
      52  152 - #ffffff
      72  156 - #ffffff
      88  156 - #9b9da0
     144  216 - #9369e2
     160  216 - #b9621f
     168  220 - #e5d8d0
      60  256 - #dfe2e7
      72  260 - #3b3e45
      88  260 - #1b1e25
     148  260 - #afb2b7
     160  260 - #1a1d24
      28  264 - #dce0e6
     364  264 - #ffffff
     592  268 - #ffffff
      44  276 - #e3e6eb
      68  276 - #e3e6eb
      92  276 - #e3e6eb
     116  276 - #e3e6eb
     140  276 - #e3e6eb
     168  276 - #e3e6eb
      28  284 - #dce0e6
      56  288 - #929497
      28  300 - #dce0e6
     100  428 - #000000
     104  428 - #b3b3b3
     260  428 - #000000
     464  428 - #ffffff
      32  432 - #ffffff
      52  432 - #010101
      68  432 - #ffffff
      84  432 - #808080
     100  432 - #000000
     104  432 - #b3b3b3
     116  432 - #808080
     172  432 - #878787
     200  432 - #ffffff
     228  432 - #000000
     260  432 - #000000
      88  452 - #e2e2e2
      44  456 - #c0c0c0
      88  456 - #e2e2e2
     120  456 - #d8d8d8
       4  592 - #ffffff
     204  592 - #ffffff
     364  592 - #ffffff
     592  592 - #ffffff
";

/// One selection goes over the blocks of a selectable markdown view. The
/// copy has no marks. A right click opens a menu with Copy, Cmd or Ctrl
/// with A takes all the text, and a link still opens on a click.
#[view]
struct MarkdownSelection {
    #[init]
    role:     Label,
    text:     MarkdownView,
    step:     Label,
    selected: Label,
}

impl Setup for MarkdownSelection {
    fn setup(self: Weak<Self>) {
        self.set_color(WHITE);

        self.role
            .set_text("selectable markdown view")
            .set_text_size(14)
            .set_text_color(GRAY)
            .set_alignment(TextAlignment::Left);
        self.role.place().t(8).lr(12).h(20);

        self.text.set_text(TEXT).set_opens_links(false).set_selectable(true);
        self.text.place().t(32).lr(12).h(380);

        self.step
            .set_text("nothing done yet")
            .set_text_size(16)
            .set_alignment(TextAlignment::Left);
        self.step.place().t(420).lr(12).h(24);

        self.selected
            .set_text("selected: nothing")
            .set_multiline(true)
            .set_text_size(14)
            .set_text_color(GRAY)
            .set_alignment(TextAlignment::Left)
            .set_vertical_alignment(VerticalAlignment::Top);
        self.selected.place().t(446).lr(12).h(150);
    }
}

impl MarkdownSelection {
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

impl ViewTest for MarkdownSelection {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        set_record_probe_count(96);

        from_main(|| UIManager::set_drag_scrolling(false));
        // The view lays its text out on the turn after it got it.
        wait_for_next_frame();
        wait_for_next_frame();

        let root = view.text.weak_view();

        // From the heading over the paragraph into the first bullet.
        let (heading, bullet) = from_main(move || {
            (
                point_of(label_with(root, "A heading"), "heading"),
                point_after(label_with(root, "First bullet"), "First"),
            )
        });
        inject_touches(drag(heading, bullet));
        assert_eq!(
            view.show("drag from the heading into the first bullet"),
            "heading\n\nPlain text with bold words and a link to a page in it.\n\n• First"
        );
        check_colors(CHECK_1)?;

        // A word of the code block.
        let answer = from_main(move || point_of(label_with(root, "let answer"), "nswer"));
        inject_touches(click(answer));
        inject_touches(click(answer));
        assert_eq!(view.show("double click on answer in the code"), "answer");
        check_colors(CHECK_2)?;

        // 2 cells of the table, a tab stands between them in the copy.
        let (name, value) = from_main(move || {
            (
                point_of(label_with(root, "Name"), "Name"),
                point_after(label_with(root, "Value"), "Value"),
            )
        });
        inject_touches(drag(name, value));
        assert_eq!(
            view.show("drag over the 2 head cells of the table"),
            "Name\tValue"
        );
        check_colors(CHECK_3)?;

        inject_modifiers(ModifiersState::SUPER);
        inject_keys("a");
        inject_modifiers(ModifiersState::empty());
        assert_eq!(view.show("Cmd or Ctrl with A"), drawn());
        check_colors(CHECK_4)?;

        // The menu of a right click copies what is selected.
        #[cfg(desktop)]
        from_main(|| {
            Clipboard::set_in_process(true);
            Clipboard::set_text("not copied yet").expect("the clipboard takes a text");
        });
        inject_right_click(heading.0, heading.1);
        from_main(|| {
            let menu = ContextMenu::open();
            assert!(menu.is_ok(), "the right click opened no menu");
            assert_eq!(menu.items()[0].title(), "Copy");
            assert!(menu.items()[0].is_enabled());
            assert_eq!(menu.items()[1].title(), "Select All");
        });
        view.show("right click, the menu has Copy and Select All");
        check_colors(CHECK_5)?;

        let copy = from_main(|| ContextMenu::open().items()[0].absolute_frame().center());
        inject_touches(click((copy.x, copy.y)));
        from_main(|| {
            assert!(ContextMenu::open().is_null(), "Copy left the menu open");
            assert_eq!(TextSelection::text(), drawn(), "Copy dropped the selection");
            #[cfg(desktop)]
            assert_eq!(
                Clipboard::get_text().expect("the clipboard has the copy"),
                drawn()
            );
        });

        // A click on the link still reaches it, a drag over it does not.
        let tapped = from_main(move || {
            let (sender, receiver) = channel();
            view.text.link_tapped.val(move |url| {
                sender.send(url).expect("the test waits for the link");
            });
            receiver
        });
        let (link, link_end) = from_main(move || {
            let label = label_with(root, "link to a page");
            (point_of(label, "nk to a page"), point_after(label, "link to a"))
        });
        inject_touches(click(link));
        assert_eq!(
            tapped.try_recv().ok().as_deref(),
            Some("https://example.com/page")
        );
        assert_eq!(view.show("click on the link, it is tapped"), "");

        // From another place than the click, or the 2 presses are a double
        // click.
        let inside = from_main(move || point_of(label_with(root, "link to a page"), "k to a page"));
        inject_touches(drag(inside, link_end));
        assert!(tapped.try_recv().is_err(), "a drag over the link tapped it");
        assert_eq!(view.show("drag over the link, it is not tapped"), "k to a");
        check_colors(CHECK_6)?;

        Ok(())
    }
}
