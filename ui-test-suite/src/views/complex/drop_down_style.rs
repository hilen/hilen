use anyhow::Result;
use hilen::{
    refs::Weak,
    ui::{Color, Container, Setup, TextDropDown, ViewData, ViewTest, view},
    ui_test::{check_colors, inject_touches, set_record_probe_count},
};

/// A styled drop down on a card: its own color, border and corners,
/// dark text and the chevron, closed and then open. Pins that the view
/// setters style the box and that the open list wears the same look.
#[view]
struct DropDownStyle {
    #[init]
    card: Container,
    drop: TextDropDown,
}

impl Setup for DropDownStyle {
    fn setup(mut self: Weak<Self>) {
        self.card.set_color(Color::rgb(0.92, 0.94, 0.97)).set_corner_radius(16);
        self.card.place().tl(40).size(300, 260);

        self.drop.set_values(vec!["One", "Two", "Three"]);
        self.drop.set_text_color(Color::rgb(0.10, 0.12, 0.17)).set_text_size(16);
        self.drop
            .set_color(Color::rgb(0.98, 0.98, 1.0))
            .set_corner_radius(10)
            .set_border_width(1)
            .set_border_color(Color::rgb(0.80, 0.84, 0.90));
        self.drop.place().t(70).l(80).size(180, 40);
    }
}

impl ViewTest for DropDownStyle {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        set_record_probe_count(128);
        check_colors(COLORS_1)?;

        inject_touches(TOUCHES_1);
        check_colors(COLORS_2)?;

        // The panel opens 6 under the box at y 116, rows are 36 tall, so
        // "Two" spans 156..192.
        inject_touches(TOUCHES_2);
        anyhow::ensure!(view.drop.value() == "Two", "picked {}", view.drop.value());
        check_colors(COLORS_3)?;

        Ok(())
    }
}

const COLORS_1: &str = r"
    4    4 - #597c95
    512    4 - #597c95
    580    4 - #597c95
    420    8 - #597c95
    108   40 - #ebf0f7
    156   40 - #ebf0f7
    188   40 - #ebf0f7
    220   40 - #ebf0f7
    280   40 - #ebf0f7
    48   44 - #ebf0f7
    336   48 - #ebf0f7
    460   56 - #597c95
    248   60 - #ebf0f7
    592   68 - #597c95
    148   72 - #fafaff
    188   72 - #fafaff
    304   76 - #ebf0f7
    80   80 - #ccd6e6
    516   80 - #597c95
    80   84 - #ccd6e6
    80   88 - #ccd6e6
    112   88 - #fafaff
    80   92 - #ccd6e6
    112   92 - #fafaff
    120   92 - #323742
    216   92 - #fafaff
    252   92 - #fafaff
    40   96 - #ebf0f7
    80   96 - #ccd6e6
    336   96 - #ebf0f7
    80  100 - #cfd9e7
    380  100 - #597c95
    156  104 - #fafaff
    192  116 - #ebf0f7
    448  116 - #597c95
    308  120 - #ebf0f7
    548  128 - #597c95
    104  132 - #ebf0f7
    228  132 - #ebf0f7
    268  132 - #ebf0f7
    40  136 - #ebf0f7
    72  136 - #ebf0f7
    144  140 - #ebf0f7
    336  152 - #ebf0f7
    84  168 - #ebf0f7
    120  168 - #ebf0f7
    192  168 - #ebf0f7
    248  168 - #ebf0f7
    296  168 - #ebf0f7
    592  168 - #597c95
    508  172 - #597c95
    44  176 - #ebf0f7
    220  184 - #ebf0f7
    416  188 - #597c95
    148  192 - #ebf0f7
    336  196 - #ebf0f7
    108  200 - #ebf0f7
    300  204 - #ebf0f7
    76  212 - #ebf0f7
    252  212 - #ebf0f7
    40  216 - #ebf0f7
    172  216 - #ebf0f7
    208  216 - #ebf0f7
    136  224 - #ebf0f7
    540  232 - #597c95
    100  240 - #ebf0f7
    236  240 - #ebf0f7
    288  240 - #ebf0f7
    332  244 - #ebf0f7
    472  244 - #597c95
    160  248 - #ebf0f7
    60  256 - #ebf0f7
    4  260 - #597c95
    128  260 - #ebf0f7
    196  260 - #ebf0f7
    264  264 - #ebf0f7
    404  268 - #597c95
    232  272 - #ebf0f7
    304  272 - #ebf0f7
    588  280 - #597c95
    92  284 - #ebf0f7
    336  288 - #ebf0f7
    48  296 - #ebf0f7
    132  296 - #ebf0f7
    184  296 - #ebf0f7
    280  296 - #ebf0f7
    512  308 - #597c95
    4  332 - #597c95
    240  344 - #597c95
    428  344 - #597c95
    592  348 - #597c95
    52  360 - #597c95
    344  372 - #597c95
    184  376 - #597c95
    476  376 - #597c95
    536  380 - #597c95
    104  392 - #597c95
    4  400 - #597c95
    424  400 - #597c95
    260  404 - #597c95
    472  436 - #597c95
    204  440 - #597c95
    528  440 - #597c95
    592  440 - #597c95
    388  444 - #597c95
    88  452 - #597c95
    4  456 - #597c95
    316  472 - #597c95
    148  480 - #597c95
    44  496 - #597c95
    532  496 - #597c95
    244  500 - #597c95
    440  504 - #597c95
    100  512 - #597c95
    296  524 - #597c95
    592  524 - #597c95
    188  528 - #597c95
    372  532 - #597c95
    4  536 - #597c95
    240  564 - #597c95
    520  568 - #597c95
    96  572 - #597c95
    448  576 - #597c95
    300  580 - #597c95
    4  592 - #597c95
    184  592 - #597c95
    388  592 - #597c95
    592  592 - #597c95
";

const TOUCHES_1: &str = "
    170 90 b
    170 90 e
";

const COLORS_2: &str = r"
    448    4 - #597c95
    568    4 - #597c95
    104   40 - #ebf0f7
    156   40 - #ebf0f7
    220   40 - #ebf0f7
     48   44 - #ebf0f7
    336   48 - #ebf0f7
    276   60 - #ebf0f7
    188   72 - #fafaff
     80   84 - #00daff
     80   88 - #00daff
    112   88 - #fafaff
     80   92 - #00daff
    112   92 - #fafaff
    120   92 - #313541
     80   96 - #00daff
    312  100 - #ebf0f7
     40  104 - #ebf0f7
    592  112 - #597c95
     96  116 - #ccd6e5
    108  116 - #ccd6e5
    120  116 - #ccd6e5
    132  116 - #ccd6e5
    144  116 - #ccd6e5
    164  116 - #ccd6e5
    180  116 - #ccd6e5
    200  116 - #ccd6e5
    216  116 - #ccd6e5
    236  116 - #ccd6e5
    256  116 - #cdd1d8
     76  124 - #ced2d9
    260  128 - #b4b8bd
    484  128 - #597c95
     80  136 - #ccd6e6
    112  136 - #e6f7ff
    244  136 - #00daff
    112  140 - #e6f7ff
    120  140 - #17ddff
    260  140 - #b4b8bd
     76  148 - #c9ced3
    336  148 - #ebf0f7
    260  152 - #b4b8bd
    156  160 - #fafaff
    260  160 - #b4b8bd
     80  168 - #ccd6e6
    260  168 - #b4b8bd
    112  172 - #232733
    196  172 - #fafaff
    112  176 - #232733
    260  176 - #b4b8bd
    260  184 - #b4b8bd
     76  188 - #c9ced3
    308  188 - #ebf0f7
    260  192 - #b4b8bd
    260  200 - #b4b8bd
     80  208 - #ccd6e6
    112  208 - #232733
    260  208 - #b4b8bd
    112  212 - #232733
    124  212 - #1a1f2b
    128  212 - #262b37
    136  212 - #fafaff
    260  216 - #b4b8bd
    260  224 - #b4b8bd
    336  224 - #ebf0f7
     80  228 - #b1b5ba
     96  232 - #9ea2a6
    104  232 - #9ea2a6
    116  232 - #9ea2a6
    128  232 - #9ea2a6
    140  232 - #9ea2a6
    148  232 - #9ea2a6
    164  232 - #9ea2a6
    180  232 - #9ea2a6
    196  232 - #9ea2a6
    208  232 - #9ea2a6
    212  232 - #9ea2a6
    216  232 - #9ea2a6
    220  232 - #9ea2a6
    224  232 - #9ea2a6
    228  232 - #9ea2a6
    232  232 - #9ea2a6
    236  232 - #9ea2a6
    240  232 - #9ea2a6
    244  232 - #9ea2a6
    248  232 - #9ea2a6
    252  232 - #a1a4a9
     88  236 - #bcc0c6
    108  236 - #bbbfc5
    156  236 - #bbbfc5
    172  236 - #bbbfc5
    188  236 - #bbbfc5
    200  236 - #bbbfc5
    212  236 - #bbbfc5
    224  236 - #bbbfc5
    248  236 - #bbbfc5
    592  236 - #597c95
    120  240 - #d6dae1
    132  240 - #d6dae1
    144  240 - #d6dae1
    204  240 - #d6dae1
    216  240 - #d6dae1
    232  240 - #d6dae1
     40  252 - #ebf0f7
    296  260 - #ebf0f7
     92  280 - #ebf0f7
    464  280 - #597c95
     48  296 - #ebf0f7
    132  296 - #ebf0f7
    196  296 - #ebf0f7
    264  296 - #ebf0f7
    328  296 - #ebf0f7
    592  356 - #597c95
    404  364 - #597c95
     92  384 - #597c95
    244  404 - #597c95
    492  416 - #597c95
      4  440 - #597c95
    344  444 - #597c95
    152  460 - #597c95
    592  476 - #597c95
    244  504 - #597c95
     68  516 - #597c95
    444  556 - #597c95
    300  588 - #597c95
      4  592 - #597c95
    164  592 - #597c95
    592  592 - #597c95
";

const TOUCHES_2: &str = "
    170 174 b
    170 174 e
";

const COLORS_3: &str = r"
      4    4 - #597c95
    512    4 - #597c95
    580    4 - #597c95
    420    8 - #597c95
    108   40 - #ebf0f7
    156   40 - #ebf0f7
    220   40 - #ebf0f7
    280   40 - #ebf0f7
     48   44 - #ebf0f7
    336   48 - #ebf0f7
    460   56 - #597c95
    252   60 - #ebf0f7
    136   64 - #ebf0f7
    188   68 - #ebf0f7
    400   68 - #597c95
    592   68 - #597c95
    304   76 - #ebf0f7
     80   80 - #ccd6e6
    516   80 - #597c95
     80   84 - #ccd6e6
     80   88 - #ccd6e6
    112   88 - #232733
    216   88 - #fafaff
     80   92 - #ccd6e6
    112   92 - #232733
    152   92 - #fafaff
     40   96 - #ebf0f7
     80   96 - #ccd6e6
    248   96 - #fafaff
    336   96 - #ebf0f7
     80  100 - #cfd9e7
    280  100 - #ebf0f7
    188  112 - #ebf0f7
    448  116 - #597c95
    308  120 - #ebf0f7
    380  128 - #597c95
    548  128 - #597c95
    100  132 - #ebf0f7
    224  132 - #ebf0f7
    268  132 - #ebf0f7
     40  136 - #ebf0f7
    136  140 - #ebf0f7
    332  152 - #ebf0f7
    160  164 - #ebf0f7
    192  164 - #ebf0f7
     84  168 - #ebf0f7
    120  168 - #ebf0f7
    244  168 - #ebf0f7
    592  168 - #597c95
    292  172 - #ebf0f7
    508  172 - #597c95
     44  176 - #ebf0f7
    416  188 - #597c95
    144  192 - #ebf0f7
    184  192 - #ebf0f7
    224  192 - #ebf0f7
    336  192 - #ebf0f7
    104  200 - #ebf0f7
    304  204 - #ebf0f7
     72  208 - #ebf0f7
    248  212 - #ebf0f7
     40  216 - #ebf0f7
    204  216 - #ebf0f7
    172  220 - #ebf0f7
    284  232 - #ebf0f7
    540  232 - #597c95
    144  236 - #ebf0f7
    100  240 - #ebf0f7
    228  240 - #ebf0f7
    332  240 - #ebf0f7
    472  244 - #597c95
     60  256 - #ebf0f7
      4  260 - #597c95
    124  264 - #ebf0f7
    176  264 - #ebf0f7
    268  264 - #ebf0f7
    304  268 - #ebf0f7
    404  268 - #597c95
    232  272 - #ebf0f7
    588  280 - #597c95
     92  284 - #ebf0f7
    336  288 - #ebf0f7
     48  296 - #ebf0f7
    132  296 - #ebf0f7
    168  296 - #ebf0f7
    200  296 - #ebf0f7
    280  296 - #ebf0f7
    512  308 - #597c95
    244  344 - #597c95
    428  344 - #597c95
    592  348 - #597c95
     52  360 - #597c95
    184  364 - #597c95
    344  372 - #597c95
    476  376 - #597c95
    536  380 - #597c95
    104  392 - #597c95
      4  400 - #597c95
    424  400 - #597c95
    260  404 - #597c95
    200  436 - #597c95
    472  436 - #597c95
    528  440 - #597c95
    592  440 - #597c95
    388  444 - #597c95
     88  452 - #597c95
      4  456 - #597c95
    316  472 - #597c95
    148  480 - #597c95
     44  496 - #597c95
    532  496 - #597c95
    244  500 - #597c95
    440  504 - #597c95
    100  512 - #597c95
    296  524 - #597c95
    592  524 - #597c95
    188  528 - #597c95
    372  532 - #597c95
      4  536 - #597c95
    240  564 - #597c95
    520  568 - #597c95
     96  572 - #597c95
    448  576 - #597c95
    300  580 - #597c95
      4  592 - #597c95
    184  592 - #597c95
    388  592 - #597c95
    592  592 - #597c95
";
