use anyhow::Result;
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::color::Color,
    refs::Weak,
    ui::{
        Anchor::Left, Container, DynamicColor, Label, Setup, Theme, ThemeMode, ToolButton, ViewData,
        ViewFrame, ViewTest, view,
    },
    ui_test::{check_colors, inject_touches},
};

const BAR: DynamicColor = DynamicColor::new(Color::hex("#F4F4F6"), Color::hex("#27272A"));
const STATUS: DynamicColor = DynamicColor::new(Color::hex("#1C1C1E"), Color::hex("#F2F2F4"));

/// A toolbar of 4 `ToolButton`s on a panel, the toolbar of a 3D editor:
/// Labels is a toggle, Save and the 2 exports are plain buttons. The
/// status line under them names what the taps did.
#[view]
struct ToolButtonTest {
    saves: u32,

    #[init]
    bar:        Container,
    labels:     ToolButton,
    save:       ToolButton,
    export_3mf: ToolButton,
    export_stl: ToolButton,
    status:     Label,
}

impl ToolButtonTest {
    fn show_status(&self) {
        let labels = if self.labels.is_on() { "on" } else { "off" };
        self.status.set_text(format!("toolbar: labels {labels}, saves {}", self.saves));
    }
}

impl Setup for ToolButtonTest {
    fn setup(mut self: Weak<Self>) {
        self.bar.set_color(BAR);
        self.bar.place().back();

        self.labels.set_icon("tool_tag.svg").set_text("Labels");
        self.save.set_icon("tool_save.svg").set_text("Save");
        self.export_3mf.set_icon("tool_download.svg").set_text("Export 3MF");
        self.export_stl.set_icon("tool_download.svg").set_text("Export STL");

        self.labels.place().t(12).l(12);
        self.save.place().t(12).anchor(Left, self.labels, 8);
        self.export_3mf.place().t(12).anchor(Left, self.save, 8);
        self.export_stl.place().t(12).anchor(Left, self.export_3mf, 8);

        self.labels.on_tap(move || {
            self.labels.set_on(!self.labels.is_on());
            self.show_status();
        });
        self.save.on_tap(move || {
            self.saves += 1;
            self.show_status();
        });
        self.export_stl.on_tap(move || {
            self.saves += 100;
            self.show_status();
        });

        self.status.set_text_size(16).set_text_color(STATUS);
        self.status.place().t(72).lr(12).h(30);
        self.show_status();
    }
}

impl ViewTest for ToolButtonTest {
    fn canvas() -> (u32, u32) {
        (600, 120)
    }

    fn before_start() {
        from_main(|| Theme::set_mode(ThemeMode::Light));
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        from_main(move || {
            assert!(view.save.x() >= view.labels.max_x() + 7.9);
            assert!(view.export_3mf.x() >= view.save.max_x() + 7.9);
            assert!(view.export_3mf.width() > view.save.width());
            assert!(view.export_stl.max_x() < 600.0);
        });

        check_colors(LIGHT)?;

        tap(view.labels);
        from_main(move || assert!(view.labels.is_on()));
        check_colors(LABELS_ON)?;

        press(view.save, "b");
        check_colors(SAVE_PRESSED)?;
        press(view.save, "e");
        from_main(move || assert_eq!(view.saves, 1));

        from_main(move || {
            view.export_stl.set_enabled(false);
        });
        tap(view.export_stl);
        from_main(move || assert_eq!(view.saves, 1));
        check_colors(STL_DISABLED)?;

        from_main(|| Theme::set_mode(ThemeMode::Dark));
        wait_for_next_frame();
        check_colors(DARK)?;

        press(view.save, "b");
        check_colors(DARK_SAVE_PRESSED)?;
        press(view.save, "e");
        from_main(move || assert_eq!(view.saves, 2));

        from_main(|| Theme::set_mode(ThemeMode::System));

        Ok(())
    }
}

/// A press at the right end of the button, away from the icon and the
/// text, so the marker of human mode covers no probe.
fn press(button: Weak<ToolButton>, event: &str) {
    let (x, y) = from_main(move || (button.max_x() - 4.0, button.y() + 6.0));
    inject_touches(format!("{x} {y} {event}"));
}

fn tap(button: Weak<ToolButton>) {
    press(button, "b");
    press(button, "e");
}

/// Light theme, every button at rest, dark icons and texts on the panel.
const LIGHT: &str = r"
             520    4 - #f4f4f6
              76   24 - #555557
             216   24 - #3e3e40
             280   24 - #29292b
              28   28 - #f4f4f6
              60   28 - #1c1c1e
             216   28 - #3e3e40
             252   28 - #4b4b4d
             268   28 - #1d1d1f
             344   28 - #f4f4f6
             368   28 - #818183
             392   28 - #7a7a7c
              48   32 - #48484a
             220   32 - #474749
             352   32 - #959597
             296   84 - #8d8d8f
             328   84 - #1c1c1e
             208   88 - #a2a2a4
             216   88 - #f4f4f6
             224   88 - #f4f4f6
             236   88 - #1c1c1e
             276   88 - #6c6c6e
             288   88 - #1c1c1e
             292   88 - #616163
             296   88 - #8d8d8f
             316   88 - #f4f4f6
             368   88 - #616163
             388   88 - #f4f4f6
               4  112 - #f4f4f6
             116  112 - #f4f4f6
             488  112 - #f4f4f6
             592  112 - #f4f4f6
";

/// Labels is on, a pale blue fill under a blue icon and text.
const LABELS_ON: &str = r"
             592    4 - #f4f4f6
             492   12 - #f4f4f6
              60   24 - #1774ff
              76   24 - #428eff
             216   24 - #3e3e40
             280   24 - #29292b
              28   28 - #e1ecff
              60   28 - #0a6cff
              76   28 - #428eff
             252   28 - #4b4b4d
             268   28 - #1d1d1f
             344   28 - #f4f4f6
             368   28 - #818183
             392   28 - #7a7a7c
              48   32 - #3686ff
             220   32 - #474749
             352   32 - #959597
             232   84 - #b8b8ba
             236   84 - #878789
             268   84 - #1c1c1e
             280   84 - #303032
             212   88 - #f4f4f6
             232   88 - #b8b8ba
             284   88 - #f4f4f6
             292   88 - #616163
             316   88 - #f4f4f6
             324   88 - #bfbfc1
             352   88 - #1c1c1e
             368   88 - #616163
             384   88 - #1c1c1e
             120  112 - #f4f4f6
             516  112 - #f4f4f6
";

/// Save held down, a gray fill behind it.
const SAVE_PRESSED: &str = r"
             592    4 - #f4f4f6
             116   12 - #d6d6dd
             168   12 - #d6d6dd
             492   12 - #f4f4f6
              60   24 - #1774ff
              76   24 - #428eff
             216   24 - #3e3e40
             280   24 - #29292b
              28   28 - #e1ecff
              60   28 - #0a6cff
              76   28 - #428eff
             252   28 - #4b4b4d
             368   28 - #818183
             392   28 - #7a7a7c
              48   32 - #3686ff
             152   32 - #d6d6dd
             156   32 - #d6d6dd
             220   32 - #474749
             352   32 - #959597
             124   40 - #d6d6dd
             232   84 - #b8b8ba
             268   84 - #1c1c1e
             212   88 - #f4f4f6
             232   88 - #b8b8ba
             284   88 - #f4f4f6
             292   88 - #616163
             316   88 - #f4f4f6
             324   88 - #bfbfc1
             352   88 - #1c1c1e
             368   88 - #616163
             384   88 - #1c1c1e
             516  112 - #f4f4f6
";

/// Export STL disabled, its icon and text faint.
const STL_DISABLED: &str = r"
             520    4 - #f4f4f6
              60   24 - #1774ff
              76   24 - #428eff
             216   24 - #3e3e40
             280   24 - #29292b
              28   28 - #e1ecff
              60   28 - #0a6cff
              76   28 - #428eff
             252   28 - #4b4b4d
             268   28 - #1d1d1f
             356   28 - #f4f4f6
             368   28 - #d2d2d7
             392   28 - #d0d0d5
              48   32 - #3686ff
             220   32 - #474749
             352   32 - #d8d8dd
             232   84 - #b8b8ba
             236   84 - #878789
             268   84 - #1c1c1e
             280   84 - #303032
             388   84 - #89898b
             212   88 - #f4f4f6
             232   88 - #b8b8ba
             284   88 - #f4f4f6
             292   88 - #616163
             324   88 - #bfbfc1
             352   88 - #1c1c1e
             368   88 - #616163
             388   88 - #89898b
             120  112 - #f4f4f6
             488  112 - #f4f4f6
             592  112 - #f4f4f6
";

/// The same toolbar in the dark theme, light icons and texts.
const DARK: &str = r"
             520    4 - #27272a
              60   24 - #579ef5
              76   24 - #4a87d2
             216   24 - #d2d2d4
             280   24 - #e6e6e8
              28   28 - #1f3352
              60   28 - #5aa5ff
              76   28 - #4a87d2
             252   28 - #c6c6c8
             268   28 - #f1f1f3
             368   28 - #434349
             392   28 - #45454b
              48   32 - #4e8edc
             220   32 - #c9c9cc
             352   32 - #3e3e43
             232   84 - #606062
             236   84 - #8e8e90
             268   84 - #f2f2f4
             388   84 - #8b8b8e
             212   88 - #27272a
             232   88 - #606062
             284   88 - #27272a
             292   88 - #b1b1b3
             316   88 - #27272a
             324   88 - #58585b
             328   88 - #f1f1f3
             352   88 - #f2f2f4
             368   88 - #b1b1b3
             388   88 - #8b8b8e
             120  112 - #27272a
             488  112 - #27272a
             592  112 - #27272a
";

/// Save held down in the dark theme.
const DARK_SAVE_PRESSED: &str = r"
             520    4 - #27272a
             116   12 - #46464c
              60   24 - #579ef5
              76   24 - #4a87d2
             216   24 - #d2d2d4
             280   24 - #e6e6e8
              28   28 - #1f3352
              60   28 - #5aa5ff
              76   28 - #4a87d2
             252   28 - #c6c6c8
             368   28 - #434349
             392   28 - #45454b
              48   32 - #4e8edc
             156   32 - #46464c
             220   32 - #c9c9cc
             352   32 - #3e3e43
             108   40 - #46464c
             148   40 - #46464c
             232   84 - #606062
             236   84 - #8e8e90
             268   84 - #f2f2f4
             388   84 - #8b8b8e
             212   88 - #27272a
             232   88 - #606062
             284   88 - #27272a
             292   88 - #b1b1b3
             328   88 - #f1f1f3
             352   88 - #f2f2f4
             368   88 - #b1b1b3
             388   88 - #8b8b8e
             488  112 - #27272a
             592  112 - #27272a
";
