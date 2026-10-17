use anyhow::{Result, bail, ensure};

use crate::{
    self as hilen, AppRunner,
    deps::{hreads::from_main, refs::Weak},
    gm::color::{BLACK, BLUE, GREEN, LIGHT_GRAY, ORANGE, RED, WHITE},
    ui::{
        Container, Label, Setup, Shadow, TextAlignment, ViewData, ViewFrame, ViewSubviews, ViewTest,
        damage::{self, Frames, WHY_IPHONE_SAMPLES, WHY_NO_KEPT_FRAME, WHY_OFF},
        view,
    },
    ui_test::check_colors,
    window::Screenshot,
};

const ROWS: usize = 5;

/// A partial frame may draw this share of the window again, 1 part in so
/// many. Every step of the test changes a small part of the canvas.
const SMALL_PART: u64 = 10;

const MARK_ON_THE_SHADOW: &str = r"
             592    4 - #e7e7e7
             292   24 - #bfbfbf
             336   32 - #555555
             476   68 - #cccccc
              96   72 - #383838
             316  132 - #a2a2e7
             124  156 - #a2a2e7
             224  156 - #676793
             568  164 - #ffffff
             408  180 - #00ff00
             476  180 - #9b9b9b
              92  196 - #696969
             188  196 - #a2a2e7
             536  232 - #000ddb
              48  236 - #e7e7e7
             148  240 - #020202
             232  240 - #020202
             496  244 - #004aa4
             420  252 - #00737f
             568  260 - #009c5a
             460  268 - #00c435
             488  272 - #00d923
             528  276 - #00ed10
              20  328 - #000000
              64  332 - #ffcb00
             140  360 - #ffffff
             216  360 - #919191
             156  556 - #7d7d7d
             444  560 - #e7e7e7
             212  564 - #212121
             280  564 - #424242
             376  564 - #c2c2c2
            ";

const MARK_BACK_AND_BIGGER: &str = r"
             336   32 - #555555
              96   72 - #383838
             540   72 - #ffffff
             228   76 - #010101
              64  108 - #00ff00
             152  116 - #171717
             124  156 - #a2a2e7
             172  156 - #a2a2e7
             224  156 - #676793
             560  180 - #9e9e9e
             316  188 - #a2a2e7
             460  188 - #d2d2d2
             140  196 - #4d4d6e
             160  228 - #e7e7e7
             424  232 - #000ddb
              92  236 - #696969
             232  240 - #020202
             516  244 - #004aa4
             464  252 - #00737f
             552  260 - #009c5a
             492  268 - #00c435
             524  272 - #00d923
             592  272 - #e7e7e7
             416  276 - #00ed10
             140  360 - #ffffff
             216  360 - #919191
              20  392 - #000000
             276  400 - #ffcb00
             268  408 - #ffcb00
             388  560 - #e7e7e7
             228  564 - #d0d0d0
             336  564 - #ababab
            ";

const NO_SHADOW: &str = r"
             308   24 - #787878
             324   28 - #959595
             292   32 - #bfbfbf
              76   72 - #5b5b5b
             500   72 - #ffffff
             228   76 - #010101
             112  116 - #cccccc
             152  116 - #171717
              48  156 - #e7e7e7
             124  156 - #5c5ce7
             224  156 - #3a3a93
             316  188 - #5c5ce7
             140  196 - #2c2c6e
             188  196 - #5c5ce7
              96  232 - #383838
             556  232 - #000ddb
             232  240 - #020202
             448  244 - #004aa4
             592  248 - #e7e7e7
             520  252 - #00737f
             412  260 - #009c5a
             480  268 - #00c435
             556  276 - #00ed10
             140  360 - #ffffff
             216  360 - #919191
              20  392 - #000000
             276  400 - #ffcb00
             268  408 - #ffcb00
             268  556 - #7b7b7b
             208  560 - #c5c5c5
             376  560 - #e7e7e7
             348  564 - #b5b5b5
            ";

/// A box that cuts what is inside it at its rounded outline.
#[view]
struct ClipBox {}

impl Setup for ClipBox {
    fn clips_to_bounds(&self) -> bool {
        true
    }
}

// A frame draws only the pixels that changed since the frame before, see
// `ui::damage`. Every step here changes a small part of the fixture and
// then compares the screen with a whole frame of the same state, pixel
// for pixel. A value the drawer reads and the paint key leaves out, or a
// region that is too small, shows as a pixel that differs.
#[view]
struct PartialFrames {
    rows:      Vec<Weak<Label>>,
    card_text: Weak<Label>,
    clip_text: Weak<Label>,
    dot:       Weak<Container>,

    #[init]
    title:  Label,
    card:   Container,
    ramp:   Container,
    glass:  Container,
    clip:   ClipBox,
    mark:   Container,
    status: Label,
}

impl Setup for PartialFrames {
    fn setup(mut self: Weak<Self>) {
        self.set_color(LIGHT_GRAY);

        self.title.set_text("partial frames").set_text_size(22);
        self.title.set_frame((20, 10, 560, 34));

        for row in 0..ROWS {
            let label = self.add_view::<Label>();
            label
                .set_text(format!("row {}: the quick brown fox", row + 1))
                .set_text_size(18);
            label.set_alignment(TextAlignment::Left);
            label.set_frame((
                20.0,
                60.0 + 40.0 * f32::from(u8::try_from(row).unwrap_or_default()),
                360.0,
                30.0,
            ));
            self.rows.push(label);
        }

        self.card.set_color(WHITE).set_corner_radius(12).set_shadow(Shadow::default());
        self.card.set_frame((410, 70, 160, 110));
        self.card_text = self.card.add_view::<Label>();
        self.card_text.set_text("card, a shadow").set_text_size(14);
        self.card_text.set_frame((0, 40, 160, 30));

        self.ramp.set_gradient(BLUE, GREEN).set_corner_radius(8);
        self.ramp.set_frame((410, 230, 160, 50));

        self.glass.set_color(BLUE.with_alpha(0.3));
        self.glass.set_frame((120, 130, 200, 90));

        self.clip
            .set_color(WHITE)
            .set_corner_radius(24)
            .set_border_color(BLACK)
            .set_border_width(2);
        self.clip.set_frame((20, 300, 260, 120));
        self.clip_text = self.clip.add_view::<Label>();
        self.clip_text.set_text("a clip, a dot in it").set_text_size(14);
        self.clip_text.set_frame((60, 45, 190, 30));
        self.dot = self.clip.add_view::<Container>();
        self.dot.set_color(ORANGE).set_corner_radius(15);
        self.dot.set_frame((16, 16, 30, 30));

        self.mark.set_color(RED).set_corner_radius(4);
        self.mark.set_frame((60, 65, 20, 20));

        self.status.set_text("nothing changed yet").set_text_size(18);
        self.status.set_frame((20, 540, 560, 40));
    }
}

/// Whether this run can draw partial frames at all. A surface with 1
/// sample keeps no frame, an iPhone with several samples loses it, and
/// `HILEN_DAMAGE=0` switches them off.
pub(super) fn partial_frames_possible(frames: Frames) -> bool {
    ![WHY_NO_KEPT_FRAME, WHY_IPHONE_SAMPLES, WHY_OFF].contains(&frames.why)
}

pub(super) fn same_pixels(partial: &Screenshot, whole: &Screenshot, step: &str) -> Result<()> {
    ensure!(
        partial.size == whole.size,
        "{step}: the 2 screens differ in size, {:?} and {:?}",
        partial.size,
        whole.size
    );

    let width = usize::try_from(partial.size.width)?;
    let Some(index) = partial.data.iter().zip(&whole.data).position(|(a, b)| a != b) else {
        return Ok(());
    };
    let differ = partial.data.iter().zip(&whole.data).filter(|(a, b)| a != b).count();

    bail!(
        "{step}: {differ} pixels of the 2 screens differ, the first at {} {}: {:?} and {:?}",
        index % width,
        index / width,
        partial.data[index],
        whole.data[index]
    );
}

/// Makes a change, takes the screen that the partial frames gave, then
/// has the same state drawn whole and compares the 2.
fn step(view: Weak<PartialFrames>, name: &'static str, change: fn(Weak<PartialFrames>)) -> Result<()> {
    let before = from_main(move || {
        change(view);
        view.status.set_text(name);
        damage::frames()
    });

    let partial = AppRunner::take_screenshot()?;
    let after = from_main(damage::frames);

    if partial_frames_possible(after) {
        ensure!(
            after.whole == before.whole,
            "{name}: a whole frame was drawn for a small change: {}",
            after.why
        );
        ensure!(after.partial > before.partial, "{name}: no frame was drawn");

        let window = u64::from(partial.size.width) * u64::from(partial.size.height);
        let drawn = after.pixels - before.pixels;
        ensure!(
            drawn * SMALL_PART < window,
            "{name}: {drawn} pixels of {window} were drawn again for a small change"
        );
    }

    from_main(|| damage::whole_next("the test compares with a whole frame"));
    let whole = AppRunner::take_screenshot()?;
    let end = from_main(damage::frames);
    ensure!(
        end.whole > after.whole,
        "{name}: the frame to compare with was not whole"
    );

    same_pixels(&partial, &whole, name)
}

impl ViewTest for PartialFrames {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        // The first frames of a fixture are whole, its views were never
        // drawn. 2 screens later every view is known.
        AppRunner::take_screenshot()?;
        AppRunner::take_screenshot()?;

        step(view, "the mark is green", |view| {
            view.mark.set_color(GREEN);
        })?;
        step(view, "the mark is on the glass", |view| {
            view.mark.set_frame((200, 150, 20, 20));
        })?;
        step(view, "the mark is on the shadow of the card", |view| {
            view.mark.set_frame((400, 172, 20, 20));
        })?;
        check_colors(MARK_ON_THE_SHADOW)?;

        step(view, "row 2 has a new text", |view| {
            view.rows[1].set_text("row 2: a new text");
        })?;
        step(view, "the clip cuts the dot", |view| {
            view.dot.set_frame((236, 96, 30, 30));
        })?;
        step(view, "the mark is hidden", |view| {
            view.mark.set_hidden(true);
        })?;
        step(view, "the mark is back and bigger", |view| {
            view.mark.set_hidden(false);
            view.mark.set_frame((60, 100, 40, 40));
        })?;
        check_colors(MARK_BACK_AND_BIGGER)?;

        step(view, "the mark is gone", |mut view| {
            view.mark.remove_from_superview();
        })?;
        step(view, "the glass is more solid", |view| {
            view.glass.set_color(BLUE.with_alpha(0.6));
        })?;
        step(view, "the card has no shadow", |view| {
            view.card.set_shadow(None);
        })?;
        check_colors(NO_SHADOW)?;

        // A view that was never drawn can draw into the pass by itself, the
        // engine only learns that in a whole frame.
        let frames = from_main(move || {
            let new = view.add_view::<Container>();
            new.set_color(RED);
            new.set_frame((300, 460, 40, 40));
            view.status.set_text("a new view, drawn in a whole frame");
            damage::frames()
        });
        AppRunner::take_screenshot()?;
        let after = from_main(damage::frames);
        ensure!(
            after.whole > frames.whole,
            "a new view must be drawn in a whole frame"
        );

        Ok(())
    }
}
