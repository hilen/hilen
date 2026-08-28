use anyhow::Result;

use crate::{
    self as hilen,
    deps::{hreads::from_main, refs::Weak},
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
             592    4 - #597c95
             256   40 - #869fb2
             304   40 - #e9eef1
             344   40 - #ffffff
             256   52 - #869fb2
             292   52 - #a9bbc8
             256   56 - #869fb2
             276   56 - #597c95
             332   60 - #f5f7f8
             100  152 - #00ff00
             496  156 - #00a600
             264  188 - #003000
             288  192 - #00ff00
             316  192 - #000100
             288  200 - #009400
             336  200 - #009600
             344  200 - #00fa00
             292  204 - #00ff00
             288  208 - #007c00
             496  240 - #00a600
             172  248 - #00ff00
             256  292 - #d2d200
             280  292 - #4a4a00
             292  292 - #ffff00
             280  300 - #4a4a00
             288  300 - #949400
             340  300 - #363600
             312  304 - #010100
             292  308 - #7c7c00
             496  324 - #ffff00
             100  368 - #ffcb00
             256  392 - #d2a700
             280  392 - #4a3b00
             292  392 - #ffcb00
             320  392 - #010100
             280  400 - #4a3b00
             288  404 - #ffcb00
             288  408 - #7c6300
             292  408 - #7c6300
             344  408 - #907300
             592  452 - #597c95
             248  540 - #c6d2da
             316  540 - #c6d2da
             336  540 - #c6d2da
             356  544 - #feffff
             292  548 - #597c95
             268  552 - #597c95
             332  560 - #f5f7f8
            ",
    )
}

// After scrolling up 150: ITEM 1 cut at the top edge, ITEM 4 at the bottom.
fn check_items_cut() -> Result<()> {
    check_colors(
        r"
               4    4 - #597c95
             256   40 - #869fb2
             304   40 - #e9eef1
             356   44 - #fefeff
             292   52 - #a9bbc8
             256   56 - #869fb2
             332   56 - #597c95
             144  152 - #ffff00
             256  152 - #d2d200
             280  152 - #4a4a00
             288  152 - #ffff00
             292  156 - #ffff00
             340  156 - #363600
             496  204 - #a68400
             256  240 - #d2a700
             280  240 - #4a3b00
             320  240 - #010100
             292  244 - #ffcb00
             100  252 - #ffcb00
             256  256 - #d2a700
             280  256 - #4a3b00
             292  260 - #100d00
             496  272 - #a68400
             592  300 - #597c95
             440  312 - #ff00ff
             264  340 - #580058
             272  340 - #580058
             280  340 - #4a004a
             320  340 - #010001
             256  344 - #d200d2
             288  344 - #ff00ff
             292  352 - #ff00ff
             256  356 - #d200d2
             340  360 - #000000
             496  396 - #ff00ff
             100  412 - #00ffff
             256  440 - #00d2d2
             264  440 - #005858
             280  440 - #004a4a
             292  444 - #00ffff
             256  448 - #00d2d2
             340  448 - #00ffff
             428  448 - #00ffff
             248  540 - #c6d2da
             316  540 - #c6d2da
             284  544 - #fefeff
             356  544 - #feffff
             336  556 - #597c95
            ",
    )
}

// The photo enters from the bottom and is cut at the scroll edge.
fn check_photo_cut() -> Result<()> {
    check_colors(
        r"
               4    4 - #597c95
             256   40 - #869fb2
             344   40 - #ffffff
             292   52 - #a9bbc8
             256   56 - #869fb2
             276   56 - #597c95
             592  148 - #597c95
             344  152 - #00daff
             428  152 - #00daff
             100  192 - #ff0000
             200  192 - #ff0000
             288  232 - #ff0000
             320  232 - #010000
             288  240 - #940000
             344  244 - #ff0000
             292  248 - #7c0000
             156  292 - #daaa7c
             496  304 - #daaa7c
             264  328 - #292017
             280  332 - #3f3124
             316  332 - #010100
             340  344 - #000000
             288  348 - #6a533c
             496  364 - #8e6f51
             100  392 - #e3bbc0
             408  392 - #deb1b3
             496  400 - #3a5161
             216  412 - #c68171
             220  412 - #c38275
             468  416 - #b58e79
             240  428 - #624531
             264  428 - #352618
             224  432 - #c9a990
             364  436 - #4b4130
             280  440 - #754d39
             332  440 - #b79b7f
             352  440 - #644c39
             296  444 - #a27a5c
             148  448 - #f5dad3
             284  448 - #a6745d
             424  448 - #b59480
             464  448 - #c49192
             496  448 - #c99698
             248  540 - #c6d2da
             316  540 - #c6d2da
             356  544 - #feffff
             292  548 - #597c95
             332  560 - #f5f7f8
            ",
    )
}

// All the way down: the photo bottom aligns with the scroll bottom,
// ITEM 5 is cut at the top edge.
fn check_bottom() -> Result<()> {
    check_colors(
        r"
               4    4 - #597c95
             256   40 - #869fb2
             304   40 - #e9eef1
             344   40 - #ffffff
             292   52 - #a9bbc8
             256   56 - #869fb2
             276   56 - #597c95
             592  124 - #597c95
             172  152 - #ff0000
             288  192 - #ff0000
             320  192 - #010000
             288  200 - #940000
             344  204 - #ff0000
             292  208 - #7c0000
             492  248 - #ff0000
             100  252 - #daaa7c
             264  288 - #292017
             292  292 - #daaa7c
             316  292 - #010100
             288  300 - #7f6348
             340  304 - #000000
             292  308 - #6a533c
             100  340 - #daaa7c
             496  352 - #3a5161
             216  372 - #c68171
             220  372 - #c38275
             444  384 - #9a6651
             264  388 - #352618
             224  392 - #c9a990
             248  392 - #55412a
             412  392 - #97715a
             340  396 - #4c3d25
             364  396 - #4b4130
             492  396 - #c99796
             276  400 - #7b5541
             160  404 - #f4d9d0
             296  404 - #a27a5c
             284  408 - #a6745d
             464  408 - #c49192
             104  436 - #e2afae
             204  448 - #dfb3ab
             412  448 - #a27f69
             496  448 - #b49882
             248  540 - #c6d2da
             316  540 - #c6d2da
             356  544 - #feffff
             292  548 - #597c95
             332  560 - #f5f7f8
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
