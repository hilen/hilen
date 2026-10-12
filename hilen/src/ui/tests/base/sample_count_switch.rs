use anyhow::{Result, bail, ensure};

use crate::{
    self as hilen, AppRunner,
    deps::{hreads::from_main, refs::Weak},
    gm::color::{BLUE, GREEN, LIGHT_GRAY, RED, WHITE},
    ui::{Container, DrawingView, Label, Setup, Shadow, ViewData, ViewFrame, ViewTest, view},
    ui_test::check_colors,
    window::{Screenshot, msaa_sample_count, set_msaa_sample_count},
};

/// How far a color channel may be off after the count went away and came
/// back. The brushes of the fonts are new then and put each glyph at
/// another place of their picture, which moves the edge of a glyph by 1
/// step of 255 in a few pixels.
const GLYPH_STEP: u8 = 2;

// An app sets the samples per pixel while it runs, like the smoothing
// setting of a game, see `set_msaa_sample_count`. Every pipeline, every
// font brush and the textures of the frame are built for 1 count, so a
// switch makes all of them again. One that is missed draws into a pass
// with another count, and wgpu ends the app there.
#[view]
struct SampleCountSwitch {
    #[init]
    title:  Label,
    card:   Container,
    text:   Label,
    ramp:   Container,
    path:   DrawingView,
    status: Label,
}

impl Setup for SampleCountSwitch {
    fn setup(mut self: Weak<Self>) {
        self.set_color(LIGHT_GRAY);

        self.title.set_text("sample count switch").set_text_size(22);
        self.title.set_frame((20, 10, 560, 34));

        self.card.set_color(WHITE).set_corner_radius(16).set_shadow(Shadow::default());
        self.card.set_frame((40, 80, 240, 140));

        self.text.set_text("a card, a text").set_text_size(18);
        self.text.set_frame((40, 130, 240, 40));

        self.ramp.set_gradient(BLUE, GREEN).set_corner_radius(10);
        self.ramp.set_frame((330, 80, 230, 140));

        // The only kind of drawing that the count changes the look of,
        // the slanted edges of a path.
        self.path.set_frame((40, 270, 520, 220));
        self.path.add_path([(0, 220), (260, 0), (520, 220)], RED);

        self.status.set_text_size(18);
        self.status.set_frame((20, 540, 560, 40));
    }
}

/// Switches to `count` and says so on the screen.
fn switch(view: Weak<SampleCountSwitch>, count: u32) {
    from_main(move || {
        set_msaa_sample_count(count);
        let samples = if count == 1 { "sample" } else { "samples" };
        view.status.set_text(format!("{count} {samples} per pixel"));
    });
}

fn switch_there_and_back(view: Weak<SampleCountSwitch>) -> Result<()> {
    switch(view, 4);
    let first = AppRunner::take_screenshot()?;
    ensure!(
        from_main(msaa_sample_count) == 4,
        "the count is 4 after it was set to 4"
    );
    check_colors(FOUR_SAMPLES)?;

    switch(view, 1);
    ensure!(
        from_main(msaa_sample_count) == 1,
        "the count is 1 after it was set to 1"
    );
    check_colors(ONE_SAMPLE)?;

    switch(view, 4);
    let again = AppRunner::take_screenshot()?;

    same_look(&first, &again)
}

fn same_look(first: &Screenshot, again: &Screenshot) -> Result<()> {
    ensure!(first.size == again.size, "the screen changed its size");

    let width = usize::try_from(first.size.width)?;
    let far = |a: u8, b: u8| a.abs_diff(b) > GLYPH_STEP;
    let differ = first
        .data
        .iter()
        .zip(&again.data)
        .position(|(a, b)| far(a.r, b.r) || far(a.g, b.g) || far(a.b, b.b) || far(a.a, b.a));

    let Some(index) = differ else {
        return Ok(());
    };

    bail!(
        "4 samples look different after 1 sample was on, the first pixel at {} {}: {:?} and {:?}",
        index % width,
        index / width,
        first.data[index],
        again.data[index]
    );
}

impl ViewTest for SampleCountSwitch {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        // The run has its own count, `HILEN_MSAA` picks it, and the tests
        // after this one are recorded for it.
        let of_the_run = from_main(msaa_sample_count);

        let result = switch_there_and_back(view);

        from_main(move || set_msaa_sample_count(of_the_run));

        result
    }
}

const FOUR_SAMPLES: &str = r"
             256   20 - #000000
             392   20 - #5c5c5c
             296   28 - #e7e7e7
             328   28 - #434343
             372   28 - #919191
             232   32 - #3e3e3e
             392   32 - #5c5c5c
             244   36 - #181818
              48   80 - #c9c9c9
             168   80 - #ffffff
             488   80 - #0001e6
             280  100 - #b1b1b1
             556  116 - #0042ab
             432  136 - #00678a
             340  140 - #006e83
             128  152 - #ffffff
             152  152 - #ffffff
             184  152 - #c1c1c1
             188  152 - #010101
             504  176 - #00b048
             388  196 - #00d427
             276  208 - #ffffff
             452  216 - #00f906
             160  220 - #9b9b9b
              44  224 - #cfcfcf
             592  312 - #e7e7e7
             352  376 - #ff0000
              44  488 - #ff0000
             344  560 - #e7e7e7
             368  560 - #858585
             264  564 - #010101
             592  592 - #e7e7e7
            ";

const ONE_SAMPLE: &str = r"
             256   20 - #000000
             392   20 - #5c5c5c
             368   24 - #353535
             220   28 - #7e7e7e
             328   28 - #434343
             392   32 - #5c5c5c
             244   36 - #181818
              48   80 - #c9c9c9
             488   80 - #0001e6
             280  104 - #b1b1b1
             556  116 - #0042ab
             440  136 - #00678a
             344  140 - #006e83
             128  152 - #ffffff
             152  152 - #ffffff
             184  152 - #c1c1c1
             188  152 - #000000
             208  152 - #747474
             508  176 - #00b048
             396  196 - #00d427
             276  208 - #ffffff
             456  216 - #00f906
             160  220 - #9b9b9b
              44  224 - #cfcfcf
             592  320 - #e7e7e7
              44  488 - #ff0000
             232  560 - #131313
             280  560 - #e7e7e7
             336  560 - #a9a9a9
             340  560 - #e7e7e7
             336  564 - #a9a9a9
             592  592 - #e7e7e7
            ";
