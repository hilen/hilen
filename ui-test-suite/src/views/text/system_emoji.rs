use anyhow::Result;
use hilen::{
    dispatch::from_main,
    gm::LossyConvert,
    refs::{Weak, manage::DataManager},
    ui::{Font, Label, Setup, TextAlignment, ViewFrame, ViewSubviews, ViewTest, view},
    ui_test::{check_colors, checkpoint, set_record_probe_count},
};

/// The same emoji as `Color emoji`, so the rows compare.
const EMOJI: &str = "😀🐶🍕🚀🎉👍🌈🔥";

/// Roboto text with emoji it has no glyphs for.
const MIXED: &str = "Hi 😀 there 🚀 ok";

/// Apple Color Emoji from the system, an sbix font mapped from disk, over
/// the two bundled fonts the other platforms draw with.
#[view]
struct SystemEmoji {
    rows:  Vec<Weak<Label>>,
    #[init]
    mixed: Label,
    /// The system strikes far below their own size, right aligned.
    #[init]
    small: Label,
    /// The system strikes above the UI size, centered.
    #[init]
    big:   Label,
}

fn system_font() -> Weak<Font> {
    Font::system_emoji().expect("Apple Color Emoji is on every Apple device")
}

impl Setup for SystemEmoji {
    fn setup(mut self: Weak<Self>) {
        let fonts = [
            system_font(),
            Font::get("TwemojiColr0.ttf"),
            Font::get("NotoColorEmojiColr1.ttf"),
        ];
        for (row, font) in fonts.into_iter().enumerate() {
            let y: f32 = 4.0 + 66.0 * row.lossy_convert();
            let label = self.add_view::<Label>();
            label
                .set_frame((6.0, y, 588.0, 60.0))
                .set_font(font)
                .set_text(EMOJI)
                .set_text_size(40)
                .set_alignment(TextAlignment::Left);
            self.rows.push(label);
        }

        self.mixed
            .set_frame((6, 208, 588, 56))
            .set_text(MIXED)
            .set_text_size(36)
            .set_alignment(TextAlignment::Left);

        self.small
            .set_frame((6, 270, 588, 30))
            .set_font(system_font())
            .set_text(EMOJI)
            .set_text_size(16)
            .set_alignment(TextAlignment::Right);

        self.big
            .set_frame((6, 310, 588, 140))
            .set_font(system_font())
            .set_text("😀🐶🍕🚀")
            .set_text_size(96)
            .set_alignment(TextAlignment::Center);
    }
}

impl ViewTest for SystemEmoji {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        set_record_probe_count(120);

        check_colors(NO_FALLBACK)?;
        checkpoint("the system font over the bundled ones, the Roboto label shows boxes")?;

        from_main(|| {
            Font::set_fallbacks([system_font()]);
        });

        check_colors(FALLBACK)?;
        checkpoint("the system font registered as fallback, the Roboto emoji are the platform's")?;

        from_main(Font::reset_fallbacks);

        Ok(())
    }
}

const NO_FALLBACK: &str = r"
             208    8 - #597c95
              32   16 - #fffba4
             244   16 - #ffd55c
              84   20 - #f7f6f5
             328   20 - #ff6933
              76   24 - #dfdfdd
              92   24 - #b6b1ae
             152   24 - #f80807
             124   28 - #eda94f
             312   36 - #ff8e0a
             252   44 - #fcc637
             324   44 - #fcfbd8
              68   84 - #662113
             176   84 - #55acee
             204   84 - #aa8dd8
             288   84 - #eb2027
             164   88 - #55acee
              24   92 - #ffcc4d
              92   92 - #bc7f66
             280   92 - #f19020
             172   96 - #55acee
             200   96 - #a0041e
             296   96 - #226798
             160  100 - #a0041e
             184  100 - #9266cc
             244  100 - #ee9547
             292  100 - #8767ac
             336  104 - #f4900c
              72  108 - #f4c7b5
             124  108 - #be1931
             188  108 - #ea596e
             204  108 - #dd2e44
             268  108 - #f19020
             276  108 - #5c903f
             280  108 - #226798
             216  112 - #77b255
             184  116 - #ea596e
             156  148 - #f5b03e
             396  148 - #f54731
              84  152 - #865b51
             344  152 - #ff5117
             264  156 - #f48fb1
             196  160 - #8dafbf
             304  160 - #a1965b
             356  160 - #97cc37
             116  164 - #865b51
             248  164 - #f48fb1
             364  164 - #00c0e9
             104  168 - #2f2f2f
             236  168 - #703e2d
             348  168 - #97cc37
              84  172 - #ffeac8
             148  172 - #ffcc8a
             356  172 - #8177fa
             392  172 - #fff387
              40  176 - #422b0d
             344  176 - #97cc37
             348  176 - #00c0e9
             408  176 - #ff7700
             100  180 - #ffeac8
             192  184 - #ca2c31
             264  184 - #f48fb1
             288  184 - #b55e19
             344  184 - #42c59c
             328  188 - #78757e
              52  224 - #000000
             104  224 - #2e404d
             188  228 - #597c95
              32  236 - #2f414e
             116  236 - #12191e
             144  244 - #000001
             228  244 - #425b6e
              76  248 - #3e5768
             192  248 - #3e5768
             536  284 - #ffd560
             164  324 - #ffca18
             348  324 - #e0b25e
             484  328 - #98cef4
             248  336 - #faf8f4
             316  340 - #deae58
             120  344 - #ffa407
             364  344 - #c62a11
             460  344 - #000102
             272  348 - #dad7d3
             320  348 - #8a3b0b
             344  352 - #f4b557
             168  356 - #a66707
             224  356 - #9e8a76
             420  356 - #ff0706
             444  356 - #3984be
             440  360 - #327db7
             112  364 - #97774f
             328  368 - #ce220e
             416  368 - #f64e00
             348  372 - #eebc79
             360  372 - #b5150b
             408  372 - #ff9915
             436  372 - #8fb1d0
             456  372 - #235074
             156  376 - #602800
             256  376 - #b76f0a
             292  376 - #864e03
             116  380 - #d16b00
             172  380 - #dee0e1
             376  380 - #937d60
             436  380 - #590004
             140  384 - #7a3d00
             192  384 - #d37a06
             432  384 - #550000
             452  384 - #eb0200
             352  388 - #e6ba78
             276  392 - #d4d2cf
             368  392 - #da973c
             232  396 - #cac5c0
             264  400 - #a6998c
             436  400 - #fb4f00
             156  408 - #a1763d
             248  408 - #e27687
             252  408 - #e97a8c
             592  592 - #597c95
            ";

const FALLBACK: &str = r"
              32   16 - #fffba4
              80   16 - #fdfcf8
              92   24 - #b6b1ae
             204   24 - #fdd948
             320   24 - #ff9c47
             124   28 - #eda94f
             336   32 - #ec1f00
              76   40 - #dedddc
             164   40 - #f30300
             252   44 - #fcc637
              68   84 - #662113
             176   84 - #55acee
             204   84 - #aa8dd8
             164   88 - #55acee
              24   92 - #ffcc4d
              92   92 - #bc7f66
             280   92 - #f19020
             172   96 - #55acee
             248   96 - #f2a64d
             292   96 - #226798
             304   96 - #f4900c
             160  100 - #a0041e
             184  100 - #9266cc
             204  100 - #a0041e
             292  100 - #8767ac
              96  104 - #d99e82
             336  104 - #f4900c
              72  108 - #f4c7b5
             124  108 - #be1931
             188  108 - #ea596e
             276  108 - #5c903f
             196  112 - #dd2e44
             216  112 - #77b255
             184  116 - #ea596e
             264  116 - #eb2027
             276  116 - #226798
             156  148 - #f5b03e
              84  152 - #865b51
             244  152 - #f44336
             344  152 - #ff5117
             400  152 - #f85124
             264  156 - #f48fb1
             196  160 - #8dafbf
             304  160 - #a1965b
             364  160 - #97cc37
             116  164 - #865b51
             248  164 - #f48fb1
             352  164 - #97cc37
             360  164 - #00c0e9
             104  168 - #2f2f2f
             236  168 - #703e2d
              84  172 - #ffeac8
             148  172 - #ffcc8a
             344  172 - #97cc37
             356  172 - #8177fa
             392  172 - #fff387
              40  176 - #422b0d
             408  176 - #ff7700
             100  180 - #ffeac8
             332  180 - #ff8e00
             348  180 - #00c0e9
             192  184 - #ca2c31
             264  184 - #f48fb1
             344  184 - #42c59c
              84  216 - #fffee6
              52  224 - #000000
             124  224 - #314552
             268  224 - #435e71
              28  228 - #394f5f
             164  232 - #020203
              80  240 - #a56707
             248  240 - #000000
              28  244 - #394f5f
              44  244 - #202d36
             136  244 - #161e24
             184  244 - #597c95
             536  284 - #ffd560
             164  324 - #ffca18
             344  324 - #d5a552
             372  328 - #9f460f
             484  328 - #98cef4
             248  336 - #faf8f4
             316  340 - #deae58
             120  344 - #ffa407
             272  348 - #dad7d3
             320  348 - #8a3b0b
             376  348 - #948269
             448  348 - #5d7e90
             344  352 - #f4b557
             168  356 - #a66707
             224  356 - #9e8a76
             444  356 - #3984be
             440  360 - #327db7
             112  364 - #97774f
             328  368 - #ce220e
             360  372 - #b5150b
             408  372 - #ff9915
             436  372 - #8fb1d0
             456  372 - #235074
             156  376 - #602800
             292  376 - #864e03
             372  376 - #cc3012
             116  380 - #d16b00
             172  380 - #dee0e1
             436  380 - #590004
             140  384 - #7a3d00
             192  384 - #d37a06
             376  384 - #8b7f6a
             432  384 - #550000
             252  392 - #191918
             276  392 - #d4d2cf
             356  392 - #e3b05c
             452  392 - #f30300
             232  396 - #cac5c0
             264  400 - #a6998c
             436  400 - #fb4f00
             156  408 - #a1763d
             248  408 - #e27687
             252  408 - #e97a8c
             592  592 - #597c95
            ";
