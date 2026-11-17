use std::fmt::Write;

use anyhow::{Result, bail, ensure};
use hilen::{
    gm::LossyConvert,
    refs::Weak,
    ui::{
        BLACK, BLUE, Color, Container, Label, RED, Setup, U8Color, ViewData, ViewFrame, ViewTest, WHITE, view,
    },
    ui_test::{capture_screenshot, check_colors, checkpoint},
    window::Screenshot,
};

/// The side of a card and the gap around it, in pixels of the canvas.
const CARD: f32 = 160.0;
const LEFT: [f32; 3] = [20.0, 220.0, 420.0];
const TOP_ROW: f32 = 60.0;
const BOTTOM_ROW: f32 = 320.0;

/// How far a channel may be from the computed color. The frame is 8 bit,
/// so a fade rounds once in the group image and once in the frame.
const TOLERANCE: i32 = 2;

/// Parts that lie over each other: a blue box over a red box over a white
/// card, and text over both boxes.
#[view]
struct GroupCard {
    #[init]
    red:   Container,
    blue:  Container,
    title: Label,
}

impl Setup for GroupCard {
    fn setup(self: Weak<Self>) {
        self.set_color(WHITE)
            .set_border_color(BLACK)
            .set_border_width(4)
            .set_corner_radius(16);

        self.red.set_frame((20, 20, 80, 80));
        self.red.set_color(RED);
        self.blue.set_frame((60, 60, 80, 80));
        self.blue.set_color(BLUE);
        self.title.set_frame((20, 50, 120, 40));
        self.title.set_text("group").set_text_color(BLACK).set_text_size(30);
    }
}

/// A holder with rounded corners that cuts what is inside it.
#[view]
struct RoundHolder {
    #[init]
    card: GroupCard,
}

impl Setup for RoundHolder {
    fn setup(self: Weak<Self>) {
        self.set_corner_radius(60);
        self.card.set_frame((0, 0, CARD, CARD));
        self.card.set_group_opacity(true).set_opacity(0.5);
    }

    fn clips_to_bounds(&self) -> bool {
        true
    }
}

/// A group that holds a group.
#[view]
struct OuterGroup {
    #[init]
    card: GroupCard,
}

impl Setup for OuterGroup {
    fn setup(self: Weak<Self>) {
        self.set_group_opacity(true).set_opacity(0.5);
        self.card.set_frame((0, 0, CARD, CARD));
        self.card.set_group_opacity(true).set_opacity(0.5);
    }
}

/// The same card 6 times over one flat color. Proves `set_group_opacity`
/// fades a view as one picture: a pixel of the grouped card is the pixel of
/// the full card mixed with the background, wherever parts overlap. The
/// plain fade next to it shows the red box through the blue one. Also a
/// group at opacity 1, a group inside a rounded clip and a group in a group.
#[view]
struct GroupOpacity {
    #[init]
    full:    GroupCard,
    plain:   GroupCard,
    grouped: GroupCard,
    solid:   GroupCard,
    round:   RoundHolder,
    nested:  OuterGroup,
    names:   Label,
}

impl Setup for GroupOpacity {
    fn setup(self: Weak<Self>) {
        self.set_color(Color::hex("#ffd24a"));

        self.full.set_frame((LEFT[0], TOP_ROW, CARD, CARD));
        self.plain.set_frame((LEFT[1], TOP_ROW, CARD, CARD));
        self.plain.set_opacity(0.5);
        self.grouped.set_frame((LEFT[2], TOP_ROW, CARD, CARD));
        self.grouped.set_group_opacity(true).set_opacity(0.5);

        self.solid.set_frame((LEFT[0], BOTTOM_ROW, CARD, CARD));
        self.solid.set_group_opacity(true);
        self.round.set_frame((LEFT[1], BOTTOM_ROW, CARD, CARD));
        self.nested.set_frame((LEFT[2], BOTTOM_ROW, CARD, CARD));

        self.names.set_frame((0, 520, 600, 60));
        self.names.set_text("full, plain fade, group fade").set_text_size(26);
    }
}

fn channels(color: U8Color) -> [i32; 3] {
    [i32::from(color.r), i32::from(color.g), i32::from(color.b)]
}

/// The color of `full` under `opacity` over `back`.
fn mixed(full: U8Color, back: U8Color, opacity: f32) -> [i32; 3] {
    let (full, back) = (channels(full), channels(back));
    [0, 1, 2].map(|at| {
        let mix = f64::from(full[at]) * f64::from(opacity) + f64::from(back[at]) * f64::from(1.0 - opacity);
        mix.round().lossy_convert()
    })
}

/// A pixel of a card that is off: its place in the card, its color and the
/// color it should have.
type OffPixel = (u16, u16, [i32; 3], [i32; 3]);

/// The pixel of the card at `origin` that is furthest from the full card
/// under `opacity`, when any is past the tolerance. `inside` picks the
/// pixels of the card that are looked at.
fn worst(
    shot: &Screenshot,
    origin: (f32, f32),
    opacity: f32,
    inside: impl Fn(u16, u16) -> bool,
) -> Option<OffPixel> {
    let back = shot.get_pixel((10.0, 10.0));
    let mut worst: Option<(i32, OffPixel)> = None;
    for y in 0..160_u16 {
        for x in 0..160_u16 {
            if !inside(x, y) {
                continue;
            }
            let (dx, dy) = (f32::from(x), f32::from(y));
            let full = shot.get_pixel((LEFT[0] + dx, TOP_ROW + dy));
            let got = channels(shot.get_pixel((origin.0 + dx, origin.1 + dy)));
            let want = mixed(full, back, opacity);
            let miss = (0..3).map(|at| (got[at] - want[at]).abs()).max().unwrap_or(0);
            if miss > TOLERANCE && worst.as_ref().is_none_or(|(most, _)| miss > *most) {
                worst = Some((miss, (x, y, got, want)));
            }
        }
    }
    worst.map(|(_, pixel)| pixel)
}

impl ViewTest for GroupOpacity {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        checkpoint(
            "the right card of the top row fades as one picture, the middle one shows red through blue",
        )?;

        let shot = capture_screenshot()?;
        let mut report = String::new();
        let whole = |_: u16, _: u16| true;

        // Every pixel of the grouped card, text and edges too.
        if let Some((x, y, got, want)) = worst(&shot, (LEFT[2], TOP_ROW), 0.5, whole) {
            writeln!(
                report,
                "group fade: pixel {x} {y} of the card is {got:?}, the mix is {want:?}"
            )?;
        }
        // A group with nothing to fade draws like the plain card.
        if let Some((x, y, got, want)) = worst(&shot, (LEFT[0], BOTTOM_ROW), 1.0, whole) {
            writeln!(
                report,
                "group at opacity 1: pixel {x} {y} is {got:?}, the full card is {want:?}"
            )?;
        }
        // A group in a group is one picture faded twice.
        if let Some((x, y, got, want)) = worst(&shot, (LEFT[2], BOTTOM_ROW), 0.25, whole) {
            writeln!(
                report,
                "group in a group: pixel {x} {y} is {got:?}, the mix is {want:?}"
            )?;
        }
        // Inside the rounded clip the middle of the card is the group fade,
        // and the corner of the card outside the arc is cut away.
        let middle = |x: u16, y: u16| (40..120).contains(&x) && (40..120).contains(&y);
        if let Some((x, y, got, want)) = worst(&shot, (LEFT[1], BOTTOM_ROW), 0.5, middle) {
            writeln!(
                report,
                "group in a rounded clip: pixel {x} {y} is {got:?}, the mix is {want:?}"
            )?;
        }
        if !report.is_empty() {
            bail!("{report}");
        }

        let back = channels(shot.get_pixel((10.0, 10.0)));
        let corner = channels(shot.get_pixel((LEFT[1] + 6.0, BOTTOM_ROW + 6.0)));
        ensure!(
            corner == back,
            "the corner of the group outside the rounded clip is {corner:?}, the background is {back:?}"
        );

        // The plain fade is what the group is for: the red box shows
        // through the blue one where they overlap.
        let overlap = channels(shot.get_pixel((LEFT[1] + 95.0, TOP_ROW + 95.0)));
        let alone = channels(shot.get_pixel((LEFT[1] + 125.0, TOP_ROW + 125.0)));
        ensure!(
            overlap != alone,
            "the plain fade shows nothing through the blue box, {overlap:?}"
        );
        let overlap = channels(shot.get_pixel((LEFT[2] + 95.0, TOP_ROW + 95.0)));
        let alone = channels(shot.get_pixel((LEFT[2] + 125.0, TOP_ROW + 125.0)));
        ensure!(
            overlap == alone,
            "the group fade shows the red box through the blue one, {overlap:?} against {alone:?}"
        );

        check_colors(CHECK)
    }
}

/// The recorded look of all 6 cards.
const CHECK: &str = r"
     572   64 - #806925
     176   68 - #000000
     316  124 - #7f3a9a
      96  128 - #0000e7
     280  132 - #512563
     332  132 - #8075c5
     532  132 - #806998
      80  136 - #00003e
     468  136 - #ff6925
     324  144 - #403a62
     156  196 - #0000e7
     304  196 - #8075c5
     556  196 - #806998
      24  212 - #000000
     424  212 - #806925
     172  324 - #000000
     328  324 - #ffe9a4
     576  328 - #bf9d37
     104  340 - #ff0000
     512  340 - #ff9d37
     276  388 - #e76925
     468  388 - #ff9d37
      68  396 - #ff0000
     472  404 - #bf9d37
     268  416 - #ff6925
     376  436 - #806925
     172  468 - #ffffff
     276  548 - #ffd24a
     372  548 - #32290f
     440  552 - #4f4117
     176  556 - #62511c
       4  592 - #ffd24a
";
