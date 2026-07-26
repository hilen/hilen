use anyhow::Result;
use test_engine::{
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
             320    4 - #354a59
             556   40 - #816567
             396   80 - #714131
             528   96 - #5b392c
             268  104 - #990000
             420  104 - #312b1f
             472  116 - #2e2516
             452  120 - #4f3e31
             552  144 - #6e5450
             344  160 - #886465
             520  168 - #5f4d40
             488  192 - #624f41
             556  196 - #6c5b4e
             240  212 - #fe0000
             288  212 - #fe0000
             372  212 - #597b94
             428  220 - #55768e
             160  232 - #fdfdfd
             164  236 - #e9e9e9
             304  256 - #d7d7d7
             332  256 - #dedede
             248  260 - #ffffff
             348  260 - #909090
             348  264 - #909090
             300  268 - #010101
             164  288 - #e8e8e8
             440  304 - #597b94
             200  328 - #00ff00
             592  328 - #354a59
             228  332 - #00ff00
             348  332 - #ff0000
             372  348 - #ff0000
             248  352 - #00ff00
             304  376 - #ffffff
             428  376 - #466276
             180  380 - #b4b4b4
             188  380 - #acacac
             168  384 - #f9f9f9
             396  496 - #000000
             284  504 - #990000
             464  512 - #000000
             256  516 - #990000
             424  516 - #354a59
             536  516 - #354a59
             328  520 - #0b0f12
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
             340  168 - #e0a4a6
             504  192 - #8c705a
             168  232 - #d4d4d4
             228  252 - #5c5c5c
             364  256 - #1c1c1c
             228  260 - #5c5c5c
             276  260 - #ffffff
             332  260 - #e5e5e5
             380  260 - #ffffff
             296  264 - #000000
             344  264 - #2c2c2c
             164  308 - #e8e8e8
             220  332 - #00ff00
             288  336 - #00ff00
             592  340 - #597c95
             372  344 - #ff0000
             228  352 - #00ff00
             344  352 - #0c0000
             188  380 - #acacac
             268  380 - #ac0000
             424  380 - #466175
             252  504 - #3d0000
             536  504 - #597c95
             280  508 - #ac0000
             284  508 - #ac0000
             340  508 - #283843
             420  508 - #3c5465
             388  512 - #597c95
             476  512 - #000000
             256  516 - #ff0000
             340  516 - #283843
               4  592 - #ffffff
             200  592 - #ff0000
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
