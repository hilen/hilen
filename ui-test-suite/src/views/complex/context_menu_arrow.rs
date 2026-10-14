use anyhow::Result;
use hilen::{
    dispatch::from_main,
    gm::color::{BLACK, Color, WHITE},
    refs::Weak,
    ui::{
        BlurView, Button, CLEAR, Container, ContextMenu, DynamicColor, Label, MenuAlign, MenuItem, MenuStyle,
        NamedKey, Rect, Setup, TailEdge, ViewData, ViewFrame, ViewSubviews, ViewTest, view,
    },
    ui_test::{check_colors, inject_named_key, inject_touches},
};

/// Menus with an arrow at their button. Over the bar of a player the menu
/// stays above the timeline and its blurred arrow points down across it at
/// the button. A button at the left edge shows that the arrow still points
/// at its button after the menu slid back inside the screen. A plain menu
/// with a border opens under its button with the arrow on top.
#[view]
struct ContextMenuArrow {
    #[init]
    stripes:  Container,
    caption:  Label,
    bar:      Container,
    timeline: Container,
    captions: Button,
    edge:     Button,
    plain:    Button,
}

const STRIPES: [Color; 4] = [
    Color::rgb(0.9, 0.3, 0.2),
    Color::rgb(0.95, 0.8, 0.2),
    Color::rgb(0.2, 0.7, 0.4),
    Color::rgb(0.2, 0.4, 0.9),
];

fn video_style() -> MenuStyle {
    let on_video = |color: Color| DynamicColor::new(color, color);
    MenuStyle {
        background: on_video(BLACK.with_alpha(0.6)),
        blur: 16.0,
        border_width: 0.0,
        text: on_video(Color::rgb(0.93, 0.93, 0.93)),
        highlight: on_video(WHITE.with_alpha(0.1)),
        corner_radius: 12.0,
        shadow: false,
        arrow: 8.0,
        ..MenuStyle::default()
    }
}

fn items() -> Vec<MenuItem> {
    vec![
        MenuItem::new("Off", || {}).checked(false),
        MenuItem::new("English", || {}).checked(true),
        MenuItem::new("Lithuanian", || {}).checked(false),
    ]
}

impl ContextMenuArrow {
    /// The x of the button with the top of the timeline, the menu goes
    /// over the timeline and its arrow points at the button.
    fn over_timeline(self: Weak<Self>, button: Weak<Button>) {
        let anchor = Rect::new(button.x(), self.timeline.y(), button.frame().size.width, 0.0);
        video_style().show_above_frame(items(), anchor, MenuAlign::Center);
    }
}

impl Setup for ContextMenuArrow {
    fn setup(self: Weak<Self>) {
        self.stripes.place().back();
        for color in STRIPES {
            let stripe = self.stripes.add_view::<Container>();
            stripe.set_color(color);
        }
        self.stripes.place().all_hor();

        // The stripes are a sibling with subviews, which draw over the
        // views that follow it, so those go in front by hand.
        self.caption
            .set_text("video under the menus")
            .set_text_size(16)
            .set_color(CLEAR);
        self.caption.place().t(20).l(200).size(240, 32);
        self.caption.bump_z_position(0.000_1);

        self.bar.set_color(BLACK.with_alpha(0.8));
        self.bar.place().lrb(0).h(48);
        self.bar.bump_z_position(0.000_1);

        self.timeline.set_color(Color::rgb(0.43, 0.43, 0.46));
        self.timeline.place().lr(20).b(60).h(4);
        self.timeline.bump_z_position(0.000_1);

        for button in [self.captions, self.edge, self.plain] {
            button.set_color(WHITE);
            button.bump_z_position(0.000_2);
        }

        self.captions.set_text("Captions");
        self.captions.place().b(8).l(270).size(140, 32);
        self.captions.on_tap(move || self.over_timeline(self.captions));

        self.edge.set_text("Edge");
        self.edge.place().b(8).l(4).size(80, 32);
        self.edge.on_tap(move || self.over_timeline(self.edge));

        self.plain.set_text("Plain");
        self.plain.place().t(80).l(40).size(120, 32);
        self.plain.on_tap(move || {
            let style = MenuStyle {
                arrow: 8.0,
                ..MenuStyle::default()
            };
            style.show_below(items(), self.plain, MenuAlign::Center);
        });
    }
}

fn blur_tail_x(menu: Weak<ContextMenu>) -> f32 {
    let blur = menu
        .subviews()
        .iter()
        .find_map(|view| view.downcast_view::<BlurView>())
        .expect("a blurred menu has a BlurView");
    let tail = blur.tail().expect("a menu with an arrow has a tail");
    assert_eq!(tail.edge, TailEdge::Bottom);
    tail.x
}

impl ViewTest for ContextMenuArrow {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        inject_touches("340 576 b\n340 576 e");
        from_main(move || {
            let menu = ContextMenu::open();
            assert!(menu.is_ok());
            let middle = view.captions.frame().center().x;
            assert!((menu.x() + menu.frame().size.width / 2.0 - middle).abs() <= 0.5);
            assert_eq!(menu.max_y(), view.timeline.y() - 4.0);
            assert_eq!(menu.x() + blur_tail_x(menu), middle);
        });
        check_colors(OVER_TIMELINE)?;
        inject_named_key(NamedKey::Escape);
        from_main(|| assert!(ContextMenu::open().is_null()));

        inject_touches("44 576 b\n44 576 e");
        from_main(move || {
            let menu = ContextMenu::open();
            assert!(menu.is_ok());
            assert_eq!(menu.x(), 0.0);
            assert_eq!(blur_tail_x(menu), view.edge.frame().center().x);
        });
        check_colors(AT_THE_EDGE)?;
        inject_named_key(NamedKey::Escape);
        from_main(|| assert!(ContextMenu::open().is_null()));

        inject_touches("100 96 b\n100 96 e");
        from_main(move || {
            let menu = ContextMenu::open();
            assert!(menu.is_ok());
            assert_eq!(menu.y(), view.plain.max_y() + 4.0);
            let middle = view.plain.frame().center().x;
            assert!((menu.x() + menu.frame().size.width / 2.0 - middle).abs() <= 0.5);
        });
        check_colors(PLAIN_BELOW)?;
        inject_named_key(NamedKey::Escape);
        from_main(|| assert!(ContextMenu::open().is_null()));

        Ok(())
    }
}

const OVER_TIMELINE: &str = r"
 332   32 - #0f361f
 256   36 - #f2cc33
 264   36 - #79661a
 272   36 - #f2cc33
 304   36 - #33b266
 312   36 - #1a5933
 344   36 - #1a5933
 376   36 - #000000
  40   84 - #ffffff
  80  104 - #ffffff
 128  104 - #ffffff
 288  256 - #f2cc33
 592  260 - #3366e5
   4  328 - #e54d33
 440  348 - #33b266
 316  448 - #204926
 416  448 - #14472a
 316  472 - #3f6244
 356  480 - #678674
 328  504 - #8aa091
 260  508 - #5f5115
 484  536 - #6e6e75
 592  552 - #0a142e
  60  572 - #b8b8b8
 340  572 - #c9c9c9
  72  576 - #e6e6e6
 312  576 - #8a8a8a
  60  580 - #b8b8b8
 280  580 - #010101
 372  580 - #010101
  16  584 - #727272
 216  592 - #30290a
";

const AT_THE_EDGE: &str = r"
 332   32 - #0f361f
 256   36 - #f2cc33
 264   36 - #79661a
 304   36 - #33b266
 312   36 - #1a5933
 376   36 - #000000
 592   76 - #3366e5
  40   84 - #ffffff
  88   84 - #000000
 128  104 - #ffffff
   4  260 - #e54d33
 228  300 - #f2cc33
 448  300 - #33b266
 156  440 - #5f4014
  56  448 - #5c1f14
  68  452 - #e1dbda
  56  472 - #723f35
  56  480 - #5c1f14
  96  480 - #936d67
  68  508 - #a98d88
 100  508 - #b8a29e
 480  536 - #6e6e75
 592  552 - #0a142e
  20  564 - #272727
 280  564 - #ffffff
  60  572 - #b8b8b8
 340  572 - #c9c9c9
  20  576 - #9d9d9d
 312  576 - #8a8a8a
 372  580 - #010101
  16  584 - #727272
 188  592 - #30290a
";

const PLAIN_BELOW: &str = r"
 256   36 - #f2cc33
 264   36 - #79661a
 272   36 - #f2cc33
 304   36 - #33b266
 344   36 - #1a5933
 376   36 - #000000
 592   92 - #3366e5
 112   96 - #707070
  68  104 - #000000
 180  132 - #bc9e28
  20  136 - #d1d1d6
  84  144 - #8d8d8d
  76  172 - #ffffff
  88  196 - #777777
 112  200 - #2c2c2c
 176  208 - #ffffff
  20  216 - #b53d28
 152  216 - #a38922
 448  304 - #33b266
   4  360 - #e54d33
 220  360 - #f2cc33
 480  536 - #6e6e75
 192  552 - #30290a
 592  552 - #0a142e
  16  564 - #272727
 280  564 - #ffffff
  60  572 - #b8b8b8
 340  572 - #c9c9c9
  72  576 - #e6e6e6
 312  576 - #8a8a8a
  60  580 - #b8b8b8
 372  580 - #010101
";
