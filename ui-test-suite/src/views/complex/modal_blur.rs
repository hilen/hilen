use anyhow::Result;
use test_engine::{
    OnceEvent,
    dispatch::from_main,
    refs::Weak,
    ui::{
        BLACK, Container, GREEN, ImageView, Label, ModalView, RED, Setup, Shadow, Size, UIColor, ViewData,
        ViewTest, WHITE, view,
    },
    ui_test::{check_colors, set_record_probe_count},
};

// A real looking dialog over a frosted backdrop: the modal blurs the
// whole scene behind it and dims it with the scrim tint, while the
// dialog itself stays crisp.
#[view]
struct BlurModal {
    event: OnceEvent,

    #[init]
    title: Label,
    yes:   Label,
    no:    Label,
}

impl Setup for BlurModal {
    fn setup(self: Weak<Self>) {
        self.set_color(WHITE);
        self.set_corner_radius(16);
        self.set_shadow(Shadow::default());

        self.title.set_text("Blurred behind?");
        self.title.set_text_size(28);
        self.title.place().lrt(12).h(60);

        self.yes.set_text("Yes");
        self.yes.set_color(GREEN);
        self.yes.set_corner_radius(8);
        self.yes.place().size(100, 36).b(16).l(20);

        self.no.set_text("No");
        self.no.set_color(RED);
        self.no.set_corner_radius(8);
        self.no.place().size(100, 36).b(16).r(20);
    }
}

impl ModalView for BlurModal {
    fn modal_event(&self) -> &OnceEvent<()> {
        &self.event
    }

    fn modal_size() -> Size {
        (260, 160).into()
    }

    fn modal_scrim_color() -> UIColor {
        BLACK.with_alpha(0.4).into()
    }

    fn modal_blur() -> f32 {
        25.0
    }
}

// The same busy background as the scrim test: strips, a photo and
// text, so the blur and tint have every pipeline behind them.
#[view]
struct ModalBlurTest {
    #[init]
    white: Container,
    red:   Container,
    photo: ImageView,
    text:  Label,
}

impl Setup for ModalBlurTest {
    fn setup(self: Weak<Self>) {
        self.white.set_color(WHITE);
        self.white.place().tl(0).size(200, 600);

        self.red.set_color(RED);
        self.red.place().t(0).l(200).size(120, 600);

        self.photo.set_image("cat.png");
        self.photo.place().t(40).l(340).size(220, 160);

        self.text.set_text("Behind the modal");
        self.text.set_text_size(40);
        self.text.place().t(480).l(220).size(360, 60);
    }
}

fn check_blurred() -> Result<()> {
    check_colors(
        r"
               4    4 - #999999
             288    4 - #8f0809
             592    4 - #364a59
             180   12 - #997878
             488   44 - #615b60
             324   76 - #76353a
             400  112 - #806c64
             212  116 - #992f2f
             308  156 - #811b1e
             548  164 - #54504e
             456  188 - #635b58
             208  252 - #ffffff
             224  252 - #d6d6d6
             288  252 - #000000
             392  256 - #ffffff
             268  260 - #ffffff
             300  260 - #cecece
             332  260 - #252525
             224  264 - #d6d6d6
             252  264 - #d1d1d1
             356  264 - #9a9a9a
             308  268 - #000000
             376  268 - #010101
             428  312 - #ffffff
             172  324 - #ffffff
             276  328 - #00ff00
             220  332 - #00ff00
             232  332 - #000000
             312  332 - #ff0000
             344  332 - #000000
             352  336 - #ff0000
             408  344 - #ff0000
             372  348 - #ff0000
             228  352 - #00ff00
             248  352 - #00ff00
             348  352 - #ff0000
             200  360 - #00ff00
             260  380 - #660101
             292  380 - #5e0708
             328  380 - #3d2026
             420  380 - #263540
             196  460 - #995555
             324  480 - #58272f
             208  556 - #95393a
               4  580 - #83888b
             268  592 - #721e24
             336  592 - #453f4b
             592  592 - #354a59
        ",
    )
}

fn check_restored() -> Result<()> {
    check_colors(
        r"
               4    4 - #ffffff
             280    4 - #ff0000
             556   40 - #d7a9ab
             340   44 - #ecc7ce
             408   68 - #cb8c7a
             528   88 - #ae7b68
             540   88 - #946450
             504   92 - #a27d66
             448   96 - #d4bea3
             468  100 - #c4a890
             524  100 - #8d5443
             408  104 - #ceb29a
             436  104 - #2a1406
             200  124 - #ff0000
             480  124 - #9e7f63
             428  128 - #c0907c
             428  136 - #e0b9b4
             516  152 - #856953
             556  156 - #a28671
             340  176 - #e5b2b1
             448  180 - #c79d87
             532  188 - #a08069
             504  192 - #8c705a
             404  196 - #ddb7ac
             320  304 - #597c95
               4  344 - #ffffff
             592  348 - #597c95
             256  496 - #ff0000
             264  496 - #000000
             552  496 - #030405
             388  500 - #000000
             252  504 - #3d0000
             452  504 - #000000
             280  508 - #ac0000
             284  508 - #ac0000
             340  508 - #283843
             360  508 - #597c95
             420  508 - #3c5465
             428  508 - #3c5465
             508  508 - #597c95
             288  512 - #ff0000
             340  512 - #283843
             260  516 - #ff0000
             312  516 - #1c0000
             340  516 - #283843
             476  516 - #000000
             536  516 - #597c95
             328  520 - #12191e
        ",
    )
}

impl ViewTest for ModalBlurTest {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        set_record_probe_count(48);

        let modal = from_main(BlurModal::prepare_modally);

        check_blurred()?;

        modal.hide_modal(());

        check_restored()?;

        Ok(())
    }
}
