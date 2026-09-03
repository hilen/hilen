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
             252    4 - #990000
             592    4 - #354a59
             372   40 - #8c7578
             556   80 - #7d5f5e
             428  100 - #281d13
             208  104 - #990000
             476  108 - #37261a
             340  116 - #866a6b
             440  132 - #5c3e31
             512  156 - #503f34
             556  196 - #6c5b4e
             192  212 - #fefefe
             372  212 - #597b94
             244  216 - #ee0000
             296  220 - #ffffff
             428  220 - #55768e
             160  232 - #fdfdfd
             164  236 - #e9e9e9
             344  260 - #494949
             224  264 - #343434
             348  264 - #8e8e8e
             348  268 - #8e8e8e
             268  276 - #010101
             164  288 - #e8e8e8
             440  304 - #597b94
             200  328 - #00ff00
             592  328 - #354a59
             224  336 - #00ff00
             288  336 - #00ff00
             348  336 - #ff0000
             164  344 - #e8e8e8
             244  352 - #00ff00
             372  352 - #ff0000
             428  376 - #466276
             180  380 - #b4b4b4
             188  380 - #acacac
             168  384 - #f9f9f9
             308  392 - #fb0000
             364  496 - #222f39
             260  500 - #990000
             408  508 - #000001
             472  512 - #07090b
             508  512 - #354a59
             284  516 - #990000
             536  516 - #354a59
             256  520 - #5d0000
               4  592 - #999999
        ",
    )
}

// Hiding the modal removes the scrim with it, everything is back to
// full brightness.
fn check_restored() -> Result<()> {
    check_colors(
        r"
             200    4 - #ff0000
             556   40 - #d5a8a9
             408   72 - #c5826e
             528   88 - #ae7d69
             512   96 - #9d7661
             428  100 - #433020
             424  108 - #55452b
             476  108 - #5b3f2b
             520  112 - #a38065
             340  116 - #e0b1b3
             552  116 - #c99697
             452  120 - #896d55
             436  132 - #a36c54
             468  148 - #c9a58f
             508  160 - #927662
               4  164 - #ffffff
             508  192 - #987a62
             556  192 - #b89c86
             348  196 - #e1afae
             416  196 - #dab2a7
             476  196 - #b7947e
             228  256 - #ff0000
             592  300 - #597c95
             432  336 - #597c95
             256  496 - #000000
             360  496 - #415b6d
             396  496 - #25333d
             552  496 - #415a6c
             384  500 - #121a1f
             260  504 - #ff0000
             316  504 - #000000
             280  508 - #ff0000
             384  508 - #121a1f
             424  508 - #597c95
             528  508 - #364b5a
             360  512 - #415b6d
             396  512 - #25333d
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
               4  592 - #ffffff
        ",
    )
}

// The default scrim is transparent, a modal without the override
// leaves the background untouched.
fn check_undimmed_modal() -> Result<()> {
    check_colors(
        r"
               4    4 - #ffffff
             304    4 - #ff0000
             556   40 - #d5a8a9
             408   72 - #c5826e
             200   76 - #ff0000
             528   88 - #ae7d69
             428  100 - #433020
             476  108 - #5b3f2b
             552  116 - #c99697
             452  120 - #896d55
             436  132 - #a36c54
             320  148 - #597c95
             508  160 - #927662
             384  184 - #ebc5be
             556  192 - #b89c86
             452  196 - #d0ab98
             492  196 - #a4846d
             168  232 - #d4d4d4
             264  260 - #ffffff
             296  260 - #8c8c8c
             380  260 - #ffffff
             228  268 - #000000
             296  268 - #8c8c8c
             304  268 - #000000
             592  300 - #597c95
             164  308 - #e8e8e8
             192  336 - #00ff00
             288  336 - #00ff00
             256  344 - #00ff00
             356  344 - #9e0000
             228  352 - #00ff00
             432  368 - #4a677b
             188  380 - #acacac
             316  380 - #ac0000
             360  496 - #415b6d
             396  496 - #25333d
             548  496 - #395060
             384  500 - #121a1f
             468  504 - #000001
             424  508 - #597c95
             536  508 - #597c95
             360  512 - #415b6d
             396  512 - #25333d
             252  516 - #ff0000
             284  516 - #ff0000
             384  516 - #121a1f
             256  520 - #9b0000
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
