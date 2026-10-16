use anyhow::Result;
use hilen::{
    dispatch::from_main,
    gm::color::{GRAY, WHITE},
    refs::Weak,
    ui::{Label, Setup, TextAlignment, TextField, UIManager, ViewData, ViewTest, view},
    ui_test::{check_colors, set_record_probe_count},
};

/// The text of a left aligned field starts a number of points in, which is
/// a different number of screen pixels at every UI scale. The caret of a
/// focused field must move with the text when the scale changes. A field
/// focused in the `setup` of the first screen of an app gets the screen scale
/// only after that, and its caret stood inside the first letter of the
/// placeholder.
#[view]
struct TextFieldCaretScale {
    #[init]
    hint:  Label,
    field: TextField,
}

impl Setup for TextFieldCaretScale {
    fn setup(mut self: Weak<Self>) {
        self.hint.set_text("the caret sits before the P").set_text_size(16);
        self.hint.place().t(20).lr(10).h(30);

        self.field.set_placeholder("Password").set_text_size(30);
        self.field.set_placeholder_color(GRAY).set_selected_color(WHITE);
        self.field.set_alignment(TextAlignment::Left);
        // 12 points from the left, so the caret is on a pixel column the
        // probe grid of 4 pixels samples, at scale 1 and at scale 2.
        self.field.place().t(70).lr(12).h(60);
        self.field.focus();
    }
}

impl ViewTest for TextFieldCaretScale {
    fn perform_test(_: Weak<Self>) -> Result<()> {
        set_record_probe_count(128);

        check_colors(SCALE_1)?;

        from_main(|| UIManager::override_scale(2.0));
        check_colors(SCALE_2)?;

        from_main(|| UIManager::override_scale(1.0));

        Ok(())
    }
}

const SCALE_1: &str = r"
      72    4 - #597c95
     268   32 - #000000
     384   32 - #577991
     212   36 - #597c95
     220   36 - #12191e
     224   36 - #000000
     228   36 - #1d2830
     240   36 - #597c95
     268   36 - #000000
     284   36 - #40596c
     308   36 - #597c95
     324   36 - #0a0e11
     332   36 - #597c95
     348   36 - #1d2830
     356   36 - #1c272f
     372   36 - #1d2830
      80   72 - #ffffff
     180   72 - #ffffff
     232   72 - #ffffff
     292   72 - #ffffff
     380   72 - #ffffff
     436   72 - #ffffff
     464   72 - #ffffff
     520   72 - #ffffff
     580   72 - #ffffff
     320   76 - #ffffff
     492   76 - #ffffff
     260   84 - #ffffff
     348   84 - #ffffff
      28   88 - #000000
     208   88 - #ffffff
     412   88 - #ffffff
      28   92 - #000000
      32   92 - #bcbcbc
      36   92 - #ffffff
     156   92 - #bcbcbc
      28   96 - #000000
      32   96 - #bcbcbc
      36   96 - #ffffff
      40   96 - #ffffff
      44   96 - #bcbcbc
      96   96 - #bcbcbc
     104   96 - #bcbcbc
     128   96 - #bcbcbc
     136   96 - #bcbcbc
     148   96 - #bcbcbc
     156   96 - #bcbcbc
      28  100 - #000000
      32  100 - #bcbcbc
      36  100 - #bcbcbc
      40  100 - #bcbcbc
      60  100 - #c2c2c2
      72  100 - #ffffff
      88  100 - #ffffff
      92  100 - #ffffff
     112  100 - #bcbcbc
     124  100 - #ffffff
     128  100 - #ffffff
     136  100 - #bcbcbc
     152  100 - #ffffff
     156  100 - #bcbcbc
     304  100 - #ffffff
     444  100 - #ffffff
     520  100 - #ffffff
     552  100 - #ffffff
      28  104 - #000000
      32  104 - #bcbcbc
      56  104 - #ffffff
      60  104 - #c2c2c2
     108  104 - #bcbcbc
     124  104 - #ffffff
     128  104 - #ffffff
     136  104 - #bcbcbc
     152  104 - #ffffff
     156  104 - #bcbcbc
     384  104 - #ffffff
     492  104 - #ffffff
      28  108 - #000000
      32  108 - #bcbcbc
      56  108 - #bcbcbc
      60  108 - #bcbcbc
     100  108 - #bcbcbc
     128  108 - #bcbcbc
     136  108 - #bcbcbc
     156  108 - #bcbcbc
      28  112 - #000000
     224  112 - #ffffff
      80  128 - #ffffff
     196  128 - #ffffff
     248  128 - #ffffff
     284  128 - #ffffff
     328  128 - #ffffff
     364  128 - #ffffff
     408  128 - #ffffff
     460  128 - #ffffff
     524  128 - #ffffff
     584  128 - #ffffff
       4  192 - #597c95
     388  212 - #597c95
     164  216 - #597c95
      72  240 - #597c95
     548  244 - #597c95
     308  248 - #597c95
     436  284 - #597c95
     188  308 - #597c95
     288  336 - #597c95
     372  336 - #597c95
     100  348 - #597c95
       4  360 - #597c95
     572  360 - #597c95
     452  388 - #597c95
     232  408 - #597c95
     344  412 - #597c95
      48  428 - #597c95
     128  444 - #597c95
     592  460 - #597c95
     496  484 - #597c95
       4  496 - #597c95
     352  508 - #597c95
      84  512 - #597c95
     164  528 - #597c95
     252  540 - #597c95
     496  584 - #597c95
       4  592 - #597c95
     100  592 - #597c95
     316  592 - #597c95
     400  592 - #597c95
     592  592 - #597c95
";

const SCALE_2: &str = r"
       4    4 - #597c95
     592    4 - #597c95
     132   60 - #0a0e10
     312   60 - #000000
     472   60 - #223039
     280   64 - #000000
     292   64 - #000000
     336   64 - #000001
     352   64 - #000000
     364   64 - #000000
     392   64 - #000000
     416   64 - #000000
     480   64 - #000000
     120   68 - #394f5f
     124   68 - #597c95
     156   68 - #597c95
     200   68 - #000001
     224   68 - #597c95
     236   68 - #000000
     256   68 - #597c95
     272   68 - #3b5262
     412   68 - #394f5f
     424   68 - #000000
     444   68 - #597c95
     120   72 - #394f5f
     140   72 - #23313b
     172   72 - #000001
     184   72 - #597c95
     272   72 - #3b5262
     280   72 - #070a0c
     316   72 - #597c95
     332   72 - #000000
     348   72 - #141c22
     376   72 - #12191e
     412   72 - #394f5f
     432   72 - #2c3d4a
     140   76 - #23313b
     152   76 - #597c95
     196   76 - #597c95
     208   76 - #000001
     236   76 - #000000
     260   76 - #597c95
     272   76 - #3b5262
     292   76 - #597c95
     324   76 - #000001
     348   76 - #141c22
     364   76 - #597c95
     388   76 - #000000
     396   76 - #597c95
     432   76 - #2c3d4a
     448   76 - #597c95
     224   80 - #000001
     336   80 - #000001
     108  140 - #ffffff
     164  140 - #ffffff
     240  140 - #ffffff
     348  140 - #ffffff
     516  140 - #ffffff
     568  140 - #ffffff
     396  152 - #ffffff
      56  168 - #000000
     476  168 - #ffffff
     316  180 - #cccccc
      56  184 - #000000
      76  188 - #ffffff
     104  192 - #bcbcbc
     192  192 - #bcbcbc
     244  192 - #bcbcbc
     276  192 - #bcbcbc
     308  192 - #bcbcbc
      88  196 - #bcbcbc
     112  196 - #ffffff
     136  196 - #ffffff
     140  196 - #ffffff
     152  196 - #bcbcbc
     168  196 - #ffffff
     172  196 - #ffffff
     208  196 - #bcbcbc
     260  196 - #bcbcbc
     548  196 - #ffffff
      56  200 - #000000
      72  200 - #bcbcbc
     224  200 - #bcbcbc
     196  204 - #bcbcbc
     236  204 - #bcbcbc
     248  204 - #ffffff
     252  204 - #ffffff
     272  204 - #bcbcbc
     300  204 - #ffffff
     304  204 - #ffffff
     316  204 - #cccccc
     104  208 - #ffffff
     116  208 - #d2d2d2
     392  208 - #ffffff
      56  212 - #000000
     108  212 - #ffffff
     132  212 - #bcbcbc
     144  212 - #ffffff
     164  212 - #bcbcbc
     176  212 - #ffffff
     100  216 - #bcbcbc
     200  216 - #bcbcbc
     220  216 - #bcbcbc
     244  216 - #bcbcbc
     256  216 - #bcbcbc
     276  216 - #bfbfbf
     296  216 - #bcbcbc
     312  216 - #bcbcbc
      64  220 - #dedede
     120  220 - #dedede
     172  220 - #bcbcbc
      56  232 - #000000
     504  236 - #ffffff
     432  252 - #ffffff
     360  256 - #ffffff
     572  256 - #ffffff
      96  360 - #597c95
     456  368 - #597c95
     224  404 - #597c95
     560  424 - #597c95
       4  456 - #597c95
     400  476 - #597c95
     128  484 - #597c95
     468  580 - #597c95
      60  592 - #597c95
     208  592 - #597c95
     340  592 - #597c95
     592  592 - #597c95
";
