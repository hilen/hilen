use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::color::Color,
    refs::Weak,
    ui::{
        DynamicColor, Label, Setup, TextAlignment, TextDropDown, Theme, ThemeMode, UIManager, View, ViewData,
        ViewFrame, ViewSubviews, ViewTest, WeakView, view,
    },
    ui_test::{check_colors, inject_touches},
};

const PAGE: DynamicColor = DynamicColor::new(Color::hex("#FFFFFF"), Color::hex("#1C1C1E"));
const STATUS: DynamicColor = DynamicColor::new(Color::hex("#1C1C1E"), Color::hex("#F2F2F4"));
const BORDER: DynamicColor = DynamicColor::new(Color::hex("#C8CDD5"), Color::hex("#48484A"));

const LONG: &str = "Tomas Kazlauskas from the studio";

/// What every shown label with the long value shows, in the order of the
/// tree.
fn long_values() -> Vec<String> {
    fn walk(view: WeakView, found: &mut Vec<String>) {
        if view.is_hidden_in_tree() {
            return;
        }
        if let Some(label) = view.downcast::<Label>()
            && label.text() == LONG
        {
            found.push(label.display_text(label.width()).to_string());
        }
        for sub in view.subviews() {
            walk(sub.weak_view(), found);
        }
    }
    from_main(|| {
        let mut found = vec![];
        walk(UIManager::root_view().weak_view(), &mut found);
        found
    })
}

/// 2 drop downs with the same long value. The narrow box ends it with an
/// ellipsis before its arrow, the wide box shows it whole. Opened, the
/// narrow list ends the same value with an ellipsis in its row too.
#[view]
struct DropDownEllipsisTest {
    #[init]
    narrow: TextDropDown,
    wide:   TextDropDown,
    status: Label,
}

impl Setup for DropDownEllipsisTest {
    fn setup(self: Weak<Self>) {
        self.set_color(PAGE);
        for drop in [self.narrow, self.wide] {
            drop.set_values(["Nobody", LONG]);
            drop.set_value(LONG);
            // The runner draws a label with no size at 32 points.
            drop.set_text_size(16);
            drop.set_corner_radius(8).set_border_width(1).set_border_color(BORDER);
        }
        self.narrow.place().t(20).l(20).size(150, 40);
        self.wide.place().t(20).l(190).size(320, 40);

        self.status.set_alignment(TextAlignment::Center);
        self.status.set_text_size(16).set_text_color(STATUS);
        self.status.place().b(20).lr(12).h(30);
        self.status.set_text("closed: the narrow box ends the name with ...");
    }
}

impl ViewTest for DropDownEllipsisTest {
    fn canvas() -> (u32, u32) {
        (530, 240)
    }

    fn before_start() {
        from_main(|| Theme::set_mode(ThemeMode::Light));
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        settle();
        check_colors(CLOSED)?;
        let closed = long_values();
        ensure!(closed.len() == 2, "2 boxes show the long value, found {closed:?}");
        ensure!(
            closed[0].ends_with('…') && closed[0].len() < LONG.len(),
            "the narrow box ends the value with an ellipsis, it shows {:?}",
            closed[0]
        );
        ensure!(
            closed[1] == LONG,
            "the wide box shows the whole value, it shows {:?}",
            closed[1]
        );

        // The middle of the narrow box.
        inject_touches("95 40 b\n95 40 e");
        from_main(move || {
            view.status.set_text("open: its row ends the name with ... too");
        });
        settle();
        check_colors(OPEN)?;
        let open = long_values();
        let cut = open.iter().filter(|text| text.ends_with('…')).count();
        ensure!(
            cut == 2,
            "the narrow box and its row end the value with an ellipsis, found {open:?}"
        );
        ensure!(
            open.iter().any(|text| text == LONG),
            "the wide box still shows the whole value, found {open:?}"
        );

        // A press outside closes the list.
        inject_touches("500 160 b\n500 160 e");
        settle();
        from_main(|| Theme::set_mode(ThemeMode::System));
        Ok(())
    }
}

fn settle() {
    for _ in 0..3 {
        wait_for_next_frame();
    }
}

/// Closed: the narrow box with the cut name, the wide box with the whole one.
const CLOSED: &str = r"
             524    4 - #ffffff
              72   20 - #c8cdd5
             160   20 - #c8cdd5
             316   20 - #c8cdd5
             392   20 - #c8cdd5
             460   20 - #c8cdd5
             256   36 - #393939
             380   36 - #000000
              36   40 - #0a0a0a
             216   40 - #ffffff
             240   40 - #000000
             264   40 - #ffffff
             280   40 - #ffffff
             356   40 - #ffffff
             372   40 - #000000
             380   40 - #000000
             432   40 - #ffffff
             108   44 - #505050
             164  116 - #ffffff
              76  148 - #ffffff
             524  152 - #ffffff
             188  204 - #1c1c1e
             244  204 - #ffffff
             336  204 - #1c1c1e
             172  208 - #555556
             188  208 - #404041
             320  208 - #1c1c1e
             336  208 - #1c1c1e
             364  208 - #1c1c1e
             392  208 - #1c1c1e
               4  232 - #ffffff
             480  232 - #ffffff
";

/// Open: the list under the narrow box, its row with the cut name.
const OPEN: &str = r"
             524    4 - #ffffff
              72   20 - #00daff
             128   20 - #00daff
             160   20 - #00daff
             316   20 - #c8cdd5
             392   20 - #c8cdd5
             460   20 - #c8cdd5
             256   36 - #393939
             380   36 - #000000
              36   40 - #0a0a0a
             240   40 - #000000
             264   40 - #ffffff
             372   40 - #000000
             108   44 - #505050
              24   64 - #e5e5e5
             164   64 - #e3e3e3
              56   88 - #ffffff
              36  120 - #09dbff
              36  124 - #09dbff
             108  128 - #4ae5ff
             428  132 - #ffffff
             168  152 - #e6e6e6
             524  184 - #ffffff
             148  204 - #ffffff
             200  204 - #ffffff
             288  204 - #1c1c1e
             344  204 - #515152
             192  208 - #1c1c1e
             236  208 - #1c1c1e
             344  208 - #515152
             348  208 - #1c1c1e
               4  232 - #ffffff
";
