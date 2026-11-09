use anyhow::Result;
use hilen::{
    refs::Weak,
    ui::{AccentCard, BarSide, Color, Label, Setup, TextAlignment, ViewData, ViewSubviews, ViewTest, view},
    ui_test::{check_colors, set_record_probe_count},
};

const CARD: Color = Color::hex("#0d2d4a");
const BAR: Color = Color::hex("#0078d4");
const OUTLINE: Color = Color::hex("#0a3c66");
const TEXT: Color = Color::hex("#82b8e4");
const GROUND: Color = Color::hex("#11131a");

// Each card sits on a dark ground, so a bar that was not cut by a round
// corner would show its square end against the ground there.
#[view]
struct AccentCardTest {
    #[init]
    left:        AccentCard,
    right:       AccentCard,
    wide:        AccentCard,
    short:       AccentCard,
    outlined:    AccentCard,
    square:      AccentCard,
    left_cap:    Label,
    right_cap:   Label,
    wide_cap:    Label,
    short_cap:   Label,
    outline_cap: Label,
    square_cap:  Label,
}

impl Setup for AccentCardTest {
    fn setup(self: Weak<Self>) {
        self.set_color(GROUND);

        for card in [
            self.left,
            self.right,
            self.wide,
            self.short,
            self.outlined,
            self.square,
        ] {
            card.set_color(CARD);
            card.set_bar_color(BAR);
        }

        self.left.place().t(48).l(20).size(260, 80);

        self.right.set_bar_side(BarSide::Right);
        self.right.place().t(48).l(320).size(260, 80);

        self.wide.set_corner_radius(16);
        self.wide.set_bar_width(8);
        self.wide.place().t(188).l(20).size(260, 80);

        self.short.place().t(188).l(320).size(260, 16);

        self.outlined.set_border_color(OUTLINE).set_border_width(1);
        self.outlined.place().t(328).l(20).size(260, 80);

        self.square.set_corner_radius(0);
        self.square.place().t(328).l(320).size(260, 80);

        // A title in the card starts at the content edge and is cut by the
        // card where it runs past the right side.
        for card in [self.left, self.wide, self.outlined, self.square] {
            let title = card.add_view::<Label>();
            title
                .set_text("LiveLens Grooming and the sprint planning")
                .set_text_size(14)
                .set_alignment(TextAlignment::Left)
                .set_text_inset(0)
                .set_text_color(TEXT)
                .set_color(CARD);
            title.place().t(8).l(card.content_left()).size(300, 20);
        }

        // Each caption sits right above its card.
        for (label, text, x, y) in [
            (self.left_cap, "bar on the left, title cut", 20, 20),
            (self.right_cap, "bar on the right", 320, 20),
            (self.wide_cap, "bar 8, radius 16", 20, 160),
            (self.short_cap, "card 16 tall", 320, 160),
            (self.outline_cap, "1 point outline", 20, 300),
            (self.square_cap, "square card", 320, 300),
        ] {
            label
                .set_text(text)
                .set_text_size(14)
                .set_alignment(TextAlignment::Left)
                .set_text_color(Color::hex("#c8cfdf"))
                .set_color(GROUND);
            label.place().t(y).l(x).size(260, 24);
        }
    }
}

impl ViewTest for AccentCardTest {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        set_record_probe_count(64);
        check_colors(CARDS)
    }
}

const CARDS: &str = r"
      40   32 - #11131a
      68   32 - #4e525c
      80   32 - #8e93a0
     120   32 - #696d78
     160   32 - #11131a
     340   32 - #11131a
     368   32 - #4e525c
     380   32 - #8e93a0
     408   32 - #c8cfdf
     428   32 - #696d78
      40   68 - #3f698c
     108   68 - #0d2d4a
     168   68 - #527fa5
     188   68 - #6ea0c9
     192   68 - #75a9d3
     272   68 - #82b8e4
     572   80 - #0d2d4a
      20  116 - #0078d4
     476  116 - #0d2d4a
     576  124 - #0078d4
      92  168 - #a3a9b7
      40  172 - #11131a
     120  172 - #7b808c
     340  172 - #11131a
     376  172 - #c8cfdf
     388  172 - #9fa5b3
     396  172 - #c8cfdf
     524  200 - #0d2d4a
      24  204 - #0078d4
      60  208 - #0d2d4a
     120  208 - #0d2d4a
     172  208 - #527fa5
     192  208 - #6ea0c9
     244  208 - #0d2d4a
     276  208 - #82b8e4
      20  240 - #0078d4
      40  308 - #c4cbdb
     104  308 - #c8cfdf
     408  308 - #6b707b
      64  312 - #979daa
      92  312 - #7f8490
     352  312 - #afb5c4
      48  316 - #999eac
      40  348 - #3f698c
     116  348 - #0d2d4a
     168  348 - #527fa5
     188  348 - #6ea0c9
     268  348 - #3a6386
     276  348 - #5e8db4
     340  348 - #3f698c
     356  348 - #0d2d4a
     464  348 - #82b7e3
     476  348 - #82b8e4
     488  348 - #6ea0c9
     568  348 - #3a6386
     576  348 - #5e8db4
     104  404 - #0d2d4a
     320  404 - #0078d4
     408  404 - #0d2d4a
     476  480 - #11131a
     592  540 - #11131a
     208  552 - #11131a
       4  592 - #11131a
     408  592 - #11131a
";
