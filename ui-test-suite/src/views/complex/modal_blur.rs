use anyhow::Result;
use hilen::{
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
             284    4 - #910607
             180   20 - #997878
             488   44 - #615b60
             324   76 - #76353a
             404   92 - #7e6963
             212  120 - #992f2f
             548  120 - #5d5253
             308  160 - #811b1e
             492  188 - #5a5450
             288  252 - #000000
             208  256 - #b1b1b1
             388  256 - #ffffff
             256  260 - #000000
             208  264 - #b1b1b1
             240  264 - #000000
             268  264 - #1e1e1e
             304  264 - #ffffff
             324  264 - #1e1e1e
             224  268 - #000000
             280  268 - #010101
             356  268 - #646464
             592  296 - #354a59
             172  320 - #ffffff
             324  328 - #ff0000
             400  328 - #ff0000
             288  336 - #00ff00
             356  340 - #9e0000
             256  344 - #00ff00
             356  344 - #9e0000
             228  352 - #00ff00
             372  352 - #ff0000
             192  356 - #00ff00
             348  356 - #ff0000
               4  360 - #999999
             428  364 - #ffffff
             236  380 - #670707
             268  380 - #660101
             328  380 - #3d2026
             336  380 - #35252d
             416  380 - #24323d
             420  380 - #263540
             196  472 - #995555
             324  512 - #52252d
               4  584 - #7e8488
             188  592 - #735d62
             280  592 - #6f1f25
             592  592 - #354a59
        ",
    )
}

fn check_restored() -> Result<()> {
    check_colors(
        r"
               4    4 - #ffffff
             276    4 - #ff0000
             556   40 - #d7a9ab
             340   44 - #ecc7ce
             408   68 - #cb8c7a
             528   88 - #ae7b68
             540   88 - #946450
             504   92 - #a27d66
             468  100 - #c4a890
             524  100 - #8d5443
             408  104 - #ceb29a
             436  104 - #2a1406
             480  124 - #9e7f63
             200  128 - #ff0000
             428  128 - #c0907c
             428  136 - #e0b9b4
             516  152 - #856953
             556  156 - #a28671
             340  176 - #e5b2b1
             448  180 - #c79d87
             532  188 - #a08069
             504  192 - #8c705a
             404  196 - #ddb7ac
             324  308 - #597c95
             592  332 - #597c95
               4  348 - #ffffff
             256  496 - #000000
             360  496 - #415b6d
             396  496 - #25333d
             552  496 - #415a6c
             260  504 - #ff0000
             316  504 - #000000
             280  508 - #ff0000
             424  508 - #597c95
             528  508 - #364b5a
             360  512 - #415b6d
             396  512 - #25333d
             472  512 - #0b0f12
             496  512 - #000001
             516  512 - #597c95
             284  516 - #ff0000
             340  516 - #000000
             384  516 - #121a1f
             536  516 - #597c95
             256  520 - #9b0000
             396  520 - #25333d
             460  520 - #000000
             552  520 - #415a6c
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
