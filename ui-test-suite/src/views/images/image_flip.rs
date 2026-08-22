use anyhow::Result;
use hilen::{
    refs::Weak,
    ui::{ImageView, Setup, ViewFrame, ViewSubviews, ViewTest, view},
    ui_test::check_colors,
};

#[view]
struct ImageFlip {
    #[init]
    tl: ImageView,
    tr: ImageView,
    bl: ImageView,
    br: ImageView,
}

impl Setup for ImageFlip {
    fn setup(mut self: Weak<Self>) {
        self.apply_to::<ImageView>(|i| {
            i.set_image("cat.png");
        });

        self.tl.set_frame((50, 50, 150, 150));
        self.tr.set_frame((250, 50, 150, 150));
        self.tr.flip_x = true;
        self.bl.set_frame((50, 250, 150, 150));
        self.bl.flip_y = true;
        self.br.set_frame((250, 250, 150, 150));
        self.br.flip_x = true;
        self.br.flip_y = true;
    }
}

impl ViewTest for ImageFlip {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        check_colors(
            r"
             592    4 - #597c95
              52   52 - #eac7cd
             180   52 - #d9afb0
             312   52 - #dfb7b8
             396   52 - #eac7cd
             280  128 - #b3907a
              52  144 - #dea6a7
             184  148 - #ac907a
             252  156 - #a48873
             380  160 - #f0d2c8
             192  188 - #b99b83
             280  188 - #a68870
             116  192 - #d1a28e
             336  192 - #d0a18f
             336  256 - #cfa08e
             592  256 - #597c95
             280  260 - #a6866f
             188  264 - #af9179
              52  268 - #dea8a8
             156  288 - #c6a592
             256  288 - #997e69
             396  308 - #dfa7a8
             196  340 - #cb9998
             252  344 - #cb9998
              52  376 - #eec7cc
             156  396 - #dcb2b3
             272  396 - #dcaeb0
             384  396 - #e7c0c5
             556  424 - #597c95
               4  592 - #597c95
             328  592 - #597c95
             592  592 - #597c95
        ",
        )?;

        Ok(())
    }
}
