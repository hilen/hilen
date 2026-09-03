use anyhow::Result;
use hilen::{
    refs::Weak,
    ui::{Color, Container, DropDown, Setup, ViewData, ViewTest, view},
    ui_test::{check_colors, inject_touches, set_record_probe_count},
};

/// A styled drop down on a card: its own color, border and corners,
/// dark text and the chevron, closed and then open. Pins that the view
/// setters style the box and that the open list wears the same look.
#[view]
struct DropDownStyle {
    #[init]
    card: Container,
    drop: DropDown<&'static str>,
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
        anyhow::ensure!(view.drop.value() == &"Two", "picked {}", view.drop.value());
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
    164   40 - #ebf0f7
    236   40 - #ebf0f7
    44   48 - #ebf0f7
    336   48 - #ebf0f7
    284   52 - #ebf0f7
    200   68 - #ebf0f7
    80   84 - #00daff
    112   88 - #fafaff
    112   92 - #fafaff
    120   92 - #323742
    80   96 - #00daff
    308   96 - #ebf0f7
    592  112 - #597c95
    100  116 - #00daff
    112  116 - #00daff
    124  116 - #00daff
    144  116 - #00daff
    164  116 - #00daff
    180  116 - #00daff
    200  116 - #00daff
    208  116 - #00daff
    216  116 - #00daff
    228  116 - #00daff
    236  116 - #00daff
    248  116 - #00daff
    256  116 - #cdd1d8
    76  124 - #ced2d9
    260  128 - #b4b8bd
    484  128 - #597c95
    80  132 - #00daff
    112  136 - #e6f7ff
    244  136 - #00daff
    260  136 - #b4b8bd
    76  140 - #c9ced3
    112  140 - #e6f7ff
    120  140 - #19ddff
    260  144 - #b4b8bd
    336  148 - #ebf0f7
    80  152 - #00daff
    260  152 - #b4b8bd
    264  160 - #d0d4da
    80  164 - #00daff
    260  168 - #b4b8bd
    76  172 - #c9ced3
    112  172 - #232733
    112  176 - #232733
    188  176 - #fafaff
    260  176 - #b4b8bd
    80  184 - #00daff
    260  184 - #b4b8bd
    308  188 - #ebf0f7
    76  192 - #c9ced3
    260  192 - #b4b8bd
    260  196 - #b4b8bd
    260  200 - #b4b8bd
    80  204 - #00daff
    260  204 - #b4b8bd
    112  208 - #232733
    260  208 - #b4b8bd
    76  212 - #c9ced3
    112  212 - #232733
    124  212 - #1a1f2b
    128  212 - #2f333f
    136  212 - #fafaff
    260  212 - #b4b8bd
    260  216 - #b4b8bd
    260  220 - #b4b8bd
    260  224 - #b4b8bd
    336  224 - #ebf0f7
    80  228 - #b1b5ba
    260  228 - #b8bcc1
    88  232 - #9fa3a7
    96  232 - #9ea2a6
    108  232 - #9ea2a6
    116  232 - #9ea2a6
    128  232 - #9ea2a6
    140  232 - #9ea2a6
    148  232 - #9ea2a6
    156  232 - #9ea2a6
    164  232 - #9ea2a6
    172  232 - #9ea2a6
    180  232 - #9ea2a6
    188  232 - #9ea2a6
    196  232 - #9ea2a6
    208  232 - #9ea2a6
    220  232 - #9ea2a6
    232  232 - #9ea2a6
    240  232 - #9ea2a6
    248  232 - #9ea2a6
    252  232 - #a1a4a9
    592  236 - #597c95
    88  240 - #d6dbe1
    100  240 - #d6dae1
    112  240 - #d6dae1
    124  240 - #d6dae1
    140  240 - #d6dae1
    156  240 - #d6dae1
    172  240 - #d6dae1
    188  240 - #d6dae1
    204  240 - #d6dae1
    216  240 - #d6dae1
    232  240 - #d6dae1
    244  240 - #d6dae1
    252  240 - #d7dce2
    40  252 - #ebf0f7
    300  256 - #ebf0f7
    464  280 - #597c95
    48  296 - #ebf0f7
    128  296 - #ebf0f7
    188  296 - #ebf0f7
    268  296 - #ebf0f7
    328  296 - #ebf0f7
    592  356 - #597c95
    492  416 - #597c95
    4  440 - #597c95
    152  444 - #597c95
    344  444 - #597c95
    592  476 - #597c95
    248  492 - #597c95
    76  516 - #597c95
    444  556 - #597c95
    300  588 - #597c95
    4  592 - #597c95
    152  592 - #597c95
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
    104   40 - #ebf0f7
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
    516   80 - #597c95
    80   84 - #00daff
    80   88 - #00daff
    112   88 - #232733
    216   88 - #fafaff
    40   92 - #ebf0f7
    80   92 - #00daff
    112   92 - #232733
    152   92 - #fafaff
    80   96 - #00daff
    248   96 - #fafaff
    336   96 - #ebf0f7
    280  100 - #ebf0f7
    188  112 - #ebf0f7
    448  116 - #597c95
    308  120 - #ebf0f7
    100  128 - #ebf0f7
    380  128 - #597c95
    548  128 - #597c95
    40  132 - #ebf0f7
    224  132 - #ebf0f7
    268  132 - #ebf0f7
    136  140 - #ebf0f7
    296  152 - #ebf0f7
    332  152 - #ebf0f7
    84  164 - #ebf0f7
    160  164 - #ebf0f7
    192  164 - #ebf0f7
    120  168 - #ebf0f7
    244  168 - #ebf0f7
    592  168 - #597c95
    44  172 - #ebf0f7
    508  172 - #597c95
    216  188 - #ebf0f7
    288  188 - #ebf0f7
    416  188 - #597c95
    144  192 - #ebf0f7
    336  192 - #ebf0f7
    180  196 - #ebf0f7
    104  200 - #ebf0f7
    40  212 - #ebf0f7
    76  216 - #ebf0f7
    248  216 - #ebf0f7
    312  216 - #ebf0f7
    204  220 - #ebf0f7
    540  232 - #597c95
    144  236 - #ebf0f7
    176  236 - #ebf0f7
    100  240 - #ebf0f7
    288  240 - #ebf0f7
    336  240 - #ebf0f7
    228  244 - #ebf0f7
    472  244 - #597c95
    60  248 - #ebf0f7
    4  256 - #597c95
    124  264 - #ebf0f7
    264  264 - #ebf0f7
    176  268 - #ebf0f7
    308  268 - #ebf0f7
    404  268 - #597c95
    232  276 - #ebf0f7
    88  280 - #ebf0f7
    588  280 - #597c95
    336  288 - #ebf0f7
    48  292 - #ebf0f7
    128  296 - #ebf0f7
    160  296 - #ebf0f7
    200  296 - #ebf0f7
    284  296 - #ebf0f7
    512  308 - #597c95
    376  328 - #597c95
    428  344 - #597c95
    248  348 - #597c95
    592  348 - #597c95
    48  356 - #597c95
    188  360 - #597c95
    344  372 - #597c95
    476  376 - #597c95
    536  380 - #597c95
    104  392 - #597c95
    424  400 - #597c95
    4  404 - #597c95
    260  408 - #597c95
    192  428 - #597c95
    60  436 - #597c95
    472  436 - #597c95
    528  440 - #597c95
    592  440 - #597c95
    388  444 - #597c95
    4  464 - #597c95
    312  468 - #597c95
    148  480 - #597c95
    532  496 - #597c95
    48  500 - #597c95
    240  504 - #597c95
    440  504 - #597c95
    104  516 - #597c95
    296  524 - #597c95
    592  524 - #597c95
    180  532 - #597c95
    372  532 - #597c95
    4  536 - #597c95
    240  568 - #597c95
    520  568 - #597c95
    96  576 - #597c95
    448  576 - #597c95
    300  580 - #597c95
    4  592 - #597c95
    184  592 - #597c95
    388  592 - #597c95
    592  592 - #597c95
";
