use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{
        Color, Container, Label, ScrollView, Setup, TextAlignment, ViewData, ViewFrame, ViewSubviews,
        ViewTest, view,
    },
    ui_test::{check_colors, checkpoint},
};

const PAGE: Color = Color::hex("#F2F4F7");
const CARD: Color = Color::hex("#FFFFFF");
const CHIP: Color = Color::hex("#CFE3FF");
const BAR: Color = Color::hex("#2D6CDF");

const LONG: &str = "A stack lays out its subviews top to bottom. This text wraps, so it gets taller when \
                    the page gets narrower, and everything under it moves down.";

/// A page made of a vertical stack in a scroll view: a wrapping label, a
/// card that is a stack with padding, a row of chips, a hidden view that
/// takes no space and a bar with a fixed height. No frame is set by hand.
#[view]
struct StackLayout {
    stack:  Weak<Container>,
    intro:  Weak<Label>,
    card:   Weak<Container>,
    row:    Weak<Container>,
    hidden: Weak<Container>,
    bar:    Weak<Container>,

    #[init]
    scroll: ScrollView,
}

impl Setup for StackLayout {
    fn setup(mut self: Weak<Self>) {
        self.scroll.set_color(PAGE);
        self.scroll.place().t(0).l(0).w(600).b(0);

        self.stack = self.scroll.add_view();
        self.stack.place().t(0).lr(0).stack_ver().all(12).padding(16);

        self.intro = self.stack.add_view();
        self.intro.set_text(LONG).set_text_size(24).set_multiline(true);
        self.intro.set_alignment(TextAlignment::Left);

        self.card = self.stack.add_view();
        self.card.set_color(CARD).set_corner_radius(10);
        self.card.place().stack_ver().all(6).padding(12);
        for text in ["Card title", "A second line in the card"] {
            let label = self.card.add_view::<Label>();
            label.set_text(text).set_text_size(22).set_alignment(TextAlignment::Left);
        }

        self.row = self.stack.add_view();
        self.row.place().stack_hor().all(8);
        for text in ["Chip", "Another chip", "Third"] {
            let chip = self.row.add_view::<Label>();
            chip.set_text(text).set_text_size(20).set_color(CHIP).set_corner_radius(8);
        }

        self.hidden = self.stack.add_view();
        self.hidden.set_color(Color::hex("#FF0000")).place().h(200);
        self.hidden.set_hidden(true);

        self.bar = self.stack.add_view();
        self.bar.set_color(BAR).place().h(40);
    }
}

impl ViewTest for StackLayout {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        settle();
        let wide = check_stack(view, 600.0)?;
        check_colors(WIDE)?;
        checkpoint("wide: text, card, chips and bar stacked, the hidden view takes no space")?;

        from_main(move || {
            view.scroll.place().clear().t(0).l(0).w(320).b(0);
        });
        settle();
        let narrow = check_stack(view, 320.0)?;
        ensure!(
            narrow.intro > wide.intro,
            "the text wraps to more lines when narrow, {} then {}",
            wide.intro,
            narrow.intro
        );
        check_colors(NARROW)?;
        checkpoint("narrow: the text wraps more and everything under it moved down")?;

        Ok(())
    }
}

struct Heights {
    intro: f32,
}

fn check_stack(view: Weak<StackLayout>, width: f32) -> Result<Heights> {
    from_main(move || {
        let inner = width - 32.0;
        let intro = *view.intro.frame();
        let card = *view.card.frame();
        let row = *view.row.frame();
        let bar = *view.bar.frame();
        let stack = *view.stack.frame();

        ensure!(
            near(intro.origin.x, 16.0) && near(intro.origin.y, 16.0),
            "intro at {intro:?}"
        );
        ensure!(near(intro.size.width, inner), "intro width {intro:?}");
        ensure!(intro.size.height > 30.0, "intro fits its text {intro:?}");

        ensure!(
            near(card.origin.y, intro.max_y() + 12.0),
            "card under intro {card:?}"
        );
        ensure!(near(card.size.width, inner), "card width {card:?}");
        let lines: Vec<_> = view.card.subviews().iter().map(|label| *label.frame()).collect();
        ensure!(
            near(lines[0].origin.y, 12.0) && near(lines[0].origin.x, 12.0),
            "card padding {lines:?}"
        );
        ensure!(
            near(lines[1].origin.y, lines[0].max_y() + 6.0),
            "card gap {lines:?}"
        );
        ensure!(
            near(card.size.height, lines[1].max_y() + 12.0),
            "card fits its lines {card:?}"
        );

        ensure!(near(row.origin.y, card.max_y() + 12.0), "row under card {row:?}");
        let chips: Vec<_> = view.row.subviews().iter().map(|chip| *chip.frame()).collect();
        ensure!(chips[0].origin.x == 0.0, "first chip {chips:?}");
        ensure!(
            near(chips[1].origin.x, chips[0].max_x() + 8.0),
            "chip gap {chips:?}"
        );
        ensure!(
            chips[0].size.width < chips[1].size.width,
            "chips fit their text {chips:?}"
        );
        ensure!(
            near(row.size.height, chips[0].size.height),
            "row is as tall as a chip {row:?}"
        );

        // The hidden view takes no space, the bar follows the row.
        ensure!(near(bar.origin.y, row.max_y() + 12.0), "bar under row {bar:?}");
        ensure!(near(bar.size.height, 40.0), "bar keeps its height {bar:?}");
        ensure!(
            near(stack.size.height, bar.max_y() + 16.0),
            "stack fits {stack:?}"
        );
        ensure!(
            near(view.scroll.content_height(), stack.size.height),
            "scroll content {} and stack {}",
            view.scroll.content_height(),
            stack.size.height
        );

        Ok(Heights {
            intro: intro.size.height,
        })
    })
}

fn settle() {
    for _ in 0..4 {
        wait_for_next_frame();
    }
}

const WIDE: &str = r"
             196   24 - #000000
             540   28 - #f2f4f7
              68   32 - #67686a
             304   32 - #818284
             480   32 - #c2c3c6
             336   52 - #b6b8ba
             408   60 - #515152
             444   60 - #4a4b4c
             280   64 - #68696b
             120   68 - #2c2c2d
              56   88 - #626364
             212   88 - #383839
             336   88 - #a4a6a8
             516   88 - #3c3c3d
             160   92 - #2b2b2c
             388   92 - #a8a9ab
             472   92 - #010101
              48  136 - #ffffff
             116  136 - #c6c6c6
              88  140 - #a9a9a9
             280  164 - #000000
             144  172 - #6d6d6d
             212  172 - #848484
              20  216 - #cfe3ff
             192  216 - #cee2fe
              80  220 - #808d9e
             144  220 - #cfe3ff
             228  220 - #000000
             408  280 - #2d6cdf
             580  280 - #2d6cdf
               4  592 - #f2f4f7
             592  592 - #f2f4f7
            ";
const NARROW: &str = r"
              40   28 - #f2f4f7
             208   36 - #808183
             160   60 - #696a6c
             280   60 - #010101
              88   88 - #acadaf
             120   88 - #323333
              60  108 - #98999b
             216  108 - #6e6f70
             284  136 - #000000
              56  144 - #313232
             100  144 - #414142
             152  144 - #858688
             184  164 - #232324
             216  168 - #b1b2b4
             124  172 - #cdcfd1
              72  176 - #5c5d5e
             592  220 - #597c95
              88  244 - #a9a9a9
             116  244 - #c6c6c6
             212  276 - #848484
             160  284 - #424242
             280  284 - #000000
             192  328 - #cee2fe
              32  332 - #aabad1
             104  332 - #cfe3ff
             164  332 - #8996a9
             192  332 - #cee2fe
             192  336 - #cee2fe
             300  392 - #2d6cdf
               4  592 - #f2f4f7
             308  592 - #f2f4f7
             592  592 - #597c95
            ";

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.01
}
