use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::color::Color,
    refs::Weak,
    ui::{
        Container, Hover, Label, Setup, TextAlignment, Tooltip, ViewData, ViewFrame, ViewTest, ViewTooltip,
        view,
    },
    ui_test::{check_colors, inject_touches, wait_for_tooltip},
};

const PAGE: Color = Color::hex("#F4F5F7");
const INK: Color = Color::hex("#20232A");
const POINTER: Color = Color::hex("#E11D48");

/// A point off every box, where the pointer rests between the steps.
const AWAY: &str = "300 200 m";

const ROOM_BELOW: &str = r"
             508    4 - #b8e0c0
             236   16 - #20232a
              40   20 - #f4f5f7
              72   20 - #f4f5f7
             112   20 - #63666b
             164   20 - #43464c
             200   20 - #20232a
             284   20 - #f4f5f7
             192   64 - #cfd8e3
             104   76 - #e11d48
             128   76 - #cfd8e3
              16   96 - #cfd8e3
             144  100 - #333338
             224  108 - #333338
             124  116 - #333338
             168  116 - #333338
             420  140 - #b8e0c0
             592  160 - #f4f5f7
             448  300 - #20232a
             468  300 - #61746a
             480  300 - #b8e0c0
             180  380 - #f0d060
              16  420 - #f0d060
             420  464 - #b8e0c0
             148  532 - #f0d060
             576  576 - #e8c0d0
             540  580 - #e8c0d0
             560  588 - #e8c0d0
               4  592 - #f4f5f7
             296  592 - #f4f5f7
             568  592 - #e8c0d0
             588  592 - #e8c0d0
";

const CORNER: &str = r"
             508    4 - #b8e0c0
              48   20 - #595b61
              88   20 - #64676c
             116   20 - #f4f5f7
             136   20 - #f4f5f7
             176   20 - #f4f5f7
             200   20 - #393c42
             212   20 - #20232a
             108   76 - #cfd8e3
             128   76 - #cfd8e3
              16   96 - #cfd8e3
             192   96 - #cfd8e3
             420  136 - #b8e0c0
             448  296 - #20232a
             436  300 - #21242b
             480  300 - #b8e0c0
             160  356 - #f0d060
              16  428 - #f0d060
             552  544 - #333338
             536  552 - #333338
             564  556 - #aaaaac
             568  556 - #afafb1
             580  556 - #333338
             552  560 - #333338
             592  576 - #d3afbd
             540  580 - #e4bdcd
             556  588 - #e8c0d0
             568  588 - #e11d48
             572  588 - #e11d48
               4  592 - #f4f5f7
             276  592 - #f4f5f7
             588  592 - #e8c0d0
";

const TALL: &str = r"
             592    4 - #f4f5f7
             120   20 - #f4f5f7
             156   20 - #6d7075
             216   20 - #f4f5f7
             244   20 - #f4f5f7
             276   20 - #20232a
              16   52 - #cfd8e3
             108   76 - #cfd8e3
             128   76 - #cfd8e3
             420  108 - #b8e0c0
             592  172 - #f4f5f7
             436  300 - #21242b
             468  300 - #61746a
             480  300 - #b8e0c0
             172  308 - #333338
             112  316 - #dedfe1
             180  320 - #b9b9bb
             188  320 - #bebec0
             232  324 - #333338
             144  328 - #333338
              16  376 - #f0d060
             456  452 - #b8e0c0
             192  464 - #f0d060
             324  528 - #f4f5f7
             576  576 - #e8c0d0
             104  580 - #e11d48
             108  580 - #e11d48
             540  580 - #e8c0d0
             560  588 - #e8c0d0
             424  592 - #b8e0c0
             568  592 - #e8c0d0
             588  592 - #e8c0d0
";

const FULL: &str = r"
             468   12 - #b2b2b4
             552   12 - #b9b9bb
             164   16 - #20232a
              76   20 - #f4f5f7
             112   20 - #f4f5f7
             136   20 - #f4f5f7
             248   20 - #64676c
             460   20 - #333338
             592   20 - #333338
             332   24 - #303339
             484   28 - #93b299
             508   28 - #93b299
             108   76 - #cfd8e3
             128   76 - #cfd8e3
              16   96 - #cfd8e3
             192   96 - #cfd8e3
             420  124 - #b8e0c0
             508  192 - #b8e0c0
             436  300 - #21242b
             468  300 - #61746a
             480  300 - #b8e0c0
             184  340 - #f0d060
              16  352 - #f0d060
             592  424 - #f4f5f7
             420  444 - #b8e0c0
             140  476 - #f0d060
               4  524 - #f4f5f7
             464  580 - #e11d48
             556  588 - #e8c0d0
             228  592 - #f4f5f7
             568  592 - #e8c0d0
             588  592 - #e8c0d0
";

/// A tooltip with no room below the pointer goes 8 points above the top
/// edge of its view, so it never covers the view the pointer rests on, like
/// a small button in the bottom corner of a window. With room below it stays
/// under the pointer. A view as tall as the screen leaves no room above
/// either, there the tooltip stays inside the screen at the top.
#[view]
struct TooltipAboveView {
    #[init]
    page:    Container,
    status:  Label,
    top:     Label,
    tall:    Label,
    full:    Label,
    corner:  Label,
    pointer: Container,
}

impl Setup for TooltipAboveView {
    fn setup(self: Weak<Self>) {
        self.page.set_color(PAGE);
        self.page.place().back();

        self.status
            .set_text("pointer away")
            .set_text_size(14)
            .set_alignment(TextAlignment::Left)
            .set_text_color(INK);
        self.status.place().t(8).l(16).size(380, 22);

        for (label, text, color, tooltip) in [
            (self.top, "room below", "#cfd8e3", "Under the pointer"),
            (self.tall, "tall view", "#f0d060", "Above the tall view"),
            (self.full, "full height", "#b8e0c0", "No room above either"),
            (self.corner, "corner", "#e8c0d0", "Settings"),
        ] {
            label
                .set_text(text)
                .set_text_size(14)
                .set_alignment(TextAlignment::Center)
                .set_text_color(INK);
            label.set_color(color);
            label.set_tooltip(tooltip);
        }
        self.top.set_frame((16, 50, 180, 50));
        self.tall.set_frame((16, 340, 180, 260));
        self.full.set_frame((420, 0, 90, 600));
        self.corner.set_frame((540, 576, 60, 24));

        self.pointer.set_color(POINTER).set_corner_radius(4);
        self.pointer.set_frame((0, 0, 8, 8)).set_hidden(true);
    }
}

impl TooltipAboveView {
    /// Moves the pointer to `x`, `y`, draws its marker, says what should be
    /// seen and waits for the tooltip.
    fn rest(view: Weak<Self>, x: f32, y: f32, status: &str) {
        let status = status.to_string();
        from_main(move || {
            view.pointer.set_frame((x - 4.0, y - 4.0, 8.0, 8.0));
            view.pointer.set_hidden(false);
            view.status.set_text(status);
        });
        inject_touches(format!("{x} {y} m"));
        wait_for_tooltip();
    }

    fn away(view: Weak<Self>) {
        from_main(move || {
            view.pointer.set_hidden(true);
            view.status.set_text("pointer away");
        });
        inject_touches(AWAY);
        wait_for_next_frame();
    }
}

impl ViewTest for TooltipAboveView {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        from_main(Hover::clear);
        Self::away(view);
        ensure!(
            from_main(|| Tooltip::shown().is_null()),
            "no tooltip with the pointer away"
        );

        Self::rest(view, 106.0, 75.0, "room below, the tooltip is under the pointer");
        let origin = from_main(|| Tooltip::shown().frame().origin);
        ensure!(origin == (118, 93).into(), "under the pointer, got {origin:?}");
        check_colors(ROOM_BELOW)?;

        Self::rest(view, 570.0, 588.0, "corner, the tooltip is above the box");
        let (shown, corner) = from_main(move || (*Tooltip::shown().frame(), *view.corner.frame()));
        ensure!(
            shown.origin == (600.0 - shown.width(), 544.0).into(),
            "8 points above the corner box, at the right edge, got {shown:?}"
        );
        ensure!(
            shown.max_y() <= corner.y(),
            "the tooltip covers the corner box, {shown:?}"
        );
        check_colors(CORNER)?;

        Self::rest(view, 106.0, 580.0, "tall view, the tooltip is above its top edge");
        let shown = from_main(|| *Tooltip::shown().frame());
        ensure!(
            shown.origin == (118, 308).into(),
            "8 points above the tall box, got {shown:?}"
        );
        check_colors(TALL)?;

        Self::rest(
            view,
            465.0,
            580.0,
            "full height, no room above, the tooltip is at the top",
        );
        let shown = from_main(|| *Tooltip::shown().frame());
        ensure!(
            shown.origin == (600.0 - shown.width(), 0.0).into(),
            "at the top right of the screen, got {shown:?}"
        );
        check_colors(FULL)?;

        Self::away(view);
        ensure!(
            from_main(|| Tooltip::shown().is_null()),
            "the tooltip hides with the pointer away"
        );

        Ok(())
    }
}
