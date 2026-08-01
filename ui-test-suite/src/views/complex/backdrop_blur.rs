use anyhow::Result;
use test_engine::{
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
             592    4 - #597c95
             304   12 - #df5a5e
             208   16 - #ff8686
             216   32 - #ff7070
             240   36 - #ff4a4a
             332   36 - #bb8993
             368   36 - #898892
             376   36 - #afb0bb
             280   40 - #000000
             352   40 - #887d85
             360   40 - #b4acb7
             252   44 - #ff4343
             484   52 - #c3b5b9
             388   88 - #e0c7c3
             212   92 - #f47777
             444   92 - #cfb7ab
             336  112 - #c69499
             584  116 - #8f9faf
             508  120 - #c6ab9e
             156  132 - #efe8e8
             232  152 - #f65151
             332  156 - #cb8d92
             436  172 - #d1ad97
             556  172 - #b3957d
             368  180 - #e2b8b5
             532  188 - #a08069
             504  192 - #8c705a
             260  264 - #ff0000
               4  288 - #ffffff
             592  352 - #597c95
             200  384 - #ff0000
             276  492 - #1c0000
             320  496 - #2e414e
             404  496 - #24333d
             496  496 - #40596c
             276  504 - #3d0000
             300  508 - #ac0000
             304  508 - #ac0000
             380  508 - #597c95
             428  508 - #172026
             480  508 - #597c95
             308  512 - #ff0000
             268  516 - #690000
             280  516 - #ff0000
             364  516 - #000000
             332  520 - #12191e
             516  520 - #12191e
               4  592 - #ffffff
        ",
    )
}

fn check_restored() -> Result<()> {
    check_colors(
        r"
             516   40 - #dcb2b3
             340   44 - #ecc7ce
             408   68 - #cb8c7a
             540   88 - #946450
             504   92 - #a27d66
             468  100 - #c4a890
             524  100 - #8d5443
             408  104 - #ceb29a
             476  108 - #311707
             236  116 - #ff0000
             300  116 - #4e0000
             352  116 - #000000
             132  120 - #ffffff
             156  124 - #ffffff
             200  124 - #000000
             300  124 - #4e0000
             276  132 - #010000
             428  136 - #e0b9b4
             516  148 - #9e8068
             556  156 - #a28671
             436  172 - #d1ad97
             532  172 - #a1836c
             508  176 - #876c57
             504  192 - #8c705a
             372  196 - #e3b9b3
             548  196 - #b99d87
             216  256 - #ff0000
             336  316 - #597c95
             592  348 - #597c95
             200  388 - #ff0000
               4  408 - #ffffff
             268  496 - #690000
             320  496 - #2e414e
             404  496 - #24333d
             496  496 - #40596c
             348  504 - #090c0f
             404  504 - #24333d
             300  508 - #ac0000
             304  508 - #ac0000
             480  508 - #597c95
             376  512 - #597c95
             428  512 - #172026
             268  516 - #690000
             304  516 - #ff0000
             428  516 - #172026
             496  516 - #40596c
             516  520 - #12191e
              88  592 - #ffffff
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
