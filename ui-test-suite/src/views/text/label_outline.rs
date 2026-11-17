use anyhow::Result;
use hilen::{
    refs::Weak,
    ui::{BLACK, Color, Container, Label, Setup, ViewData, ViewFrame, ViewTest, WHITE, view},
    ui_test::{check_colors, set_record_probe_count},
};

const CHECK_1: &str = r"
       4    4 - #ffffff
     116    4 - #ffffff
     180    4 - #ffffff
     256    4 - #ffffff
     312    4 - #000000
     376    4 - #000000
     428    4 - #000000
     492    4 - #000000
     592    4 - #000000
      60   16 - #ffffff
     544   24 - #000000
     340   52 - #6e6e6e
     436   52 - #6e6e6e
     448   52 - #6e6e6e
     328   56 - #2d2d2d
      44   60 - #ffffff
     312   60 - #8e8e8e
     328   60 - #2d2d2d
     392   60 - #dadada
     436   60 - #000000
      92   64 - #ffffff
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
     220   72 - #ffffff
     448   72 - #ffffff
     528   72 - #000000
     592   72 - #000000
     156   84 - #ffffff
       8   92 - #ffffff
     112  108 - #ffffff
     264  108 - #ffffff
     488  108 - #000000
     344  112 - #000000
     412  112 - #000000
     544  120 - #000000
      60  136 - #ffffff
     592  136 - #000000
     172  148 - #000000
     376  148 - #d9d9d9
     200  152 - #6e6e6e
     376  152 - #d9d9d9
     464  152 - #6e6e6e
     124  156 - #ffffff
     144  156 - #9f9f9f
     224  156 - #ffffff
     276  156 - #ffffff
     308  156 - #9d9d9d
     376  156 - #d9d9d9
     144  160 - #9f9f9f
     172  160 - #000000
     288  160 - #393939
     308  160 - #9d9d9d
     328  160 - #000000
     344  160 - #dadada
     376  160 - #d9d9d9
     400  160 - #fefefe
     464  160 - #000000
     472  160 - #dadada
     144  164 - #9f9f9f
     160  164 - #ffffff
     192  164 - #ffffff
     216  164 - #000000
     228  164 - #262626
     232  164 - #262626
     236  164 - #262626
     244  164 - #fefefe
     344  164 - #dadada
     356  164 - #f9f9f9
     376  164 - #d9d9d9
     392  164 - #262626
     396  164 - #262626
     400  164 - #262626
     424  164 - #000000
     436  164 - #262626
     440  164 - #262626
     444  164 - #262626
     472  164 - #dadada
     120  168 - #000000
     204  168 - #6e6e6e
     376  168 - #d9d9d9
     536  168 - #000000
     132  172 - #ffffff
     172  172 - #ffffff
     252  172 - #fefefe
     280  172 - #fefefe
     300  172 - #fefefe
     324  172 - #fefefe
       4  180 - #ffffff
      68  188 - #ffffff
     252  212 - #ffffff
     352  212 - #000000
     196  216 - #ffffff
     520  216 - #000000
     572  224 - #000000
      44  236 - #ffffff
     128  248 - #555555
     396  248 - #a6a6a6
     396  252 - #a6a6a6
     436  252 - #6e6e6e
     484  252 - #6e6e6e
     152  256 - #000000
     332  256 - #373737
     396  256 - #a6a6a6
     112  260 - #ffffff
     160  260 - #202020
     228  260 - #000000
     268  260 - #000000
     308  260 - #000000
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
     208  264 - #010101
     236  264 - #000000
     252  264 - #525252
     328  264 - #6a6a6a
     332  264 - #373737
     352  264 - #ffffff
     384  264 - #c1c1c1
     396  264 - #a6a6a6
     412  264 - #262626
     416  264 - #262626
     420  264 - #262626
     444  264 - #000000
     456  264 - #262626
     460  264 - #262626
     464  264 - #262626
     492  264 - #a7a7a7
     128  268 - #555555
     176  268 - #ffffff
     196  268 - #ffffff
     272  268 - #ffffff
     332  268 - #373737
     396  268 - #a6a6a6
     128  272 - #555555
     148  272 - #ffffff
     220  272 - #000000
     304  272 - #fefefe
     320  272 - #ffffff
     344  272 - #ffffff
     560  272 - #000000
     112  276 - #000000
      12  300 - #ffffff
      68  308 - #ffffff
     528  308 - #000000
     592  308 - #000000
     192  344 - #c0c0c0
     268  344 - #c0c0c0
     292  344 - #808080
     356  344 - #808080
     380  344 - #7f7f7f
     180  348 - #c0c0c0
     196  348 - #c2c2c2
     264  348 - #c9c9c9
     268  348 - #c0c0c0
     272  348 - #e5e5e5
     292  348 - #808080
     296  348 - #c0c0c0
     340  348 - #808080
     420  348 - #808080
     464  348 - #808080
     176  352 - #9b9b9b
     184  352 - #9b9b9b
     268  352 - #bfbfbf
     272  352 - #e5e5e5
     292  352 - #808080
     296  352 - #c0c0c0
       4  356 - #ffffff
     140  356 - #808080
     160  356 - #8e8e8e
     168  356 - #adadad
     180  356 - #c0c0c0
     192  356 - #c0c0c0
     200  356 - #9d9d9d
     204  356 - #8b8b8b
     212  356 - #c0c0c0
     224  356 - #c0c0c0
     240  356 - #808080
     256  356 - #bfbfbf
     272  356 - #e5e5e5
     292  356 - #808080
     296  356 - #c0c0c0
     308  356 - #7f7f7f
     328  356 - #7f7f7f
     392  356 - #000000
     408  356 - #7f7f7f
     420  356 - #808080
     436  356 - #000000
     452  356 - #7f7f7f
      60  360 - #ffffff
     148  360 - #c0c0c0
     156  360 - #c0c0c0
     160  360 - #8e8e8e
     168  360 - #adadad
     176  360 - #dedede
     184  360 - #d2d2d2
     200  360 - #9d9d9d
     204  360 - #8b8b8b
     228  360 - #b6b6b6
     232  360 - #c0c0c0
     272  360 - #e5e5e5
     292  360 - #808080
     296  360 - #c0c0c0
     320  360 - #000000
     340  360 - #808080
     352  360 - #454545
     372  360 - #222222
     376  360 - #2c2c2c
     444  360 - #808080
     464  360 - #808080
     588  360 - #000000
     132  364 - #c0c0c0
     136  364 - #808080
     148  364 - #c0c0c0
     156  364 - #c0c0c0
     160  364 - #8e8e8e
     176  364 - #dedede
     184  364 - #d2d2d2
     192  364 - #c0c0c0
     200  364 - #9d9d9d
     204  364 - #8b8b8b
     212  364 - #c0c0c0
     224  364 - #c0c0c0
     228  364 - #b6b6b6
     232  364 - #c0c0c0
     236  364 - #898989
     240  364 - #898989
     244  364 - #898989
     264  364 - #c9c9c9
     272  364 - #e5e5e5
     296  364 - #bfbfbf
     308  364 - #7f7f7f
     352  364 - #454545
     372  364 - #222222
     376  364 - #2c2c2c
     400  364 - #767676
     420  364 - #808080
     536  364 - #000000
     148  368 - #c0c0c0
     156  368 - #bfbfbf
     168  368 - #c0c0c0
     180  368 - #c0c0c0
     200  368 - #9d9d9d
     204  368 - #8b8b8b
     212  368 - #c0c0c0
     228  368 - #b6b6b6
     272  368 - #e5e5e5
     316  368 - #7f7f7f
     328  368 - #808080
     340  368 - #808080
     352  368 - #454545
     372  368 - #222222
     376  368 - #2c2c2c
     408  368 - #7f7f7f
     452  368 - #7f7f7f
     464  368 - #808080
     140  372 - #c0c0c0
     144  372 - #c0c0c0
     160  372 - #c0c0c0
     164  372 - #c0c0c0
     172  372 - #808080
     184  372 - #c0c0c0
     192  372 - #c0c0c0
     204  372 - #8b8b8b
     224  372 - #c0c0c0
     240  372 - #c0c0c0
     260  372 - #bfbfbf
     268  372 - #bfbfbf
     276  372 - #c0c0c0
     292  372 - #808080
     308  372 - #7f7f7f
     324  372 - #7f7f7f
     388  372 - #808080
     416  372 - #808080
     440  372 - #808080
     460  372 - #808080
     276  376 - #c0c0c0
     500  400 - #000000
       4  416 - #ffffff
      96  416 - #ffffff
     592  416 - #000000
     252  436 - #597c95
     420  436 - #597c95
     508  452 - #597c95
     356  456 - #597c95
     192  460 - #597c95
     128  476 - #597c95
       4  484 - #597c95
     276  492 - #597c95
     428  500 - #597c95
      68  504 - #597c95
     592  516 - #597c95
     496  524 - #597c95
     364  528 - #597c95
     232  532 - #597c95
     116  536 - #597c95
     172  544 - #597c95
     296  548 - #597c95
     592  588 - #597c95
       4  592 - #597c95
      96  592 - #597c95
     248  592 - #597c95
     336  592 - #597c95
     428  592 - #597c95
     524  592 - #597c95
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
