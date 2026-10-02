use anyhow::Result;
use hilen::{
    OnceEvent,
    dispatch::from_main,
    refs::Weak,
    ui::{
        BLACK, Color, Label, ModalView, Setup, Size, TextAlignment, TextDropDown, UIColor, ViewData,
        ViewTest, WHITE, view,
    },
    ui_test::{check_colors, inject_touches, set_record_probe_count},
};

/// A dialog with a dark scrim behind it and a drop down in it, the shape
/// of the add wallet form of the wallet app.
#[view]
struct DropDownModal {
    event: OnceEvent,

    #[init]
    title: Label,
    drop:  TextDropDown,
}

impl Setup for DropDownModal {
    fn setup(mut self: Weak<Self>) {
        self.set_color(WHITE);
        self.set_corner_radius(16);

        self.title.set_text("the open list is white").set_text_size(18);
        self.title.set_alignment(TextAlignment::Left);
        self.title.place().t(16).l(24).r(24).h(30);

        self.drop.set_values(vec!["One", "Two", "Three"]);
        self.drop.set_text_color(Color::rgb(0.10, 0.12, 0.17)).set_text_size(16);
        self.drop
            .set_color(WHITE)
            .set_corner_radius(8)
            .set_border_width(1)
            .set_border_color(BLACK.with_alpha(0.12));
        self.drop.place().t(60).l(40).size(220, 40);
    }
}

impl ModalView for DropDownModal {
    fn modal_event(&self) -> &OnceEvent<()> {
        &self.event
    }

    fn modal_size() -> Size {
        (300, 260).into()
    }

    fn modal_scrim_color() -> UIColor {
        BLACK.with_alpha(0.4).into()
    }
}

/// The scrim of a modal dims the page behind it and nothing of the modal.
/// The open list of a drop down inside the modal belongs to the modal, it
/// must keep the color of its box and not come out dimmed.
#[view]
struct DropDownInModal {
    #[init]
    page: Label,
}

impl Setup for DropDownInModal {
    fn setup(self: Weak<Self>) {
        self.page.set_text("the page behind is dimmed").set_text_size(20);
        self.page.set_color(WHITE);
        self.page.place().back();
    }
}

impl ViewTest for DropDownInModal {
    fn perform_test(_: Weak<Self>) -> Result<()> {
        set_record_probe_count(128);

        let modal = from_main(DropDownModal::prepare_modally);
        check_colors(COLORS_1)?;

        // A real pointer rests on the box before it clicks. The hover
        // makes the box remember its half transparent border, and the
        // list copies that border.
        inject_touches("300 250 m");
        inject_touches("300 250 b\n300 250 e");
        check_colors(COLORS_2)?;

        modal.hide_modal(());

        Ok(())
    }
}

const COLORS_1: &str = r"
               4    4 - #999999
             100    4 - #999999
             308    4 - #999999
             384    4 - #999999
             592    4 - #999999
             228    8 - #999999
             452   40 - #999999
             156   44 - #999999
             528   48 - #999999
              72   64 - #999999
             592   68 - #999999
             364   80 - #999999
               4   88 - #999999
             252   96 - #999999
             188  100 - #999999
             312  112 - #999999
             476  112 - #999999
             416  116 - #999999
             552  120 - #999999
             136  132 - #999999
              48  148 - #999999
             208  172 - #ffffff
             264  172 - #ffffff
             308  172 - #ffffff
             356  172 - #ffffff
             392  172 - #ffffff
             592  172 - #999999
             152  180 - #ffffff
             448  184 - #ffffff
             192  200 - #000000
             224  200 - #ffffff
             232  200 - #000000
             284  200 - #050505
             336  200 - #000000
             340  200 - #525252
             524  200 - #999999
               4  204 - #999999
             192  204 - #000000
             204  204 - #626262
             224  204 - #ffffff
             232  204 - #000000
             252  204 - #6f6f6f
             256  204 - #c8c8c8
             332  204 - #151515
             336  204 - #000000
             232  208 - #090909
             416  208 - #ffffff
             376  212 - #ffffff
             156  220 - #ffffff
             304  224 - #ffffff
              68  232 - #999999
             348  232 - #ffffff
             448  240 - #ffffff
             592  240 - #999999
             188  244 - #ffffff
             280  244 - #ffffff
             396  244 - #ffffff
             224  248 - #ffffff
             224  252 - #ffffff
             236  252 - #535760
             152  256 - #ffffff
             320  256 - #ffffff
             360  260 - #ffffff
             508  264 - #999999
             420  268 - #ffffff
             160  288 - #ffffff
             392  288 - #ffffff
               4  292 - #999999
             192  292 - #ffffff
             244  292 - #ffffff
             320  292 - #ffffff
             448  292 - #ffffff
             284  296 - #ffffff
              88  300 - #999999
             352  300 - #ffffff
             540  320 - #999999
             152  324 - #ffffff
             404  328 - #ffffff
             444  328 - #ffffff
             228  332 - #ffffff
             316  332 - #ffffff
             272  336 - #ffffff
             360  336 - #ffffff
             188  340 - #ffffff
             336  356 - #ffffff
             396  356 - #ffffff
             220  360 - #ffffff
              68  364 - #999999
             448  364 - #ffffff
             152  372 - #ffffff
             368  372 - #ffffff
               4  376 - #999999
             292  376 - #ffffff
             196  380 - #ffffff
             244  380 - #ffffff
             408  384 - #ffffff
             332  388 - #ffffff
             592  400 - #999999
             448  404 - #ffffff
             512  408 - #999999
             204  416 - #ffffff
             372  416 - #ffffff
             284  420 - #ffffff
             164  428 - #ffffff
             240  428 - #ffffff
             332  428 - #ffffff
             416  428 - #ffffff
              16  440 - #999999
              92  448 - #999999
             568  456 - #999999
             516  488 - #999999
             292  492 - #999999
             156  500 - #999999
             440  500 - #999999
               4  504 - #999999
             592  512 - #999999
              68  520 - #999999
             360  520 - #999999
             228  524 - #999999
             516  568 - #999999
             148  572 - #999999
             440  576 - #999999
               4  592 - #999999
              76  592 - #999999
             216  592 - #999999
             292  592 - #999999
             368  592 - #999999
             592  592 - #999999
";

const COLORS_2: &str = r"
               4    4 - #999999
             308    4 - #999999
             592    4 - #999999
             452   40 - #999999
             156   44 - #999999
             364   80 - #999999
             252   96 - #999999
             508  112 - #999999
              52  116 - #999999
             592  152 - #999999
             308  172 - #ffffff
             392  172 - #ffffff
             152  180 - #ffffff
             448  184 - #ffffff
             192  200 - #000000
             224  200 - #ffffff
             232  200 - #000000
             284  200 - #050505
             336  200 - #000000
             340  200 - #525252
             192  204 - #000000
             204  204 - #626262
             224  204 - #ffffff
             232  204 - #000000
             252  204 - #6f6f6f
             256  204 - #c8c8c8
             332  204 - #151515
             336  204 - #000000
             232  208 - #090909
             436  220 - #ffffff
               4  224 - #999999
             400  228 - #ffffff
             536  228 - #999999
             360  236 - #ffffff
             152  240 - #ffffff
             188  240 - #ffffff
             288  240 - #ffffff
             324  240 - #ffffff
             224  248 - #ffffff
             224  252 - #ffffff
             236  252 - #535760
             448  256 - #ffffff
             192  276 - #dddddd
             204  276 - #e0e0e0
             216  276 - #e0e0e0
             228  276 - #e0e0e0
             240  276 - #e0e0e0
             256  276 - #e0e0e0
             268  276 - #e0e0e0
             288  276 - #e0e0e0
             308  276 - #e0e0e0
             320  276 - #e0e0e0
             332  276 - #e0e0e0
             344  276 - #e0e0e0
             360  276 - #e0e0e0
             376  276 - #e0e0e0
             392  276 - #e0e0e0
             152  280 - #ffffff
             412  284 - #d5d5d5
             188  292 - #cbcbcb
             224  296 - #ebfcff
             412  296 - #d3d3d3
             188  300 - #cbcbcb
             224  300 - #ebfcff
             236  300 - #3be3ff
             592  304 - #999999
              56  308 - #999999
             188  312 - #cbcbcb
             328  312 - #ebfcff
             412  312 - #d3d3d3
             448  312 - #ffffff
             276  316 - #ffffff
             188  320 - #cbcbcb
             188  328 - #cbcbcb
             220  328 - #363b46
             412  328 - #d3d3d3
             360  332 - #ffffff
             244  336 - #ffffff
             308  340 - #ffffff
             152  344 - #ffffff
             412  344 - #d3d3d3
             188  348 - #cbcbcb
             272  360 - #ffffff
             336  360 - #ffffff
             412  360 - #d3d3d3
             188  364 - #cbcbcb
             220  364 - #363b46
             228  368 - #a4a6ab
             228  372 - #a4a6ab
             412  376 - #d3d3d3
             448  376 - #ffffff
             540  376 - #999999
             188  384 - #cbcbcb
             152  388 - #ffffff
               4  392 - #999999
             412  392 - #dddddd
             200  396 - #cbcbcb
             232  396 - #cbcbcb
             240  396 - #cbcbcb
             256  396 - #cbcbcb
             264  396 - #cbcbcb
             272  396 - #cbcbcb
             280  396 - #cbcbcb
             288  396 - #cbcbcb
             296  396 - #cbcbcb
             308  396 - #cbcbcb
             320  396 - #cbcbcb
             328  396 - #cbcbcb
             336  396 - #cbcbcb
             348  396 - #cbcbcb
             360  396 - #cbcbcb
             384  396 - #cbcbcb
             400  396 - #cbcbcb
             216  400 - #e8e8e8
             248  400 - #e8e8e8
             372  400 - #e8e8e8
             448  416 - #ffffff
             156  424 - #ffffff
             592  448 - #999999
             508  488 - #999999
              60  492 - #999999
             360  504 - #999999
             240  508 - #999999
             448  560 - #999999
             156  580 - #999999
               4  592 - #999999
             308  592 - #999999
             592  592 - #999999
";
