use anyhow::Result;
use hilen::{
    AppRunner,
    dispatch::from_main,
    refs::{Weak, manage::DataManager},
    ui::{
        BLACK, Font, Label, RED, RunStyle, Screenshot, Setup, TextAlignment, U8Color, UIColor, ViewData,
        ViewFrame, ViewTest, WHITE, view,
    },
    ui_test::{check_colors, set_record_probe_count},
};

const TEXT: &str = "kept gone kept";

/// The word "gone" of the text above, in bytes.
const GONE: std::ops::Range<usize> = 5..9;

const PLAIN_FRAME: (u32, u32, u32, u32) = (20, 20, 560, 50);
const STRUCK_FRAME: (u32, u32, u32, u32) = (20, 90, 560, 50);
const COLORED_FRAME: (u32, u32, u32, u32) = (20, 160, 560, 50);

/// No glyph of the test text is this wide in 1 color, only the line is.
const LINE_PIXELS: usize = 30;

const STRUCK: &str = r"
             4    4 - #597c95
            48    4 - #597c95
           112    4 - #597c95
           152    4 - #597c95
           188    4 - #597c95
           352    4 - #597c95
           468    4 - #597c95
           524    4 - #597c95
           228   20 - #ffffff
           284   20 - #ffffff
           412   20 - #ffffff
           576   20 - #ffffff
           376   32 - #ffffff
            20   36 - #ffffff
           488   36 - #ffffff
           140   40 - #000000
            56   44 - #000000
            64   44 - #ffffff
            68   44 - #000000
            88   44 - #ffffff
           140   44 - #000000
           176   44 - #171717
            52   48 - #ffffff
            64   48 - #ffffff
            68   48 - #000000
            88   48 - #ffffff
           100   48 - #ffffff
           128   48 - #ffffff
           140   48 - #000000
           156   48 - #ffffff
           328   48 - #ffffff
            60   52 - #1b1b1b
           252   52 - #ffffff
           452   56 - #ffffff
           556   60 - #ffffff
           216   64 - #ffffff
           416   64 - #ffffff
           512   64 - #ffffff
             4   68 - #597c95
           288   68 - #ffffff
           380   68 - #ffffff
           592   68 - #597c95
           180   80 - #597c95
           252   92 - #ffffff
           356   92 - #ffffff
           480   96 - #ffffff
           320  100 - #ffffff
           400  100 - #ffffff
           528  104 - #ffffff
           576  104 - #ffffff
             4  108 - #597c95
           140  108 - #000000
           216  108 - #ffffff
           436  108 - #ffffff
            60  112 - #1b1b1b
           140  112 - #000000
            40  116 - #000000
            52  116 - #000000
            60  116 - #000000
            64  116 - #ffffff
            68  116 - #000000
            88  116 - #ffffff
           100  116 - #ffffff
           124  116 - #010101
           128  116 - #010101
           140  116 - #000000
           152  116 - #010101
           156  116 - #010101
           164  116 - #ffffff
           168  116 - #ffffff
           176  116 - #171717
            60  120 - #1b1b1b
           140  120 - #000000
           284  120 - #ffffff
            60  124 - #1b1b1b
           372  124 - #ffffff
           464  128 - #ffffff
           316  136 - #ffffff
           408  136 - #ffffff
           500  136 - #ffffff
           548  140 - #597c95
           592  144 - #597c95
           228  148 - #597c95
           112  152 - #597c95
           192  152 - #597c95
             4  160 - #597c95
           292  160 - #ffffff
           340  160 - #ffffff
           452  160 - #ffffff
           256  172 - #ffffff
           400  172 - #ffffff
           488  172 - #ffffff
           140  180 - #000000
            52  184 - #ffffff
            56  184 - #000000
            64  184 - #ffffff
            68  184 - #000000
            88  184 - #ffffff
           140  184 - #000000
           176  184 - #171717
           212  184 - #ffffff
           532  184 - #ffffff
            52  188 - #ffffff
            56  188 - #ffffff
            64  188 - #ffffff
            68  188 - #000000
            88  188 - #ffffff
           128  188 - #ffffff
           140  188 - #000000
           156  188 - #ffffff
           592  188 - #597c95
            60  192 - #1b1b1b
            16  196 - #597c95
           376  196 - #ffffff
           240  208 - #ffffff
           296  208 - #ffffff
           344  208 - #ffffff
           432  208 - #ffffff
           480  208 - #ffffff
           560  208 - #ffffff
           196  224 - #597c95
           520  228 - #597c95
             4  232 - #597c95
            44  232 - #ffffff
            84  232 - #ffffff
           144  232 - #ffffff
           388  232 - #ffffff
           268  236 - #ffffff
           320  236 - #ffffff
           456  236 - #ffffff
           228  240 - #ffffff
           356  240 - #ffffff
           576  240 - #ffffff
            52  272 - #666666
           168  272 - #3e3e3e
           192  272 - #575757
           204  272 - #070707
           284  272 - #040404
           332  272 - #0b0b0b
           344  272 - #000000
           440  272 - #cccccc
           472  272 - #000000
           524  272 - #cecece
            64  276 - #212121
            72  276 - #ffffff
            88  276 - #ffffff
           100  276 - #858585
           104  276 - #ffffff
           108  276 - #ebebeb
           116  276 - #ffffff
           120  276 - #000000
           124  276 - #373737
           148  276 - #b7b7b7
           152  276 - #ffffff
           156  276 - #010101
           164  276 - #ffffff
           168  276 - #000000
           188  276 - #ffffff
           204  276 - #070707
           212  276 - #ffffff
           224  276 - #ffffff
           232  276 - #3a3a3a
           264  276 - #ffffff
           276  276 - #ffffff
           284  276 - #040404
           304  276 - #ffffff
           308  276 - #010101
           312  276 - #9d9d9d
           316  276 - #ffffff
           320  276 - #000000
           328  276 - #ffffff
           332  276 - #000000
           344  276 - #000000
           380  276 - #000000
           384  276 - #ffffff
           392  276 - #000000
           396  276 - #ffffff
           400  276 - #070707
           408  276 - #ffffff
           420  276 - #ffffff
           440  276 - #cccccc
           444  276 - #ffffff
           472  276 - #000000
           484  276 - #ffffff
           524  276 - #1d1d1d
           572  276 - #ffffff
            52  280 - #666666
            64  280 - #212121
            68  280 - #ffffff
            72  280 - #ffffff
            88  280 - #ffffff
           116  280 - #ffffff
           120  280 - #ffffff
           140  280 - #ffffff
           144  280 - #ffffff
           156  280 - #010101
           164  280 - #ffffff
           168  280 - #000000
           180  280 - #ffffff
           188  280 - #ffffff
           212  280 - #ffffff
           216  280 - #000000
           220  280 - #ffffff
           224  280 - #ffffff
           264  280 - #ffffff
           276  280 - #ffffff
           280  280 - #ffffff
           304  280 - #ffffff
           308  280 - #000000
           320  280 - #000000
           328  280 - #ffffff
           332  280 - #000000
           344  280 - #000000
           368  280 - #ffffff
           380  280 - #000000
           384  280 - #ffffff
           392  280 - #010101
           408  280 - #ffffff
           416  280 - #000000
           444  280 - #ffffff
           460  280 - #ffffff
           472  280 - #000000
           484  280 - #ffffff
           500  280 - #000000
           536  280 - #ffffff
           380  284 - #010101
           472  284 - #666666
           524  284 - #666666
             4  288 - #597c95
           144  296 - #000000
           192  296 - #ffffff
           204  296 - #ffffff
            88  300 - #000000
           108  300 - #404040
           124  300 - #000000
           144  300 - #010101
           172  300 - #d0d0d0
           180  300 - #606060
           236  300 - #000000
            56  304 - #ffffff
            80  304 - #ffffff
            88  304 - #000000
            96  304 - #8b8b8b
           104  304 - #ffffff
           112  304 - #000000
           116  304 - #000000
           124  304 - #000000
           140  304 - #ffffff
           144  304 - #000000
           152  304 - #585858
           156  304 - #000000
           160  304 - #000000
           172  304 - #d0d0d0
           180  304 - #606060
           188  304 - #ffffff
           192  304 - #f6f6f6
           212  304 - #313131
           228  304 - #ffffff
           236  304 - #000000
           244  304 - #8b8b8b
           248  304 - #000000
           252  304 - #000000
            88  308 - #000000
            96  308 - #8b8b8b
           108  308 - #404040
           124  308 - #000000
           144  308 - #010101
           152  308 - #585858
           172  308 - #d0d0d0
           180  308 - #606060
           236  308 - #000000
           244  308 - #8b8b8b
           592  312 - #597c95
           292  316 - #ffffff
           416  316 - #ffffff
           348  320 - #ffffff
           524  320 - #ffffff
           468  324 - #ffffff
           384  336 - #ffffff
             4  340 - #597c95
           552  340 - #ffffff
           316  344 - #ffffff
            68  348 - #ffffff
           124  348 - #ffffff
           272  348 - #ffffff
           436  348 - #ffffff
           504  348 - #ffffff
           180  352 - #597c95
           592  352 - #597c95
           220  372 - #ffffff
           336  372 - #ffffff
           532  372 - #ffffff
           300  376 - #ffffff
           408  376 - #ffffff
           472  376 - #ffffff
             4  380 - #597c95
           368  384 - #ffffff
           572  384 - #ffffff
           140  388 - #000000
           440  388 - #ffffff
            60  392 - #1b1b1b
           140  392 - #000000
           264  392 - #ffffff
            40  396 - #000000
            52  396 - #000000
            60  396 - #000000
            64  396 - #ffffff
            68  396 - #000000
            88  396 - #ffffff
           100  396 - #ffffff
           124  396 - #010101
           128  396 - #010101
           140  396 - #000000
           152  396 - #010101
           156  396 - #010101
           164  396 - #ffffff
           168  396 - #ffffff
           176  396 - #171717
            60  400 - #1b1b1b
            96  400 - #ffffff
           108  400 - #ffffff
           112  400 - #ffffff
           116  400 - #ffffff
           120  400 - #ffffff
           140  400 - #000000
            60  404 - #1b1b1b
            88  404 - #000000
            92  404 - #000000
            96  404 - #000000
           100  404 - #000000
           104  404 - #000000
           108  404 - #000000
           112  404 - #000000
           116  404 - #000000
           120  404 - #000000
           124  404 - #000000
           128  404 - #000000
           516  404 - #ffffff
           316  408 - #ffffff
           228  412 - #ffffff
           552  412 - #ffffff
           408  416 - #ffffff
           460  416 - #ffffff
           284  428 - #597c95
           160  432 - #597c95
             4  436 - #597c95
           356  436 - #597c95
            40  440 - #597c95
           572  440 - #597c95
           196  444 - #597c95
           108  448 - #597c95
           252  448 - #597c95
           484  448 - #597c95
           312  464 - #597c95
           444  464 - #597c95
           520  464 - #597c95
            64  468 - #597c95
           148  468 - #597c95
           392  472 - #597c95
           276  476 - #597c95
           352  480 - #597c95
           104  484 - #597c95
             4  488 - #597c95
           552  488 - #597c95
           212  492 - #597c95
           592  492 - #597c95
           172  496 - #597c95
           420  500 - #597c95
           136  504 - #597c95
           500  504 - #597c95
            64  508 - #597c95
           320  508 - #597c95
           272  512 - #597c95
           460  516 - #597c95
           100  520 - #597c95
           536  528 - #597c95
           576  528 - #597c95
           360  532 - #597c95
            40  536 - #597c95
           164  536 - #597c95
           208  536 - #597c95
           412  540 - #597c95
             4  548 - #597c95
            76  552 - #597c95
           124  552 - #597c95
           496  552 - #597c95
           244  560 - #597c95
           552  564 - #597c95
           300  568 - #597c95
           592  568 - #597c95
           160  576 - #597c95
           460  580 - #597c95
             4  588 - #597c95
            64  588 - #597c95
           348  588 - #597c95
           120  592 - #597c95
           196  592 - #597c95
           268  592 - #597c95
           400  592 - #597c95
           524  592 - #597c95
";

#[view]
struct LabelStrikethrough {
    #[init]
    plain:   Label,
    struck:  Label,
    colored: Label,
    wrapped: Label,
    both:    Label,
}

fn mono() -> Weak<Font> {
    Font::get("DroidSansMono.ttf")
}

impl Setup for LabelStrikethrough {
    fn setup(self: Weak<Self>) {
        for label in [self.plain, self.struck] {
            label.set_color(WHITE).set_text_color(BLACK).set_text_size(22);
            label.set_text(TEXT).set_alignment(TextAlignment::Left);
        }

        self.plain.set_frame(PLAIN_FRAME);
        self.struck.set_frame(STRUCK_FRAME);
        self.struck.set_font_runs([(GONE, RunStyle::strikethrough())]);

        // The line takes the color the text has there.
        self.colored.set_color(WHITE).set_text_color(BLACK).set_text_size(22);
        self.colored.set_text(TEXT).set_alignment(TextAlignment::Left);
        self.colored.set_frame(COLORED_FRAME);
        self.colored.set_font_runs([(GONE, RunStyle::strikethrough())]);
        self.colored.set_color_runs([(GONE, UIColor::Plain(RED))]);

        // A line that crosses a wrap follows every piece, in a run font
        // too, mono is wider than the label font.
        self.wrapped.set_color(WHITE).set_text_color(BLACK).set_text_size(22);
        self.wrapped
            .set_text("This offer ended last week and the price below is no longer the right one.");
        self.wrapped.set_multiline(true).set_alignment(TextAlignment::Left);
        self.wrapped.place().t(230).lr(20).h(120);
        self.wrapped.set_font_runs([
            (5..40, RunStyle::strikethrough()),
            (40..52, RunStyle::font(mono()).struck()),
        ]);

        // A run can have both lines, under the text and through it.
        self.both.set_color(WHITE).set_text_color(BLACK).set_text_size(22);
        self.both.set_text(TEXT).set_alignment(TextAlignment::Left);
        self.both.place().t(370).lr(20).h(50);
        self.both.set_font_runs([(GONE, RunStyle::underline().struck())]);
    }
}

impl ViewTest for LabelStrikethrough {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        set_record_probe_count(400);

        // The recorder pins few pixels of a line 1 pixel high, so the line
        // itself is found by its length: a row of one color no glyph has.
        let shot = AppRunner::take_screenshot()?;
        let black = U8Color::rgba(0, 0, 0, 255);
        let red = U8Color::rgba(255, 0, 0, 255);
        assert!(!has_line(&shot, PLAIN_FRAME, black), "a plain label has a line");
        assert!(has_line(&shot, STRUCK_FRAME, black), "no line through the text");
        // The line takes the color of the text it crosses.
        assert!(
            has_line(&shot, COLORED_FRAME, red),
            "no red line through red text"
        );
        assert!(
            !has_line(&shot, COLORED_FRAME, black),
            "a black line through red text"
        );

        check_colors(STRUCK)?;

        // With the runs gone the label is its plain twin again, pixel
        // for pixel, the line left nothing behind.
        from_main(move || {
            view.struck.clear_font_runs();
        });
        let shot = AppRunner::take_screenshot()?;
        assert!(
            region(&shot, PLAIN_FRAME) == region(&shot, STRUCK_FRAME),
            "a cleared strikethrough still draws"
        );

        Ok(())
    }
}

/// Whether some row of the frame has `LINE_PIXELS` pixels of `color` in a row.
fn has_line(shot: &Screenshot, frame: (u32, u32, u32, u32), color: U8Color) -> bool {
    let (x, y, w, h) = frame;
    (y..y + h).any(|row| {
        let mut run = 0;
        (x..x + w).any(|col| {
            run = if shot.get_pixel((col, row)) == color {
                run + 1
            } else {
                0
            };
            run >= LINE_PIXELS
        })
    })
}

fn region(shot: &Screenshot, frame: (u32, u32, u32, u32)) -> Vec<U8Color> {
    let (x, y, w, h) = frame;
    let mut pixels = Vec::with_capacity((w * h) as usize);

    for row in y..y + h {
        for col in x..x + w {
            pixels.push(shot.get_pixel((col, row)));
        }
    }

    pixels
}
