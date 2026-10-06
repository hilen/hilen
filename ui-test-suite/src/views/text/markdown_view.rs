use anyhow::Result;
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{BLACK, MarkdownView, Setup, ViewData, ViewFrame, ViewTest, WHITE, view},
    ui_test::{check_colors, inject_touches, set_record_probe_count},
};

/// The first line is a link, so a tap on a known place hits it.
const TEXT: &str = r"[The first line is a link](https://example.com/page) and plain text after it.

# Heading 1
## Heading 2

Plain, **bold**, *italic*, ***both***, ~~struck~~ and `inline code`.

- A bullet
- Another bullet
  - A nested one

1. First
2. Second

- [ ] A task not done
- [x] A task done

> A quote with **bold** inside.

```rust
// A comment.
fn main() {
    println!(42);
}
```

| Name | Value |
|------|-------|
| Rows | 2     |
| Wide | A longer cell |

---

The end.
";

const WIDE: f32 = 600.0;
const NARROW: f32 = 300.0;

const DRAWN: &str = r"
             4    4 - #ffffff
           124    4 - #ffffff
           320    4 - #ffffff
           376    4 - #ffffff
           424    4 - #ffffff
           504    4 - #ffffff
           588    4 - #ffffff
           632    4 - #ffffff
           548   12 - #ffffff
            92   24 - #2f7df6
           188   24 - #4e4e4e
            40   28 - #5293f8
            48   28 - #ffffff
            64   28 - #80aff9
            68   28 - #80aff9
            84   28 - #478cf7
            92   28 - #2f7df6
            96   28 - #a4c6fb
           152   28 - #ffffff
           196   28 - #ffffff
           228   28 - #1e1e1e
           248   28 - #b8b8b8
           264   28 - #d9d9d9
           268   28 - #1f1f1f
           288   28 - #434343
           292   28 - #858585
           464   28 - #ffffff
            48   32 - #ffffff
            52   32 - #ffffff
           100   32 - #ffffff
           152   32 - #ffffff
           340   32 - #ffffff
           388   40 - #ffffff
           624   40 - #ffffff
             4   48 - #ffffff
           588   48 - #ffffff
            48   52 - #000000
            84   52 - #000000
            48   56 - #000000
            56   56 - #010101
            68   56 - #000000
            80   56 - #000000
            84   56 - #010101
           100   56 - #000000
           132   56 - #979797
           488   56 - #ffffff
           524   56 - #ffffff
            48   60 - #010101
            56   60 - #010101
            60   60 - #010101
            68   60 - #000000
            72   60 - #000000
            80   60 - #ffffff
            84   60 - #000000
            96   60 - #000000
           104   60 - #6c6c6c
           108   60 - #000000
           112   60 - #ffffff
           116   60 - #000000
           132   60 - #979797
           420   60 - #ffffff
            40   64 - #626262
            48   64 - #010101
            72   64 - #010101
            84   64 - #000000
            96   64 - #010101
           104   64 - #6c6c6c
           116   64 - #000000
           132   64 - #979797
           316   72 - #ffffff
           368   72 - #ffffff
           188   76 - #ffffff
           256   76 - #ffffff
           632   76 - #ffffff
           452   80 - #ffffff
           596   80 - #ffffff
            48   88 - #cccccc
            76   88 - #c5c5c5
           156   88 - #ffffff
           224   88 - #ffffff
           400   88 - #ffffff
            40   92 - #000000
            44   92 - #000000
            48   92 - #cccccc
            52   92 - #010101
            72   92 - #000000
            84   92 - #a4a4a4
            88   92 - #000000
           104   92 - #000000
           488   92 - #ffffff
            48   96 - #cccccc
            72   96 - #010101
            84   96 - #a4a4a4
            88   96 - #010101
            92   96 - #919191
           104   96 - #000000
           344   96 - #ffffff
           560   96 - #ffffff
             4  100 - #ffffff
           100  100 - #ffffff
           104  100 - #000000
           604  112 - #ffffff
           420  116 - #ffffff
            40  120 - #ffffff
           112  120 - #000000
           144  120 - #000000
           164  120 - #020202
           208  120 - #2a2a2a
           372  120 - #ffffff
           460  120 - #ffffff
            64  124 - #cfcfcf
           132  124 - #ffffff
           180  124 - #000000
           184  124 - #000000
           188  124 - #000000
           192  124 - #000000
           196  124 - #000000
           200  124 - #000000
           204  124 - #000000
           208  124 - #000000
           212  124 - #000000
           224  124 - #9c9c9c
           232  124 - #d1d1d1
           236  124 - #ffffff
           248  124 - #d29379
           260  124 - #f2dfd7
           280  124 - #d69a83
           284  124 - #bd5e39
           292  124 - #faf4f1
           308  124 - #ffffff
           316  124 - #ffffff
           324  124 - #ffffff
           524  124 - #ffffff
             4  140 - #ffffff
            60  148 - #ffffff
            88  148 - #aaaaaa
           100  148 - #000000
           632  148 - #ffffff
           560  156 - #ffffff
           276  160 - #ffffff
           356  160 - #ffffff
           448  160 - #ffffff
           500  160 - #ffffff
           596  160 - #ffffff
           128  168 - #dddddd
           316  168 - #ffffff
           396  168 - #ffffff
            76  172 - #ffffff
            84  172 - #ffffff
           112  172 - #ffffff
           128  172 - #dddddd
           140  172 - #040404
           240  172 - #ffffff
           192  176 - #ffffff
            40  180 - #ffffff
             4  184 - #ffffff
           128  188 - #a0a0a0
           112  192 - #6a6a6a
           616  192 - #ffffff
           280  196 - #ffffff
           528  196 - #ffffff
           348  200 - #ffffff
           476  200 - #ffffff
           248  208 - #ffffff
           432  212 - #ffffff
           572  212 - #ffffff
            60  216 - #595959
            60  220 - #d3d3d3
            64  220 - #c4c4c4
           216  220 - #ffffff
            64  224 - #c4c4c4
            68  224 - #111111
           308  224 - #ffffff
           164  228 - #ffffff
           388  228 - #ffffff
             8  232 - #ffffff
           504  236 - #ffffff
           632  236 - #ffffff
           100  240 - #6d6d6d
            88  244 - #000000
           460  244 - #ffffff
           584  244 - #ffffff
           252  248 - #ffffff
           340  248 - #ffffff
           544  248 - #ffffff
           416  260 - #ffffff
           184  264 - #ffffff
           296  264 - #ffffff
            40  268 - #ffffff
            44  268 - #ffffff
           608  268 - #ffffff
            36  272 - #6b7280
            40  272 - #ffffff
            44  272 - #ffffff
            48  272 - #b5b9c0
           128  272 - #000000
            36  276 - #6c7381
            40  276 - #ffffff
            44  276 - #ffffff
            48  276 - #b5b9c0
           100  276 - #2c2c2c
           104  276 - #1c1c1c
           128  276 - #000000
           140  276 - #929292
           144  276 - #050505
           376  276 - #ffffff
           572  280 - #ffffff
           220  284 - #ffffff
           500  284 - #ffffff
           452  288 - #ffffff
           536  288 - #ffffff
            40  292 - #2f7df6
            44  292 - #2f7df6
            48  292 - #2f7df6
           104  292 - #3a3a3a
           332  292 - #ffffff
            36  296 - #2f7df6
            48  296 - #2f7df6
            72  296 - #ffffff
           104  296 - #000000
           120  296 - #020202
           256  296 - #ffffff
            40  300 - #2f7df6
            44  300 - #2f7df6
            48  300 - #2f7df6
           596  304 - #ffffff
           632  304 - #ffffff
             4  308 - #ffffff
           292  308 - #ffffff
           412  308 - #ffffff
            36  320 - #dce0e6
            36  324 - #dce0e6
            72  324 - #898e99
            76  324 - #787f8b
            88  324 - #6e7583
           132  324 - #838995
           148  324 - #757c89
           156  324 - #6b7280
           164  324 - #a1a5ae
           372  324 - #ffffff
           524  324 - #ffffff
            36  328 - #dce0e6
            68  328 - #6b7280
            76  328 - #6b7280
           132  328 - #838995
           148  328 - #757c89
           156  328 - #6b7280
           164  328 - #a1a5ae
           172  328 - #b8bcc2
           576  332 - #ffffff
           256  336 - #ffffff
           332  336 - #ffffff
           448  336 - #ffffff
           488  340 - #ffffff
           212  344 - #eef1f5
           404  352 - #eef1f5
             4  356 - #ffffff
           368  360 - #eef1f5
           104  364 - #eef1f5
           112  364 - #eef1f5
           120  364 - #6b7280
           128  364 - #7d8491
           136  364 - #d4d8de
           144  364 - #eef1f5
           184  364 - #eef1f5
           292  364 - #eef1f5
           532  364 - #eef1f5
           616  364 - #ffffff
           260  372 - #eef1f5
           328  372 - #eef1f5
           112  376 - #eef1f5
           432  376 - #eef1f5
           472  376 - #eef1f5
            64  380 - #dfdaf2
            88  380 - #b5cfee
            92  380 - #5f9ce4
           104  380 - #99beeb
           112  380 - #eef1f5
           104  384 - #93bbea
           228  384 - #eef1f5
           356  392 - #eef1f5
           560  392 - #eef1f5
             4  396 - #ffffff
           136  396 - #0a6ada
           152  396 - #c6d9f0
           168  396 - #b9621f
           172  396 - #eef1f5
           400  396 - #eef1f5
           176  400 - #d9b9a1
           512  400 - #eef1f5
           256  408 - #eef1f5
           296  408 - #eef1f5
            40  412 - #eef1f5
            80  416 - #eef1f5
           328  416 - #eef1f5
           436  416 - #eef1f5
           476  416 - #eef1f5
           592  420 - #eef1f5
           364  424 - #eef1f5
           632  424 - #ffffff
             4  432 - #ffffff
           116  432 - #fefefe
           192  432 - #fefefe
           536  432 - #fefefe
           232  436 - #ffffff
           392  444 - #ffffff
           500  444 - #ffffff
           336  448 - #ffffff
            36  452 - #dce0e6
            36  456 - #dce0e6
            64  456 - #0f0f0f
            80  456 - #262627
           156  456 - #a8aaad
           564  456 - #ffffff
           604  456 - #ffffff
            36  460 - #dce0e6
            76  460 - #000000
           148  460 - #000000
           168  460 - #000000
           280  460 - #ffffff
            36  464 - #dce0e6
           208  464 - #eaedf1
            36  468 - #dce0e6
           112  468 - #eaedf1
            36  472 - #dce0e6
           244  472 - #ffffff
           444  472 - #ffffff
            36  476 - #dce0e6
            36  480 - #dce0e6
           484  480 - #ffffff
            36  484 - #dce0e6
           632  484 - #ffffff
            36  488 - #dce0e6
           524  488 - #ffffff
            36  492 - #dce0e6
            84  492 - #ffffff
            88  492 - #ffffff
           140  492 - #b3b3b3
           368  492 - #ffffff
           560  492 - #ffffff
            36  496 - #dce0e6
           408  496 - #ffffff
            36  500 - #dce0e6
           316  500 - #ffffff
            36  504 - #dce0e6
            40  504 - #e5e8ec
            44  504 - #e5e8ec
            48  504 - #e5e8ec
            52  504 - #e5e8ec
            56  504 - #e5e8ec
            60  504 - #e5e8ec
            64  504 - #e5e8ec
            68  504 - #e5e8ec
            72  504 - #e5e8ec
            76  504 - #e5e8ec
            80  504 - #e5e8ec
            84  504 - #e5e8ec
            88  504 - #e5e8ec
            92  504 - #e5e8ec
            96  504 - #e5e8ec
           100  504 - #e5e8ec
           104  504 - #e5e8ec
           108  504 - #e5e8ec
           112  504 - #e5e8ec
           116  504 - #e5e8ec
           120  504 - #e5e8ec
           124  504 - #e5e8ec
           128  504 - #e5e8ec
           132  504 - #e5e8ec
           136  504 - #e5e8ec
           140  504 - #e5e8ec
           144  504 - #e5e8ec
           148  504 - #e5e8ec
           152  504 - #e5e8ec
           156  504 - #e5e8ec
           160  504 - #e5e8ec
           164  504 - #e5e8ec
           168  504 - #e5e8ec
           172  504 - #e5e8ec
           176  504 - #e5e8ec
           180  504 - #e5e8ec
           184  504 - #e5e8ec
           188  504 - #e5e8ec
           192  504 - #e5e8ec
           196  504 - #e5e8ec
           200  504 - #e5e8ec
           204  504 - #e5e8ec
           208  504 - #e5e8ec
           212  504 - #e5e8ec
           216  504 - #e5e8ec
           220  504 - #e5e8ec
            36  508 - #dce0e6
           268  508 - #ffffff
           444  508 - #ffffff
            36  512 - #dce0e6
            36  516 - #dce0e6
           144  516 - #ffffff
           152  516 - #ffffff
           164  516 - #141414
           208  516 - #a9a9a9
           584  516 - #ffffff
           620  516 - #ffffff
            36  520 - #dce0e6
            76  520 - #434343
           140  520 - #4e4e4e
           156  520 - #ffffff
           180  520 - #ffffff
           184  520 - #010101
           204  520 - #ffffff
           208  520 - #a9a9a9
           480  520 - #ffffff
            36  524 - #dce0e6
            36  528 - #dce0e6
           348  532 - #ffffff
           404  536 - #ffffff
           308  540 - #ffffff
           532  540 - #ffffff
           444  544 - #ffffff
           236  548 - #ffffff
             4  552 - #ffffff
           596  552 - #ffffff
           632  552 - #ffffff
           120  556 - #ffffff
           372  556 - #ffffff
           184  560 - #ffffff
           272  560 - #ffffff
           480  564 - #ffffff
            84  568 - #010101
            40  572 - #2b2b2b
            76  572 - #000000
            84  572 - #000000
           540  572 - #ffffff
           344  576 - #ffffff
           576  580 - #ffffff
           148  584 - #ffffff
           216  584 - #ffffff
           392  584 - #ffffff
           304  588 - #ffffff
           444  588 - #ffffff
           512  592 - #ffffff
           268  596 - #ffffff
           112  600 - #ffffff
           184  600 - #ffffff
           476  604 - #ffffff
           596  608 - #ffffff
             4  612 - #ffffff
           328  612 - #ffffff
           632  612 - #ffffff
           360  616 - #ffffff
           548  616 - #ffffff
            84  620 - #ffffff
           216  620 - #ffffff
           432  620 - #ffffff
           148  624 - #ffffff
           248  628 - #ffffff
            52  632 - #ffffff
           300  632 - #ffffff
           500  632 - #ffffff
           184  636 - #ffffff
           400  636 - #ffffff
           112  640 - #ffffff
           584  644 - #ffffff
           620  644 - #ffffff
           460  648 - #ffffff
           272  656 - #ffffff
           348  656 - #ffffff
           528  660 - #ffffff
           168  668 - #ffffff
             4  672 - #ffffff
            92  672 - #ffffff
           204  672 - #ffffff
           492  672 - #ffffff
           560  676 - #ffffff
           596  676 - #ffffff
           632  676 - #ffffff
           136  680 - #ffffff
            56  684 - #ffffff
           244  684 - #ffffff
           388  684 - #ffffff
           296  688 - #ffffff
           432  688 - #ffffff
            24  704 - #ffffff
           176  708 - #ffffff
           472  708 - #ffffff
           580  708 - #ffffff
           616  708 - #ffffff
           100  712 - #ffffff
           344  712 - #ffffff
           524  712 - #ffffff
           220  720 - #ffffff
           308  720 - #ffffff
           376  720 - #ffffff
           268  724 - #ffffff
           408  728 - #ffffff
           144  732 - #ffffff
            64  740 - #ffffff
           560  740 - #ffffff
           596  740 - #ffffff
           632  740 - #ffffff
            16  744 - #ffffff
           332  744 - #ffffff
           444  744 - #ffffff
           188  748 - #ffffff
           368  752 - #ffffff
           484  752 - #ffffff
           112  756 - #ffffff
           300  756 - #ffffff
           240  760 - #ffffff
           528  764 - #ffffff
           404  772 - #ffffff
           584  772 - #ffffff
           620  772 - #ffffff
           156  776 - #ffffff
            32  784 - #ffffff
           444  784 - #ffffff
            76  788 - #ffffff
           200  788 - #ffffff
           328  788 - #ffffff
           488  796 - #ffffff
           252  800 - #ffffff
           368  800 - #ffffff
           632  804 - #ffffff
           536  808 - #ffffff
           120  812 - #ffffff
           412  812 - #ffffff
            52  816 - #ffffff
           292  816 - #ffffff
             4  820 - #ffffff
           168  820 - #ffffff
           456  824 - #ffffff
           336  828 - #ffffff
           212  832 - #ffffff
           572  832 - #ffffff
           608  832 - #ffffff
           504  836 - #ffffff
           256  844 - #ffffff
           380  844 - #ffffff
            36  848 - #ffffff
            80  848 - #ffffff
           540  848 - #ffffff
           424  856 - #ffffff
           140  860 - #ffffff
           304  860 - #ffffff
           592  864 - #ffffff
           632  864 - #ffffff
           468  868 - #ffffff
           184  872 - #ffffff
             4  876 - #ffffff
           228  876 - #ffffff
           348  876 - #ffffff
           500  876 - #ffffff
           532  884 - #ffffff
           104  888 - #ffffff
           268  888 - #ffffff
           392  888 - #ffffff
           316  892 - #ffffff
           156  896 - #ffffff
           620  896 - #ffffff
           440  900 - #ffffff
           584  900 - #ffffff
            52  904 - #ffffff
           484  912 - #ffffff
           552  912 - #ffffff
           196  916 - #ffffff
           236  916 - #ffffff
           132  924 - #ffffff
           276  924 - #ffffff
           316  928 - #ffffff
           408  928 - #ffffff
            88  932 - #ffffff
           632  932 - #ffffff
             4  936 - #ffffff
           168  936 - #ffffff
           528  936 - #ffffff
           596  936 - #ffffff
           360  940 - #ffffff
           452  948 - #ffffff
           492  952 - #ffffff
            52  956 - #ffffff
           208  956 - #ffffff
           252  956 - #ffffff
           292  960 - #ffffff
           332  960 - #ffffff
           416  964 - #ffffff
           572  964 - #ffffff
           148  968 - #ffffff
           100  980 - #ffffff
           376  984 - #ffffff
           476  988 - #ffffff
           512  988 - #ffffff
            12  992 - #ffffff
            64  992 - #ffffff
           188  992 - #ffffff
           232  992 - #ffffff
           272  992 - #ffffff
           312  992 - #ffffff
           440  992 - #ffffff
           548  992 - #ffffff
           596  992 - #ffffff
           632  992 - #ffffff
";

#[view]
struct MarkdownViewTest {
    #[init]
    text: MarkdownView,
}

impl Setup for MarkdownViewTest {
    fn setup(self: Weak<Self>) {
        self.set_color(WHITE);
        self.text.set_text_color(BLACK);
        // A tap must not open a browser on the machine that runs the test.
        self.text.set_opens_links(false);
        self.text.set_text(TEXT);
        let height = self.text.height_for_width(WIDE);
        self.text.set_frame((20.0, 20.0, WIDE, height));
    }
}

impl ViewTest for MarkdownViewTest {
    fn canvas() -> (u32, u32) {
        (640, 1000)
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        set_record_probe_count(600);

        check_colors(DRAWN)?;

        // A tap on the link gives its address, a tap on plain text nothing.
        let tapped = from_main(move || {
            let (sender, receiver) = std::sync::mpsc::channel();
            view.text.link_tapped.val(move |url| {
                sender.send(url).expect("the test waits for the link");
            });
            receiver
        });
        inject_touches("60 28 b\n60 28 e");
        assert_eq!(
            tapped.try_recv().ok().as_deref(),
            Some("https://example.com/page")
        );
        inject_touches("400 28 b\n400 28 e");
        assert!(tapped.try_recv().is_err(), "a tap on plain text hit a link");

        // The same text needs more height in less width, it wraps.
        let (wide, narrow) = from_main(move || {
            let wide = view.text.height_for_width(WIDE);
            let narrow = view.text.height_for_width(NARROW);
            view.text.height_for_width(WIDE);
            (wide, narrow)
        });
        assert!(
            narrow > wide,
            "{narrow} at {NARROW} is not above {wide} at {WIDE}"
        );

        Ok(())
    }
}
