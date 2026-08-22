use anyhow::Result;
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{ImageMode, Setup, UIManager, ViewData, ViewTest, ViewTouch, view},
    ui_test::helpers::check_colors,
};

#[view]
struct ImageView {
    #[init]
    image_view: hilen::ui::ImageView,
}

impl Setup for ImageView {
    fn setup(self: Weak<Self>) {
        self.enable_touch();

        self.image_view.place().tl(100).size(280, 280);
        self.image_view.set_image("gradient.png");
    }
}

impl ViewTest for ImageView {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        check_gradient()?;
        check_resized_gradient(view)?;
        check_cat_with_debug_frames(view)?;
        check_aspect_fit(view)?;
        check_tall_aspect_fit(view)?;

        from_main(UIManager::disable_debug_frames);

        Ok(())
    }
}

fn check_gradient() -> Result<()> {
    check_colors(
        r"
             108  104 - #71f24a
             248  104 - #347683
             280  104 - #275a9c
             344  104 - #0d21d3
             148  112 - #60cf48
             188  112 - #4fac56
             220  112 - #42906a
             316  112 - #1b3ab7
             376  128 - #1b1bd9
             116  144 - #68d340
             372  172 - #4343b1
             364  188 - #5151a1
             116  208 - #709b2f
             104  240 - #807f26
             376  240 - #81817d
             368  264 - #96966a
             116  268 - #926824
             364  288 - #acac59
             300  300 - #597c95
             104  308 - #af4b1e
             368  320 - #c8c850
             376  352 - #e6e64f
             252  360 - #df9035
             276  360 - #e1a43a
             160  372 - #e34a27
             104  376 - #e63323
             216  376 - #ea742e
             296  376 - #f0b63f
             308  376 - #f2c042
             340  376 - #f6dc4a
             368  376 - #faf451
             592  592 - #597c95
        ",
    )?;

    Ok(())
}

fn check_resized_gradient(view: Weak<ImageView>) -> Result<()> {
    from_main(move || {
        view.image_view.place().clear().tl(100).size(400, 400);
    });

    check_colors(
        r"
               4    4 - #597c95
             104  104 - #74f84b
             164  104 - #61d249
             284  104 - #3c8875
             400  104 - #193fb6
             440  104 - #0e26ce
             496  104 - #0404f0
             360  108 - #26589e
             312  116 - #357580
             260  120 - #459764
             496  160 - #2828ce
             104  164 - #68d33f
             124  216 - #68b236
             496  224 - #5050a7
             124  276 - #778e2c
             476  296 - #7e7e7a
             300  300 - #597c95
             104  332 - #8e6d22
             488  332 - #95956b
             124  360 - #9b5e22
             476  372 - #aeae58
             112  392 - #ac4e1f
             476  420 - #cdcd4e
             496  460 - #e6e64f
             248  472 - #dc682c
             380  472 - #e4b53f
             200  480 - #df5027
             296  492 - #e98532
             104  496 - #e73323
             340  496 - #ee9e38
             400  496 - #f3c242
             440  496 - #f6da49
        ",
    )?;

    Ok(())
}

fn check_cat_with_debug_frames(view: Weak<ImageView>) -> Result<()> {
    from_main(move || {
        UIManager::enable_debug_frames();
        view.image_view.place().clear().tl(140).size(280, 200);
        view.image_view.set_image("cat.png");
    });

    check_colors(
        r"
             592    4 - #597c95
             144  144 - #ebc6cd
             276  144 - #e1b9ba
             416  144 - #d7a9ab
             324  156 - #dcb2b3
             372  168 - #d19f9e
             232  172 - #cd998b
             280  196 - #cdb19b
             400  208 - #cf9999
             168  220 - #e9bbbd
             348  232 - #bd9f85
             296  244 - #af9074
             212  252 - #e0bfae
             384  264 - #a58973
             336  268 - #c7a792
             412  268 - #c68f92
             368  284 - #937761
             392  288 - #9a7b66
             308  292 - #d1b19c
             244  296 - #d7b7a2
             164  300 - #e6b3b2
             372  304 - #a48570
             400  312 - #a6886e
             352  316 - #876a58
             356  324 - #977961
             260  332 - #cfa08e
             204  336 - #e0bcb0
             300  344 - #597c95
             592  384 - #597c95
               4  592 - #597c95
             256  592 - #597c95
             592  592 - #597c95
        ",
    )?;

    Ok(())
}

fn check_aspect_fit(mut view: Weak<ImageView>) -> Result<()> {
    from_main(move || {
        view.image_view.mode = ImageMode::AspectFit;
        view.image_view.place().clear().tl(140).size(280, 200);
    });

    check_colors(
        r"
               4    4 - #597c95
             592    4 - #597c95
             212  144 - #ebc6cd
             292  144 - #dfb5b7
             344  144 - #d8aaac
             252  152 - #edc5c6
             276  172 - #e1b7b8
             304  184 - #deaeae
             236  192 - #e4bcbd
             348  196 - #ce9c9b
             336  220 - #cc9a99
             308  236 - #b59679
             212  240 - #e5b7ba
             348  244 - #c99798
             304  264 - #c3a08a
             332  264 - #a58973
             224  268 - #f4d7cf
             344  276 - #a78b76
             252  280 - #d6b5a4
             280  284 - #c9a78e
             328  292 - #a48570
             344  296 - #997d65
             304  308 - #ba9986
             340  312 - #a6886e
             220  324 - #dc9e9f
             324  332 - #a4846d
             260  336 - #d5afa2
             592  336 - #597c95
             300  344 - #597c95
               4  592 - #597c95
             256  592 - #597c95
             592  592 - #597c95
        ",
    )?;

    Ok(())
}

fn check_tall_aspect_fit(view: Weak<ImageView>) -> Result<()> {
    from_main(move || {
        view.image_view.place().clear().tl(140).size(100, 400);
    });

    check_colors(
        r"
             288    4 - #597c95
             592    4 - #597c95
               4   24 - #597c95
             440  128 - #597c95
             144  272 - #ebc6cd
             196  272 - #dfb5b7
             232  272 - #dcaeb0
             168  284 - #e9c1c2
             216  284 - #deb0b0
             204  300 - #deaeae
             536  300 - #597c95
             236  304 - #d19f9e
             156  308 - #e4bcbd
             232  320 - #cb999c
             160  328 - #f1d6cb
             228  328 - #cc9a99
             236  336 - #ca9698
             144  344 - #e3b1b4
             224  344 - #c49291
             236  352 - #c79596
             212  356 - #c6a28a
             172  364 - #d7b4a0
             208  372 - #c7a792
             232  380 - #987c66
             156  384 - #ecc7bf
             192  392 - #ceaa94
             168  404 - #e2bdb4
             148  408 - #dca0a0
             228  408 - #bb9d85
               4  592 - #597c95
             364  592 - #597c95
             592  592 - #597c95
        ",
    )?;

    Ok(())
}
