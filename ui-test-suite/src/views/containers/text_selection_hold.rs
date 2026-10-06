use anyhow::Result;
#[cfg(desktop)]
use hilen::system::Clipboard;
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{
        CellRegistry, ContextMenu, GRAY, Label, Setup, TableData, TableView, TextAlignment, TextSelection,
        UIManager, VerticalAlignment, View, ViewData, ViewFrame, ViewTest, WHITE, view,
    },
    ui_test::{
        check_colors, inject_long_press, inject_touches, set_record_probe_count, system_input::wait_until,
    },
};

use crate::text_points::{click, label_with, point_of, shown};

const ROWS: usize = 40;

fn line_text(row: usize) -> String {
    format!("Row {row}: a single line of text")
}

#[view]
struct HoldCell {
    #[init]
    text: Label,
}

impl Setup for HoldCell {
    fn setup(self: Weak<Self>) {
        self.text
            .set_text_size(18)
            .set_alignment(TextAlignment::Left)
            .set_vertical_alignment(VerticalAlignment::Top)
            .set_selectable(true);
        self.text.place().t(8).lr(8).b(0);
    }
}

const CHECK_1: &str = r"
     108   16 - #bcbcbc
     232   16 - #d4d4d4
      56   20 - #ffffff
     300   20 - #e6e6e6
     344   20 - #d1d1d1
     400   32 - #bcbcbc
     460   32 - #bcbcbc
     576   32 - #bcbcbc
     124   52 - #010101
     176   52 - #7c7c7c
     216   52 - #1f1f1f
     584   56 - #a6a6a6
      12   76 - #bcbcbc
     584   80 - #a6a6a6
      80   84 - #151515
      80   88 - #151515
     120   88 - #323232
     176   88 - #7c7c7c
     224   88 - #999999
     584   92 - #a6a6a6
     584  104 - #a6a6a6
     112  112 - #b8d2ff
     124  112 - #b8d2ff
     140  112 - #b8d2ff
     148  112 - #b8d2ff
     152  112 - #b8d2ff
      40  116 - #000000
     112  116 - #b8d2ff
     116  116 - #b8d2ff
     132  116 - #b5cffb
     140  116 - #b5cffb
     148  116 - #b5cffb
     152  116 - #b5cffb
     584  116 - #a6a6a6
     192  120 - #e8e8e8
     212  120 - #e8e8e8
     248  120 - #e8e8e8
     264  120 - #e8e8e8
     276  120 - #e9e9e9
      80  128 - #4d4d4d
     584  128 - #a6a6a6
     112  132 - #b5cffb
     284  132 - #d4d4d4
     284  148 - #d3d3d3
     116  152 - #e8e8e8
      12  156 - #bcbcbc
      52  160 - #ffffff
     284  164 - #d3d3d3
     116  172 - #e8e8e8
     284  180 - #d3d3d3
     132  188 - #b3b3b3
     152  188 - #b3b3b3
     192  192 - #d3d3d3
     256  192 - #d3d3d3
     264  192 - #d3d3d3
     272  192 - #d3d3d3
     216  196 - #1d1d1d
     224  196 - #8f8f8f
      40  224 - #000000
      80  224 - #000000
     120  232 - #323232
     176  232 - #7c7c7c
     240  232 - #858585
     428  260 - #ffffff
     592  264 - #ffffff
      80  268 - #ffffff
     120  268 - #323232
     176  268 - #7c7c7c
     224  268 - #999999
      12  280 - #bcbcbc
     120  304 - #323232
     172  304 - #bcbcbc
     240  304 - #858585
      40  332 - #000000
      80  340 - #ffffff
     124  340 - #010101
     176  340 - #7c7c7c
     224  340 - #999999
      52  376 - #ffffff
     136  376 - #ffffff
     172  376 - #bcbcbc
     216  376 - #1f1f1f
     240  376 - #858585
      80  408 - #151515
     172  408 - #2c2c2c
     480  420 - #ffffff
     288  428 - #000000
      72  432 - #e6e6e6
     200  432 - #ffffff
     240  432 - #707070
     320  432 - #808080
      44  456 - #c0c0c0
     104  456 - #bdbdbd
     220  592 - #ffffff
     384  592 - #ffffff
     576  592 - #ffffff
";

const CHECK_2: &str = r"
     108   16 - #bcbcbc
     232   16 - #d4d4d4
      56   20 - #ffffff
     116   20 - #fefefe
     152   20 - #ffffff
     224   20 - #e5e5e5
     232   20 - #d4d4d4
     284   20 - #bcbcbc
     300   20 - #e6e6e6
     320   20 - #cecece
     344   20 - #d1d1d1
     404   32 - #bcbcbc
     464   32 - #bcbcbc
     524   32 - #bcbcbc
     584   32 - #bcbcbc
     584   44 - #a6a6a6
      80   48 - #ffffff
     140   52 - #000000
     176   52 - #7c7c7c
     240   52 - #858585
     584   56 - #a6a6a6
     584   68 - #a6a6a6
      12   76 - #bcbcbc
     584   80 - #a6a6a6
      80   84 - #151515
      80   88 - #151515
     140   88 - #000000
     176   88 - #7c7c7c
     216   88 - #1f1f1f
     584   92 - #a6a6a6
     584  104 - #a6a6a6
     584  116 - #a6a6a6
      40  120 - #ffffff
     120  124 - #323232
     176  124 - #7c7c7c
     224  124 - #999999
     584  124 - #a6a6a6
      80  128 - #4d4d4d
     584  128 - #a6a6a6
      64  156 - #000000
      12  160 - #bcbcbc
     124  160 - #010101
     172  160 - #bcbcbc
     240  160 - #858585
      64  192 - #000000
     120  196 - #323232
     172  196 - #bcbcbc
     176  196 - #7c7c7c
     216  196 - #1f1f1f
     240  196 - #858585
     412  200 - #ffffff
      40  228 - #ffffff
      80  232 - #ffffff
     124  232 - #010101
     176  232 - #7c7c7c
     224  232 - #999999
      52  268 - #ffffff
     120  268 - #323232
     176  268 - #7c7c7c
     216  268 - #1f1f1f
     240  268 - #858585
     592  276 - #ffffff
      12  280 - #bcbcbc
      80  296 - #000000
     124  304 - #010101
     172  304 - #bcbcbc
     176  304 - #7c7c7c
     224  304 - #999999
      40  332 - #000000
      80  340 - #ffffff
     120  340 - #323232
     176  340 - #7c7c7c
     184  340 - #999999
     240  340 - #858585
      12  364 - #bcbcbc
     476  364 - #ffffff
      52  376 - #ffffff
     120  376 - #323232
     172  376 - #bcbcbc
     176  376 - #7c7c7c
     240  376 - #858585
      12  408 - #bcbcbc
      80  408 - #151515
     172  408 - #2c2c2c
     216  408 - #000000
     356  444 - #ffffff
     592  448 - #ffffff
      44  456 - #c0c0c0
      88  456 - #e2e2e2
     116  456 - #e9e9e9
     124  456 - #dedede
     132  456 - #bcbcbc
       4  592 - #ffffff
     200  592 - #ffffff
     360  592 - #ffffff
     592  592 - #ffffff
";

const CHECK_3: &str = r"
     108   16 - #bcbcbc
      56   20 - #ffffff
     168   20 - #ffffff
     220   20 - #c0c0c0
     248   20 - #ffffff
     300   20 - #e6e6e6
     344   20 - #d1d1d1
     424   32 - #bcbcbc
     504   32 - #bcbcbc
      12   36 - #bcbcbc
     584   36 - #a6a6a6
     176   52 - #7c7c7c
     224   52 - #999999
     584   56 - #a6a6a6
      88   76 - #b8d2ff
     144   76 - #b8d2ff
     584   76 - #a6a6a6
      36   80 - #b8d2ff
     216   88 - #161a1f
      64   92 - #b8d2ff
     180   92 - #b8d2ff
     584  100 - #a6a6a6
     116  112 - #b8d2ff
     248  112 - #b8d2ff
     584  112 - #a6a6a6
      40  120 - #b8d2ff
     152  124 - #6e7e99
     216  124 - #161a1f
     584  124 - #a6a6a6
      80  128 - #383f4d
     180  128 - #b8d2ff
     208  128 - #b8d2ff
      40  156 - #b8d2ff
     192  156 - #e8e8e8
     264  156 - #e8e8e8
     296  156 - #e8e8e8
     312  156 - #e8e8e8
     100  168 - #b8d2ff
     144  168 - #b8d2ff
     592  168 - #ffffff
     324  172 - #dadada
     156  192 - #e1e1e1
     320  192 - #d1d1d6
      52  196 - #ffffff
      12  200 - #bcbcbc
     320  212 - #d1d1d6
     156  216 - #e1e1e1
      80  224 - #000000
     248  228 - #d3d3d3
     268  228 - #d3d3d3
     284  228 - #d3d3d3
     304  228 - #d3d3d3
     120  232 - #323232
     176  232 - #747474
     216  232 - #1d1d1d
      12  252 - #bcbcbc
      64  264 - #000000
     152  268 - #999999
     184  268 - #999999
     224  268 - #999999
     456  288 - #ffffff
      40  296 - #000000
      80  296 - #000000
     120  304 - #323232
     176  304 - #7c7c7c
     240  304 - #858585
     592  324 - #ffffff
      12  332 - #bcbcbc
      64  336 - #000000
     120  340 - #323232
     176  340 - #7c7c7c
     224  340 - #999999
      40  368 - #000000
      80  372 - #ffffff
     120  376 - #323232
     172  376 - #bcbcbc
     240  376 - #858585
      80  408 - #151515
     172  408 - #2c2c2c
     216  408 - #000000
      48  432 - #6e6e6e
     132  432 - #d7d7d7
     284  432 - #d4d4d4
     440  452 - #c7c7c7
     100  456 - #ffffff
     152  456 - #cecece
     192  456 - #cecece
     236  456 - #ffffff
     332  456 - #dcdcdc
     364  456 - #e9e9e9
     396  456 - #dadada
     508  456 - #e9e9e9
     544  456 - #bcbcbc
       4  592 - #ffffff
     300  592 - #ffffff
     592  592 - #ffffff
";

const CHECK_4: &str = r"
     232   16 - #d4d4d4
     116   20 - #fefefe
     224   20 - #e5e5e5
     232   20 - #d4d4d4
     300   20 - #e6e6e6
     340   20 - #bcbcbc
     344   20 - #d1d1d1
     400   32 - #bcbcbc
     460   32 - #bcbcbc
     520   32 - #bcbcbc
     584   32 - #bcbcbc
     176   36 - #797979
      80   40 - #4d4d4d
      40   64 - #000000
     112   72 - #212121
     172   72 - #2c2c2c
     196   72 - #999999
     252   72 - #c1c1c1
      52  108 - #ffffff
     140  108 - #010101
     176  108 - #797979
     196  108 - #999999
     252  108 - #c1c1c1
      12  136 - #bcbcbc
     148  144 - #ffffff
     188  144 - #c8c8c8
     236  144 - #999999
      88  148 - #4d4d4d
      92  148 - #4d4d4d
     568  172 - #ffffff
      12  176 - #bcbcbc
      64  176 - #000000
     216  176 - #000000
     112  180 - #212121
     176  180 - #797979
     252  180 - #c1c1c1
      40  208 - #000000
     432  212 - #ffffff
     112  216 - #212121
     176  216 - #797979
     236  216 - #999999
      12  248 - #bcbcbc
      80  252 - #ffffff
     112  252 - #212121
     140  252 - #010101
     176  252 - #797979
     196  252 - #999999
     236  252 - #999999
      40  284 - #ffffff
     112  288 - #212121
     176  288 - #797979
     248  288 - #ffffff
     584  312 - #a6a6a6
      12  320 - #bcbcbc
     216  320 - #000000
     584  320 - #a6a6a6
     112  324 - #212121
     172  324 - #2c2c2c
     252  324 - #c1c1c1
     584  332 - #a6a6a6
     584  344 - #a6a6a6
     448  348 - #ffffff
      64  356 - #000000
     584  356 - #a6a6a6
     140  360 - #010101
     176  360 - #797979
     188  360 - #c8c8c8
     236  360 - #999999
      12  364 - #bcbcbc
     584  368 - #a6a6a6
     584  380 - #a6a6a6
     584  392 - #a6a6a6
     112  396 - #212121
     180  396 - #bdbdbd
     236  396 - #999999
      40  400 - #ffffff
     584  404 - #a6a6a6
     268  428 - #545454
      64  432 - #dedede
     104  432 - #707070
     160  432 - #9e9e9e
     212  432 - #878787
     248  432 - #e6e6e6
     268  432 - #545454
     308  432 - #ffffff
     344  432 - #ffffff
      44  456 - #c0c0c0
      88  456 - #e2e2e2
     116  456 - #e9e9e9
     124  456 - #dedede
     472  484 - #ffffff
     116  568 - #ffffff
       4  592 - #ffffff
     224  592 - #ffffff
     388  592 - #ffffff
     592  592 - #ffffff
";

/// On a touch screen a drag scrolls the list, so it cannot select. A
/// long press selects the word under the finger, the finger then drags
/// the end of the selection, and the release opens a menu with Copy.
#[view]
struct TextSelectionHold {
    #[init]
    role:     Label,
    table:    TableView,
    step:     Label,
    selected: Label,
}

impl Setup for TextSelectionHold {
    fn setup(mut self: Weak<Self>) {
        self.set_color(WHITE);

        self.role
            .set_text("table with selectable text, driven like a touch screen")
            .set_text_size(14)
            .set_text_color(GRAY)
            .set_alignment(TextAlignment::Left);
        self.role.place().t(8).lr(12).h(20);

        self.table.set_data_source(self).register_cell::<HoldCell>();
        self.table.set_text_selectable(true);
        self.table.set_border_color(GRAY).set_border_width(1);
        self.table.place().t(32).lr(12).h(380);

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

impl TableData for TextSelectionHold {
    fn number_of_cells(&self) -> usize {
        ROWS
    }

    fn cell_height(&self, _: usize) -> f32 {
        36.0
    }

    fn setup_cell(&mut self, index: usize, registry: &mut CellRegistry) -> Weak<dyn View> {
        let cell = registry.cell::<HoldCell>();
        cell.text.set_text(line_text(index));
        cell
    }

    fn cell_selected(&mut self, _: usize) {}
}

impl TextSelectionHold {
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

    fn label(self: Weak<Self>, row: usize) -> Weak<Label> {
        let (_, cell) = self
            .table
            .visible_cells()
            .into_iter()
            .find(|(index, _)| *index == row)
            .unwrap_or_else(|| panic!("row {row} is not on screen"));
        label_with(cell, "Row")
    }
}

impl ViewTest for TextSelectionHold {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        set_record_probe_count(96);

        // What a phone does by default, set here so a desktop runs it too.
        from_main(|| UIManager::set_drag_scrolling(true));
        #[cfg(desktop)]
        from_main(|| Clipboard::set_in_process(true));

        let single = from_main(move || point_of(view.label(2), "ngle line"));
        inject_long_press(single.0, single.1);
        // The menu opens on the turn after the release.
        wait_for_next_frame();
        assert_eq!(view.show("long press on single in row 2, then release"), "single");
        from_main(|| {
            let menu = ContextMenu::open();
            assert!(menu.is_ok(), "the release of the long press opened no menu");
            assert_eq!(menu.items()[0].title(), "Copy");
            assert!(menu.items()[0].is_enabled());
        });
        check_colors(CHECK_1)?;

        let copy = from_main(|| ContextMenu::open().items()[0].absolute_frame().center());
        inject_touches(click((copy.x, copy.y)));
        from_main(|| {
            assert!(ContextMenu::open().is_null(), "Copy left the menu open");
            #[cfg(desktop)]
            assert_eq!(
                Clipboard::get_text().expect("the clipboard has the copy"),
                "single"
            );
        });
        assert_eq!(view.show("tap on Copy, the word stays selected"), "single");

        // A tap on text drops the selection.
        let other = from_main(move || point_of(view.label(5), "text"));
        inject_touches(click(other));
        assert_eq!(view.show("tap on row 5"), "");
        check_colors(CHECK_2)?;

        // A hold and then a move of the same finger: the end of the
        // selection goes with the finger, word by word, and the list stays.
        let (row_1, row_3) =
            from_main(move || (point_of(view.label(1), "ow 1"), point_of(view.label(3), "line")));
        inject_touches(format!("{} {} b", row_1.0, row_1.1));
        wait_until("the hold selects the word", || !TextSelection::is_empty())?;
        inject_touches(format!("{} {} m\n{} {} e", row_3.0, row_3.1, row_3.0, row_3.1));
        wait_for_next_frame();
        assert_eq!(
            view.show("hold on Row of row 1, then move to line of row 3"),
            format!("{}\n{}\nRow 3: a single line", line_text(1), line_text(2))
        );
        from_main(move || {
            assert!(
                view.table.content_offset().abs() < 0.5,
                "the hold let the list scroll"
            );
            assert!(ContextMenu::open().is_ok(), "the release opened no menu");
        });
        check_colors(CHECK_3)?;
        from_main(ContextMenu::dismiss_open);

        // A drag with no hold scrolls the list and selects nothing.
        inject_touches(click(other));
        let (from, to) = from_main(move || {
            let frame = *view.table.absolute_frame();
            (
                (frame.center().x, frame.max_y() - 60.0),
                (frame.center().x, frame.y() + 60.0),
            )
        });
        inject_touches(format!(
            "{} {} b\n{} {} m\n{} {} m\n{} {} e",
            from.0,
            from.1,
            from.0,
            f32::midpoint(from.1, to.1),
            to.0,
            to.1,
            to.0,
            to.1
        ));
        // The list flies on after the release. It is fast enough to reach
        // its end, where it stays, so the check waits for the end.
        wait_until("the list comes to rest at its end", move || {
            view.table.content_offset() <= view.table.height() - view.table.content_height() + 0.5
        })?;
        assert_eq!(view.show("drag up with no hold, the list scrolls to its end"), "");
        from_main(move || {
            assert!(
                view.table.content_offset() < -100.0,
                "the drag did not scroll the list"
            );
        });
        check_colors(CHECK_4)?;

        Ok(())
    }
}
