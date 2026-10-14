use anyhow::Result;
use hilen::{
    dispatch::from_main,
    gm::color::{BLACK, Color, WHITE},
    refs::Weak,
    ui::{
        BlurView, Button, CLEAR, Container, ContextMenu, DynamicColor, Font, Label, MenuAlign, MenuItem,
        MenuStyle, NamedKey, Point, Setup, ViewData, ViewFrame, ViewSubviews, ViewTest, view,
    },
    ui_test::{check_colors, inject_named_key, inject_touches},
};

/// A menu with its own look over a picture, like the subtitle menu of a
/// video player: half clear black over a blur, no border, no shadow, tall
/// rows and medium text. The stripes stand for the video, they show soft
/// through the menu.
#[view]
struct ContextMenuStyle {
    #[init]
    stripes: Container,
    caption: Label,
    bar:     Container,
    button:  Button,
}

const STRIPES: [Color; 4] = [
    Color::rgb(0.9, 0.3, 0.2),
    Color::rgb(0.95, 0.8, 0.2),
    Color::rgb(0.2, 0.7, 0.4),
    Color::rgb(0.2, 0.4, 0.9),
];

fn style() -> MenuStyle {
    let on_video = |color: Color| DynamicColor::new(color, color);
    MenuStyle {
        background: on_video(BLACK.with_alpha(0.6)),
        blur: 16.0,
        border_width: 0.0,
        separator: on_video(WHITE.with_alpha(0.2)),
        text: on_video(Color::rgb(0.93, 0.93, 0.93)),
        highlight: on_video(WHITE.with_alpha(0.1)),
        corner_radius: 12.0,
        item_height: 40.0,
        text_size: 15.0,
        font: Some(Font::medium()),
        shadow: false,
        ..MenuStyle::default()
    }
}

fn items() -> Vec<MenuItem> {
    vec![
        MenuItem::new("Off", || {}).checked(false),
        MenuItem::new("English", || {}).checked(true),
        MenuItem::new("Lithuanian", || {}).checked(false),
        MenuItem::separator(),
        MenuItem::new("Find subtitles", || {}),
    ]
}

impl Setup for ContextMenuStyle {
    fn setup(self: Weak<Self>) {
        self.stripes.place().back();
        for color in STRIPES {
            let stripe = self.stripes.add_view::<Container>();
            stripe.set_color(color);
        }
        self.stripes.place().all_hor();

        // The stripes are a sibling with subviews, which draw over the
        // views that follow it, so those go in front by hand.
        self.caption.set_text("video under the menu").set_text_size(16).set_color(CLEAR);
        self.caption.place().t(20).l(20).size(240, 32);
        self.caption.bump_z_position(0.000_1);

        self.bar.set_color(BLACK.with_alpha(0.8));
        self.bar.place().lrb(0).h(48);
        self.bar.bump_z_position(0.000_1);

        self.button.set_color(WHITE);
        self.button.bump_z_position(0.000_2);
        self.button.set_text("Subtitles");
        self.button.place().b(8).r(40).size(160, 32);
        self.button.on_tap(move || {
            style().show_above(items(), self.button, MenuAlign::Right);
        });
    }
}

impl ViewTest for ContextMenuStyle {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        inject_touches("500 576 b\n500 576 e");
        from_main(move || {
            let menu = ContextMenu::open();
            assert!(menu.is_ok());
            assert_eq!(
                Point::new(menu.max_x(), menu.max_y()),
                Point::new(view.button.max_x(), view.button.y() - 4.0)
            );
            assert_eq!(menu.items().len(), 4);
            assert!(
                menu.items()
                    .iter()
                    .all(|item| (item.frame().size.height - 40.0).abs() < f32::EPSILON)
            );
            let blurs = menu
                .subviews()
                .iter()
                .filter(|view| view.downcast_view::<BlurView>().is_some())
                .count();
            assert_eq!(blurs, 1);
        });
        check_colors(STYLED_MENU)?;
        inject_named_key(NamedKey::Escape);
        from_main(|| assert!(ContextMenu::open().is_null()));

        Ok(())
    }
}

const STYLED_MENU: &str = r"
 156   32 - #625215
  72   36 - #32110b
  76   36 - #010000
  88   36 - #73271a
  96   36 - #e54d33
 128   36 - #e54d33
 168   36 - #79661a
 192   36 - #000000
 448  108 - #33b266
 592  248 - #3366e5
 152  352 - #f2cc33
 556  384 - #14295c
 464  400 - #ededed
 456  440 - #14334a
 476  444 - #687692
 500  444 - #364873
 400  472 - #144729
 484  484 - #b6bbc8
 504  484 - #c1c5cf
 520  484 - #ededed
 456  532 - #8e9aa3
 504  532 - #162856
   4  548 - #e54d33
 312  552 - #0a2414
 456  564 - #7d7d7d
 496  568 - #474747
 488  572 - #dbdbdb
 452  576 - #d8d8d8
 420  580 - #4f4f4f
 504  584 - #000000
 556  588 - #ffffff
 208  592 - #30290a
";
