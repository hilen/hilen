use std::ops::Deref;

use anyhow::Result;
use parking_lot::Mutex;

use crate::{
    self as hilen,
    deps::{hreads::from_main, refs::Weak},
    gm::color::{Color, WHITE},
    ui::{CellRegistry, Label, Setup, TableData, TableView, View, ViewData, ViewTest, view},
    ui_test::{check_colors, inject_touches, set_record_probe_count},
};

static SELECTED: Mutex<String> = Mutex::new(String::new());

const NATURAL: &str = r"
              60    4 - #3a4a5a
             336    4 - #3a4a5a
             396    4 - #26303b
             592    4 - #597c95
               4    8 - #3a4a5a
             144   12 - #3a4a5a
             192   12 - #9099a2
             264   20 - #afb5bc
             164   24 - #3a4a5a
             192   24 - #9099a2
             200   24 - #ffffff
             396   24 - #26303b
             232   28 - #3a4a5a
             100   36 - #3a4a5a
             396   40 - #93979b
             396   44 - #93979b
             500   44 - #597c95
             164   48 - #454b51
             236   56 - #22282e
             184   60 - #e2e8ee
               4   64 - #e2e8ee
             160   68 - #21272d
             200   68 - #20262c
             320   88 - #f2f5f8
             160   92 - #21272d
             592   92 - #597c95
             188  104 - #f2f5f8
             236  108 - #979b9f
             240  108 - #979b9f
             396  112 - #f2f5f8
             164  132 - #e2e8ee
             196  136 - #20262c
              64  140 - #e2e8ee
             240  144 - #21272d
             520  144 - #597c95
             356  164 - #f2f5f8
             160  172 - #21272d
             296  172 - #f2f5f8
             196  176 - #20262c
             236  176 - #f2f5f8
             592  196 - #597c95
             160  208 - #454b51
             240  208 - #454b51
               4  216 - #e2e8ee
             188  220 - #e2e8ee
             160  228 - #21272d
              84  232 - #e2e8ee
             316  236 - #e2e8ee
             488  240 - #597c95
             164  256 - #f2f5f8
             236  260 - #f2f5f8
             200  268 - #20262c
             396  280 - #e2e8ee
             592  284 - #597c95
             168  292 - #e2e8ee
             188  300 - #e2e8ee
             236  300 - #21272d
             160  304 - #21272d
              56  308 - #e2e8ee
             320  316 - #e2e8ee
             164  328 - #484d52
             196  336 - #20262c
             492  340 - #597c95
             160  344 - #21272d
             232  344 - #f2f5f8
             388  356 - #f2f5f8
              96  360 - #e2e8ee
             236  376 - #e2e8ee
             160  380 - #20262c
             184  380 - #e2e8ee
             196  388 - #e2e8ee
               4  400 - #3a4a5a
             364  400 - #3a4a5a
             316  404 - #3a4a5a
             192  412 - #9099a2
             268  412 - #3a4a5a
             144  416 - #3a4a5a
              72  420 - #3a4a5a
             192  420 - #9099a2
             236  420 - #ffffff
             164  424 - #3a4a5a
             192  424 - #9099a2
             216  424 - #3a4a5a
             188  428 - #3a4a5a
             396  436 - #3a4a5a
             556  436 - #597c95
             240  448 - #e2e8ee
             152  452 - #5c6268
             228  452 - #989ea4
             152  456 - #5c6268
             176  460 - #e2e8ee
             228  460 - #989ea4
             152  464 - #5c6268
             200  464 - #20262c
             152  468 - #5c6268
             228  468 - #989ea4
              52  488 - #f2f5f8
             152  492 - #61666c
             228  492 - #a2a6aa
             152  496 - #61666c
             172  500 - #f1f4f7
             228  500 - #a2a6aa
             152  508 - #61666c
             192  508 - #20262c
             228  508 - #a2a6aa
             244  508 - #979b9f
             248  508 - #979b9f
             340  512 - #f2f5f8
             152  532 - #5c6268
             488  532 - #597c95
             228  536 - #989ea4
             172  540 - #e2e8ee
             152  544 - #5c6268
             192  548 - #20262c
             228  548 - #989ea4
              80  556 - #e2e8ee
             152  572 - #61666c
             228  572 - #a2a6aa
               4  580 - #f2f5f8
             176  580 - #f2f5f8
             248  580 - #21272d
             152  584 - #61666c
             156  588 - #f2f5f8
             196  588 - #f2f5f8
             228  588 - #a2a6aa
             308  588 - #f2f5f8
             388  592 - #f2f5f8
             592  592 - #597c95
            ";

const FIRST_PINNED: &str = r"
              72    4 - #3a4a5a
             336    4 - #3a4a5a
             580    4 - #597c95
             144   12 - #3a4a5a
             192   12 - #9099a2
             396   12 - #26303b
             192   20 - #9099a2
             236   20 - #ffffff
             264   20 - #afb5bc
             164   24 - #3a4a5a
             192   24 - #9099a2
             212   24 - #3a4a5a
             488   28 - #597c95
               4   36 - #3a4a5a
             184   44 - #e2e8ee
             240   44 - #21272d
             396   44 - #93979b
             396   48 - #93979b
             160   72 - #21272d
             236   76 - #f2f5f8
             184   80 - #f2f5f8
             160   88 - #21272d
              84   92 - #f2f5f8
             160  108 - #454b51
               8  112 - #e2e8ee
             332  116 - #e2e8ee
             240  120 - #20262c
             188  124 - #e2e8ee
             160  128 - #21272d
             488  128 - #597c95
             160  156 - #21272d
             236  160 - #f2f5f8
             188  164 - #f2f5f8
               4  188 - #e2e8ee
             160  188 - #454b51
             384  192 - #e2e8ee
             592  192 - #597c95
             236  200 - #21272d
             184  204 - #e2e8ee
             160  208 - #21272d
              80  216 - #e2e8ee
             312  220 - #f2f5f8
             160  232 - #21272d
             236  244 - #f2f5f8
             160  248 - #21272d
             200  248 - #20262c
              20  252 - #f2f5f8
             388  264 - #e2e8ee
             160  268 - #454b51
             188  280 - #e2e8ee
             240  280 - #21272d
             168  288 - #e2e8ee
             204  288 - #e2e8ee
             512  288 - #597c95
              64  300 - #3a4a5a
             320  300 - #3a4a5a
             144  312 - #3a4a5a
             192  312 - #9099a2
               4  316 - #3a4a5a
             192  320 - #9099a2
             216  320 - #3a4a5a
             192  324 - #9099a2
             236  324 - #ffffff
             168  328 - #3c4c5c
             100  336 - #3a4a5a
             284  336 - #3a4a5a
             348  336 - #3a4a5a
             396  336 - #3a4a5a
             228  352 - #989ea4
             152  356 - #5c6268
             196  356 - #20262c
             176  364 - #e2e8ee
             244  364 - #21272d
             152  368 - #5c6268
             228  368 - #989ea4
               4  380 - #f2f5f8
             328  384 - #f2f5f8
             484  388 - #597c95
             152  392 - #61666c
             228  392 - #a2a6aa
             588  392 - #597c95
             188  400 - #21272d
             396  404 - #f2f5f8
             152  408 - #61666c
             228  408 - #a2a6aa
             244  408 - #979b9f
             248  408 - #979b9f
              72  420 - #e2e8ee
             152  428 - #454b51
             240  436 - #e2e8ee
             188  440 - #21272d
             152  444 - #5c6268
             172  444 - #e2e8ee
               8  448 - #e2e8ee
             228  448 - #989ea4
             360  460 - #f2f5f8
             156  472 - #f2f5f8
             228  472 - #a2a6aa
             248  476 - #20262c
             172  484 - #f1f4f7
             152  488 - #61666c
             192  488 - #20262c
             228  488 - #a2a6aa
             592  488 - #597c95
             496  492 - #597c95
              52  500 - #e2e8ee
             248  508 - #454b51
             152  512 - #5c6268
             228  512 - #989ea4
             172  520 - #e2e8ee
             152  524 - #5c6268
             192  528 - #20262c
             228  528 - #989ea4
             312  540 - #f2f5f8
             152  552 - #61666c
             228  552 - #a2a6aa
             172  564 - #f1f4f7
             200  564 - #20262c
             244  564 - #f2f5f8
             152  568 - #61666c
              80  572 - #f2f5f8
             388  576 - #f2f5f8
               4  592 - #e2e8ee
             152  592 - #5c6268
             228  592 - #989ea4
             240  592 - #e2e8ee
             488  592 - #597c95
             592  592 - #597c95
            ";

const PUSHED_AWAY: &str = r"
              56    4 - #3a4a5a
             136    4 - #fefefe
             220    4 - #3a4a5a
             348    4 - #3a4a5a
             592    4 - #597c95
             192    8 - #9099a2
             264    8 - #afb5bc
               4   12 - #3a4a5a
             192   12 - #9099a2
             164   16 - #3a4a5a
             236   16 - #ffffff
             308   32 - #3a4a5a
             396   32 - #26303b
             256   40 - #ffffff
             496   40 - #597c95
             140   44 - #3a4a5a
             160   48 - #3a4a5a
             192   48 - #9099a2
             192   52 - #9099a2
             216   52 - #3a4a5a
             396   52 - #26303b
             236   56 - #ffffff
             172   60 - #3a4a5a
               4   68 - #3a4a5a
              76   68 - #3a4a5a
             396   72 - #93979b
             244   80 - #20262c
             312   84 - #e2e8ee
             188   88 - #20262c
             172   92 - #e2e8ee
             152   96 - #5c6268
             228   96 - #989ea4
             244   96 - #21272d
             188  128 - #20262c
             228  128 - #a2a6aa
               4  132 - #f2f5f8
              84  136 - #f2f5f8
             152  136 - #61666c
             528  136 - #597c95
             228  160 - #989ea4
             152  164 - #5c6268
             240  172 - #e2e8ee
             348  172 - #e2e8ee
             176  176 - #e2e8ee
              36  184 - #e2e8ee
             188  208 - #20262c
             244  208 - #f2f5f8
             152  216 - #61666c
             228  216 - #a2a6aa
             248  216 - #21272d
             492  236 - #597c95
             248  240 - #3b4147
             188  248 - #20262c
             160  252 - #21272d
             316  252 - #e2e8ee
             228  256 - #989ea4
             592  268 - #597c95
              76  272 - #f2f5f8
             396  272 - #f2f5f8
             228  280 - #a2a6aa
             152  284 - #61666c
               4  296 - #f2f5f8
             176  296 - #f2f5f8
             228  296 - #a2a6aa
             244  300 - #20262c
             240  320 - #757b81
             244  320 - #757b81
             312  320 - #e2e8ee
             176  328 - #e2e8ee
             204  332 - #20262c
             396  332 - #e2e8ee
             152  336 - #5c6268
             228  336 - #989ea4
              68  360 - #f2f5f8
             228  360 - #a2a6aa
             592  360 - #597c95
               4  368 - #f2f5f8
             176  368 - #f2f5f8
             204  372 - #20262c
             152  376 - #61666c
             244  376 - #f2f5f8
             352  376 - #f2f5f8
             152  404 - #5c6268
             240  404 - #e2e8ee
             512  404 - #597c95
             176  408 - #e2e8ee
             228  416 - #989ea4
             156  420 - #e2e8ee
             196  420 - #e2e8ee
              20  432 - #3a4a5a
             396  432 - #3a4a5a
              80  436 - #3a4a5a
             140  444 - #3a4a5a
             232  444 - #ffffff
             312  444 - #3a4a5a
             176  448 - #3a4a5a
             192  448 - #9099a2
             192  452 - #9099a2
             200  452 - #ffffff
             592  452 - #597c95
             164  456 - #3a4a5a
             220  460 - #3a4a5a
             260  460 - #f3f4f5
             156  484 - #e2e8ee
             244  484 - #21272d
             380  488 - #e2e8ee
              12  492 - #e2e8ee
             152  496 - #5c6268
             244  496 - #21272d
             480  496 - #597c95
              72  500 - #e2e8ee
             176  500 - #21272d
             220  500 - #2c3238
             316  516 - #f2f5f8
             248  524 - #20262c
             188  528 - #20262c
             152  536 - #61666c
             220  540 - #2d3339
             592  544 - #597c95
               4  552 - #e2e8ee
             396  560 - #e2e8ee
             152  564 - #5c6268
             176  576 - #e2e8ee
             244  576 - #e2e8ee
             220  580 - #2c3238
              84  592 - #f2f5f8
             324  592 - #f2f5f8
             464  592 - #597c95
            ";

const SECOND_PINNED: &str = r"
              76    4 - #3a4a5a
             316    4 - #3a4a5a
             396    4 - #3a4a5a
             592    4 - #597c95
             144   12 - #3a4a5a
             192   12 - #9099a2
             156   20 - #ffffff
             192   20 - #9099a2
             236   20 - #ffffff
             180   24 - #3a4a5a
             192   24 - #9099a2
             216   24 - #3a4a5a
             168   28 - #3c4c5c
             280   32 - #3a4a5a
               4   36 - #3a4a5a
             396   40 - #9d9fa1
             156   44 - #f2f5f8
             200   48 - #f1f4f7
             500   52 - #597c95
             176   56 - #f2f5f8
             228   56 - #a2a6aa
             376   60 - #f2f5f8
             300   76 - #e2e8ee
             396   76 - #93979b
              72   84 - #e2e8ee
             176   88 - #e2e8ee
             152   96 - #5c6268
             228   96 - #989ea4
             228  120 - #a2a6aa
             188  128 - #20262c
             248  132 - #21272d
             152  136 - #61666c
             336  140 - #f2f5f8
             536  148 - #597c95
              32  156 - #e2e8ee
             248  160 - #3b4147
             188  168 - #20262c
             228  168 - #989ea4
             172  172 - #e2e8ee
             152  176 - #5c6268
             152  204 - #61666c
             228  204 - #a2a6aa
             304  204 - #f2f5f8
             396  208 - #f2f5f8
             204  212 - #20262c
             176  216 - #f2f5f8
             244  216 - #f2f5f8
             484  216 - #597c95
              72  228 - #f2f5f8
             240  240 - #757b81
             244  240 - #757b81
             152  244 - #5c6268
             204  252 - #20262c
             176  256 - #e2e8ee
             228  256 - #989ea4
             332  268 - #e2e8ee
               4  272 - #f2f5f8
             228  280 - #a2a6aa
             164  284 - #21272d
             248  284 - #21272d
             396  288 - #f2f5f8
             592  288 - #597c95
              84  292 - #f2f5f8
             204  292 - #20262c
             152  296 - #61666c
             228  296 - #a2a6aa
             476  300 - #597c95
             248  324 - #21272d
             176  328 - #e2e8ee
             228  332 - #989ea4
             152  336 - #5c6268
             196  340 - #e2e8ee
              52  352 - #3a4a5a
             312  352 - #3a4a5a
             372  352 - #3a4a5a
             140  364 - #3a4a5a
             232  364 - #ffffff
             176  368 - #3a4a5a
             192  368 - #9099a2
             212  368 - #3a4a5a
               4  372 - #3a4a5a
             192  372 - #9099a2
             164  376 - #3a4a5a
              92  380 - #3a4a5a
             220  380 - #3a4a5a
             260  380 - #f3f4f5
             340  388 - #3a4a5a
             152  404 - #5c6268
             244  404 - #21272d
             456  404 - #597c95
             188  408 - #20262c
             164  412 - #e2e8ee
             244  416 - #21272d
             220  420 - #2c3238
               4  428 - #e2e8ee
              84  428 - #e2e8ee
             392  428 - #e2e8ee
             328  432 - #f2f5f8
             548  440 - #597c95
             152  444 - #61666c
             248  444 - #20262c
             176  452 - #f2f5f8
             220  460 - #2d3339
             156  488 - #e2e8ee
             188  488 - #20262c
             236  488 - #e2e8ee
             172  492 - #e2e8ee
             396  492 - #e2e8ee
             220  500 - #2c3238
             484  500 - #597c95
              60  508 - #e2e8ee
             324  508 - #e2e8ee
             248  520 - #20262c
             176  528 - #f2f5f8
             204  532 - #20262c
             152  536 - #61666c
             248  536 - #21272d
             220  540 - #2d3339
             248  560 - #3b4147
             152  564 - #5c6268
             232  568 - #e1e7ed
             176  576 - #e2e8ee
             220  580 - #2c3238
               4  588 - #e2e8ee
             472  588 - #597c95
              80  592 - #f2f5f8
             352  592 - #f2f5f8
             592  592 - #597c95
            ";

const MIDDLE: &str = r"
              16    4 - #3a4a5a
              76    4 - #3a4a5a
             396    4 - #3a4a5a
             556    4 - #597c95
             260    8 - #3a4a5a
             128   12 - #fefefe
             180   12 - #929ba4
             328   16 - #3a4a5a
             152   20 - #465564
             216   20 - #fafbfb
             156   40 - #e2e8ee
             232   40 - #e2e8ee
             252   44 - #7e848a
             192   48 - #20262c
             216   48 - #81878d
             252   48 - #7e848a
               8   60 - #f2f5f8
              76   72 - #f2f5f8
             216   76 - #898d92
             236   76 - #f2f5f8
             140   84 - #b0b4b8
             184   84 - #22282e
             340   84 - #f2f5f8
             216   88 - #898d92
             256   88 - #979b9f
             492  100 - #597c95
             144  108 - #454b51
             168  120 - #e2e8ee
             216  120 - #81878d
             140  124 - #a5abb1
             240  124 - #20262c
             192  128 - #20262c
              56  136 - #e2e8ee
             144  152 - #f1f4f7
             324  152 - #f2f5f8
             216  156 - #898d92
             592  156 - #597c95
             168  160 - #f2f5f8
             256  160 - #21272d
             396  160 - #f2f5f8
             192  168 - #20262c
             216  168 - #898d92
             256  188 - #454b51
             140  192 - #a5abb1
             216  200 - #81878d
             168  204 - #e2e8ee
             240  204 - #20262c
               4  208 - #e2e8ee
             336  232 - #f2f5f8
             216  236 - #898d92
             248  236 - #22282e
             140  240 - #b0b4b8
             184  244 - #22282e
             216  248 - #898d92
              64  256 - #f2f5f8
             516  256 - #597c95
               4  272 - #e2e8ee
             140  272 - #a5abb1
             256  276 - #22282e
             196  280 - #21272d
             232  280 - #e2e8ee
             160  284 - #21272d
             324  304 - #f2f5f8
             396  304 - #9d9fa1
             140  312 - #b0b4b8
              76  316 - #f2f5f8
             236  316 - #f2f5f8
             216  320 - #898d92
             396  320 - #9d9fa1
             168  324 - #f2f5f8
             256  324 - #f0f3f6
              16  336 - #f2f5f8
             396  340 - #93979b
             144  348 - #454b51
             592  356 - #597c95
             216  360 - #81878d
             492  360 - #597c95
             164  364 - #e2e8ee
             240  364 - #20262c
             140  368 - #a5abb1
             188  368 - #e2e8ee
              84  380 - #3a4a5a
             316  380 - #3a4a5a
             396  380 - #3a4a5a
               4  388 - #3a4a5a
             260  388 - #3a4a5a
             132  392 - #3a4a5a
             228  400 - #ffffff
             152  404 - #3a4a5a
             204  404 - #3a4a5a
             176  408 - #3a4a5a
             356  408 - #3a4a5a
              48  416 - #3a4a5a
             236  432 - #22282e
             148  440 - #20262c
             216  440 - #81878d
             252  440 - #7e848a
             168  444 - #e2e8ee
             192  448 - #20262c
             344  456 - #e2e8ee
             436  456 - #597c95
             236  472 - #22282e
             520  472 - #597c95
             140  480 - #b0b4b8
             216  480 - #898d92
             168  484 - #f2f5f8
             256  488 - #979b9f
               4  500 - #e2e8ee
             140  512 - #a5abb1
             248  516 - #e2e8ee
             168  520 - #e2e8ee
             216  520 - #81878d
              72  524 - #e2e8ee
             320  524 - #e2e8ee
             236  528 - #22282e
             396  536 - #e2e8ee
             140  552 - #b0b4b8
             256  552 - #20262c
             188  556 - #20262c
             236  556 - #22282e
             156  564 - #f2f5f8
             216  568 - #898d92
             256  588 - #454b51
              56  592 - #e2e8ee
             144  592 - #e1e7ed
             336  592 - #e2e8ee
             484  592 - #597c95
             592  592 - #597c95
            ";

const BOTTOM: &str = r"
              44    4 - #3a4a5a
             320    4 - #3a4a5a
             396    4 - #3a4a5a
             592    4 - #597c95
             260    8 - #3a4a5a
             128   12 - #fefefe
             180   12 - #929ba4
             152   24 - #3a4a5a
             192   24 - #ffffff
             228   24 - #ffffff
               4   32 - #3a4a5a
              84   32 - #3a4a5a
             292   36 - #3a4a5a
             360   48 - #f2f5f8
             140   52 - #b0b4b8
             248   56 - #22282e
             168   64 - #f2f5f8
             216   64 - #898d92
              16   80 - #e2e8ee
              80   84 - #e2e8ee
             144   88 - #454b51
             256   88 - #454b51
             216  100 - #81878d
             492  100 - #597c95
             168  104 - #e2e8ee
             588  104 - #597c95
             144  128 - #484d52
             348  128 - #f2f5f8
             216  136 - #898d92
               4  140 - #f2f5f8
             184  144 - #22282e
             232  144 - #f2f5f8
              64  148 - #f2f5f8
             140  148 - #b0b4b8
             216  148 - #898d92
             144  168 - #454b51
             216  180 - #81878d
             164  184 - #e2e8ee
             252  184 - #e2e8ee
             140  188 - #a5abb1
             188  188 - #e2e8ee
             312  200 - #3a4a5a
             396  200 - #3a4a5a
             592  200 - #597c95
             492  204 - #597c95
               4  208 - #3a4a5a
              72  212 - #3a4a5a
             180  212 - #929ba4
             136  220 - #fefefe
             204  220 - #3a4a5a
             248  220 - #fefefe
             272  220 - #fefefe
             172  224 - #3a4a5a
             224  228 - #3a4a5a
             352  236 - #3a4a5a
             144  248 - #454b51
             196  260 - #21272d
             216  260 - #81878d
             252  260 - #7e848a
             156  264 - #e2e8ee
              56  280 - #f2f5f8
             144  288 - #484d52
             304  288 - #f2f5f8
             216  296 - #898d92
             196  300 - #21272d
             496  300 - #597c95
             168  304 - #f2f5f8
             360  304 - #f2f5f8
             140  308 - #b0b4b8
             216  308 - #898d92
             248  308 - #21272d
             140  336 - #a5abb1
             252  336 - #e2e8ee
             168  340 - #e2e8ee
             216  340 - #81878d
              72  348 - #e2e8ee
             192  348 - #20262c
             316  348 - #e2e8ee
             396  352 - #e2e8ee
               4  356 - #e2e8ee
             140  372 - #b0b4b8
             256  372 - #20262c
             256  376 - #20262c
             216  380 - #898d92
             256  380 - #21272d
             172  384 - #21272d
             248  384 - #21272d
             256  384 - #20262c
             256  388 - #21272d
             492  396 - #597c95
             592  396 - #597c95
             140  412 - #a5abb1
             324  412 - #e2e8ee
             396  412 - #e2e8ee
             172  420 - #21272d
             216  420 - #81878d
             252  424 - #e2e8ee
              64  436 - #e2e8ee
             140  452 - #b0b4b8
               4  456 - #f2f5f8
             248  456 - #22282e
             216  460 - #898d92
             172  464 - #21272d
             340  484 - #e2e8ee
             508  492 - #597c95
             188  496 - #20262c
             140  500 - #a5abb1
             216  500 - #81878d
             164  504 - #e2e8ee
             252  504 - #20262c
               4  520 - #f2f5f8
              72  524 - #f2f5f8
             140  532 - #b0b4b8
             216  536 - #898d92
             168  544 - #f2f5f8
             252  544 - #f2f5f8
             216  548 - #898d92
             324  556 - #f2f5f8
             396  556 - #9d9fa1
             396  572 - #93979b
             216  580 - #81878d
             256  580 - #20262c
             144  588 - #e1e7ed
             188  588 - #e2e8ee
             492  588 - #597c95
              56  592 - #e2e8ee
             396  592 - #93979b
             592  592 - #597c95
            ";

const ROW_H: f32 = 40.0;

/// Every tenth row is a section header, like a diff hunk header.
fn is_header(index: usize) -> bool {
    index.is_multiple_of(10)
}

// Section headers pin to the top of the viewport and the next header
// pushes the pinned one away. Taps prove a pinned header covers the row
// geometry under it, the pinned list pins the exact frames, and the
// color probes prove the pinned cell draws over the rows it covers.
#[view]
struct TableStickyRows {
    #[init]
    table: TableView,
}

impl Setup for TableStickyRows {
    fn setup(mut self: Weak<Self>) {
        self.table.place().tl(0).size(400, 600);
        self.table.set_data_source(self).register_cell::<Label>();
        self.table.set_sticky_rows(true);
        self.table.reload_data();
    }
}

impl TableData for TableStickyRows {
    fn cell_height(&self, _: usize) -> f32 {
        ROW_H
    }

    fn number_of_cells(&self) -> usize {
        200
    }

    fn is_sticky(&self, index: usize) -> bool {
        is_header(index)
    }

    // Styled like a real sectioned list: dark header bars with light
    // text over plain labeled rows on alternating fills, so a human run
    // reads which section is pinned straight off the screen.
    fn setup_cell(&mut self, index: usize, registry: &mut CellRegistry) -> Weak<dyn View> {
        let cell = registry.cell::<Label>();
        if is_header(index) {
            cell.set_text(format!("Section {}", index / 10));
            cell.set_text_color(WHITE);
            cell.set_color(Color::hex("#3a4a5a"));
        } else {
            cell.set_text(format!("Row {index}"));
            cell.set_text_color(Color::hex("#20262c"));
            cell.set_color(if index.is_multiple_of(2) {
                Color::hex("#f2f5f8")
            } else {
                Color::hex("#e2e8ee")
            });
        }
        cell
    }

    fn cell_selected(&mut self, index: usize) {
        *SELECTED.lock() += &format!("|{index}|");
    }
}

impl ViewTest for TableStickyRows {
    fn perform_test(mut view: Weak<Self>) -> Result<()> {
        set_record_probe_count(128);
        // Unscrolled, every row sits at its natural position.
        inject_touches(
            "
                200  20   b
                200  20   e
                200  60   b
                200  60   e
            ",
        );
        assert_eq!(SELECTED.lock().deref(), "|0||1|");
        SELECTED.lock().clear();
        check_colors(NATURAL)?;

        // Scrolled into section 0: header 0 pins to the top and covers
        // the geometry of row 3, which starts 20 points under the edge.
        from_main(move || {
            view.table.set_content_offset(-100);
            assert_eq!(view.table.pinned, vec![(0, 0.0, ROW_H), (10, 300.0, ROW_H)]);
        });
        inject_touches(
            "
                200  20   b
                200  20   e
                200  50   b
                200  50   e
                200  310  b
                200  310  e
            ",
        );
        assert_eq!(SELECTED.lock().deref(), "|0||3||10|");
        SELECTED.lock().clear();
        check_colors(FIRST_PINNED)?;

        // The next header pushes the pinned one away: header 10 is 30
        // points from the top, so header 0 keeps only a 30 point sliver.
        from_main(move || {
            view.table.set_content_offset(-370);
            assert_eq!(
                view.table.pinned,
                vec![(0, -10.0, ROW_H), (10, 30.0, ROW_H), (20, 430.0, ROW_H)]
            );
        });
        inject_touches(
            "
                200  20   b
                200  20   e
                200  35   b
                200  35   e
            ",
        );
        assert_eq!(SELECTED.lock().deref(), "|0||10|");
        SELECTED.lock().clear();
        check_colors(PUSHED_AWAY)?;

        // One section further the old header is fully pushed out and the
        // new one owns the top. Row 12 starts 30 points under the header.
        from_main(move || {
            view.table.set_content_offset(-450);
            assert_eq!(view.table.pinned, vec![(10, 0.0, ROW_H), (20, 350.0, ROW_H)]);
        });
        inject_touches(
            "
                200  20   b
                200  20   e
                200  60   b
                200  60   e
            ",
        );
        assert_eq!(SELECTED.lock().deref(), "|10||12|");
        SELECTED.lock().clear();

        // The pinned header bar draws over the rows it covers, the plain
        // rows around it keep their own fills, and the next header rides
        // at its natural position.
        check_colors(SECOND_PINNED)?;

        // Deep in the middle of the list: section 10 owns the top at
        // offset -4020, row 102 sits 60 points under the edge and
        // section 11 rides at its natural position.
        from_main(move || {
            view.table.set_content_offset(-4020);
            assert_eq!(view.table.pinned, vec![(100, 0.0, ROW_H), (110, 380.0, ROW_H)]);
        });
        inject_touches(
            "
                200  20   b
                200  20   e
                200  60   b
                200  60   e
                200  390  b
                200  390  e
            ",
        );
        assert_eq!(SELECTED.lock().deref(), "|100||102||110|");
        SELECTED.lock().clear();
        check_colors(MIDDLE)?;

        // Clamped to the very bottom: section 18 pins at the top,
        // section 19 rides at 200, and the last row is flush with the
        // bottom edge.
        from_main(move || {
            view.table.set_content_offset(-1_000_000);
            assert_eq!(view.table.pinned, vec![(180, 0.0, ROW_H), (190, 200.0, ROW_H)]);
        });
        inject_touches(
            "
                200  20   b
                200  20   e
                200  220  b
                200  220  e
                200  590  b
                200  590  e
            ",
        );
        assert_eq!(SELECTED.lock().deref(), "|180||190||199|");
        SELECTED.lock().clear();
        check_colors(BOTTOM)?;

        Ok(())
    }
}
