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
              28   20 - #bfbfc1
              60   24 - #29292b
             120   24 - #bfbfc1
             200   24 - #515154
             216   24 - #3e3e40
             280   24 - #29292b
              76   28 - #555557
             216   28 - #3e3e40
             252   28 - #4b4b4d
             268   28 - #1d1d1f
             344   28 - #f4f4f6
             364   28 - #f4f4f6
             368   28 - #818183
             392   28 - #7a7a7c
              48   32 - #474749
             320   32 - #f4f4f6
             352   32 - #959597
             296   84 - #8d8d8f
             328   84 - #1c1c1e
             208   88 - #a2a2a4
             224   88 - #f4f4f6
             236   88 - #1c1c1e
             276   88 - #6c6c6e
             296   88 - #8d8d8f
             316   88 - #f4f4f6
             368   88 - #616163
             388   88 - #f4f4f6
               4  112 - #f4f4f6
             108  112 - #f4f4f6
             488  112 - #f4f4f6
             592  112 - #f4f4f6
";

/// Labels is on, a pale blue fill under a blue icon and text.
const LABELS_ON: &str = r"
             520    4 - #f4f4f6
              28   20 - #accdff
              60   24 - #1774ff
              76   24 - #428eff
             120   24 - #bfbfc1
             200   24 - #515154
             216   24 - #3e3e40
             280   24 - #29292b
              60   28 - #0a6cff
              76   28 - #428eff
             120   28 - #1c1c1e
             252   28 - #4b4b4d
             268   28 - #1d1d1f
             344   28 - #f4f4f6
             368   28 - #818183
             392   28 - #7a7a7c
              48   32 - #3586ff
             324   32 - #f4f4f6
             352   32 - #959597
             232   84 - #b8b8ba
             236   84 - #878789
             268   84 - #1c1c1e
             388   84 - #f4f4f6
             212   88 - #f4f4f6
             232   88 - #b8b8ba
             284   88 - #f4f4f6
             324   88 - #bfbfc1
             352   88 - #1c1c1e
             368   88 - #616163
               4  112 - #f4f4f6
             488  112 - #f4f4f6
             592  112 - #f4f4f6
";

/// Save held down, a gray fill behind it.
const SAVE_PRESSED: &str = r"
             520    4 - #f4f4f6
             140   12 - #d6d6dd
             168   12 - #d6d6dd
              28   20 - #accdff
              60   24 - #1774ff
              76   24 - #428eff
             116   24 - #d2d2d8
             120   24 - #a8a8ae
             216   24 - #3e3e40
             280   24 - #29292b
              60   28 - #0a6cff
              76   28 - #428eff
             120   28 - #1c1c1e
             252   28 - #4b4b4d
             368   28 - #818183
             392   28 - #7a7a7c
              48   32 - #3586ff
             152   32 - #d6d6dd
             156   32 - #d6d6dd
             320   32 - #f4f4f6
             352   32 - #959597
             172   40 - #d6d6dd
             232   84 - #b8b8ba
             212   88 - #f4f4f6
             232   88 - #b8b8ba
             284   88 - #f4f4f6
             324   88 - #bfbfc1
             352   88 - #1c1c1e
             368   88 - #616163
             388   88 - #f4f4f6
             488  112 - #f4f4f6
             592  112 - #f4f4f6
";

/// Export STL disabled, its icon and text faint.
const STL_DISABLED: &str = r"
             520    4 - #f4f4f6
              28   20 - #accdff
              60   24 - #1774ff
              76   24 - #428eff
             200   24 - #515154
             216   24 - #3e3e40
             280   24 - #29292b
              32   28 - #e1ecff
              60   28 - #0a6cff
              76   28 - #428eff
             120   28 - #1c1c1e
             252   28 - #4b4b4d
             268   28 - #1d1d1f
             368   28 - #d2d2d7
             392   28 - #d0d0d5
              48   32 - #3586ff
             320   32 - #f4f4f6
             328   32 - #c5c5cb
             352   32 - #d8d8dd
             236   84 - #878789
             268   84 - #1c1c1e
             388   84 - #89898b
             212   88 - #f4f4f6
             228   88 - #f4f4f6
             284   88 - #f4f4f6
             324   88 - #bfbfc1
             352   88 - #1c1c1e
             368   88 - #616163
             388   88 - #89898b
               4  112 - #f4f4f6
             488  112 - #f4f4f6
             592  112 - #f4f4f6
";

/// The same toolbar in the dark theme, light icons and texts.
const DARK: &str = r"
             520    4 - #27272a
              28   20 - #2e4f7d
              60   24 - #579ef5
              76   24 - #4a87d2
             116   24 - #262629
             200   24 - #c0c0c1
             216   24 - #d2d2d4
             280   24 - #e6e6e8
              60   28 - #5aa5ff
              76   28 - #4a87d2
             116   28 - #262629
             120   28 - #f2f2f4
             252   28 - #c6c6c8
             368   28 - #434349
             392   28 - #45454b
              48   32 - #4e8edc
             328   32 - #4e4e55
             352   32 - #3e3e43
             236   84 - #8e8e90
             268   84 - #f2f2f4
             388   84 - #8b8b8e
             212   88 - #27272a
             232   88 - #606062
             284   88 - #27272a
             316   88 - #27272a
             324   88 - #58585b
             352   88 - #f2f2f4
             368   88 - #b1b1b3
             388   88 - #8b8b8e
               4  112 - #27272a
             488  112 - #27272a
             592  112 - #27272a
";

/// Save held down in the dark theme.
const DARK_SAVE_PRESSED: &str = r"
             520    4 - #27272a
             152   12 - #46464c
              28   20 - #2e4f7d
              60   24 - #579ef5
              76   24 - #4a87d2
             120   24 - #717176
             216   24 - #d2d2d4
             280   24 - #e6e6e8
              60   28 - #5aa5ff
              76   28 - #4a87d2
             120   28 - #f2f2f4
             252   28 - #c6c6c8
             392   28 - #45454b
              48   32 - #4e8edc
             204   32 - #27272a
             328   32 - #4e4e55
             352   32 - #3e3e43
             144   36 - #46464c
             168   40 - #46464c
             232   84 - #606062
             236   84 - #8e8e90
             268   84 - #f2f2f4
             388   84 - #8b8b8e
             212   88 - #27272a
             232   88 - #606062
             284   88 - #27272a
             324   88 - #58585b
             352   88 - #f2f2f4
             368   88 - #b1b1b3
             388   88 - #8b8b8e
             488  112 - #27272a
             592  112 - #27272a
";
