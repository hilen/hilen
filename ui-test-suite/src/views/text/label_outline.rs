use anyhow::Result;
use hilen::{
    refs::Weak,
    ui::{BLACK, Color, Container, Label, Setup, ViewData, ViewFrame, ViewTest, WHITE, view},
    ui_test::{check_colors, set_record_probe_count},
};

const CHECK_1: &str = r"
       4    4 - #ffffff
     104    4 - #ffffff
     176    4 - #ffffff
     252    4 - #ffffff
     308    4 - #000000
     376    4 - #000000
     428    4 - #000000
     504    4 - #000000
     592    4 - #000000
     548   32 - #000000
      48   36 - #ffffff
     140   40 - #ffffff
     340   52 - #6e6e6e
     436   52 - #6e6e6e
     448   52 - #6e6e6e
     324   56 - #737373
     328   56 - #2d2d2d
     500   56 - #000000
     312   60 - #8e8e8e
     328   60 - #2d2d2d
     392   60 - #dadada
     436   60 - #000000
     592   60 - #000000
     300   64 - #000000
     312   64 - #8e8e8e
     324   64 - #737373
     328   64 - #2d2d2d
     340   64 - #000000
     364   64 - #262626
     368   64 - #262626
     392   64 - #dadada
     408   64 - #262626
     416   64 - #262626
     328   68 - #2d2d2d
      88   72 - #ffffff
     220   72 - #ffffff
     448   72 - #ffffff
      12   80 - #ffffff
     156   88 - #ffffff
     540   88 - #000000
     268  100 - #ffffff
     492  108 - #000000
     112  112 - #ffffff
     416  116 - #000000
     592  116 - #000000
      64  132 - #ffffff
     184  148 - #808080
     260  148 - #808080
     376  148 - #d9d9d9
     200  152 - #6f6f6f
     376  152 - #d9d9d9
     464  152 - #6e6e6e
     536  152 - #000000
       4  156 - #ffffff
     124  156 - #ffffff
     144  156 - #9f9f9f
     156  156 - #626262
     276  156 - #ffffff
     308  156 - #9d9d9d
     376  156 - #d9d9d9
     392  156 - #000000
     144  160 - #9f9f9f
     172  160 - #808080
     188  160 - #808080
     216  160 - #6d6d6d
     228  160 - #343434
     248  160 - #ffffff
     260  160 - #808080
     288  160 - #9c9c9c
     308  160 - #9d9d9d
     328  160 - #000000
     344  160 - #dadada
     376  160 - #d9d9d9
     472  160 - #dadada
     144  164 - #9f9f9f
     228  164 - #262626
     232  164 - #262626
     236  164 - #262626
     344  164 - #dadada
     356  164 - #f9f9f9
     368  164 - #000000
     376  164 - #d9d9d9
     392  164 - #262626
     396  164 - #262626
     400  164 - #262626
     424  164 - #000000
     436  164 - #262626
     440  164 - #262626
     444  164 - #262626
     472  164 - #dadada
     184  168 - #808080
     204  168 - #b7b7b7
     260  168 - #808080
     376  168 - #d9d9d9
     128  172 - #ffffff
     172  172 - #ffffff
     216  172 - #6d6d6d
     248  172 - #fefefe
     280  172 - #fefefe
     300  172 - #fefefe
     324  172 - #fefefe
     476  172 - #ffffff
     592  176 - #000000
      76  188 - #ffffff
     516  208 - #000000
     352  212 - #000000
     196  216 - #ffffff
      40  228 - #ffffff
     560  240 - #000000
     128  248 - #555555
     396  248 - #a6a6a6
     396  252 - #a6a6a6
     436  252 - #6e6e6e
     484  252 - #6e6e6e
     152  256 - #000000
     308  256 - #fefefe
     332  256 - #373737
     396  256 - #a6a6a6
     112  260 - #ffffff
     128  260 - #000000
     160  260 - #202020
     228  260 - #000000
     268  260 - #000000
     328  260 - #6a6a6a
     332  260 - #373737
     344  260 - #000000
     368  260 - #000000
     384  260 - #c1c1c1
     396  260 - #a6a6a6
     476  260 - #ffffff
     492  260 - #a7a7a7
     140  264 - #010101
     160  264 - #202020
     192  264 - #010101
     236  264 - #000000
     252  264 - #525252
     260  264 - #000000
     328  264 - #6a6a6a
     332  264 - #373737
     384  264 - #c1c1c1
     396  264 - #a6a6a6
     412  264 - #262626
     416  264 - #262626
     420  264 - #262626
     444  264 - #000000
     456  264 - #262626
     460  264 - #262626
     464  264 - #262626
     128  268 - #555555
     176  268 - #ffffff
     204  268 - #ffffff
     332  268 - #373737
     396  268 - #a6a6a6
     128  272 - #555555
     148  272 - #ffffff
     220  272 - #000000
     268  272 - #fefefe
     304  272 - #fefefe
     320  272 - #ffffff
     348  272 - #ffffff
     484  272 - #ffffff
     496  272 - #ffffff
     112  276 - #000000
       4  300 - #ffffff
      60  300 - #ffffff
     528  304 - #000000
     592  304 - #000000
     104  324 - #ffffff
     192  344 - #bfbfbf
     268  344 - #bfbfbf
     356  344 - #808080
     380  344 - #7f7f7f
     180  348 - #bfbfbf
     196  348 - #e0e0e0
     264  348 - #e4e4e4
     292  348 - #bfbfbf
     296  348 - #bfbfbf
     340  348 - #808080
     420  348 - #808080
     464  348 - #808080
     176  352 - #9b9b9b
     184  352 - #9b9b9b
     196  352 - #e0e0e0
     268  352 - #bfbfbf
     292  352 - #bfbfbf
     296  352 - #bfbfbf
      56  356 - #ffffff
     160  356 - #c6c6c6
     168  356 - #adadad
     172  356 - #898989
     180  356 - #bfbfbf
     196  356 - #e0e0e0
     200  356 - #9c9c9c
     204  356 - #8a8a8a
     212  356 - #bfbfbf
     224  356 - #bfbfbf
     256  356 - #bfbfbf
     292  356 - #bfbfbf
     296  356 - #bfbfbf
     308  356 - #7f7f7f
     324  356 - #000000
     392  356 - #000000
     408  356 - #7f7f7f
     432  356 - #808080
     452  356 - #7f7f7f
       4  360 - #ffffff
     148  360 - #bfbfbf
     156  360 - #bfbfbf
     160  360 - #c6c6c6
     168  360 - #adadad
     172  360 - #8a8a8a
     184  360 - #e8e8e8
     196  360 - #e0e0e0
     200  360 - #9c9c9c
     204  360 - #8a8a8a
     228  360 - #dadada
     232  360 - #bfbfbf
     240  360 - #999999
     264  360 - #e4e4e4
     292  360 - #bfbfbf
     340  360 - #808080
     352  360 - #454545
     372  360 - #222222
     376  360 - #2c2c2c
     420  360 - #808080
     444  360 - #808080
     464  360 - #808080
     536  360 - #000000
     592  360 - #000000
     132  364 - #bfbfbf
     156  364 - #bfbfbf
     160  364 - #c6c6c6
     172  364 - #8a8a8a
     184  364 - #e8e8e8
     192  364 - #bfbfbf
     196  364 - #e0e0e0
     200  364 - #9c9c9c
     204  364 - #8a8a8a
     208  364 - #bfbfbf
     212  364 - #bfbfbf
     224  364 - #bfbfbf
     228  364 - #dadada
     232  364 - #bfbfbf
     236  364 - #898989
     240  364 - #898989
     244  364 - #898989
     264  364 - #e4e4e4
     308  364 - #7f7f7f
     352  364 - #454545
     372  364 - #222222
     376  364 - #2c2c2c
     392  364 - #000000
     400  364 - #767676
     456  364 - #000000
     148  368 - #bfbfbf
     156  368 - #bfbfbf
     172  368 - #8a8a8a
     180  368 - #bfbfbf
     196  368 - #e0e0e0
     200  368 - #9c9c9c
     204  368 - #8a8a8a
     212  368 - #bfbfbf
     228  368 - #dadada
     268  368 - #bfbfbf
     276  368 - #b3b3b3
     296  368 - #bfbfbf
     316  368 - #7f7f7f
     328  368 - #808080
     340  368 - #808080
     352  368 - #454545
     372  368 - #222222
     376  368 - #2c2c2c
     408  368 - #7f7f7f
     420  368 - #808080
     144  372 - #bfbfbf
     160  372 - #bfbfbf
     164  372 - #bfbfbf
     172  372 - #8a8a8a
     184  372 - #bfbfbf
     192  372 - #bfbfbf
     204  372 - #8a8a8a
     224  372 - #bfbfbf
     240  372 - #bfbfbf
     260  372 - #bfbfbf
     276  372 - #bfbfbf
     280  372 - #bfbfbf
     308  372 - #7f7f7f
     320  372 - #7f7f7f
     392  372 - #808080
     416  372 - #808080
     440  372 - #808080
     460  372 - #808080
     276  376 - #bfbfbf
       4  416 - #ffffff
      88  416 - #ffffff
     500  416 - #000000
     592  416 - #000000
     148  432 - #597c95
     336  436 - #597c95
     260  448 - #597c95
      52  464 - #597c95
     400  464 - #597c95
     192  472 - #597c95
     488  492 - #597c95
     312  500 - #597c95
       8  504 - #597c95
      96  504 - #597c95
     576  504 - #597c95
     244  516 - #597c95
     156  528 - #597c95
     376  528 - #597c95
     444  540 - #597c95
      60  560 - #597c95
     204  572 - #597c95
     496  580 - #597c95
       4  592 - #597c95
     116  592 - #597c95
     296  592 - #597c95
     404  592 - #597c95
     584  592 - #597c95
";

/// White subtitle text over a white band and over a black band, 3 times:
/// plain, with a black outline, and with a black shadow. Proves the outline
/// rings every glyph and the shadow sits behind and beside it, so the text
/// reads over the white band where the plain one is lost, and that a faded
/// label fades its outline with it.
#[view]
struct LabelOutline {
    #[init]
    light:    Container,
    dark:     Container,
    plain:    Label,
    outlined: Label,
    shadowed: Label,
    faded:    Label,
}

impl Setup for LabelOutline {
    fn setup(self: Weak<Self>) {
        self.light.set_frame((0, 0, 300, 420));
        self.light.set_color(WHITE);
        self.dark.set_frame((300, 0, 300, 420));
        self.dark.set_color(BLACK);

        let rows = [
            (self.plain, "plain subtitle text", 30),
            (self.outlined, "outlined subtitle text", 130),
            (self.shadowed, "shadowed subtitle text", 230),
            (self.faded, "outlined, half faded", 330),
        ];
        for (label, text, top) in rows {
            label.set_frame((0, top, 600, 60));
            label.set_text(text).set_text_size(40).set_text_color(WHITE);
        }
        self.outlined.set_text_outline(BLACK, 2);
        self.shadowed.set_text_shadow(Color::rgb(0.0, 0.0, 0.0), (3, 3));
        self.faded.set_text_outline(BLACK, 2).set_opacity(0.5);
    }
}

impl ViewTest for LabelOutline {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        set_record_probe_count(320);
        check_colors(CHECK_1)?;
        Ok(())
    }
}
