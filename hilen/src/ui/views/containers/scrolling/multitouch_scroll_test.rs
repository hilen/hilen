use anyhow::Result;

use crate::{
    self as hilen,
    deps::{hreads::from_main, refs::Weak},
    gm::{
        LossyConvert,
        color::{BLACK, BLUE, Color, GRAY, GREEN, WHITE},
    },
    ui::{
        Container, Label, ScrollView, Setup, TextAlignment, UIManager, ViewData, ViewSubviews, ViewTest, view,
    },
    ui_test::inject_touches,
};

const ROW_PITCH: f32 = 88.0;
const ROW_HEIGHT: f32 = 84.0;
const ROWS: usize = 60;

/// Two fingers must scroll two side by side scroll views on their own. Before
/// per finger touch ids every finger was id 1, so both scrolls fought over one
/// touch and jittered.
#[view]
struct MultitouchScroll {
    #[init]
    left:  ScrollView,
    right: ScrollView,
}

impl Setup for MultitouchScroll {
    fn setup(self: Weak<Self>) {
        self.left.place().left_half();
        self.left.set_color("#E9E9EE");
        fill(self.left, BLUE, "Inbox");

        self.right.place().right_half();
        self.right.set_color("#E9E9EE");
        fill(self.right, GREEN, "Album");
    }
}

impl ViewTest for MultitouchScroll {
    // This test drives finger drag gestures, which scroll only with
    // drag scrolling on, the touch platform default.
    fn before_start() {
        UIManager::set_drag_scrolling(true);
    }

    fn canvas() -> (u32, u32) {
        (640, 1000)
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        let offsets = move || {
            from_main(move || {
                (
                    view.left.get_scroll_content_offset(),
                    view.right.get_scroll_content_offset(),
                )
            })
        };

        // Both fingers drag their lists up at the same time over 20 steps: the
        // left by 800, the right by 500. Many small steps make a long smooth
        // simultaneous scroll that ends at two different offsets.
        inject_touches(drag_both(960, 40, 25, 20));
        assert_eq!(offsets(), (-800.0, -500.0));

        // A third finger landing on the right list while finger 2 still holds
        // it is ignored, so neither offset moves.
        inject_touches(
            "
            520 700 b 3
            520 400 m 3
        ",
        );
        assert_eq!(offsets(), (-800.0, -500.0));

        // Finger 3 and finger 1 both lift. Neither may drop finger 2's hold on
        // the right list, so finger 2 keeps dragging it further.
        inject_touches(
            "
            520 400 e 3
            120 160 e 1
            500 260 m 2
        ",
        );
        let right = from_main(move || view.right.get_scroll_content_offset());
        assert!((right + 700.0).abs() < f32::EPSILON);

        inject_touches("500 260 e 2");

        Ok(())
    }
}

/// Interleaved two finger drag. Both fingers begin together and step up the
/// screen for `steps` moves, the left by `left_dy` and the right by `right_dy`
/// each step, so the two lists scroll at the same time by different amounts.
fn drag_both(begin_y: i32, left_dy: i32, right_dy: i32, steps: i32) -> String {
    let mut seq = format!("120 {begin_y} b 1\n500 {begin_y} b 2\n");
    for k in 1..=steps {
        let ly = begin_y - left_dy * k;
        let ry = begin_y - right_dy * k;
        seq.push_str(&format!("120 {ly} m 1\n500 {ry} m 2\n"));
    }
    seq
}

fn fill(mut scroll: Weak<ScrollView>, accent: Color, prefix: &'static str) {
    for i in 0..ROWS {
        let row = scroll.add_view::<MultitouchRow>();
        row.set_row(i, accent, prefix);
        let y: f32 = i.lossy_convert();
        row.place().h(ROW_HEIGHT).lr(8).t(y * ROW_PITCH);
    }
    let content: f32 = ROWS.lossy_convert();
    scroll.set_content_height(content * ROW_PITCH);
}

#[view]
struct MultitouchRow {
    #[init]
    avatar:   Container,
    title:    Label,
    subtitle: Label,
    badge:    Label,
    divider:  Container,
}

impl MultitouchRow {
    fn set_row(self: Weak<Self>, index: usize, accent: Color, prefix: &str) -> Weak<Self> {
        self.set_color(if index.is_multiple_of(2) {
            WHITE
        } else {
            Color::rgb(0.95, 0.95, 0.98)
        });

        self.avatar
            .set_color(accent)
            .set_corner_radius(12)
            .place()
            .size(56, 56)
            .l(14)
            .center_y();

        self.title
            .set_text(format!("{prefix} {index}"))
            .set_text_size(21)
            .set_text_color(BLACK)
            .set_alignment(TextAlignment::Left)
            .place()
            .l(84)
            .r(72)
            .t(16)
            .h(26);

        self.subtitle
            .set_text("tap to open")
            .set_text_size(14)
            .set_text_color(GRAY)
            .set_alignment(TextAlignment::Left)
            .place()
            .l(84)
            .r(72)
            .t(46)
            .h(20);

        self.badge
            .set_text(format!("{index}"))
            .set_text_size(14)
            .set_text_color(WHITE)
            .set_alignment(TextAlignment::Center)
            .set_color(accent)
            .set_corner_radius(13)
            .place()
            .size(46, 26)
            .r(14)
            .center_y();

        self.divider.set_color(Color::rgb(0.85, 0.85, 0.88)).place().h(1).lr(0).b(0);

        self
    }
}

impl Setup for MultitouchRow {
    fn setup(self: Weak<Self>) {
        self.set_corner_radius(10);
    }
}
