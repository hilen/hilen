use anyhow::{Result, ensure};
use hilen::{
    OnceEvent,
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{
        BLACK, Button, Label, ModalView, Setup, Size, TextAlignment, UIColor, ViewData, ViewFrame, ViewTest,
        WHITE, view,
    },
    ui_test::{check_colors, inject_touches},
};

const SMALL: (f32, f32) = (320.0, 160.0);
const LARGE: (f32, f32) = (420.0, 320.0);

/// A form that grows when it shows more fields. `set_modal_size` gives the
/// open modal another size and keeps it centered.
#[view]
struct ResizingForm {
    event: OnceEvent,
    large: bool,

    #[init]
    title:  Label,
    extra:  Label,
    toggle: Button,
}

impl Setup for ResizingForm {
    fn setup(mut self: Weak<Self>) {
        self.set_color(WHITE).set_corner_radius(12);

        self.title.set_text_size(20);
        self.title.set_alignment(TextAlignment::Left);
        self.title.place().lrt(16).h(40);

        self.extra.set_text("extra rows, only in the large size").set_text_size(18);
        self.extra.set_alignment(TextAlignment::Left);
        self.extra.place().lr(16).t(90).h(40);

        self.toggle.set_text_size(20);
        self.toggle.place().br(16).size(140, 40);
        self.toggle.on_tap(move || {
            self.large = !self.large;
            self.refresh();
        });

        self.refresh();
    }
}

impl ResizingForm {
    fn refresh(self: Weak<Self>) {
        let (size, name, action) = if self.large {
            (LARGE, "large", "Shrink")
        } else {
            (SMALL, "small", "Grow")
        };
        self.title.set_text(format!("modal, {name}, {} by {}", size.0, size.1));
        self.toggle.set_text(action);
        self.extra.set_hidden(!self.large);
        self.set_modal_size(size);
    }
}

impl ModalView for ResizingForm {
    fn modal_event(&self) -> &OnceEvent<()> {
        &self.event
    }

    fn modal_size() -> Size {
        SMALL.into()
    }

    fn modal_scrim_color() -> UIColor {
        BLACK.with_alpha(0.3).into()
    }
}

#[view]
struct ModalResize {}

impl Setup for ModalResize {
    fn setup(self: Weak<Self>) {}
}

/// The frame of the form as x, y, width and height.
fn frame(form: Weak<ResizingForm>) -> (f32, f32, f32, f32) {
    from_main(move || {
        let frame = form.absolute_frame();
        (frame.x(), frame.y(), frame.width(), frame.height())
    })
}

impl ViewTest for ModalResize {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        let form = from_main(ResizingForm::prepare_modally);
        wait_for_next_frame();
        ensure!(
            frame(form) == (140.0, 220.0, 320.0, 160.0),
            "the small form: {:?}",
            frame(form)
        );
        // the form opens small and centered
        check_colors(CHECK_1)?;

        // The Grow button, 16 points in from the bottom right corner.
        inject_touches("374 344 b\n374 344 e");
        wait_for_next_frame();
        ensure!(
            frame(form) == (90.0, 140.0, 420.0, 320.0),
            "the large form: {:?}",
            frame(form)
        );
        // Grow made the form large, still centered, with the extra row
        check_colors(CHECK_2)?;

        inject_touches("424 424 b\n424 424 e");
        wait_for_next_frame();
        ensure!(
            frame(form) == (140.0, 220.0, 320.0, 160.0),
            "the form after Shrink: {:?}",
            frame(form)
        );
        // Shrink made the form small again
        check_colors(CHECK_3)?;

        from_main(move || form.hide_modal(()));
        wait_for_next_frame();
        Ok(())
    }
}

const CHECK_1: &str = r"
       4    4 - #3e5768
     340    4 - #3e5768
     592    4 - #3e5768
     452  224 - #ffffff
     224  252 - #9f9f9f
     300  252 - #ffffff
     364  252 - #000000
     180  256 - #000000
     196  256 - #ffffff
     224  256 - #9f9f9f
     248  256 - #8f8f8f
     320  256 - #ffffff
     340  256 - #ffffff
     372  256 - #000000
     388  256 - #ffffff
     188  260 - #d1d1d1
     224  260 - #9f9f9f
     248  260 - #8f8f8f
     256  260 - #222222
     272  260 - #000000
     312  260 - #ffffff
     364  260 - #000000
     456  292 - #ffffff
     204  328 - #ffffff
     356  344 - #ffffff
     380  344 - #000000
     456  372 - #ffffff
     148  376 - #ffffff
     256  376 - #ffffff
       4  592 - #3e5768
     352  592 - #3e5768
     592  592 - #3e5768
";

const CHECK_2: &str = r"
     592    4 - #3e5768
     160  172 - #3f3f3f
     176  172 - #8f8f8f
     124  176 - #000000
     204  176 - #000000
     248  176 - #4b4b4b
     264  176 - #000000
     328  176 - #000000
     136  180 - #5e5e5e
     160  180 - #3f3f3f
     176  180 - #8f8f8f
     188  180 - #6c6c6c
     280  180 - #000000
     508  224 - #ffffff
     276  244 - #d7d7d7
     300  244 - #dfdfdf
     224  248 - #d7d7d7
     256  248 - #8a8a8a
     360  248 - #f7f7f7
     144  252 - #ffffff
     160  252 - #6d6d6d
     176  252 - #ffffff
     232  252 - #aeaeae
     284  252 - #aeaeae
     312  252 - #a0a0a0
     324  252 - #ffffff
     288  388 - #ffffff
     444  420 - #1c1c1c
     420  428 - #616161
     172  456 - #ffffff
       4  592 - #3e5768
     592  592 - #3e5768
";

const CHECK_3: &str = r"
       4    4 - #3e5768
     340    4 - #3e5768
     592    4 - #3e5768
     452  224 - #ffffff
     224  252 - #9f9f9f
     300  252 - #ffffff
     364  252 - #000000
     180  256 - #000000
     196  256 - #ffffff
     224  256 - #9f9f9f
     248  256 - #8f8f8f
     320  256 - #ffffff
     340  256 - #ffffff
     372  256 - #000000
     388  256 - #ffffff
     188  260 - #d1d1d1
     224  260 - #9f9f9f
     248  260 - #8f8f8f
     256  260 - #222222
     272  260 - #000000
     312  260 - #ffffff
     364  260 - #000000
     456  292 - #ffffff
     204  328 - #ffffff
     356  344 - #ffffff
     380  344 - #000000
     456  372 - #ffffff
     148  376 - #ffffff
     256  376 - #ffffff
       4  592 - #3e5768
     352  592 - #3e5768
     592  592 - #3e5768
";
