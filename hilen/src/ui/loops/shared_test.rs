//! `Shared loop pictures`: 10 marks with the same loop, a grid of 3 by 3
//! cells that light in turn, like the mark of each card in a list.
//!
//! Everywhere it pins the pixels of the frame path at a phase on stepped
//! time. Where a window has layers, macOS and the iOS and tvOS simulators,
//! it turns them on and checks that the 10 loops show 1 set of pictures
//! that was drawn once, that no step of the loops in the frames of that
//! draw takes long, that a new mark next to a live one draws nothing, and
//! that the pictures go when the last mark that shows them goes.

use anyhow::Result;

use crate::{
    self as hilen,
    deps::{hreads::from_main, refs::Weak},
    gm::{Clock, color::Color},
    ui::{
        Container, Label, LoopAnimation, Setup, TextAlignment, ViewData, ViewFrame, ViewSubviews, ViewTest,
        view,
    },
    ui_test::{check_colors, step_frames},
};

const MARKS: usize = 10;
const SIDE: f32 = 36.0;
const CELL: f32 = 10.0;
const PITCH: f32 = 13.0;
const GAP: f32 = 16.0;
/// The loop of the mark of banda, 145 pictures at 60 refreshes a second.
const PERIOD: f32 = 2.4;
const REST: Color = Color::hex("#bfdbfe");
const LIT: Color = Color::hex("#1d4ed8");
/// When each cell starts to light, as parts of the loop.
const STARTS: [f32; 9] = [0.0, 0.46, 0.17, 0.67, 0.29, 0.83, 0.08, 0.54, 0.38];
/// How long a cell is lit, as a part of the loop.
const LIT_FOR: f32 = 0.35;

/// Frames of 1/60 s on stepped time, a quarter of the loop.
const QUARTER: u32 = 36;

/// The 10 marks a quarter into the loop, the same in each.
const QUARTER_PROBES: &str = r"
     100   44 - #000000
      56   48 - #000001
     152   48 - #40596a
     192   48 - #3c5364
     128   52 - #597c95
     156  100 - #426ee1
     520  100 - #426ee1
      20  104 - #91b3f3
     100  104 - #426ee1
     236  104 - #91b3f3
     352  104 - #bfdbfe
     444  104 - #91b3f3
      52  132 - #bfdbfe
     132  132 - #6c92ea
     176  132 - #6c92ea
     300  132 - #bfdbfe
     412  132 - #bfdbfe
     488  132 - #6c92ea
      44  320 - #222f39
     172  320 - #435e71
     248  320 - #597c95
     288  320 - #314552
     332  320 - #597c95
      68  324 - #060809
     124  324 - #000000
     216  324 - #456174
     280  324 - #0f151a
     300  324 - #26353f
      96  328 - #2e404d
       4  592 - #597c95
     320  592 - #597c95
     592  592 - #597c95
";

#[view]
struct LoopMark {
    cells: Vec<Weak<Container>>,
}

impl Setup for LoopMark {
    fn setup(mut self: Weak<Self>) {
        for index in 0..9_u8 {
            let cell = self.add_view::<Container>();
            let row = f32::from(index / 3);
            let column = f32::from(index % 3);
            cell.set_corner_radius(2).set_frame((column * PITCH, row * PITCH, CELL, CELL));
            self.cells.push(cell);
        }
        self.pose(0.0);
    }
}

impl LoopMark {
    fn start(self: Weak<Self>) {
        self.run_loop(LoopAnimation::new(PERIOD, move |phase| self.pose(phase)));
    }

    fn pose(self: Weak<Self>, phase: f32) {
        for (cell, start) in self.cells.iter().zip(STARTS) {
            let turn = (phase - start).rem_euclid(1.0);
            let light = if turn < LIT_FOR { 1.0 - turn / LIT_FOR } else { 0.0 };
            let part = |rest: f32, lit: f32| rest + (lit - rest) * light;
            cell.set_color(Color::rgb(
                part(REST.r, LIT.r),
                part(REST.g, LIT.g),
                part(REST.b, LIT.b),
            ));
        }
    }
}

#[view]
struct SharedLoopPictures {
    marks: Vec<Weak<LoopMark>>,

    #[init]
    title:  Label,
    status: Label,
}

impl Setup for SharedLoopPictures {
    fn setup(self: Weak<Self>) {
        self.title
            .set_text("10 marks of 1 loop")
            .set_text_size(20)
            .set_alignment(TextAlignment::Left);
        self.title.set_frame((20, 30, 560, 40));
        self.status.set_text_size(20).set_alignment(TextAlignment::Left);
        self.status.set_frame((20, 300, 560, 40));

        for index in 0..MARKS {
            self.add_mark(index);
        }
        self.start_loops();
    }
}

impl SharedLoopPictures {
    fn add_mark(mut self: Weak<Self>, index: usize) -> Weak<LoopMark> {
        let mark = self.add_view::<LoopMark>();
        let index: u16 = u16::try_from(index).unwrap_or_default();
        mark.set_frame((20.0 + f32::from(index) * (SIDE + GAP), 100.0, SIDE, SIDE));
        self.marks.push(mark);
        mark
    }

    /// Every loop from phase 0 at the time of this frame.
    fn start_loops(self: Weak<Self>) {
        for mark in &self.marks {
            mark.start();
        }
    }

    fn say(self: Weak<Self>, text: &str) {
        let text = text.to_string();
        from_main(move || {
            self.status.set_text(text);
        });
    }
}

impl ViewTest for SharedLoopPictures {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        view.say("frame path, a quarter into the loop");
        from_main(move || {
            Clock::enter_stepped();
            view.start_loops();
        });
        step_frames(QUARTER);
        check_colors(QUARTER_PROBES)?;
        from_main(Clock::exit_stepped);

        #[cfg(apple)]
        layers::check(view)?;

        Ok(())
    }
}

/// The layers of the system, where a window has them.
#[cfg(apple)]
mod layers {
    use anyhow::{Result, bail, ensure};

    use super::{CELL, LIT, PITCH, REST, SharedLoopPictures};
    use crate::{
        deps::{hreads::from_main, refs::Weak},
        gm::LossyConvert,
        ui::{
            UIManager, View, ViewSubviews,
            loops::{
                flipbook::picture_draws,
                longest_step,
                on_layer::{first_picture_pixel, layer_state},
                set_test_layers,
            },
        },
        ui_test::{checkpoint, system_input::wait_until},
        window::Window,
    };

    /// The longest step of the loops, in milliseconds, in the frames that
    /// draw the pictures. A draw that waits for the GPU took 1 s in banda.
    const LONGEST_STEP_MS: f64 = 50.0;

    pub(super) fn check(view: Weak<SharedLoopPictures>) -> Result<()> {
        if Window::headless() {
            return Ok(());
        }

        view.say("layers on, 1 set of pictures for 10 marks");
        let before = from_main(|| {
            longest_step();
            set_test_layers(Some(true));
            picture_draws()
        });
        wait_for_layers(view)?;
        let draws = from_main(picture_draws) - before;
        let longest = from_main(longest_step) * 1000.0;
        log::info!("10 marks on layers: {draws} draws of pictures, the longest step {longest:.1} ms");
        ensure!(
            draws == 1,
            "10 loops of 1 content must draw their pictures once, drew {draws} times"
        );
        ensure!(
            longest < LONGEST_STEP_MS,
            "a step of the loops took {longest:.1} ms while the pictures were drawn"
        );

        let shown = from_main(move || {
            view.marks
                .iter()
                .map(|mark| layer_state(mark.weak_view()).map(|state| state.pictures_id))
                .collect::<Vec<_>>()
        });
        let Some(Some(first)) = shown.first().copied() else {
            bail!("the first mark has no layer");
        };
        ensure!(
            shown.iter().all(|id| *id == Some(first)),
            "every mark must show the same pictures, they show {shown:?}"
        );
        check_first_picture(view)?;
        checkpoint("10 marks on layers, 1 set of pictures drawn once")?;

        view.say("a new mark next to a live one draws nothing");
        let before = from_main(move || {
            let mut this = view;
            for mut mark in this.marks.drain(1..) {
                mark.remove_from_superview();
            }
            let mark = view.add_mark(1);
            mark.start();
            picture_draws()
        });
        wait_for_layers(view)?;
        let draws = from_main(picture_draws) - before;
        ensure!(
            draws == 0,
            "a mark next to a live one must draw nothing, drew {draws} times"
        );
        checkpoint("2 marks on layers, the new one drew nothing")?;

        view.say("the last mark goes, a new one draws again");
        let before = from_main(move || {
            let mut this = view;
            for mut mark in this.marks.drain(..) {
                mark.remove_from_superview();
            }
            picture_draws()
        });
        // The loops of the marks go in the next step, with their pictures.
        crate::deps::hreads::wait_for_next_frame();
        crate::deps::hreads::wait_for_next_frame();
        from_main(move || {
            let mark = view.add_mark(0);
            mark.start();
        });
        wait_for_layers(view)?;
        let draws = from_main(picture_draws) - before;
        ensure!(
            draws == 1,
            "the pictures must go with the last mark, a new mark drew {draws} times"
        );
        checkpoint("1 new mark on a layer, its pictures drawn again")?;

        from_main(|| set_test_layers(Some(false)));
        Ok(())
    }

    /// The picture of phase 0, drawn into a texture of its own size, has
    /// the first cell lit and the second at rest, at their places.
    fn check_first_picture(view: Weak<SharedLoopPictures>) -> Result<()> {
        let mark = view.marks[0].weak_view();
        let scale = from_main(UIManager::scale);
        // A picture starts 2 pixels of soft edge before the mark.
        let at = move |point: f32| -> u32 { (2.0 + point * scale).lossy_convert() };
        let cell = CELL / 2.0;
        for (name, x, color) in [("first", cell, LIT), ("second", PITCH + cell, REST)] {
            let pixel = from_main(move || first_picture_pixel(mark, at(x), at(cell)));
            let expected = [color.r, color.g, color.b].map(|channel| channel * 255.0);
            let Some(pixel) = pixel else {
                bail!("the first picture has no pixel at the {name} cell");
            };
            ensure!(
                pixel
                    .iter()
                    .zip(expected)
                    .all(|(pixel, expected)| (f32::from(*pixel) - expected).abs() <= 3.0),
                "the {name} cell in the first picture is {pixel:?}, it must be {expected:?}"
            );
        }
        Ok(())
    }

    fn wait_for_layers(view: Weak<SharedLoopPictures>) -> Result<()> {
        wait_until("every mark on a layer", move || {
            view.marks.iter().all(|mark| mark.__base_view().loop_layer)
        })
    }
}
