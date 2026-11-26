use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::color::Color,
    refs::Weak,
    ui::{
        DrawerShell, DynamicColor, Label, Setup, TextAlignment, Theme, ThemeMode, ViewData, ViewFrame,
        ViewSubviews, ViewTest, view,
    },
    ui_test::check_colors,
};

const PAGE: DynamicColor = DynamicColor::new(Color::hex("#FFFFFF"), Color::hex("#1C1C1E"));
const TEXT: DynamicColor = DynamicColor::new(Color::hex("#1C1C1E"), Color::hex("#F2F2F4"));
const GREEN: DynamicColor = DynamicColor::new(Color::hex("#2F9E44"), Color::hex("#51CF66"));

const LONG: &str = "A project with a name far too long for the bar of a phone";
const SHORT: &str = "Website";
/// The margin right of the title, where a cut title ends.
const RIGHT: f32 = 12.0;

/// A narrow drawer shell, so its bar shows. A long title ends with an
/// ellipsis at the right margin of the bar, with its icon and without it.
/// A short title shows whole.
#[view]
struct DrawerTitleEllipsisTest {
    status: Weak<Label>,

    #[init]
    shell: DrawerShell,
}

impl Setup for DrawerTitleEllipsisTest {
    fn setup(mut self: Weak<Self>) {
        self.shell.place().back();
        self.shell.set_title(LONG);
        self.shell.set_title_icon("tool_tag.svg", GREEN);
        self.shell.content().set_color(PAGE);

        self.status = self.shell.content().add_view::<Label>();
        self.status
            .set_text("long title with its icon: it ends with ...")
            .set_text_size(16)
            .set_text_color(TEXT)
            .set_alignment(TextAlignment::Center);
        self.status.place().t(40).lr(12).h(30);
    }
}

impl DrawerTitleEllipsisTest {
    /// What the title shows, where it ends and how wide the bar is.
    fn title(self: Weak<Self>) -> (String, f32, f32) {
        from_main(move || {
            let bar = self.shell.bar();
            let title = bar
                .subviews()
                .iter()
                .find_map(|view| view.weak_view().downcast::<Label>())
                .expect("the bar holds its title");
            (
                title.display_text(title.width()).to_string(),
                title.max_x(),
                bar.width(),
            )
        })
    }

    fn check_cut(self: Weak<Self>, state: &str) -> Result<()> {
        let (shown, end, bar) = self.title();
        ensure!(
            shown.ends_with('…') && shown.len() < LONG.len(),
            "{state}: the title ends with an ellipsis, it shows {shown:?}"
        );
        ensure!(
            (end - (bar - RIGHT)).abs() < 0.5,
            "{state}: the title ends at the right margin, at {end} of {bar}"
        );
        Ok(())
    }
}

impl ViewTest for DrawerTitleEllipsisTest {
    fn canvas() -> (u32, u32) {
        (400, 160)
    }

    fn before_start() {
        from_main(|| Theme::set_mode(ThemeMode::Light));
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        settle();
        view.check_cut("long title with its icon")?;
        check_colors(WITH_ICON)?;

        from_main(move || {
            view.shell.clear_title_icon();
            view.status.set_text("long title, no icon: it ends with ...");
        });
        settle();
        view.check_cut("long title, no icon")?;
        check_colors(NO_ICON)?;

        from_main(move || {
            view.shell.set_title(SHORT);
            view.status.set_text("short title: it shows whole");
        });
        settle();
        let (shown, ..) = view.title();
        ensure!(shown == SHORT, "a short title shows whole, it shows {shown:?}");
        check_colors(SHORT_TITLE)?;

        from_main(|| Theme::set_mode(ThemeMode::System));
        Ok(())
    }
}

fn settle() {
    for _ in 0..3 {
        wait_for_next_frame();
    }
}

/// The long title cut, right of the green icon.
const WITH_ICON: &str = r"
               4    4 - #f9f9f9
             148   20 - #1c1c1e
             308   20 - #1c1c1e
              56   24 - #f9f9f9
             104   24 - #f9f9f9
             132   24 - #f9f9f9
             180   24 - #1d1d1f
             196   24 - #f9f9f9
             212   24 - #313133
             244   24 - #f9f9f9
             288   24 - #f9f9f9
             360   24 - #2c2c2d
               8   80 - #ffffff
             392   84 - #ffffff
             108   96 - #1c1c1e
             236   96 - #1c1c1e
             304   96 - #4a4a4c
              80  100 - #ffffff
             100  100 - #1c1c1e
             128  100 - #656566
             156  100 - #ffffff
             172  100 - #383839
             200  100 - #ffffff
             220  100 - #1c1c1e
             232  100 - #232325
             248  100 - #656566
             336  144 - #ffffff
               4  152 - #ffffff
              60  152 - #ffffff
             188  152 - #ffffff
             280  152 - #ffffff
             392  152 - #ffffff
";

/// The long title cut, right of the menu button.
const NO_ICON: &str = r"
               4    4 - #f9f9f9
             388    4 - #f9f9f9
             104   20 - #3c3c3e
             148   20 - #1c1c1e
              88   24 - #2c2c2d
             120   24 - #f9f9f9
             148   24 - #1c1c1e
             200   24 - #48484a
             204   24 - #49494b
             232   24 - #39393a
             240   24 - #f9f9f9
             264   24 - #f9f9f9
             276   24 - #f9f9f9
             300   24 - #313133
             312   24 - #f9f9f9
             324   24 - #515152
             392   68 - #ffffff
              16   80 - #ffffff
              88   96 - #1c1c1e
              88  100 - #1c1c1e
             112  100 - #ffffff
             128  100 - #1c1c1e
             144  100 - #656566
             168  100 - #ffffff
             192  100 - #ffffff
             200  100 - #1c1c1e
             232  100 - #656566
             252  100 - #1c1c1e
             280  100 - #383839
               4  152 - #ffffff
             324  152 - #ffffff
             392  152 - #ffffff
";

/// The short title whole.
const SHORT_TITLE: &str = r"
               4    4 - #f9f9f9
             196    4 - #f9f9f9
             296    4 - #f9f9f9
             392    4 - #f9f9f9
              84   24 - #f9f9f9
             344   28 - #f9f9f9
              40   36 - #f9f9f9
             144   36 - #f9f9f9
             244   44 - #ffffff
             300   52 - #ffffff
             196   56 - #ffffff
              80   76 - #ffffff
              32   80 - #ffffff
             364   80 - #ffffff
             280   96 - #1c1c1e
             124  100 - #1c1c1e
             152  100 - #202022
             164  100 - #ffffff
             172  100 - #656566
             220  100 - #ffffff
             272  100 - #ffffff
             280  100 - #1c1c1e
             288  100 - #656566
             332  108 - #ffffff
              40  124 - #ffffff
               4  152 - #ffffff
              76  152 - #ffffff
             140  152 - #ffffff
             196  152 - #ffffff
             252  152 - #ffffff
             328  152 - #ffffff
             392  152 - #ffffff
";
