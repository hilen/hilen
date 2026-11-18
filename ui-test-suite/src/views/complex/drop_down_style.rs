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
    fn setup(self: Weak<Self>) {
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
     100   40 - #ebf0f7
     148   40 - #ebf0f7
     184   40 - #ebf0f7
     216   40 - #ebf0f7
     280   40 - #ebf0f7
      48   44 - #ebf0f7
     336   48 - #ebf0f7
     460   56 - #597c95
     124   64 - #ebf0f7
     252   64 - #ebf0f7
     592   68 - #597c95
     180   72 - #fafaff
     396   76 - #597c95
      80   80 - #ccd6e6
     516   80 - #597c95
      80   84 - #ccd6e6
     312   84 - #ebf0f7
      80   88 - #ccd6e6
      96   88 - #fafaff
     224   88 - #fafaff
      40   92 - #ebf0f7
      80   92 - #ccd6e6
      96   92 - #fafaff
     104   92 - #313541
      80   96 - #ccd6e6
     144   96 - #fafaff
     272   96 - #ebf0f7
      80  100 - #cfd9e7
     180  116 - #ebf0f7
     300  116 - #ebf0f7
     448  116 - #597c95
     336  120 - #ebf0f7
     548  128 - #597c95
      40  132 - #ebf0f7
      92  132 - #ebf0f7
     220  132 - #ebf0f7
     260  132 - #ebf0f7
     392  136 - #597c95
     128  140 - #ebf0f7
     292  152 - #ebf0f7
     328  156 - #ebf0f7
     156  160 - #ebf0f7
      80  164 - #ebf0f7
     112  168 - #ebf0f7
     188  168 - #ebf0f7
     240  168 - #ebf0f7
     592  168 - #597c95
     508  172 - #597c95
      40  176 - #ebf0f7
     336  188 - #ebf0f7
     416  188 - #597c95
     140  192 - #ebf0f7
     216  192 - #ebf0f7
     280  192 - #ebf0f7
     100  200 - #ebf0f7
     312  212 - #ebf0f7
      40  216 - #ebf0f7
     164  216 - #ebf0f7
     244  216 - #ebf0f7
      72  220 - #ebf0f7
     196  220 - #ebf0f7
     128  224 - #ebf0f7
     540  232 - #597c95
     288  236 - #ebf0f7
     336  240 - #ebf0f7
      96  244 - #ebf0f7
     472  244 - #597c95
     152  248 - #ebf0f7
      56  256 - #ebf0f7
     196  260 - #ebf0f7
     124  264 - #ebf0f7
     268  264 - #ebf0f7
     304  268 - #ebf0f7
     404  268 - #597c95
     232  272 - #ebf0f7
     588  280 - #597c95
      92  288 - #ebf0f7
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
     520    4 - #597c95
     424    8 - #597c95
     128   40 - #ebf0f7
     188   40 - #ebf0f7
     264   40 - #ebf0f7
      48   44 - #ebf0f7
     336   48 - #ebf0f7
     592   76 - #597c95
     220   80 - #fafaff
      80   84 - #00daff
      80   88 - #00daff
      96   88 - #fafaff
     300   88 - #ebf0f7
      80   92 - #00daff
      96   92 - #fafaff
     104   92 - #313541
      80   96 - #00daff
     168  100 - #fafaff
     256  116 - #cdd1d8
      76  124 - #ced2d9
     204  124 - #e6f7ff
     468  124 - #597c95
     260  128 - #b4b8bd
     336  132 - #ebf0f7
      96  136 - #e6f7ff
     244  136 - #00daff
     260  136 - #b4b8bd
      76  140 - #c9ced3
      96  140 - #e6f7ff
     104  140 - #17ddff
     260  144 - #b4b8bd
      76  156 - #c9ced3
     260  156 - #b4b8bd
     176  160 - #fafaff
     260  164 - #b4b8bd
      96  172 - #232733
     260  172 - #b4b8bd
      76  176 - #c9ced3
      96  176 - #232733
     260  176 - #b4b8bd
     312  176 - #ebf0f7
     260  180 - #b4b8bd
     592  180 - #597c95
     216  184 - #fafaff
     260  184 - #b4b8bd
     260  188 - #b4b8bd
      76  192 - #c9ced3
     260  192 - #b4b8bd
     260  196 - #b4b8bd
     260  200 - #b4b8bd
     260  204 - #b4b8bd
      96  208 - #232733
     260  208 - #b4b8bd
      76  212 - #c9ced3
      96  212 - #232733
     108  212 - #1a1f2b
     112  212 - #262b37
     120  212 - #fafaff
     260  212 - #b4b8bd
     260  216 - #b4b8bd
     260  220 - #b4b8bd
     336  220 - #ebf0f7
     260  224 - #b4b8bd
      80  228 - #b1b5ba
     260  228 - #b8bcc1
      92  232 - #9ea2a6
     100  232 - #9ea2a6
     108  232 - #9ea2a6
     116  232 - #9ea2a6
     124  232 - #9ea2a6
     132  232 - #9ea2a6
     140  232 - #9ea2a6
     148  232 - #9ea2a6
     156  232 - #9ea2a6
     164  232 - #9ea2a6
     172  232 - #9ea2a6
     180  232 - #9ea2a6
     192  232 - #9ea2a6
     204  232 - #9ea2a6
     212  232 - #9ea2a6
     220  232 - #9ea2a6
     232  232 - #9ea2a6
     236  232 - #9ea2a6
     240  232 - #9ea2a6
     244  232 - #9ea2a6
     248  232 - #9ea2a6
     252  232 - #a1a4a9
     252  236 - #bdc1c7
      88  240 - #d6dbe1
     104  240 - #d6dae1
     124  240 - #d6dae1
     140  240 - #d6dae1
     148  240 - #d6dae1
     156  240 - #d6dae1
     164  240 - #d6dae1
     172  240 - #d6dae1
     180  240 - #d6dae1
     188  240 - #d6dae1
     200  240 - #d6dae1
     208  240 - #d6dae1
     216  240 - #d6dae1
     228  240 - #d6dae1
     240  240 - #d6dae1
      40  252 - #ebf0f7
     300  256 - #ebf0f7
     472  256 - #597c95
     224  284 - #ebf0f7
      48  296 - #ebf0f7
     120  296 - #ebf0f7
     180  296 - #ebf0f7
     264  296 - #ebf0f7
     328  296 - #ebf0f7
     592  296 - #597c95
     424  352 - #597c95
     524  388 - #597c95
     268  412 - #597c95
       4  440 - #597c95
     156  440 - #597c95
     376  448 - #597c95
     480  480 - #597c95
     588  480 - #597c95
     232  508 - #597c95
      80  512 - #597c95
     448  576 - #597c95
     152  588 - #597c95
     300  588 - #597c95
       4  592 - #597c95
     592  592 - #597c95
";

const TOUCHES_2: &str = "
    170 174 b
    170 174 e
";

const COLORS_3: &str = r"
         4    4 - #597c95
       416    4 - #597c95
       508    4 - #597c95
       568    4 - #597c95
        96   40 - #ebf0f7
       140   40 - #ebf0f7
       176   40 - #ebf0f7
       208   40 - #ebf0f7
       276   40 - #ebf0f7
        48   44 - #ebf0f7
       332   44 - #ebf0f7
       464   52 - #597c95
       244   60 - #ebf0f7
       592   60 - #597c95
       120   64 - #ebf0f7
       172   72 - #fafaff
       300   72 - #ebf0f7
        44   76 - #ebf0f7
       396   76 - #597c95
        80   80 - #ccd6e6
        80   84 - #ccd6e6
        80   88 - #ccd6e6
        96   88 - #232733
        80   92 - #ccd6e6
        96   92 - #232733
       136   92 - #fafaff
       220   92 - #fafaff
        80   96 - #ccd6e6
       272   96 - #ebf0f7
       328   96 - #ebf0f7
        80  100 - #cfd9e7
       528  100 - #597c95
        40  108 - #ebf0f7
       176  116 - #ebf0f7
       456  116 - #597c95
       244  128 - #ebf0f7
       300  128 - #ebf0f7
       128  132 - #ebf0f7
       272  140 - #ebf0f7
        84  144 - #ebf0f7
       204  144 - #ebf0f7
       336  148 - #ebf0f7
        40  152 - #ebf0f7
       164  152 - #ebf0f7
       404  156 - #597c95
       112  160 - #ebf0f7
       592  160 - #597c95
       236  168 - #ebf0f7
       288  168 - #ebf0f7
       512  172 - #597c95
        48  184 - #ebf0f7
       136  184 - #ebf0f7
       188  184 - #ebf0f7
       456  184 - #597c95
        84  188 - #ebf0f7
       216  192 - #ebf0f7
       336  196 - #ebf0f7
       260  200 - #ebf0f7
       112  204 - #ebf0f7
        40  216 - #ebf0f7
       192  216 - #ebf0f7
       140  220 - #ebf0f7
       224  220 - #ebf0f7
       296  220 - #ebf0f7
       388  220 - #597c95
        72  224 - #ebf0f7
       540  232 - #597c95
       100  240 - #ebf0f7
       172  240 - #ebf0f7
       472  240 - #597c95
       336  244 - #ebf0f7
       204  252 - #ebf0f7
       268  252 - #ebf0f7
        60  256 - #ebf0f7
       136  256 - #ebf0f7
         4  260 - #597c95
       304  264 - #ebf0f7
       164  272 - #ebf0f7
       232  272 - #ebf0f7
       592  276 - #597c95
        92  284 - #ebf0f7
       408  284 - #597c95
       336  288 - #ebf0f7
        48  296 - #ebf0f7
       132  296 - #ebf0f7
       184  296 - #ebf0f7
       280  296 - #ebf0f7
       516  304 - #597c95
       376  332 - #597c95
       592  340 - #597c95
       240  344 - #597c95
       436  344 - #597c95
        52  360 - #597c95
       184  376 - #597c95
       340  376 - #597c95
       540  376 - #597c95
       400  388 - #597c95
       104  392 - #597c95
         4  400 - #597c95
       260  404 - #597c95
       472  424 - #597c95
       528  432 - #597c95
       204  440 - #597c95
       592  440 - #597c95
       392  444 - #597c95
        88  452 - #597c95
         4  456 - #597c95
       316  468 - #597c95
       148  480 - #597c95
       528  492 - #597c95
        44  496 - #597c95
       244  500 - #597c95
       448  500 - #597c95
       100  512 - #597c95
       592  520 - #597c95
       308  524 - #597c95
       188  528 - #597c95
       372  532 - #597c95
         4  536 - #597c95
       240  564 - #597c95
       520  568 - #597c95
        96  572 - #597c95
       300  580 - #597c95
       448  580 - #597c95
         4  592 - #597c95
       184  592 - #597c95
       360  592 - #597c95
       592  592 - #597c95
";
