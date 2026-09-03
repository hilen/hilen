use anyhow::Result;
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{BlurView, Container, ImageView, Label, RED, Setup, ViewData, ViewSubviews, ViewTest, WHITE, view},
    ui_test::{check_colors, set_record_probe_count},
};

// A frosted header card floating over a busy background: color
// strips, a photo and text. The card is tall enough to cover text and
// the photo top, shows them blurred and tinted, its own title stays
// crisp on top, and the rounded corners cut the blur off.
#[view]
struct BackdropBlurTest {
    title: Weak<Label>,

    #[init]
    white:   Container,
    red:     Container,
    photo:   ImageView,
    covered: Label,
    text:    Label,
    header:  BlurView,
}

impl Setup for BackdropBlurTest {
    fn setup(mut self: Weak<Self>) {
        self.white.set_color(WHITE);
        self.white.place().tl(0).size(200, 600);

        self.red.set_color(RED);
        self.red.place().t(0).l(200).size(120, 600);

        self.photo.set_image("cat.png");
        self.photo.place().t(40).l(340).size(220, 160);

        self.covered.set_text("Covered by frost");
        self.covered.set_text_size(32);
        self.covered.place().t(90).l(40).size(400, 60);

        self.text.set_text("Behind the blur");
        self.text.set_text_size(40);
        self.text.place().t(480).l(220).size(360, 60);

        self.header.set_blur_radius(25);
        self.header.set_color(WHITE.with_alpha(0.25));
        self.header.set_corner_radius(20);
        self.header.place().t(12).l(12).r(12).h(160);

        self.title = self.header.add_view();
        self.title.set_text("Frosted header");
        self.title.set_text_size(28);
        self.title.place().lrt(0).h(56);
    }
}

fn check_blurred() -> Result<()> {
    check_colors(
        r"
             476   12 - #90a2b3
             208   20 - #ff8686
             296   32 - #6b2526
             312   32 - #813e42
             364   32 - #75757e
             212   36 - #ff7b7b
             208   40 - #bf6464
             216   40 - #391919
             256   40 - #fe4242
             276   40 - #ce3838
             376   40 - #96939c
             236   44 - #ff4d4d
             312   44 - #833f42
             332   44 - #c08b95
             344   44 - #ba9fa9
             360   44 - #b9afb9
             388   44 - #58565a
             536   84 - #c3acab
             332   92 - #cb8f95
             212  104 - #ef7575
             388  108 - #e3cac3
             304  120 - #d65a5d
             336  144 - #c99599
             168  152 - #f6e3e3
             232  164 - #fa5151
             520  180 - #a88a73
             504  188 - #856854
             424  192 - #d9b5a7
             364  196 - #dda3a2
             500  196 - #9a7b63
             592  240 - #597c95
             296  268 - #ff0000
               4  308 - #ffffff
             428  348 - #597c95
             280  496 - #000000
             468  496 - #11181d
             404  500 - #000000
             300  508 - #ff0000
             380  508 - #597c95
             500  508 - #1d2830
             328  512 - #374d5d
             360  512 - #223039
             308  516 - #ff0000
             276  520 - #9b0000
             332  520 - #425c6f
             360  520 - #223039
             440  520 - #000001
               4  592 - #ffffff
        ",
    )
}

fn check_restored() -> Result<()> {
    check_colors(
        r"
             556   40 - #d5a8a9
             340   44 - #e7c0c6
             408   72 - #c5826e
             528   88 - #ae7d69
             428  100 - #433020
             424  108 - #55452b
             476  108 - #5b3f2b
             520  112 - #a38065
             136  116 - #ffffff
             552  116 - #c99697
             184  120 - #f0f0f0
             256  120 - #ff0000
             452  120 - #896d55
             156  124 - #ffffff
             208  124 - #000000
             304  124 - #000000
             336  124 - #597c95
             436  132 - #a36c54
             468  148 - #c9a58f
             508  160 - #927662
             556  164 - #917560
             556  192 - #b89c86
             372  196 - #e4bab4
             452  196 - #d0ab98
             492  196 - #a4846d
             592  304 - #597c95
             208  316 - #ff0000
             436  340 - #597c95
             316  384 - #ff0000
               4  388 - #ffffff
             468  496 - #11181d
             404  500 - #000000
             528  504 - #000000
             300  508 - #ff0000
             380  508 - #597c95
             480  508 - #597c95
             500  508 - #1d2830
             328  512 - #374d5d
             332  512 - #425c6f
             440  512 - #000001
             500  512 - #1d2830
             276  516 - #ff0000
             276  520 - #9b0000
             328  520 - #374d5d
             332  520 - #425c6f
             360  520 - #223039
             468  520 - #11181d
              72  592 - #ffffff
        ",
    )
}

impl ViewTest for BackdropBlurTest {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        set_record_probe_count(48);

        check_blurred()?;

        from_main(move || {
            view.header.set_hidden(true);
        });

        check_restored()?;

        Ok(())
    }
}
