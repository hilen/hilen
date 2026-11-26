use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::color::Color,
    refs::Weak,
    ui::{
        Container, DynamicColor, Label, Setup, TextAlignment, Theme, ThemeMode, ToolButton, ViewData,
        ViewFrame, ViewSubviews, ViewTest, view,
    },
    ui_test::{check_colors, set_record_probe_count},
};

const PAGE: DynamicColor = DynamicColor::new(Color::hex("#FFFFFF"), Color::hex("#1C1C1E"));
const COLUMN: DynamicColor = DynamicColor::new(Color::hex("#E4E7EC"), Color::hex("#2C2C2E"));
const STATUS: DynamicColor = DynamicColor::new(Color::hex("#1C1C1E"), Color::hex("#F2F2F4"));

const NARROW: f32 = 200.0;
const WIDE: f32 = 340.0;
const LONG: &str = "A project with a very long name";
/// The right padding of a `ToolButton`, where a cut text ends.
const PADDING: f32 = 10.0;

/// The label inside a button, the one view of its kind there.
fn label_of(button: Weak<ToolButton>) -> Weak<Label> {
    button
        .subviews()
        .iter()
        .find_map(|view| view.weak_view().downcast::<Label>())
        .expect("a ToolButton holds a label")
}

/// A column like a sidebar with 2 rows that fill its width. The long text
/// does not fit the narrow column and ends with an ellipsis at the right
/// padding, its left edge stays where the short text starts. In a wide
/// column the whole text shows.
#[view]
struct ToolButtonEllipsisTest {
    short: Weak<ToolButton>,
    long:  Weak<ToolButton>,

    #[init]
    column: Container,
    status: Label,
}

impl Setup for ToolButtonEllipsisTest {
    fn setup(mut self: Weak<Self>) {
        self.set_color(PAGE);
        self.column.set_color(COLUMN);
        self.column.place().t(12).l(12).w(NARROW).h(96);

        self.short = self.column.add_view::<ToolButton>();
        self.long = self.column.add_view::<ToolButton>();
        for (button, text, top) in [(self.short, "Board", 8), (self.long, LONG, 48)] {
            button.set_icon("tool_tag.svg").set_text(text);
            button.set_fills_width(true);
            button.place().t(top).lr(8).h(36);
        }

        self.status.set_alignment(TextAlignment::Center);
        self.status.set_text_size(16).set_text_color(STATUS);
        self.status.place().t(120).lr(12).h(30);
        self.status.set_text("narrow column: the long name ends with ...");
    }
}

impl ToolButtonEllipsisTest {
    /// What the long row shows and where its text ends, against the short row.
    fn check_rows(self: Weak<Self>, cut: bool, state: &str) -> Result<()> {
        let (short, long, button) = from_main(move || {
            let (short, long) = (label_of(self.short), label_of(self.long));
            (
                (short.x(), short.width(), short.content_size().width),
                (
                    long.x(),
                    long.max_x(),
                    long.display_text(long.width()).to_string(),
                ),
                self.long.width(),
            )
        });
        let (short_x, short_width, short_text) = short;
        let (long_x, long_end, shown) = long;
        ensure!(
            (short_width - short_text).abs() < 0.5,
            "{state}: the short label is as wide as its text, {short_width} for {short_text}"
        );
        ensure!(
            (short_x - long_x).abs() < 0.01,
            "{state}: both texts start at 1 left edge, {short_x} and {long_x}"
        );
        if cut {
            ensure!(
                shown.ends_with('…'),
                "{state}: the long text ends with an ellipsis, it shows {shown:?}"
            );
            ensure!(
                shown.len() < LONG.len(),
                "{state}: the long text is cut, it shows {shown:?}"
            );
            ensure!(
                (long_end - (button - PADDING)).abs() < 0.5,
                "{state}: the cut text ends at the right padding, at {long_end} of {button}"
            );
        } else {
            ensure!(shown == LONG, "{state}: the whole text shows, it shows {shown:?}");
        }
        Ok(())
    }
}

impl ViewTest for ToolButtonEllipsisTest {
    fn canvas() -> (u32, u32) {
        (400, 160)
    }

    fn before_start() {
        from_main(|| Theme::set_mode(ThemeMode::Light));
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        // The picker finds few probes on the thin 14 point rows, the checks
        // of `check_rows` pin the cut, these blocks pin the ground around it.
        set_record_probe_count(64);
        view.check_rows(true, "narrow column")?;
        check_colors(NARROW_COLUMN)?;

        from_main(move || {
            view.column.place().clear().t(12).l(12).w(WIDE).h(96);
            view.status.set_text("wide column: the whole name shows");
        });
        wait_for_next_frame();
        wait_for_next_frame();
        view.check_rows(false, "wide column")?;
        check_colors(WIDE_COLUMN)?;

        from_main(|| Theme::set_mode(ThemeMode::System));
        Ok(())
    }
}

/// The narrow column: Board, and the long name cut with an ellipsis.
const NARROW_COLUMN: &str = r"
              48    4 - #ffffff
             236    4 - #ffffff
             336    4 - #ffffff
             392    4 - #ffffff
              84   12 - #e4e7ec
             144   12 - #e4e7ec
             180   12 - #e4e7ec
              12   16 - #e4e7ec
             116   16 - #e4e7ec
             208   24 - #e4e7ec
             288   28 - #ffffff
              28   36 - #e4e7ec
             252   36 - #ffffff
              56   40 - #e4e7ec
              64   40 - #e4e7ec
             148   40 - #e4e7ec
             320   40 - #ffffff
             368   44 - #ffffff
             112   52 - #e4e7ec
             192   52 - #e4e7ec
              16   60 - #e4e7ec
              44   68 - #e4e7ec
             232   68 - #ffffff
             332   72 - #ffffff
              68   80 - #e4e7ec
              92   80 - #e4e7ec
             144   80 - #e4e7ec
             164   80 - #e2e5ea
             280   84 - #ffffff
             392   88 - #ffffff
             204   96 - #e4e7ec
             120  100 - #e4e7ec
             248  100 - #ffffff
              12  104 - #e4e7ec
              44  104 - #e4e7ec
              80  104 - #e4e7ec
             176  104 - #e4e7ec
             308  104 - #ffffff
             356  104 - #ffffff
             384  120 - #ffffff
             120  132 - #1c1c1e
             168  132 - #4a4a4c
             328  132 - #1d1d1f
              72  136 - #1d1d1f
              80  136 - #ffffff
             104  136 - #ffffff
             108  136 - #ffffff
             112  136 - #ffffff
             120  136 - #1c1c1e
             152  136 - #282829
             164  136 - #252527
             180  136 - #656566
             196  136 - #ffffff
             208  136 - #1c1c1e
             252  136 - #49494a
             256  136 - #1c1c1e
             260  136 - #656566
             272  136 - #656566
             288  136 - #ffffff
             324  136 - #525254
             328  136 - #1d1d1f
              24  152 - #ffffff
             356  152 - #ffffff
             388  152 - #ffffff
";

/// The wide column: the whole long name.
const WIDE_COLUMN: &str = r"
              48    4 - #ffffff
              84    4 - #ffffff
             128    4 - #ffffff
             176    4 - #ffffff
             240    4 - #ffffff
             392    4 - #ffffff
              16   12 - #e4e7ec
             208   12 - #e4e7ec
             268   12 - #e4e7ec
             296   12 - #e4e7ec
             344   20 - #e4e7ec
             164   32 - #e4e7ec
             252   36 - #e4e7ec
              56   40 - #e4e7ec
              64   40 - #e4e7ec
             116   44 - #e4e7ec
             284   44 - #e4e7ec
             364   44 - #ffffff
               8   48 - #ffffff
             208   48 - #e4e7ec
             316   48 - #e4e7ec
              88   52 - #e4e7ec
             144   52 - #e4e7ec
             392   52 - #ffffff
             176   56 - #e4e7ec
              36   68 - #e4e7ec
             256   68 - #e4e7ec
             348   72 - #e4e7ec
             288   76 - #e4e7ec
              68   80 - #e4e7ec
              92   80 - #e4e7ec
             144   80 - #e4e7ec
             164   80 - #e2e5ea
             188   80 - #e4e7ec
             228   80 - #1c1c1e
               4   84 - #ffffff
             316  100 - #e4e7ec
             392  100 - #ffffff
             120  104 - #e4e7ec
             256  104 - #e4e7ec
             284  104 - #e4e7ec
              68  112 - #ffffff
             196  112 - #ffffff
               8  116 - #ffffff
              40  120 - #ffffff
             348  120 - #ffffff
              92  132 - #202022
              88  136 - #ffffff
              92  136 - #1c1c1e
             112  136 - #ffffff
             116  136 - #1c1c1e
             120  136 - #ffffff
             152  136 - #1c1c1e
             172  136 - #ffffff
             220  136 - #ffffff
             232  136 - #656566
             276  136 - #656566
             304  136 - #ffffff
             364  144 - #ffffff
               4  152 - #ffffff
              56  152 - #ffffff
             196  152 - #ffffff
             336  152 - #ffffff
             392  152 - #ffffff
";
