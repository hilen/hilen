use anyhow::Result;
use hilen::{
    dispatch::from_main,
    gm::LossyConvert,
    refs::{Weak, manage::DataManager},
    ui::{Font, Label, Setup, TextAlignment, ViewFrame, ViewSubviews, ViewTest, view},
    ui_test::{check_colors, checkpoint, set_record_probe_count},
};

/// The eight emoji every subset font in `assets/fonts` carries.
const EMOJI: &str = "😀🐶🍕🚀🎉👍🌈🔥";

/// One row per color format: `COLRv0` layers, `COLRv1` paint graphs and CBDT
/// PNG strikes. sbix is a PNG strike too and shares the CBDT path.
const FONTS: [&str; 3] = [
    "TwemojiColr0.ttf",
    "NotoColorEmojiColr1.ttf",
    "NotoColorEmojiCbdt.ttf",
];

/// Roboto text with emoji it has no glyphs for. They draw as notdef
/// boxes until a color font is registered as a fallback.
const MIXED: &str = "Hi 😀 there 🚀 ok";

#[view]
struct ColorEmoji {
    rows:  Vec<Weak<Label>>,
    #[init]
    mixed: Label,
    /// A strike drawn far below its own size, right aligned.
    #[init]
    small: Label,
    /// A paint graph drawn far above the UI size.
    #[init]
    big:   Label,
}

impl Setup for ColorEmoji {
    fn setup(mut self: Weak<Self>) {
        for (row, font) in FONTS.into_iter().enumerate() {
            let y: f32 = 4.0 + 66.0 * row.lossy_convert();
            let label = self.add_view::<Label>();
            label
                .set_frame((6.0, y, 588.0, 60.0))
                .set_font(Font::get(font))
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
            .set_font(Font::get("NotoColorEmojiCbdt.ttf"))
            .set_text(EMOJI)
            .set_text_size(16)
            .set_alignment(TextAlignment::Right);

        self.big
            .set_frame((6, 310, 588, 140))
            .set_font(Font::get("NotoColorEmojiColr1.ttf"))
            .set_text("😀🐶🍕🚀")
            .set_text_size(96)
            .set_alignment(TextAlignment::Center);
    }
}

impl ViewTest for ColorEmoji {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        set_record_probe_count(120);

        check_colors(NO_FALLBACK)?;
        checkpoint("three color formats drawn, the Roboto label shows boxes")?;

        from_main(|| {
            Font::set_fallbacks([Font::get("TwemojiColr0.ttf")]);
        });

        check_colors(FALLBACK)?;
        checkpoint("Twemoji registered as fallback, the Roboto emoji are in color")?;

        from_main(Font::reset_fallbacks);

        Ok(())
    }
}

const NO_FALLBACK: &str = r"
             180   16 - #55acee
             204   16 - #aa8dd8
              68   20 - #662113
             120   20 - #be1931
             164   24 - #55acee
             172   24 - #55acee
              36   28 - #664500
             188   28 - #77b255
             288   28 - #5c903f
             172   32 - #55acee
             216   32 - #aa8dd8
             300   32 - #8767ac
              96   36 - #d99e82
              68   40 - #d99e82
             156   40 - #a0041e
             200   40 - #dd2e44
             280   40 - #226798
             284   40 - #8767ac
             336   40 - #f4900c
              48   44 - #664500
             168   44 - #a0041e
             220   48 - #77b255
             276   52 - #226798
             212   80 - #d94227
             356   80 - #ff5117
              96   84 - #b6bab2
             300   88 - #ffca28
             400   88 - #f9571c
             236  100 - #723f2a
             148  104 - #ffcc8a
             260  104 - #03a9f4
             360  104 - #8177fa
             388  104 - #ff8100
             196  108 - #437687
             344  108 - #97cc37
              48  112 - #ed7770
             104  112 - #ffeac8
             236  116 - #ffc107
             336  120 - #f9d81e
             208  148 - #eb6760
             344  152 - #ff5117
             256  156 - #f08fb0
             264  156 - #f38fb1
             400  156 - #fb5d15
              76  160 - #966355
             360  160 - #97cc37
             136  164 - #fcbc7c
             196  164 - #8dafbf
             252  164 - #ef8eb0
             352  164 - #97cc37
              28  172 - #f7bd2a
             260  172 - #04a8f3
             392  172 - #ffe978
             104  176 - #f6e2c1
             244  176 - #7a4e2d
             352  176 - #8177fa
             408  176 - #ff7700
             284  184 - #b67128
             308  184 - #fdc728
             344  184 - #3bc5a3
             332  188 - #b58642
             396  188 - #6d6850
             104  224 - #2e404d
             228  224 - #425b6e
             144  232 - #000000
              32  236 - #2f414e
             116  236 - #12191e
             208  236 - #000000
             236  240 - #000000
             116  244 - #12191e
              72  248 - #3e5768
             188  248 - #3e5768
             448  280 - #ffeac8
             528  284 - #f7bf27
             116  332 - #f6ba29
             476  336 - #ffffff
             148  340 - #f5b127
             528  340 - #a02422
             264  344 - #865b51
             376  344 - #cb6c22
             496  344 - #b3e1ee
             288  352 - #865b51
             216  356 - #e6aa88
             460  356 - #acbdca
             488  360 - #e1e1e1
             104  364 - #422b0d
             360  364 - #ffcc8a
             504  364 - #e1e1e1
             168  368 - #f4a524
             332  368 - #cb6c22
             472  372 - #8dafbf
             440  376 - #ca2c31
             288  380 - #d27856
             348  380 - #ed6d30
             492  380 - #8dafbf
             224  384 - #2f2f2f
             252  384 - #d27856
             380  384 - #ed6d30
              92  388 - #ffffff
             116  388 - #e0a745
             124  388 - #e0a745
             196  388 - #d27856
             456  388 - #8dafbf
             104  392 - #ffffff
             140  392 - #ffffff
             436  392 - #a02422
             240  396 - #2f2f2f
             344  400 - #ff8e00
             484  400 - #437687
             464  404 - #858585
             116  412 - #ed7770
             260  412 - #ffeac8
             452  412 - #a02524
             232  416 - #ffeac8
             368  420 - #ffcc8a
             136  424 - #eb8f00
             400  424 - #839092
             472  428 - #ca2c31
               4  592 - #597c95
             592  592 - #597c95
            ";

const FALLBACK: &str = r"
             592    4 - #597c95
             188   16 - #77b255
             204   16 - #aa8dd8
             120   20 - #be1931
             168   24 - #55acee
             288   28 - #5c903f
              76   32 - #292f33
             148   32 - #a0041e
             188   32 - #597c95
             216   32 - #aa8dd8
             300   32 - #8767ac
              96   36 - #d99e82
             168   36 - #55acee
             280   40 - #226798
             284   40 - #8767ac
              36   44 - #664500
             164   48 - #a0041e
             220   48 - #77b255
             144   52 - #d69f4c
             276   52 - #226798
              96   84 - #b6bab2
             348   84 - #ff5117
             400   84 - #f64b2c
             204   88 - #8dafbf
              80   92 - #865b51
             144   92 - #ffcc8a
             232   92 - #ffc107
             116   96 - #865b51
             256   96 - #597c95
             356   96 - #97cc37
             332  100 - #ff5117
             240  104 - #764025
             404  104 - #ff7b00
              44  112 - #ed7770
             348  112 - #00c0e9
             160  116 - #ed6d30
             196  116 - #ca2c31
             236  116 - #ffc107
             296  116 - #ffca28
             392  116 - #fff8be
             340  120 - #97cc37
             208  148 - #eb6760
              84  152 - #865b51
             296  152 - #f6bd24
             396  152 - #f95421
             256  156 - #f08fb0
             264  156 - #f38fb1
             360  160 - #97cc37
             136  164 - #fcbc7c
             196  164 - #8dafbf
             252  164 - #ef8eb0
             364  168 - #8077f6
              28  172 - #f7bd2a
             260  172 - #04a8f3
             392  172 - #ffe978
             104  176 - #f6e2c1
             244  176 - #7a4e2d
             352  176 - #8177fa
             408  176 - #ff7700
             328  180 - #ff5117
             288  184 - #b26925
             344  184 - #3bc5a3
             332  188 - #b58642
             232  220 - #55acee
              28  224 - #394f5f
              44  224 - #202d36
              88  228 - #664500
             164  232 - #020203
             216  232 - #55acee
             188  236 - #000000
              52  240 - #000000
             124  244 - #314552
             268  244 - #435e71
             208  248 - #ffac33
             448  280 - #ffeac8
             528  284 - #f7bf27
             108  332 - #f5b027
             472  336 - #ffffff
             384  340 - #cb6c22
             496  340 - #8dafbf
             528  340 - #a02422
             248  344 - #ffeac8
             284  348 - #865b51
             360  348 - #cb6c22
             160  352 - #f5ae26
             216  356 - #e6aa88
             460  356 - #acbdca
             496  356 - #e1e1e1
             376  364 - #ed6d30
             476  364 - #b3e1ee
             188  368 - #d27856
             496  368 - #3f545f
             100  372 - #422b0d
             260  372 - #d27856
             340  372 - #ed6d30
             224  380 - #2f2f2f
             252  380 - #d27856
             388  380 - #ffcc8a
             444  380 - #a02422
             460  380 - #b3e1ee
              92  388 - #ffffff
             116  388 - #e0a745
             124  388 - #e0a745
             428  388 - #ca2c31
             484  388 - #8dafbf
             104  392 - #ffffff
             140  392 - #ffffff
             256  392 - #d27856
             452  396 - #858585
             356  400 - #ffcc8a
             160  404 - #f6b428
             212  404 - #ffeac8
             104  408 - #422b0d
             236  408 - #2f2f30
             464  408 - #858585
             124  412 - #ed7770
             488  416 - #a02422
             472  428 - #ca2c31
             396  432 - #ff8e00
               4  592 - #597c95
            ";
