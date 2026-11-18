use anyhow::{Context, Result, ensure};
use hilen::{
    dispatch::from_main,
    gm::color::Color,
    refs::{Own, Weak},
    ui::{
        Button, Container, Label, NamedKey, Point, Popover, Setup, TextAlignment, View, ViewData, ViewFrame,
        ViewTest, view,
    },
    ui_test::{check_colors, inject_named_key, inject_touches},
};

const BAR: Color = Color::hex("#2B2D31");
const INK: Color = Color::hex("#20232A");

const ABOVE: &str = r"
             104   16 - #33404d
             144   16 - #20232a
             168   16 - #2f3a46
              48   20 - #4b677b
              60   20 - #597c95
              92   20 - #20232a
             112   20 - #597c95
             180   20 - #597c95
             336   52 - #cfd8e3
             240   68 - #000000
             592  204 - #597c95
             552  448 - #ffffff
             292  452 - #4a677b
             372  468 - #ffffff
             416  468 - #ffffff
             432  468 - #ffffff
             448  468 - #43464b
             408  524 - #1c1d1e
             424  528 - #cfd8e3
             432  528 - #cfd8e3
             368  536 - #cfd8e3
             292  540 - #4a677b
             552  552 - #ffffff
               4  568 - #597c95
             372  572 - #e4bdcd
             100  580 - #2b2d31
             436  580 - #e8c0d0
             392  588 - #e8c0d0
             412  588 - #e8c0d0
             444  588 - #000000
             196  592 - #2b2d31
             476  592 - #e8c0d0
";
const BELOW: &str = r"
              36   20 - #597c95
              48   20 - #4b677b
              96   20 - #2e3945
             132   20 - #597c95
             156   20 - #597c95
             336   52 - #cfd8e3
             240   64 - #000000
             144  108 - #ffffff
             216  120 - #ffffff
             232  120 - #20232a
             280  120 - #ffffff
             304  120 - #ffffff
             324  120 - #515359
             400  120 - #445f72
             264  176 - #cfd8e3
             276  176 - #cfd8e3
             256  184 - #0e0e0f
             328  192 - #cfd8e3
             140  212 - #456174
             388  212 - #ffffff
             204  216 - #415b6d
             592  332 - #597c95
               4  568 - #597c95
             476  572 - #e8c0d0
             592  572 - #2b2d31
             196  576 - #2b2d31
             436  580 - #e8c0d0
             388  588 - #2f272a
             392  588 - #e8c0d0
             412  588 - #e8c0d0
             444  588 - #000000
             460  592 - #e8c0d0
";

/// The content of the popover: a title and a button, so a tap inside has a
/// control to land on.
#[view]
struct Card {
    taps: usize,

    #[init]
    title:  Label,
    inside: Button,
}

impl Setup for Card {
    fn setup(self: Weak<Self>) {
        self.title
            .set_text("popover content")
            .set_text_size(16)
            .set_alignment(TextAlignment::Center)
            .set_text_color(INK);
        self.title.place().lrt(12).h(28);
        self.inside.set_text("Inside").set_text_size(14);
        self.inside.set_color(Color::hex("#CFD8E3")).set_corner_radius(6);
        self.inside.place().center_x().b(16).size(120, 32);
        self.inside.on_tap(move || {
            let mut card = self;
            card.taps += 1;
        });
    }
}

fn card() -> Own<dyn View> {
    let card = Card::new();
    card.set_size(260, 120);
    card
}

/// A popover opens over a button in a bar at the bottom of the window, like
/// the Format failed note of an editor, and under a button at the top. Its
/// arrow points at the middle of the button. A tap inside keeps it, a tap
/// outside and Escape close it.
#[view]
struct PopoverAnchor {
    closed: usize,

    #[init]
    status: Label,
    bar:    Container,
    note:   Button,
    top:    Button,
}

impl Setup for PopoverAnchor {
    fn setup(self: Weak<Self>) {
        self.status.set_text("nothing open").set_text_size(14).set_text_color(INK);
        self.status.place().t(8).l(16).size(560, 22);

        self.bar.set_color(BAR);
        self.bar.place().lrb(0).h(28);
        self.note.set_text("Format failed").set_text_size(13);
        self.note.set_color(Color::hex("#E8C0D0"));
        self.note.place().b(0).r(120).size(110, 28);
        self.note.on_tap(move || self.open(self.note));

        self.top.set_text("Top button").set_text_size(14);
        self.top.set_color(Color::hex("#CFD8E3"));
        self.top.place().t(50).l(200).size(140, 32);
        self.top.on_tap(move || self.open(self.top));
    }
}

impl PopoverAnchor {
    fn open(self: Weak<Self>, anchor: Weak<Button>) {
        let popover = Popover::show(card(), anchor);
        self.status.set_text(format!("popover of {}", anchor.text()));
        popover.closed.sub(move || {
            let mut this = self;
            this.closed += 1;
            this.status.set_text(format!("closed {} times", this.closed));
        });
    }

    fn inside_taps() -> usize {
        from_main(|| {
            let card = Popover::open().content().downcast::<Card>();
            card.map_or(0, |card| card.taps)
        })
    }
}

impl ViewTest for PopoverAnchor {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        inject_touches("425 586 b\n425 586 e");
        let (popover, note, sheet) = from_main(move || {
            let popover = Popover::open();
            (*popover.frame(), *view.note.frame(), popover.is_sheet())
        });
        ensure!(!sheet, "a medium window gets a popover, not a sheet");
        ensure!(
            Point::new(popover.center().x, popover.max_y()) == Point::new(note.center().x, note.y() - 4.0),
            "the popover ends 4 points over the middle of the note, {popover:?} {note:?}"
        );
        ensure!(
            popover.size == (260, 128).into(),
            "the card and the arrow, got {popover:?}"
        );
        check_colors(ABOVE)?;

        let inside = from_main(|| {
            let card = Popover::open().content().downcast::<Card>();
            card.map(|card| card.inside.absolute_frame().center())
        })
        .context("the popover holds the card")?;
        inject_touches(format!(
            "{} {} b\n{} {} e",
            inside.x, inside.y, inside.x, inside.y
        ));
        ensure!(
            Self::inside_taps() == 1,
            "the tap inside went to the button of the card"
        );
        ensure!(
            from_main(|| Popover::open().is_ok()),
            "a tap inside keeps it open"
        );

        inject_touches("300 300 b\n300 300 e");
        ensure!(from_main(|| Popover::open().is_null()), "a tap outside closes it");
        ensure!(from_main(move || view.closed) == 1, "closed fired once");

        inject_touches("270 66 b\n270 66 e");
        let (popover, top) = from_main(move || (*Popover::open().frame(), *view.top.frame()));
        ensure!(
            Point::new(popover.center().x, popover.y()) == Point::new(top.center().x, top.max_y() + 4.0),
            "with room below it opens 4 points under the middle of the button, {popover:?} {top:?}"
        );
        check_colors(BELOW)?;

        inject_named_key(NamedKey::Escape);
        ensure!(from_main(|| Popover::open().is_null()), "Escape closes it");
        ensure!(from_main(move || view.closed) == 2, "closed fired again");
        Ok(())
    }
}
