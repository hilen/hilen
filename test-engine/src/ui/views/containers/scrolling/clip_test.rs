use anyhow::Result;
use hreads::from_main;
use refs::Weak;

use crate::{
    self as test_engine,
    gm::color::{BROWN, Color, GRAY_BLUE, GREEN, LIGHT_BLUE, ORANGE, PURPLE, RED, TURQUOISE, WHITE, YELLOW},
    ui::{ImageView, Label, ScrollView, Setup, ViewData, ViewFrame, ViewSubviews, ViewTest, view},
    ui_test::{check_colors, inject_scroll, inject_touches, set_record_probe_count},
};

const ITEM_COLORS: [Color; 8] = [GREEN, YELLOW, ORANGE, PURPLE, TURQUOISE, LIGHT_BLUE, RED, BROWN];

// A header above and a footer below the scroll, with background gaps
// between them and the scroll edges. The content is a list of colored
// text items and a photo, three times taller than the scroll. While
// scrolling, items get cut mid element at both edges. Nothing may ever
// draw over the header, the footer or the gaps.
#[view]
struct ScrollClipTest {
    #[init]
    header: Label,
    footer: Label,
    scroll: ScrollView,
}

impl Setup for ScrollClipTest {
    fn setup(mut self: Weak<Self>) {
        self.header.set_text("HEADER").set_color(GRAY_BLUE).set_text_color(WHITE);
        self.header.place().tl(0).size(600, 100);

        self.footer.set_text("FOOTER").set_color(GRAY_BLUE).set_text_color(WHITE);
        self.footer.place().t(500).l(0).size(600, 100);

        self.scroll.set_content_size((400, 900));
        self.scroll.place().t(150).l(100).size(400, 300);

        for (i, color) in ITEM_COLORS.into_iter().enumerate() {
            let item = self.scroll.add_view::<Label>();
            item.set_color(color);
            item.set_text(format!("ITEM {i}"));
            item.set_frame((0, 100 * i, 400, 100));
        }

        let photo = self.scroll.add_view::<ImageView>();
        photo.set_image("cat.png");
        photo.set_frame((0, 800, 400, 100));
    }
}

// The gaps between the scroll edges and the header and footer stay
// pure background in every scroll position.
fn check_no_leak() -> Result<()> {
    check_colors(
        r"
             300  115 - #597c95
             300  145 - #597c95
             110  130 - #597c95
             490  130 - #597c95
             300  455 - #597c95
             300  490 - #597c95
             110  470 - #597c95
             490  470 - #597c95
        ",
    )
}

// The list at rest: ITEM 0..3 visible, cut only at the scroll edges.
fn check_start() -> Result<()> {
    check_colors(
        r"
               4    4 - #597c95
             592    4 - #597c95
             252   40 - #ffffff
             356   44 - #597c95
             252   48 - #ffffff
             308   48 - #597c95
             336   52 - #597c95
             252   56 - #ffffff
             196  152 - #00ff00
             412  152 - #00ff00
             496  152 - #00ff00
             104  192 - #00ff00
             288  192 - #00ff00
             340  196 - #00ff00
             344  196 - #00ff00
             288  200 - #00ff00
             292  200 - #00ff00
             340  200 - #00ff00
             344  200 - #00ff00
             436  228 - #00ff00
             592  240 - #597c95
             204  244 - #00ff00
             136  248 - #00ff00
             288  292 - #ffff00
             292  292 - #ffff00
             288  300 - #ffff00
             292  300 - #ffff00
               4  304 - #597c95
             152  320 - #ffff00
             440  328 - #ffff00
             360  344 - #ffff00
             592  368 - #597c95
             288  392 - #ffcb00
             292  392 - #ffcb00
             292  400 - #ffcb00
             104  448 - #ffcb00
             204  448 - #ffcb00
             384  448 - #ffcb00
             488  448 - #ffcb00
             244  540 - #597c95
             312  540 - #ffffff
             336  540 - #597c95
             264  548 - #597c95
             288  548 - #597c95
             312  548 - #ffffff
             312  556 - #ffffff
               4  592 - #597c95
             560  592 - #597c95
        ",
    )
}

// After scrolling up 150: ITEM 1 cut at the top edge, ITEM 4 at the bottom.
fn check_items_cut() -> Result<()> {
    check_colors(
        r"
               4    4 - #597c95
             592    4 - #597c95
             252   40 - #ffffff
             356   44 - #597c95
             308   48 - #597c95
             336   52 - #597c95
             252   56 - #ffffff
             156  152 - #ffff00
             288  152 - #ffff00
             292  152 - #ffff00
             468  152 - #ffff00
             380  156 - #ffff00
               4  160 - #597c95
             288  240 - #ffcb00
             292  240 - #ffcb00
             404  244 - #ffcb00
             288  252 - #ffcb00
             292  252 - #ffcb00
             104  276 - #ffcb00
             224  304 - #ff00ff
             496  304 - #ff00ff
             288  340 - #ff00ff
             292  340 - #ff00ff
             288  352 - #ff00ff
             292  352 - #ff00ff
             344  352 - #ff00ff
             176  360 - #ff00ff
             432  364 - #ff00ff
             104  372 - #ff00ff
             496  404 - #00ffff
             376  432 - #00ffff
             180  440 - #00ffff
             260  440 - #00ffff
             272  440 - #00ffff
             288  440 - #00ffff
             292  440 - #00ffff
             104  448 - #00ffff
             260  448 - #00ffff
             276  448 - #00ffff
             460  448 - #00ffff
             244  540 - #597c95
             312  540 - #ffffff
             336  540 - #597c95
             288  548 - #597c95
             312  548 - #ffffff
             312  556 - #ffffff
               4  592 - #597c95
             592  592 - #597c95
        ",
    )
}

// The photo enters from the bottom and is cut at the scroll edge.
fn check_photo_cut() -> Result<()> {
    check_colors(
        r"
               4    4 - #597c95
             592    4 - #597c95
             252   40 - #ffffff
             356   44 - #597c95
             308   48 - #597c95
             252   56 - #ffffff
             104  152 - #00daff
             212  152 - #00daff
             316  152 - #00daff
             400  152 - #00daff
             496  192 - #ff0000
             108  228 - #ff0000
             288  232 - #ff0000
             204  236 - #ff0000
             288  240 - #ff0000
             292  240 - #ff0000
             340  240 - #ff0000
             344  240 - #ff0000
             444  288 - #ff0000
             144  296 - #daaa7c
             592  296 - #597c95
             288  332 - #daaa7c
             292  332 - #daaa7c
             288  340 - #daaa7c
             292  340 - #daaa7c
             204  360 - #daaa7c
             104  364 - #daaa7c
             428  392 - #ddafb1
             264  396 - #e6bebf
             384  400 - #deb0b0
             496  400 - #d9a7aa
             324  408 - #deb0b0
             356  408 - #deb0b0
             168  416 - #e4bcbd
             468  432 - #ca9897
             492  432 - #cb9799
             104  444 - #e8b6b9
             228  444 - #e5c0b7
             420  444 - #b4907a
             440  444 - #c69493
             488  448 - #c89697
             244  540 - #597c95
             312  540 - #ffffff
             336  540 - #597c95
             288  548 - #597c95
             312  556 - #ffffff
               4  592 - #597c95
             592  592 - #597c95
        ",
    )
}

// All the way down: the photo bottom aligns with the scroll bottom,
// ITEM 5 is cut at the top edge.
fn check_bottom() -> Result<()> {
    check_colors(
        r"
             592    4 - #597c95
             252   40 - #ffffff
             356   44 - #597c95
             308   48 - #597c95
             252   56 - #ffffff
               4  148 - #597c95
             172  152 - #ff0000
             440  152 - #ff0000
             104  176 - #ff0000
             288  192 - #ff0000
             288  200 - #ff0000
             292  200 - #ff0000
             344  200 - #ff0000
             496  240 - #ff0000
             104  252 - #daaa7c
             200  252 - #daaa7c
             408  264 - #daaa7c
             288  292 - #daaa7c
             292  292 - #daaa7c
             288  300 - #daaa7c
             292  300 - #daaa7c
             180  352 - #e9c2c7
             420  352 - #deb0b2
             248  356 - #e9c1c2
             104  368 - #ecc5ca
             340  368 - #dfb1b1
             468  392 - #ca9897
             444  396 - #ca9897
             132  404 - #e2b0b1
             420  404 - #b4907a
             496  408 - #c99798
             444  424 - #a78873
             464  424 - #9a7e68
             468  424 - #9a7e68
             472  428 - #9a7e68
             424  432 - #987c66
             384  440 - #a88974
             464  440 - #ab8d75
             152  444 - #dfb2ad
             224  444 - #dfbbaf
             104  448 - #dda5a6
             496  448 - #b49882
             244  540 - #597c95
             312  540 - #ffffff
             336  540 - #597c95
             312  556 - #ffffff
               4  592 - #597c95
             592  592 - #597c95
        ",
    )
}

impl ViewTest for ScrollClipTest {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        set_record_probe_count(48);

        let offset = move || from_main(move || view.scroll.get_scroll_content_offset());

        check_start()?;
        check_no_leak()?;

        // Scroll in small steps: ITEM 1 gets cut mid text at the top
        // edge, ITEM 4 at the bottom edge.
        inject_touches("300 300 m");
        for _ in 0..5 {
            inject_scroll(-30);
        }
        assert!((offset() + 150.0).abs() < f32::EPSILON);
        check_items_cut()?;
        check_no_leak()?;

        // The photo enters from the bottom and is cut at the edge.
        for _ in 0..2 {
            inject_scroll(-205);
        }
        assert!((offset() + 560.0).abs() < f32::EPSILON);
        check_photo_cut()?;
        check_no_leak()?;

        // All the way down: the photo bottom aligns with the scroll
        // bottom, ITEM 5 is cut at the top.
        inject_scroll(-1000);
        assert!((offset() + 600.0).abs() < f32::EPSILON);
        check_bottom()?;
        check_no_leak()?;

        Ok(())
    }
}
