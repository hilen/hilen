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
             528   52 - #baafb4
             304   76 - #e25c5f
             336  108 - #c7959a
             212  116 - #eb7474
             152  120 - #eee9e9
             584  136 - #8e9fae
             280  140 - #eb4546
             444  148 - #d2b9a9
             208  156 - #f78383
             324  168 - #ce7d83
             376  172 - #eec9c1
             540  176 - #9f8167
             504  192 - #8c705a
             200  196 - #ff0000
             424  196 - #d2ac99
             320  292 - #597c95
               4  336 - #ffffff
             592  352 - #597c95
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
              28  592 - #ffffff
        ",
    )
}

fn check_restored() -> Result<()> {
    check_colors(
        r"
             556   40 - #d7a9ab
             340   44 - #ecc7ce
             408   68 - #cb8c7a
             540   88 - #946450
             504   92 - #a27d66
             468  100 - #c4a890
             524  100 - #8d5443
             408  104 - #ceb29a
             436  104 - #2a1406
             236  108 - #000000
             476  108 - #311707
             136  116 - #ffffff
             188  124 - #ffffff
             216  124 - #ff0000
             260  124 - #ff0000
             304  124 - #000000
             336  124 - #597c95
             428  136 - #e0b9b4
             516  148 - #9e8068
             468  152 - #c8a891
             556  156 - #a28671
             532  172 - #a1836c
             508  176 - #876c57
             504  192 - #8c705a
             372  196 - #e3b9b3
             440  196 - #d2a592
             224  248 - #ff0000
             348  316 - #597c95
             592  348 - #597c95
             200  376 - #ff0000
               4  388 - #ffffff
             384  496 - #000000
             468  496 - #11181d
             424  504 - #000001
             500  504 - #1d2830
             528  504 - #000000
             448  508 - #597c95
             480  508 - #597c95
             332  512 - #425c6f
             360  512 - #223039
             500  512 - #1d2830
             276  516 - #ff0000
             328  516 - #374d5d
             276  520 - #9b0000
             328  520 - #374d5d
             332  520 - #425c6f
             360  520 - #223039
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
