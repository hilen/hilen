use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::{Own, Weak},
    ui::{
        ChipPick, Chips, Color, DynamicColor, Label, SegmentedControl, Setup, TextAlignment, Theme,
        ThemeMode, ViewData, ViewFrame, ViewTest, view,
    },
    ui_test::{check_colors, checkpoint, inject_touches},
};

/// Chips in rows that wrap, picked 1 or many at a time, and a segmented
/// control. Taps fire `changed`, the setters called by code do not.
#[view]
struct ChipsSegmented {
    events: Own<Vec<String>>,

    #[init]
    status:   Label,
    many:     Chips,
    one:      Chips,
    segments: SegmentedControl,
}

impl Setup for ChipsSegmented {
    fn setup(self: Weak<Self>) {
        self.set_color(DynamicColor::new(Color::hex("#F7F7FA"), Color::hex("#1C1C1E")));
        self.status
            .set_text("tap the chips")
            .set_text_size(18)
            .set_alignment(TextAlignment::Left);
        self.status
            .set_text_color(DynamicColor::new(Color::hex("#1C1C1E"), Color::hex("#F2F2F4")));
        self.status.place().t(10).lr(10).h(30);

        self.many.set_pick(ChipPick::Many);
        self.many.set_texts(["Air", "Soil", "Light", "Water level", "CO2", "Humidity"]);
        self.many.place().t(50).l(20).w(300);

        self.one.set_texts(["Day", "Week", "Month"]);
        self.one.place().t(200).l(20).w(400);

        self.segments.set_texts(["List", "Grid", "Map"]);
        self.segments.place().t(260).l(20).w(300);

        let mut events = self.events.weak();
        self.many.changed.val(move |on| {
            events.push(format!("many {on:?}"));
            self.status.set_text(format!("many on: {on:?}"));
        });
        self.one.changed.val(move |on| events.push(format!("one {on:?}")));
        self.segments.changed.val(move |index| events.push(format!("segment {index}")));
    }
}

impl ViewTest for ChipsSegmented {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        settle();
        from_main(move || {
            let first = view.many.chip(0).unwrap();
            let last = view.many.chip(5).unwrap();
            ensure!(last.y() > first.y(), "the chips wrap to a second row");
            ensure!(
                near(view.many.height(), last.max_y()),
                "the height follows the rows {}",
                view.many.height()
            );
            ensure!(
                view.one.picked() == Some(0),
                "a pick of 1 starts on the first chip"
            );

            view.many.set_on_indexes([1, 3]);
            view.segments.set_picked(2);
            ensure!(view.events.is_empty(), "setters fire nothing {:?}", *view.events);
            Ok(())
        })?;
        check_colors(SET_BY_CODE)?;
        checkpoint("Soil and Water level on, Day picked, Map picked, all set by code")?;

        tap_chip(view.many.chip(0));
        tap_chip(view.many.chip(1));
        tap_chip(view.one.chip(2));
        tap_chip(view.one.chip(2));
        let segment = from_main(move || *view.segments.frame());
        tap((segment.origin.x + 50.0, segment.origin.y + 16.0));

        from_main(move || {
            ensure!(
                *view.events == ["many [0, 1, 3]", "many [0, 3]", "one [2]", "segment 0"],
                "events {:?}",
                *view.events
            );
            ensure!(
                view.many.on_texts() == ["Air", "Water level"],
                "{:?}",
                view.many.on_texts()
            );
            ensure!(view.segments.picked_text() == "List");
            Ok(())
        })?;
        check_colors(TAPPED)?;
        checkpoint("Air and Water level on, Month picked, List picked by taps")?;

        from_main(|| Theme::set_mode(ThemeMode::Dark));
        settle();
        check_colors(DARK)?;
        checkpoint("the same in the dark theme")?;
        from_main(|| Theme::set_mode(ThemeMode::System));

        Ok(())
    }
}

fn tap_chip(chip: Option<Weak<hilen::ui::ToolButton>>) {
    let chip = chip.unwrap();
    let center = from_main(move || chip.absolute_frame().center());
    tap((center.x, center.y));
}

fn tap(at: (f32, f32)) {
    let (x, y) = (at.0.round(), at.1.round());
    inject_touches(format!("{x} {y} b\n{x} {y} e"));
    settle();
}

fn settle() {
    for _ in 0..3 {
        wait_for_next_frame();
    }
}

const SET_BY_CODE: &str = r"
              64   20 - #242426
              96   20 - #bebec1
              28   24 - #1c1c1e
             108   28 - #9f9fa1
              44   32 - #7b7b7e
             112   32 - #666668
              56   64 - #d8d8dd
              96   64 - #0a6cff
             128   64 - #4e4e50
             252   64 - #589bff
             272   64 - #d9d9de
             296   64 - #f7f7fa
              40   68 - #414143
              44   68 - #58585a
              92   68 - #84b5ff
             204   68 - #b9d4ff
             272   68 - #d7d7db
              32  108 - #8f8f92
              72  108 - #6d6d6f
              40  200 - #0a6cff
              88  200 - #d1d1d6
             176  200 - #d1d1d6
             592  200 - #f7f7fa
             112  212 - #f7f7fa
              32  216 - #7bafff
             104  216 - #7a7a7c
             144  216 - #acacaf
             164  276 - #ebebf0
              68  280 - #1c1c1e
             172  280 - #929295
             256  280 - #b2b2b2
             592  592 - #f7f7fa
            ";
const TAPPED: &str = r"
              52   24 - #d5d5d7
             116   24 - #1c1c1e
              28   28 - #3c3c3e
              60   28 - #b1b1b4
              92   28 - #1d1d1f
             128   64 - #4e4e50
             252   64 - #589bff
             296   64 - #f7f7fa
              40   68 - #2e82ff
              44   68 - #458fff
              92   68 - #99999b
             204   68 - #b9d4ff
             252   68 - #589bff
             272   68 - #d7d7db
              32  108 - #8f8f92
              48  108 - #ababad
              72  108 - #6d6d6f
              44  200 - #d1d1d6
             108  200 - #d1d1d6
             156  200 - #0a6cff
             176  200 - #0a6cff
              20  216 - #d3d3d8
             104  216 - #7a7a7c
             144  216 - #98c0ff
             164  216 - #99c1ff
              88  220 - #f7f7fa
             592  232 - #f7f7fa
             164  276 - #ebebf0
              68  280 - #1c1c1e
             172  280 - #929295
             256  280 - #a4a4a8
             592  592 - #f7f7fa
            ";
const DARK: &str = r"
             112   24 - #1c1c1e
              28   28 - #d3d3d5
              60   28 - #606062
              92   28 - #f1f1f3
             156   60 - #1c1c1e
             128   64 - #c1c1c3
             252   64 - #447bc0
             296   64 - #1c1c1e
              40   68 - #5092e2
              92   68 - #78787a
             204   68 - #2a4873
             252   68 - #447bc0
             228   80 - #1f3352
              32  108 - #828284
              72  108 - #a3a3a5
             592  168 - #1c1c1e
             100  200 - #48484e
             152  200 - #5aa5ff
             168  200 - #5aa5ff
             184  204 - #1f3352
              20  216 - #45454b
             104  216 - #979799
             124  216 - #323235
             164  216 - #33598c
              60  276 - #b3b3b6
             164  276 - #2c2c30
              60  280 - #b3b3b6
              68  280 - #ffffff
             168  280 - #e3e3e5
             256  280 - #707073
             116  284 - #5a5a60
             592  592 - #1c1c1e
            ";

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.01
}
