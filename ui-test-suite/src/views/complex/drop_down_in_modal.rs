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
    fn setup(self: Weak<Self>) {
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
        self.page.set_alignment(TextAlignment::Center);
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
                592    4 - #999999
                228    8 - #999999
                380    8 - #999999
                448   32 - #999999
                524   40 - #999999
                156   44 - #999999
                 72   64 - #999999
                592   68 - #999999
                364   80 - #999999
                  4   88 - #999999
                252   96 - #999999
                472  104 - #999999
                312  112 - #999999
                412  116 - #999999
                548  116 - #999999
                136  132 - #999999
                200  136 - #999999
                 48  148 - #999999
                592  164 - #999999
                208  172 - #ffffff
                260  172 - #ffffff
                308  172 - #ffffff
                352  172 - #ffffff
                388  172 - #ffffff
                444  176 - #ffffff
                152  180 - #ffffff
                 96  184 - #999999
                520  196 - #999999
                192  200 - #000000
                224  200 - #ffffff
                232  200 - #000000
                284  200 - #050505
                336  200 - #000000
                412  200 - #ffffff
                192  204 - #000000
                224  204 - #ffffff
                232  204 - #000000
                332  204 - #151515
                336  204 - #000000
                  4  208 - #999999
                232  208 - #090909
                372  212 - #ffffff
                152  224 - #ffffff
                448  228 - #ffffff
                592  228 - #999999
                296  232 - #ffffff
                 72  236 - #999999
                400  236 - #ffffff
                264  240 - #ffffff
                176  248 - #ffffff
                208  248 - #ffffff
                208  252 - #ffffff
                220  252 - #535760
                324  252 - #ffffff
                364  256 - #ffffff
                508  260 - #999999
                416  268 - #ffffff
                152  272 - #ffffff
                284  280 - #ffffff
                240  288 - #ffffff
                316  288 - #ffffff
                448  288 - #ffffff
                  4  292 - #999999
                192  296 - #ffffff
                352  296 - #ffffff
                 92  300 - #999999
                400  300 - #ffffff
                164  304 - #ffffff
                544  312 - #999999
                268  320 - #ffffff
                312  324 - #ffffff
                420  324 - #ffffff
                228  328 - #ffffff
                372  328 - #ffffff
                152  332 - #ffffff
                340  332 - #ffffff
                448  336 - #ffffff
                192  344 - #ffffff
                260  348 - #ffffff
                404  352 - #ffffff
                220  356 - #ffffff
                504  356 - #999999
                 64  360 - #999999
                348  360 - #ffffff
                292  368 - #ffffff
                  4  376 - #999999
                248  376 - #ffffff
                156  380 - #ffffff
                204  384 - #ffffff
                412  384 - #ffffff
                448  384 - #ffffff
                324  392 - #ffffff
                272  396 - #ffffff
                380  396 - #ffffff
                592  396 - #999999
                448  416 - #ffffff
                516  416 - #999999
                292  420 - #ffffff
                204  424 - #ffffff
                164  428 - #ffffff
                244  428 - #ffffff
                340  428 - #ffffff
                376  428 - #ffffff
                416  428 - #ffffff
                 16  440 - #999999
                 92  448 - #999999
                576  456 - #999999
                296  492 - #999999
                524  492 - #999999
                156  500 - #999999
                448  500 - #999999
                  4  504 - #999999
                592  516 - #999999
                 68  520 - #999999
                228  520 - #999999
                360  524 - #999999
                516  568 - #999999
                148  572 - #999999
                440  576 - #999999
                  4  592 - #999999
                 76  592 - #999999
                220  592 - #999999
                292  592 - #999999
                372  592 - #999999
                592  592 - #999999
";

const COLORS_2: &str = r"
                  4    4 - #999999
                308    4 - #999999
                592    4 - #999999
                448   32 - #999999
                156   44 - #999999
                364   80 - #999999
                252   96 - #999999
                496  104 - #999999
                 44  120 - #999999
                576  140 - #999999
                260  172 - #ffffff
                308  172 - #ffffff
                388  172 - #ffffff
                444  176 - #ffffff
                152  180 - #ffffff
                192  200 - #000000
                224  200 - #ffffff
                232  200 - #000000
                284  200 - #050505
                336  200 - #000000
                192  204 - #000000
                224  204 - #ffffff
                232  204 - #000000
                332  204 - #151515
                336  204 - #000000
                232  208 - #090909
                432  212 - #ffffff
                532  216 - #999999
                260  228 - #ffffff
                396  228 - #ffffff
                  4  240 - #999999
                152  240 - #ffffff
                312  240 - #ebfcff
                208  248 - #ebfcff
                448  248 - #ffffff
                208  252 - #ebfcff
                220  252 - #4e5660
                276  260 - #ebfcff
                352  260 - #ebfcff
                152  276 - #ffffff
                192  276 - #dddddd
                592  280 - #999999
                188  284 - #cecece
                412  288 - #d3d3d3
                412  292 - #d3d3d3
                184  296 - #e8e8e8
                208  296 - #ebfcff
                412  296 - #d3d3d3
                208  300 - #ebfcff
                220  300 - #3be3ff
                316  300 - #ebfcff
                412  300 - #d3d3d3
                448  300 - #ffffff
                184  304 - #e8e8e8
                264  304 - #ebfcff
                364  304 - #ebfcff
                412  304 - #d3d3d3
                412  308 - #d3d3d3
                184  312 - #e8e8e8
                412  312 - #d3d3d3
                 76  316 - #999999
                412  316 - #d3d3d3
                412  320 - #d3d3d3
                412  324 - #d3d3d3
                184  328 - #e8e8e8
                204  328 - #363b46
                412  328 - #d3d3d3
                412  332 - #d3d3d3
                228  336 - #ffffff
                412  336 - #d3d3d3
                448  336 - #ffffff
                412  340 - #d3d3d3
                184  344 - #e8e8e8
                284  344 - #ffffff
                412  344 - #d3d3d3
                352  348 - #ffffff
                412  348 - #d3d3d3
                412  352 - #d3d3d3
                412  356 - #d3d3d3
                412  360 - #d3d3d3
                536  360 - #999999
                184  364 - #e8e8e8
                204  364 - #363b46
                252  364 - #ffffff
                316  364 - #ffffff
                412  364 - #d3d3d3
                412  368 - #d3d3d3
                412  372 - #d3d3d3
                448  372 - #ffffff
                184  376 - #e8e8e8
                412  380 - #d3d3d3
                184  388 - #e9e9e9
                412  388 - #d4d4d4
                  4  396 - #999999
                404  396 - #cecece
                200  400 - #e8e8e8
                220  400 - #e8e8e8
                240  400 - #e8e8e8
                252  400 - #e8e8e8
                260  400 - #e8e8e8
                272  400 - #e8e8e8
                280  400 - #e8e8e8
                288  400 - #e8e8e8
                296  400 - #e8e8e8
                304  400 - #e8e8e8
                312  400 - #e8e8e8
                320  400 - #e8e8e8
                328  400 - #e8e8e8
                340  400 - #e8e8e8
                348  400 - #e8e8e8
                356  400 - #e8e8e8
                368  400 - #e8e8e8
                384  400 - #e8e8e8
                396  400 - #e8e8e8
                448  408 - #ffffff
                156  424 - #ffffff
                420  428 - #ffffff
                592  444 - #999999
                  4  492 - #999999
                376  496 - #999999
                188  504 - #999999
                272  516 - #999999
                 92  520 - #999999
                472  536 - #999999
                  4  592 - #999999
                184  592 - #999999
                356  592 - #999999
                592  592 - #999999
";
