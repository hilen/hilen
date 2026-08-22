use anyhow::Result;
use hilen::{
    OnceEvent,
    dispatch::from_main,
    refs::Weak,
    ui::{
        BLACK, Container, GREEN, ImageView, Label, ModalView, RED, Setup, Shadow, Size, UIColor, View,
        ViewData, ViewTest, WHITE, WeakView, view,
    },
    ui_test::{check_colors, set_record_probe_count},
};

// Both modals look like a real dialog: a white rounded card with a
// shadow, a title and two buttons. Only the scrim override differs.
#[view]
struct DimModal {
    event: OnceEvent,

    #[init]
    title: Label,
    yes:   Label,
    no:    Label,
}

impl Setup for DimModal {
    fn setup(self: Weak<Self>) {
        dialog_look(self.weak_view(), self.title, self.yes, self.no, "Are you sure?");
    }
}

impl ModalView for DimModal {
    fn modal_event(&self) -> &OnceEvent<()> {
        &self.event
    }

    fn modal_size() -> Size {
        (260, 160).into()
    }

    fn modal_scrim_color() -> UIColor {
        BLACK.with_alpha(0.4).into()
    }
}

#[view]
struct DefaultModal {
    event: OnceEvent,

    #[init]
    title: Label,
    yes:   Label,
    no:    Label,
}

impl Setup for DefaultModal {
    fn setup(self: Weak<Self>) {
        dialog_look(self.weak_view(), self.title, self.yes, self.no, "No scrim here");
    }
}

impl ModalView for DefaultModal {
    fn modal_event(&self) -> &OnceEvent<()> {
        &self.event
    }

    fn modal_size() -> Size {
        (260, 160).into()
    }
}

fn dialog_look(card: WeakView, title: Weak<Label>, yes: Weak<Label>, no: Weak<Label>, text: &str) {
    card.set_color(WHITE);
    card.set_corner_radius(16);
    card.set_shadow(Shadow::default());

    title.set_text(text);
    title.set_text_size(28);
    title.place().lrt(12).h(60);

    yes.set_text("Yes");
    yes.set_color(GREEN);
    yes.set_corner_radius(8);
    yes.place().size(100, 36).b(16).l(20);

    no.set_text("No");
    no.set_color(RED);
    no.set_corner_radius(8);
    no.place().size(100, 36).b(16).r(20);
}

// The background imitates a real screen: color strips, a photo and
// text. The scrim has to dim all of it while the modal on top stays
// untouched.
#[view]
struct ModalScrimTest {
    #[init]
    white: Container,
    red:   Container,
    photo: ImageView,
    text:  Label,
}

impl Setup for ModalScrimTest {
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

// The opt-in scrim dims everything behind the modal: the strips, the
// photo and the text. The modal itself stays undimmed on top.
fn check_dimmed() -> Result<()> {
    check_colors(
        r"
               4    4 - #999999
             208    4 - #990000
             316    4 - #990000
             464   40 - #856d6e
             556   40 - #816567
             396   80 - #714131
             528   96 - #5b392c
             420  104 - #312b1f
             472  116 - #2e2516
             452  120 - #4f3e31
             524  152 - #645144
             320  156 - #354a59
             488  192 - #624f41
             356  196 - #835f5f
             556  196 - #6c5b4e
             192  212 - #fefefe
             248  212 - #fe0000
             296  220 - #ffffff
             428  220 - #55768e
             160  232 - #fdfdfd
             164  236 - #e9e9e9
             236  260 - #414141
             348  260 - #8e8e8e
             348  268 - #8e8e8e
             268  276 - #010101
             164  288 - #e8e8e8
             440  304 - #597b94
             200  328 - #00ff00
             592  328 - #354a59
             224  336 - #00ff00
             288  336 - #00ff00
             348  336 - #ff0000
             244  352 - #00ff00
             372  352 - #ff0000
             428  376 - #466276
             180  380 - #b4b4b4
             188  380 - #acacac
             168  384 - #f9f9f9
             308  392 - #fb0000
             352  392 - #587a93
             260  500 - #990000
             408  508 - #000001
             284  516 - #990000
             348  516 - #000001
             536  516 - #354a59
             256  520 - #5d0000
             472  520 - #07090b
               4  592 - #999999
        ",
    )
}

// Hiding the modal removes the scrim with it, everything is back to
// full brightness.
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

// The default scrim is transparent, a modal without the override
// leaves the background untouched.
fn check_undimmed_modal() -> Result<()> {
    check_colors(
        r"
               4    4 - #ffffff
             320    4 - #597c95
             408   68 - #cb8c7a
             200   76 - #ff0000
             340   88 - #e9c1c2
             540   88 - #946450
             504   92 - #a27d66
             468  100 - #c4a890
             524  100 - #8d5443
             408  104 - #ceb29a
             436  104 - #2a1406
             476  108 - #311707
             428  136 - #e0b9b4
             516  148 - #9e8068
             556  156 - #a28671
             240  164 - #ff0000
             340  172 - #e1a0a4
             448  180 - #c79d87
             504  192 - #8c705a
             400  196 - #dbb3a9
             168  232 - #d4d4d4
             264  260 - #ffffff
             296  260 - #8c8c8c
             380  260 - #ffffff
             228  268 - #000000
             296  268 - #8c8c8c
             304  268 - #000000
             164  308 - #e8e8e8
             288  336 - #00ff00
             256  344 - #00ff00
             356  344 - #9e0000
             592  344 - #597c95
             228  352 - #00ff00
             432  368 - #4a677b
             188  380 - #acacac
             316  380 - #ac0000
             360  496 - #415b6d
             396  496 - #25333d
             548  496 - #395060
             468  504 - #000001
             424  508 - #597c95
             536  508 - #597c95
             360  512 - #415b6d
             252  516 - #ff0000
             284  516 - #ff0000
             256  520 - #9b0000
             396  520 - #25333d
               4  592 - #ffffff
        ",
    )
}

impl ViewTest for ModalScrimTest {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        set_record_probe_count(48);

        let modal = from_main(DimModal::prepare_modally);

        check_dimmed()?;

        modal.hide_modal(());

        check_restored()?;

        let modal = from_main(DefaultModal::prepare_modally);

        check_undimmed_modal()?;

        modal.hide_modal(());

        Ok(())
    }
}
