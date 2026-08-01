use anyhow::Result;
use test_engine::{
    dispatch::from_main,
    refs::Weak,
    ui::{BLUE, Container, GREEN, Hover, RED, Setup, ViewData, ViewTest, ViewTouch, YELLOW, view},
    ui_test::{check_colors, inject_touches},
};

#[view]
struct HoverTest {
    log: Vec<(&'static str, bool)>,

    #[init]
    first:  Container,
    second: Container,
    top:    Container,
    ghost:  Container,
}

impl Setup for HoverTest {
    fn setup(self: Weak<Self>) {
        self.first.set_color(BLUE);
        self.first.place().tl(20).size(100, 100);
        self.first.enable_hover();
        self.first.touch().hovered.val(self, move |hovered| {
            let mut this = self;
            this.log.push(("first", hovered));
            this.first.set_color(if hovered { GREEN } else { BLUE });
        });

        self.second.set_color(RED);
        self.second.place().t(20).l(140).size(100, 100);
        self.second.enable_hover();
        self.second.touch().hovered.val(self, move |hovered| {
            let mut this = self;
            this.log.push(("second", hovered));
        });

        // Overlaps first. Registered later, so it wins the overlap.
        self.top.set_color(YELLOW);
        self.top.place().tl(40).size(60, 60);
        self.top.enable_hover();
        self.top.touch().hovered.val(self, move |hovered| {
            let mut this = self;
            this.log.push(("top", hovered));
        });

        self.ghost.place().t(300).l(20).size(100, 100);
        self.ghost.set_hidden(true);
        self.ghost.enable_hover();
        self.ghost.touch().hovered.val(self, move |hovered| {
            let mut this = self;
            this.log.push(("ghost", hovered));
        });
    }
}

impl ViewTest for HoverTest {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        // Cursor position and hover state can leak from previous tests.
        from_main(Hover::clear);
        from_main(move || {
            let mut this = view;
            this.log.clear();
        });

        first_hovers(view)?;
        top_wins_overlap(view)?;

        inject_touches("170 70 m");

        from_main(move || {
            assert_eq!(
                view.log,
                vec![
                    ("first", true),
                    ("first", false),
                    ("top", true),
                    ("top", false),
                    ("second", true)
                ]
            );
            assert!(view.second.is_hovered());
        });

        // Empty space. The hovered view gets an exit.
        inject_touches("350 70 m");

        from_main(move || {
            assert_eq!(*view.log.last().unwrap(), ("second", false));
            assert!(!view.second.is_hovered());
        });

        // A hidden view never hovers.
        let len_before_ghost = from_main(move || view.log.len());
        inject_touches("70 330 m");

        from_main(move || {
            assert_eq!(view.log.len(), len_before_ghost);
            assert!(!view.ghost.is_hovered());
        });

        // Cursor leaving the window clears hover.
        inject_touches("30 30 m");
        from_main(Hover::clear);

        from_main(move || {
            assert_eq!(*view.log.last().unwrap(), ("first", false));
            assert!(!view.first.is_hovered());
        });

        Ok(())
    }
}

fn first_hovers(view: Weak<HoverTest>) -> Result<()> {
    inject_touches("30 30 m");

    from_main(move || {
        assert_eq!(view.log, vec![("first", true)]);
        assert!(view.first.is_hovered());
        assert!(!view.second.is_hovered());
    });

    check_colors(
        r"
             592    4 - #597c95
              24   24 - #00ff00
             100   24 - #00ff00
             144   24 - #ff0000
             236   24 - #ff0000
             192   36 - #ff0000
              64   44 - #ffff00
              88   44 - #ffff00
              24   52 - #00ff00
             116   64 - #00ff00
              88   68 - #ffff00
             224   68 - #ff0000
              44   72 - #ffff00
              68   72 - #ffff00
             180   80 - #ff0000
              72   92 - #ffff00
              44   96 - #ffff00
              96   96 - #ffff00
             200  112 - #ff0000
              24  116 - #00ff00
              56  116 - #00ff00
              80  116 - #00ff00
             116  116 - #00ff00
             160  116 - #ff0000
             236  116 - #ff0000
             444  152 - #597c95
             300  300 - #597c95
             592  300 - #597c95
              64  356 - #597c95
               4  592 - #597c95
             300  592 - #597c95
             592  592 - #597c95
            ",
    )
}

fn top_wins_overlap(view: Weak<HoverTest>) -> Result<()> {
    // Both first and top are under the cursor. Topmost wins,
    // exit fires before enter.
    inject_touches("70 70 m");

    from_main(move || {
        assert_eq!(view.log, vec![("first", true), ("first", false), ("top", true)]);
        assert!(!view.first.is_hovered());
        assert!(view.top.is_hovered());
    });

    check_colors(
        r"
             592    4 - #597c95
              24   24 - #0000e7
             100   24 - #0000e7
             144   24 - #ff0000
             236   24 - #ff0000
             192   36 - #ff0000
              64   44 - #ffff00
              88   44 - #ffff00
              24   52 - #0000e7
             116   64 - #0000e7
              88   68 - #ffff00
             224   68 - #ff0000
              44   72 - #ffff00
              68   72 - #ffff00
             180   80 - #ff0000
              72   92 - #ffff00
              44   96 - #ffff00
              96   96 - #ffff00
             200  112 - #ff0000
              24  116 - #0000e7
              56  116 - #0000e7
              80  116 - #0000e7
             116  116 - #0000e7
             160  116 - #ff0000
             236  116 - #ff0000
             444  152 - #597c95
             300  300 - #597c95
             592  300 - #597c95
              64  356 - #597c95
               4  592 - #597c95
             300  592 - #597c95
             592  592 - #597c95
            ",
    )
}
