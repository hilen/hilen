use anyhow::Result;
use test_engine::{
    refs::Weak,
    ui::{
        Anchor::{Left, Top, X},
        BLACK, Button, Container, GREEN, PURPLE, RED, Setup, TURQUOISE, ViewData, ViewSubviews, ViewTest,
        WHITE, view,
    },
    ui_test::check_colors,
};

#[view]
struct Gradient {
    button: Weak<Button>,

    #[init]
    grad_1: Container,
    grad_2: Container,
    grad_3: Container,

    button_container: Container,
}

impl Setup for Gradient {
    fn setup(mut self: Weak<Self>) {
        self.grad_1.set_gradient(RED, GREEN).place().tl(20).size(100, 100);

        self.grad_2.set_gradient(TURQUOISE, PURPLE).set_corner_radius(28);
        self.grad_2.place().t(20).size(100, 200).anchor(Left, self.grad_1, 20);

        self.grad_3.set_gradient(WHITE, BLACK).set_corner_radius(20);
        self.grad_3.place().t(20).size(200, 100).anchor(Left, self.grad_2, 20);

        self.button_container
            .place()
            .same([X], self.grad_1)
            .anchor(Top, self.grad_2, 40)
            .size(280, 100);

        self.button = self.button_container.add_view();

        self.button.place().back();
        self.button.set_text("Button").set_gradient(WHITE, RED).set_corner_radius(40);
    }
}

impl ViewTest for Gradient {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        check_colors(
            r"
             592    4 - #597c95
              24   20 - #fe0100
             176   20 - #01feff
             320   20 - #fefefe
              88   32 - #df2000
             456   36 - #d5d5d5
             388   44 - #c1c1c1
             232   48 - #24dbff
             284   56 - #a2a2a2
              48   60 - #986700
             424   60 - #989898
             140   72 - #43bcff
             340   76 - #6f6f6f
             380   84 - #5b5b5b
             428   96 - #3c3c3c
              88  100 - #32cd00
             260  100 - #333435
             292  116 - #090909
             172  120 - #807fff
             224  124 - #857aff
             236  172 - #c23dff
             144  200 - #e619ff
             240  276 - #ffd5d5
             296  288 - #ffb6b6
             120  296 - #000000
              68  304 - #ff8e8e
             160  308 - #ff8383
             164  312 - #663030
             204  312 - #d66666
             200  316 - #ff6f6f
             256  324 - #ff5b5b
              24  332 - #ff4646
        ",
        )?;

        Ok(())
    }
}
