use anyhow::Result;
use hilen::{
    refs::Weak,
    ui::{BLACK, Container, ImageView, Label, RED, Setup, ViewData, ViewFrame, ViewTest, WHITE, view},
    ui_test::{check_colors, set_record_probe_count},
};

/// Two red labels over the same image, and a plain translucent view pushed
/// in front of the right half, the way an app builds a dim layer without
/// a `ModalView`. The right label's text must show through the overlay,
/// dimmed like its red background, not vanish into it.
#[view]
struct TranslucentOverText {
    #[init]
    open:        Label,
    covered:     Label,
    open_cat:    ImageView,
    covered_cat: ImageView,
    overlay:     Container,
    caption:     Label,
}

impl Setup for TranslucentOverText {
    fn setup(self: Weak<Self>) {
        self.open.set_text("open");
        self.covered.set_text("covered");
        for label in [self.open, self.covered] {
            label.set_color(RED).set_text_color(WHITE).set_text_size(40);
        }
        self.open.place().t(100).l(40).size(220, 120);
        self.covered.place().t(100).l(340).size(220, 120);

        for cat in [self.open_cat, self.covered_cat] {
            cat.set_image("cat.png");
        }
        self.open_cat.place().t(250).l(40).size(220, 220);
        self.covered_cat.place().t(250).l(340).size(220, 220);

        self.overlay.set_color(BLACK.with_alpha(0.4));
        self.overlay.place().right_half();
        self.overlay.bump_z_position(0.001);

        self.caption
            .set_text("40% black overlay")
            .set_text_color(WHITE)
            .set_text_size(24)
            .place()
            .b(40)
            .l(300)
            .size(300, 60);
        self.caption.bump_z_position(0.002);
    }
}

impl ViewTest for TranslucentOverText {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        set_record_probe_count(160);
        check_colors(COLORS)?;
        Ok(())
    }
}

const COLORS: &str = r"
   4    4 - #597c95
 148    4 - #597c95
 300    4 - #354a59
 404    4 - #354a59
 592    4 - #354a59
 496   12 - #354a59
 224   28 - #597c95
 124  100 - #ff0000
 256  100 - #ff0000
 344  100 - #990000
 444  100 - #990000
 556  100 - #990000
  40  104 - #ff0000
 500  104 - #990000
 176  152 - #ff7575
 340  152 - #990000
 160  156 - #ff0000
 188  156 - #fffefe
 452  156 - #990000
 488  156 - #990000
  52  160 - #ff0000
 112  160 - #ff0000
 136  160 - #ff0000
 164  160 - #ff0000
 176  160 - #ffffff
 248  160 - #ff0000
 396  160 - #990000
 416  160 - #990000
 436  160 - #999999
 480  160 - #999999
 512  160 - #990000
 124  164 - #ffffff
 140  164 - #ff0000
 148  164 - #fffefe
 168  164 - #ff1e1e
 408  164 - #990000
 156  168 - #ffffff
 164  168 - #ff0000
 188  168 - #fffefe
 488  168 - #990000
 116  172 - #ffffff
 140  172 - #fffefe
 176  172 - #fffefe
 388  172 - #999999
 412  172 - #999898
 432  172 - #999999
 452  172 - #999898
 132  176 - #fffefe
 544  200 - #990000
  40  216 - #ff0000
 256  216 - #ff0000
 340  216 - #990000
 592  224 - #354a59
  88  252 - #ebc4c9
 156  252 - #dfb7b8
 532  252 - #83696a
 472  256 - #866d6e
 224  264 - #dbadad
 116  276 - #b78c7e
 412  276 - #70564b
 256  280 - #d5a3a4
 348  288 - #8d7679
  40  292 - #e9c1c5
 152  304 - #d0b8a2
 468  304 - #756459
 112  308 - #c69d8b
 192  312 - #af8a76
 236  312 - #9e6b58
 540  312 - #5c4034
 228  316 - #ae7c69
 236  316 - #9e6a58
 236  320 - #a06757
 240  320 - #9a6753
 232  324 - #9d6551
 212  328 - #966e5a
 216  328 - #966a57
 528  328 - #5a382c
  68  332 - #e7bfbf
 340  332 - #886e6f
 428  332 - #2a1d13
 228  336 - #9e6c53
 420  336 - #3d3524
 428  336 - #0c0804
 212  340 - #8c6852
 224  340 - #8c553f
 436  340 - #23140a
 512  340 - #543e31
 128  344 - #534127
 144  344 - #ceb197
 176  344 - #422917
 420  344 - #25190c
 428  344 - #322717
 476  344 - #28190e
 132  348 - #a7896f
 220  348 - #a17e63
 256  348 - #ca9698
 484  348 - #292016
 168  352 - #2c1a0d
 468  352 - #1a1008
 476  352 - #0e0808
 476  356 - #2a2112
 480  356 - #2c2214
 440  360 - #472e20
 124  364 - #c29e7f
 372  364 - #92827d
  40  372 - #e2b0b3
 128  372 - #be8d79
 196  372 - #c4a289
 556  372 - #78595a
 136  376 - #9a644c
 172  376 - #c29c85
 436  376 - #5c3c2e
 440  376 - #583b2e
 256  380 - #c99798
  84  384 - #edd0c5
 224  388 - #b29680
 160  400 - #c4a189
 200  408 - #c4a48f
 400  408 - #837065
 512  408 - #4d3d2f
 448  416 - #7a6759
  56  420 - #e2a9a8
 356  428 - #886868
 556  428 - #685647
 256  432 - #b2947c
 108  436 - #d9b3a7
 148  436 - #d0ac96
 188  440 - #b69681
 508  440 - #534336
 476  448 - #766256
 412  460 - #836b66
 136  464 - #cc9e8b
 504  464 - #564336
  48  468 - #e3acad
 168  468 - #c7a792
 220  468 - #ad8b72
 256  468 - #b49882
 540  468 - #725f52
 592  520 - #354a59
 376  524 - #354a59
 376  528 - #354a59
 412  528 - #354a59
 444  528 - #354a59
 452  528 - #b7bec4
 476  528 - #354a59
 500  528 - #354a59
 508  528 - #dadee0
 364  532 - #ffffff
 376  532 - #354a59
 436  532 - #828f98
 452  532 - #b7bec4
 508  532 - #dadee0
 408  536 - #fefefe
 488  536 - #ffffff
 508  536 - #dadee0
  72  544 - #597c95
   4  592 - #597c95
 136  592 - #597c95
 260  592 - #597c95
 592  592 - #354a59
";
