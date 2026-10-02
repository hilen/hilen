use anyhow::Result;
use hilen::{
    refs::Weak,
    ui::{Label, Setup, TextAlignment, ViewData, ViewTest, WHITE, view},
    ui_test::{check_colors, set_record_probe_count},
};

/// A clipping box with 1 line of text wider than the box. A clip boundary
/// flushes text, so every box is its own text batch of the frame.
#[view]
struct ClippedLine {
    #[init]
    label: Label,
}

impl Setup for ClippedLine {
    fn setup(self: Weak<Self>) {
        self.set_color(WHITE);
        self.label.set_alignment(TextAlignment::Left);
        self.label.place().l(12).t(0).size(2400, 84);
    }

    fn clips_to_bounds(&self) -> bool {
        true
    }
}

/// Every box draws big text at its own size, so every batch brings glyphs
/// the glyph atlas does not hold yet. Each batch fits the atlas alone and
/// all 6 do not, so a later batch needs the atlas space of an earlier one.
/// The earlier boxes are already drawn by then and must keep their text.
#[view]
struct ClippedTextBatches {
    #[init]
    first:  ClippedLine,
    second: ClippedLine,
    third:  ClippedLine,
    fourth: ClippedLine,
    fifth:  ClippedLine,
    sixth:  ClippedLine,
}

impl Setup for ClippedTextBatches {
    fn setup(self: Weak<Self>) {
        let lines = [
            (self.first, "1 quick brown fox jumps over the lazy dog", 56),
            (self.second, "2 quick brown fox jumps over the lazy dog", 60),
            (self.third, "3 quick brown fox jumps over the lazy dog", 64),
            (self.fourth, "4 quick brown fox jumps over the lazy dog", 68),
            (self.fifth, "5 quick brown fox jumps over the lazy dog", 72),
            (self.sixth, "6 quick brown fox jumps over the lazy dog", 76),
        ];

        for (index, (line, text, size)) in lines.into_iter().enumerate() {
            line.label.set_text(text).set_text_size(size);
            line.place().l(40).t(15 + 96 * index).size(520, 84);
        }
    }
}

impl ViewTest for ClippedTextBatches {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        set_record_probe_count(256);
        check_colors(PROBES)
    }
}

/// The recorded look, 6 clean lines of text, each cut at the right edge of
/// its box.
const PROBES: &str = r"
               4    4 - #597c95
             140    4 - #597c95
             320    4 - #597c95
             592    4 - #597c95
             236   16 - #ffffff
             368   16 - #ffffff
             512   16 - #ffffff
              72   44 - #adadad
             436   44 - #585858
             172   48 - #c2c2c2
             180   48 - #5e5e5e
             296   48 - #8e8e8e
             172   52 - #c2c2c2
             172   56 - #c2c2c2
             360   56 - #000000
             544   56 - #4c4c4c
             124   60 - #ffffff
             132   60 - #ffffff
             172   60 - #c2c2c2
             276   60 - #ffffff
             280   60 - #ffffff
             296   60 - #8e8e8e
             324   60 - #ffffff
             328   60 - #ffffff
             412   60 - #adadad
             436   60 - #585858
             464   60 - #ffffff
             468   60 - #ffffff
             484   60 - #ffffff
             172   64 - #c2c2c2
             296   64 - #8e8e8e
             412   64 - #adadad
              12   68 - #597c95
             172   68 - #c2c2c2
             296   68 - #8e8e8e
             412   68 - #adadad
             172   72 - #c2c2c2
             296   72 - #8e8e8e
             412   72 - #adadad
             436   72 - #585858
              84   76 - #e6e6e6
             224   76 - #e6e6e6
             244   76 - #e6e6e6
             392   76 - #e6e6e6
             484   76 - #e6e6e6
             504   76 - #e6e6e6
             456  112 - #ffffff
             364  116 - #ffffff
             540  116 - #ffffff
              56  120 - #ffffff
             592  120 - #597c95
             236  128 - #808080
               4  132 - #597c95
             284  132 - #5a5a5a
             284  136 - #5a5a5a
              96  140 - #010101
             284  140 - #5a5a5a
             160  144 - #dddddd
             188  144 - #5f5f5f
             328  144 - #c3c3c3
             160  148 - #dddddd
             484  148 - #000000
             140  152 - #aeaeae
             160  152 - #dddddd
             224  152 - #4d4d4d
             128  156 - #ffffff
             140  156 - #aeaeae
             160  156 - #dddddd
             188  156 - #5f5f5f
             284  156 - #5a5a5a
             288  156 - #ffffff
             296  156 - #ffffff
             436  156 - #242424
             140  160 - #aeaeae
             160  160 - #dddddd
             284  160 - #5a5a5a
             380  160 - #000000
             436  160 - #242424
             436  164 - #242424
              76  168 - #010101
             188  168 - #5f5f5f
             436  168 - #242424
             532  168 - #000000
             256  172 - #000000
             348  172 - #010101
             140  176 - #aeaeae
             140  180 - #aeaeae
             592  180 - #597c95
               4  184 - #597c95
             220  192 - #ffffff
             552  212 - #ffffff
             276  216 - #ffffff
             368  216 - #ffffff
             464  216 - #ffffff
              40  224 - #ffffff
              96  232 - #000000
             160  240 - #7b7b7b
             196  240 - #606060
             332  240 - #000000
             404  240 - #010101
             592  240 - #597c95
             160  244 - #7b7b7b
             512  244 - #000000
             160  248 - #7b7b7b
             236  248 - #4a4a4a
             160  252 - #7b7b7b
             440  252 - #505050
             160  256 - #7b7b7b
             196  256 - #606060
              72  260 - #000000
             124  260 - #000000
             236  260 - #373737
             552  260 - #000000
             196  268 - #606060
             268  268 - #000000
             304  268 - #010101
             360  268 - #010101
             436  268 - #010101
              40  276 - #ffffff
              88  288 - #ffffff
             484  288 - #ffffff
             592  308 - #597c95
             400  312 - #ffffff
               4  320 - #597c95
             524  320 - #010101
             152  332 - #464646
             168  332 - #464646
             192  332 - #464646
             208  332 - #464646
             280  332 - #464646
             348  332 - #464646
             448  332 - #464646
             460  332 - #464646
             512  332 - #464646
             524  332 - #464646
             172  336 - #9c9c9c
              88  340 - #ffffff
             172  340 - #9c9c9c
             244  340 - #000000
              84  344 - #ffffff
              88  344 - #ffffff
             172  344 - #9c9c9c
              80  348 - #ffffff
              84  348 - #ffffff
             156  348 - #6e6e6e
             172  348 - #9c9c9c
             188  348 - #5d5d5d
             204  348 - #616161
             312  348 - #000000
             172  352 - #9c9c9c
             204  360 - #616161
             156  364 - #6e6e6e
             380  364 - #010101
             516  364 - #000000
             556  364 - #010101
              96  368 - #cccccc
             192  368 - #cccccc
             208  368 - #cccccc
             260  368 - #cccccc
             280  368 - #cccccc
             348  368 - #cccccc
             416  368 - #999999
             460  368 - #cccccc
             484  368 - #cccccc
             156  380 - #6e6e6e
              48  384 - #ffffff
               4  408 - #597c95
             104  416 - #e5e5e5
             324  416 - #000000
              92  420 - #939393
              96  420 - #939393
             100  420 - #939393
             552  420 - #ffffff
             556  420 - #ffffff
             548  424 - #f0f0f0
             552  424 - #ffffff
             556  424 - #ffffff
             144  428 - #010101
             180  428 - #ffffff
             396  428 - #010101
             212  432 - #636363
             480  432 - #c2c2c2
             552  432 - #727272
             428  436 - #000000
             480  436 - #c2c2c2
             256  440 - #5d5d5d
             268  440 - #000000
             480  440 - #c2c2c2
             212  444 - #636363
             368  444 - #a3a3a3
             480  444 - #c2c2c2
             504  444 - #232323
             368  448 - #a3a3a3
             480  448 - #c2c2c2
             504  448 - #232323
              76  452 - #232323
             368  452 - #a3a3a3
             480  452 - #c2c2c2
             504  452 - #232323
              24  456 - #597c95
             368  456 - #a3a3a3
             480  456 - #c2c2c2
             504  456 - #232323
             136  460 - #010101
             212  460 - #636363
             368  460 - #a3a3a3
             480  460 - #c2c2c2
             504  460 - #232323
             180  464 - #010101
             324  464 - #000000
             404  464 - #000000
             544  464 - #000000
             592  464 - #597c95
              52  496 - #ffffff
             556  500 - #ffffff
               4  504 - #597c95
             340  508 - #000000
              96  512 - #010101
             184  524 - #404040
             220  524 - #7d7d7d
             392  524 - #010101
             492  524 - #010101
             184  528 - #404040
             220  528 - #7d7d7d
             300  528 - #000000
             184  532 - #404040
             220  532 - #7d7d7d
             160  536 - #1b1b1b
             184  536 - #404040
             268  536 - #595959
             448  536 - #000000
             160  540 - #1b1b1b
             184  540 - #404040
             220  540 - #7d7d7d
             528  540 - #505050
             592  540 - #597c95
             160  544 - #1b1b1b
             184  544 - #404040
              28  548 - #597c95
             160  548 - #1b1b1b
             220  548 - #7d7d7d
             336  552 - #010101
              80  556 - #010101
             480  556 - #000000
             220  560 - #7d7d7d
             412  560 - #000000
             528  560 - #505050
             160  564 - #1b1b1b
             160  568 - #1b1b1b
             160  572 - #1b1b1b
             160  576 - #1b1b1b
             372  576 - #ffffff
               4  592 - #597c95
             308  592 - #597c95
             476  592 - #597c95
             592  592 - #597c95
";
