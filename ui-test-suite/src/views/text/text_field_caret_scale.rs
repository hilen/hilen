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
    fn setup(self: Weak<Self>) {
        self.hint.set_alignment(TextAlignment::Center);
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
        68    4 - #597c95
       476    4 - #597c95
       268   32 - #000000
       384   32 - #577991
       212   36 - #597c95
       220   36 - #12191e
       224   36 - #000000
       228   36 - #1d2830
       240   36 - #597c95
       268   36 - #000000
       308   36 - #597c95
       324   36 - #0a0e11
       332   36 - #597c95
       348   36 - #1d2830
       372   36 - #1d2830
        60   72 - #ffffff
       112   72 - #ffffff
       172   72 - #ffffff
       232   72 - #ffffff
       300   72 - #ffffff
       432   72 - #ffffff
       520   72 - #ffffff
       548   72 - #ffffff
       580   72 - #ffffff
       388   76 - #ffffff
       200   80 - #ffffff
       476   84 - #ffffff
        24   88 - #000000
       264   88 - #ffffff
       332   88 - #ffffff
        24   92 - #000000
        28   92 - #bcbcbc
        32   92 - #ffffff
       152   92 - #bcbcbc
        24   96 - #000000
        28   96 - #bcbcbc
        32   96 - #ffffff
        36   96 - #ffffff
        40   96 - #bcbcbc
        92   96 - #bcbcbc
       100   96 - #bcbcbc
       124   96 - #bcbcbc
       132   96 - #bcbcbc
       144   96 - #bcbcbc
       152   96 - #bcbcbc
       360   96 - #ffffff
        24  100 - #000000
        28  100 - #bcbcbc
        32  100 - #bcbcbc
        36  100 - #bcbcbc
        68  100 - #ffffff
        84  100 - #ffffff
        88  100 - #ffffff
       108  100 - #bcbcbc
       120  100 - #ffffff
       124  100 - #ffffff
       132  100 - #bcbcbc
       148  100 - #ffffff
       152  100 - #bcbcbc
       180  100 - #ffffff
       304  100 - #ffffff
       448  100 - #ffffff
       516  100 - #ffffff
       552  100 - #ffffff
       580  100 - #ffffff
        24  104 - #000000
        28  104 - #bcbcbc
        52  104 - #ffffff
       104  104 - #bcbcbc
       120  104 - #ffffff
       124  104 - #ffffff
       132  104 - #bcbcbc
       148  104 - #ffffff
       152  104 - #bcbcbc
        24  108 - #000000
        28  108 - #bcbcbc
        52  108 - #bcbcbc
        56  108 - #bcbcbc
        96  108 - #bcbcbc
       124  108 - #bcbcbc
       132  108 - #bcbcbc
       152  108 - #bcbcbc
        24  112 - #000000
       420  116 - #ffffff
       492  116 - #ffffff
       224  120 - #ffffff
       376  120 - #ffffff
       260  124 - #ffffff
        76  128 - #ffffff
       184  128 - #ffffff
       296  128 - #ffffff
       336  128 - #ffffff
       460  128 - #ffffff
       524  128 - #ffffff
       552  128 - #ffffff
       584  128 - #ffffff
         4  192 - #597c95
       140  200 - #597c95
       388  208 - #597c95
       472  208 - #597c95
       224  212 - #597c95
        68  240 - #597c95
       548  244 - #597c95
       308  256 - #597c95
       436  284 - #597c95
       188  300 - #597c95
       360  324 - #597c95
         4  360 - #597c95
       572  360 - #597c95
       112  380 - #597c95
       248  380 - #597c95
       460  400 - #597c95
       344  412 - #597c95
        48  428 - #597c95
       188  460 - #597c95
       548  480 - #597c95
       284  492 - #597c95
         4  496 - #597c95
       100  496 - #597c95
       368  496 - #597c95
       440  540 - #597c95
       192  560 - #597c95
         4  588 - #597c95
        96  592 - #597c95
       288  592 - #597c95
       372  592 - #597c95
       508  592 - #597c95
       592  592 - #597c95
";

const SCALE_2: &str = r"
          4    4 - #597c95
        592    4 - #597c95
        312   60 - #000000
        424   60 - #010101
        120   64 - #000000
        196   64 - #000000
        236   64 - #000000
        276   64 - #000000
        292   64 - #000000
        336   64 - #000001
        352   64 - #000000
        364   64 - #000000
        392   64 - #000000
        476   64 - #597c95
        132   68 - #000000
        156   68 - #597c95
        208   68 - #000000
        228   68 - #000001
        256   68 - #597c95
        320   68 - #597c95
        416   68 - #597c95
        424   68 - #000000
        148   72 - #000000
        172   72 - #000001
        184   72 - #597c95
        220   72 - #000001
        312   72 - #000000
        332   72 - #000000
        368   72 - #597c95
        440   72 - #000000
        124   76 - #597c95
        156   76 - #597c95
        196   76 - #597c95
        208   76 - #000001
        236   76 - #000000
        260   76 - #597c95
        292   76 - #597c95
        320   76 - #597c95
        392   76 - #597c95
        416   76 - #597c95
        424   76 - #010101
        448   76 - #597c95
        224   80 - #000001
        336   80 - #000001
        364   80 - #000001
        128  140 - #ffffff
        184  140 - #ffffff
        260  140 - #ffffff
        332  140 - #ffffff
        564  140 - #ffffff
        484  144 - #ffffff
        432  152 - #ffffff
        380  156 - #ffffff
         48  168 - #000000
         56  180 - #bcbcbc
        304  180 - #bcbcbc
         80  184 - #bcbcbc
         68  188 - #ffffff
         48  192 - #000000
        108  192 - #bcbcbc
        172  192 - #bcbcbc
        220  192 - #bcbcbc
        236  192 - #bcbcbc
        248  192 - #bcbcbc
        272  192 - #bcbcbc
        288  192 - #bcbcbc
         92  196 - #bcbcbc
        100  196 - #ffffff
        128  196 - #ffffff
        132  196 - #ffffff
        144  196 - #bcbcbc
        160  196 - #ffffff
        164  196 - #ffffff
        204  196 - #bcbcbc
        304  196 - #bcbcbc
        528  196 - #ffffff
         60  200 - #bcbcbc
         72  200 - #bcbcbc
        188  200 - #bcbcbc
        264  200 - #bcbcbc
         48  204 - #000000
        216  204 - #bcbcbc
        228  204 - #bcbcbc
        240  204 - #ffffff
        244  204 - #ffffff
        292  204 - #ffffff
        296  204 - #ffffff
        356  204 - #ffffff
        104  208 - #ffffff
        284  208 - #bcbcbc
        304  208 - #bcbcbc
        440  208 - #ffffff
         96  212 - #ffffff
        100  212 - #ffffff
        104  212 - #ffffff
        136  212 - #ffffff
        168  212 - #ffffff
        268  212 - #bfbfbf
         56  216 - #bcbcbc
        112  216 - #bcbcbc
        156  216 - #bcbcbc
        192  216 - #bcbcbc
        212  216 - #bcbcbc
        236  216 - #bcbcbc
        248  216 - #bcbcbc
        288  216 - #bcbcbc
        300  216 - #bcbcbc
         48  220 - #000000
         48  232 - #000000
        324  256 - #ffffff
        372  256 - #ffffff
        424  256 - #ffffff
        496  256 - #ffffff
        572  256 - #ffffff
        168  324 - #597c95
          4  360 - #597c95
        268  368 - #597c95
        380  368 - #597c95
        592  380 - #597c95
        120  424 - #597c95
        476  428 - #597c95
         16  476 - #597c95
        568  484 - #597c95
        300  520 - #597c95
          4  592 - #597c95
        160  592 - #597c95
        436  592 - #597c95
        592  592 - #597c95
";
