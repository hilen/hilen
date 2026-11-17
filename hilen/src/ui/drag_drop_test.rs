use std::{thread::sleep, time::Duration};

use anyhow::{Result, ensure};

use crate::{
    self as hilen,
    deps::{hreads::from_main, refs::Weak},
    gm::{LossyConvert, color::Color},
    ui::{
        Container, Drag, DragStart, DropEvent, Label, NamedKey, ScrollView, Setup, TextAlignment, UIManager,
        ViewData, ViewSubviews, ViewTest, ViewTouch, view,
    },
    ui_test::{check_colors, checkpoint, inject_mouse_touches, inject_named_key, inject_touches},
};

const CARD: f32 = 60.0;
const GAP: f32 = 10.0;
const COLUMN: Color = Color::hex("#3A3F4B");
const LIT: Color = Color::hex("#4F6B8F");
const CARD_COLOR: Color = Color::hex("#F2D29B");

/// 2 columns of cards, the right one in a scroll view, and a box that takes
/// only text. A card moves to a place in a column by a drag: a mouse lifts
/// it after a move, a finger after a hold, the column under it lights up, a
/// release drops it there, and a drop on nothing, on the text box or an
/// Escape puts it back. A quick press stays a tap of the card.
#[view]
struct DragDropTest {
    columns: [Vec<u32>; 2],
    taps:    u32,
    lit:     Option<usize>,
    ended:   Vec<bool>,
    texts:   u32,
    right:   Weak<Container>,

    #[init]
    left:     Container,
    scroll:   ScrollView,
    text_box: Label,
    status:   Label,
}

impl Setup for DragDropTest {
    fn setup(mut self: Weak<Self>) {
        self.set_color(Color::hex("#22252C"));
        self.columns = [vec![0, 1, 2], vec![]];
        self.left.place().tl(20).size(250, 400);
        self.scroll.place().t(20).l(320).size(250, 400);
        self.right = self.scroll.add_view::<Container>();
        self.right.place().t(0).lr(0).h(1000);
        self.text_box.set_text("text only").set_alignment(TextAlignment::Center);
        self.text_box.set_color(Color::hex("#6B5A7A"));
        self.text_box.place().t(440).l(20).size(250, 60);
        self.status.set_text_size(16).set_color(Color::hex("#E8E4D8"));
        self.status.place().t(440).l(320).size(260, 60);

        for column in 0..2 {
            let view = self.column_view(column);
            Drag::target::<_, u32>(view, move |event| self.column_event(column, event));
        }
        Drag::target::<_, String>(self.text_box, move |event| {
            if matches!(event, DropEvent::Drop(..)) {
                self.texts += 1;
            }
        });
        self.rebuild();
    }
}

impl DragDropTest {
    fn column_view(self: Weak<Self>, column: usize) -> Weak<Container> {
        if column == 0 { self.left } else { self.right }
    }

    fn column_event(mut self: Weak<Self>, column: usize, event: DropEvent<u32>) {
        match event {
            DropEvent::Enter(..) => self.lit = Some(column),
            DropEvent::Leave => self.lit = None,
            DropEvent::Over(..) => {}
            DropEvent::Drop(card, point) => {
                self.lit = None;
                for cards in &mut self.columns {
                    cards.retain(|c| *c != card);
                }
                let place: f32 = ((point.y - GAP) / (CARD + GAP)).round().max(0.0);
                let place: usize = place.lossy_convert();
                let cards = &mut self.columns[column];
                let place = place.min(cards.len());
                cards.insert(place, card);
            }
        }
        self.rebuild();
    }

    fn rebuild(self: Weak<Self>) {
        for column in 0..2 {
            let view = self.column_view(column);
            view.set_color(if self.lit == Some(column) { LIT } else { COLUMN });
            view.remove_all_subviews();
            for (place, card) in self.columns[column].iter().enumerate() {
                let place: f32 = place.lossy_convert();
                let label = view.add_view::<Label>();
                show_card(label, *card);
                label.place().l(GAP).r(GAP).t(GAP + place * (CARD + GAP)).h(CARD);
                label.enable_touch();
                let mut this = self;
                label.touch().up_inside.sub(label, move || {
                    this.taps += 1;
                    this.show_status();
                });
                let card = *card;
                Drag::source(label, move || {
                    let picture = Label::new();
                    show_card(picture.weak(), card);
                    let mut this = self;
                    Some(DragStart::new(card, picture).on_end(move |dropped| {
                        this.ended.push(dropped);
                        this.show_status();
                    }))
                });
            }
        }
        self.show_status();
    }

    fn show_status(self: Weak<Self>) {
        // A letter per ended drag, d for a drop and b for back.
        let ends: String = self.ended.iter().map(|dropped| if *dropped { 'd' } else { 'b' }).collect();
        let text = format!("taps {} ends {ends}", self.taps);
        self.status.set_text(text);
    }
}

fn show_card(label: Weak<Label>, card: u32) {
    label.set_text(format!("card {card}"));
    label.set_alignment(TextAlignment::Center);
    label.set_color(CARD_COLOR);
}

type State = ([Vec<u32>; 2], u32, Option<usize>, Vec<bool>);

fn state(view: Weak<DragDropTest>) -> State {
    from_main(move || (view.columns.clone(), view.taps, view.lit, view.ended.clone()))
}

fn touches(lines: &str) {
    inject_touches(lines.to_string());
}

impl ViewTest for DragDropTest {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        from_main(|| UIManager::set_drag_scrolling(false));

        touches("145 60 b\n145 60 e");
        ensure!(
            state(view) == ([vec![0, 1, 2], vec![]], 1, None, vec![]),
            "a quick press is a tap of the card, got {:?}",
            state(view)
        );

        touches("145 130 b\n145 145 m\n400 100 m");
        ensure!(
            from_main(Drag::is_dragging),
            "a mouse move of 15 points lifts card 1"
        );
        ensure!(state(view).2 == Some(1), "the right column is lit under the drag");
        check_colors(HELD)?;
        checkpoint("card 1 held over the right column, the column is lit")?;

        touches("145 470 m");
        ensure!(
            state(view).2.is_none(),
            "the text box takes no number, nothing is lit"
        );
        touches("400 60 m\n400 60 e");
        let after_drop = state(view);
        ensure!(
            after_drop == ([vec![0, 2], vec![1]], 1, None, vec![true]),
            "card 1 dropped into the right column with no tap, got {after_drop:?}"
        );
        ensure!(from_main(move || view.texts) == 0, "the text box got no drop");

        touches("145 130 b\n145 145 m\n300 560 m\n300 560 e");
        ensure!(
            state(view) == ([vec![0, 2], vec![1]], 1, None, vec![true, false]),
            "a drop on nothing puts card 2 back, got {:?}",
            state(view)
        );

        touches("145 60 b\n145 75 m\n400 200 m");
        ensure!(state(view).2 == Some(1), "card 0 held over the right column");
        inject_named_key(NamedKey::Escape);
        touches("400 200 e");
        ensure!(
            state(view) == ([vec![0, 2], vec![1]], 1, None, vec![true, false, false]),
            "Escape puts card 0 back and the release is no tap, got {:?}",
            state(view)
        );
        ensure!(!from_main(Drag::is_dragging), "no drag after Escape");
        checkpoint("card 1 in the right column, a drop on nothing and Escape put the others back")?;

        from_main(|| UIManager::set_drag_scrolling(true));
        touches("145 60 b\n145 110 m\n145 110 e");
        ensure!(
            !from_main(Drag::is_dragging),
            "a finger that moves before its hold lifts nothing"
        );

        touches("145 60 b");
        sleep(Duration::from_millis(600));
        ensure!(
            from_main(Drag::is_dragging),
            "a finger held still for 0.6 seconds lifts card 0"
        );
        touches("400 400 m");
        sleep(Duration::from_millis(400));
        let offset = from_main(move || view.scroll.get_scroll_content_offset());
        ensure!(
            offset < -5.0,
            "a drag at the bottom edge scrolls the column, its offset is {offset}"
        );
        touches("400 400 e");
        let end = state(view);
        ensure!(
            end == ([vec![2], vec![1, 0]], 1, None, vec![true, false, false, true]),
            "card 0 dropped at the end of the scrolled column, got {end:?}"
        );
        // How far the column scrolled follows the frame times, the picture is
        // checked at the top.
        from_main(move || {
            view.scroll.set_content_offset(0);
        });
        check_colors(DROPPED)?;
        checkpoint("a finger hold lifted card 0, the column scrolled, card 0 dropped last")?;

        // A browser scrolls the page on a drag, but a mouse still lifts a
        // card at once, only a finger waits for the hold.
        inject_mouse_touches("145 60 b\n145 75 m");
        ensure!(
            from_main(Drag::is_dragging),
            "a mouse move of 15 points lifts card 2 where a drag scrolls"
        );
        inject_mouse_touches("400 100 m\n400 100 e");
        let mouse = state(view);
        ensure!(
            mouse
                == (
                    [vec![], vec![1, 2, 0]],
                    1,
                    None,
                    vec![true, false, false, true, true]
                ),
            "card 2 dropped between card 1 and card 0, got {mouse:?}"
        );
        checkpoint("a mouse lifted card 2 with no hold where a drag scrolls")
    }
}

/// Card 1 held over the lit right column.
const HELD: &str = r"
             592    4 - #22252c
              24   20 - #3a3f4b
             252   32 - #f2d29b
             104   60 - #000000
             160   68 - #000000
             516   72 - #3f5672
             412  100 - #f2d29b
             364  104 - #f2d29b
             180  128 - #010101
             304  128 - #f2d29b
             104  136 - #000000
             556  156 - #4f6b8f
             180  188 - #000000
             112  204 - #f2d29b
             256  228 - #f2d29b
             472  252 - #4f6b8f
             348  280 - #4f6b8f
             592  304 - #22252c
             152  328 - #3a3f4b
              20  336 - #3a3f4b
             220  440 - #6b5a7a
             484  440 - #e8e4d8
             156  468 - #6b5a7a
             376  468 - #302f2d
             124  472 - #6b5a7a
             340  472 - #e8e4d8
             376  472 - #302f2d
             388  472 - #e8e4d8
             104  476 - #6b5a7a
             448  496 - #e8e4d8
             568  496 - #e8e4d8
               4  592 - #22252c
";

/// The cards at their new places, the column at its top.
const DROPPED: &str = r"
             260    4 - #22252c
              32   32 - #f2d29b
             556   32 - #f2d29b
             160   48 - #000000
             460   52 - #000000
             108   64 - #f2d29b
             408   64 - #f2d29b
             480   68 - #010101
             204   88 - #f2d29b
             452  128 - #f2d29b
             404  136 - #000000
             480  140 - #010101
             236  272 - #3a3f4b
              20  284 - #3a3f4b
             592  284 - #22252c
             424  304 - #3a3f4b
             248  440 - #6b5a7a
             496  440 - #e8e4d8
             576  440 - #e8e4d8
              20  444 - #6b5a7a
             376  468 - #302f2d
              92  472 - #6b5a7a
             124  472 - #6b5a7a
             200  472 - #000000
             340  472 - #e8e4d8
             376  472 - #302f2d
             388  472 - #e8e4d8
             444  472 - #e8e4d8
             156  476 - #6b5a7a
             268  496 - #6b5a7a
             520  496 - #e8e4d8
               4  592 - #22252c
";
