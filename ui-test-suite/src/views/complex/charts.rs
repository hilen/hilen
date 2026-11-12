use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{BarChart, Color, HeatGrid, LineChart, Setup, ViewData, ViewFrame, ViewTest, view},
    ui_test::{check_colors, checkpoint},
};

/// A line chart of 20,000 readings with a target band and an alert marker,
/// bars of water use and a heat grid of hours per zone and day. The 2
/// charts that follow their rows get no height from the test.
#[view]
struct Charts {
    #[init]
    line: LineChart,
    bars: BarChart,
    heat: HeatGrid,
}

impl Setup for Charts {
    fn setup(self: Weak<Self>) {
        self.set_color(Color::hex("#FFFFFF"));

        self.line.place().t(10).lr(10).h(220);
        let readings = (0..20_000).map(|minute| {
            let t = f64::from(minute);
            (
                t,
                21.0 + 3.0 * (t / 1440.0 * std::f64::consts::TAU).sin() + (t / 37.0).sin() * 0.4,
            )
        });
        self.line
            .add_band(19.0, 24.0, Color::hex("#E3F6E8"))
            .add_line(readings, Color::hex("#0A84FF"))
            .add_marker(9_000.0, Color::hex("#FF3B30"))
            .set_x_format(|minute| format!("day {}", (minute / 1440.0).floor() + 1.0));

        self.bars.place().t(250).lr(10);
        self.bars.set_bars([
            ("Zone 1", 420.0, "420 l"),
            ("Zone 2", 180.0, "180 l"),
            ("Zone 3", 610.0, "610 l"),
            ("Zone 4", 0.0, "0 l"),
        ]);

        self.heat.place().t(400).lr(10);
        self.heat.set_cells(
            ["Zone 1", "Zone 2", "Zone 3"],
            ["Mon", "Tue", "Wed", "Thu"],
            vec![
                vec![Some(0.0), Some(1.5), Some(4.0), None],
                vec![Some(2.0), Some(6.0), Some(0.5), Some(3.0)],
                vec![None, Some(0.0), Some(8.0), Some(1.0)],
            ],
            |hours| format!("{hours:.1} h"),
        );
    }
}

impl ViewTest for Charts {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        for _ in 0..4 {
            wait_for_next_frame();
        }
        from_main(move || {
            let ((x0, x1), (y0, y1)) = view.line.ranges();
            ensure!(
                x0.abs() < 0.01 && (x1 - 19_999.0).abs() < 0.01,
                "x follows the data {x0} {x1}"
            );
            ensure!(
                y0 < 18.0 && y1 > 24.0,
                "y holds the readings and the band {y0} {y1}"
            );
            ensure!(
                near(view.bars.height(), 4.0 * 28.0 + 3.0 * 8.0),
                "bars height {}",
                view.bars.height()
            );
            ensure!(
                view.heat.height() > 100.0,
                "heat grid height {}",
                view.heat.height()
            );
            Ok(())
        })?;
        check_colors(DRAWN)?;
        checkpoint("readings over a green target band with a red alert line, bars, heat grid")?;
        Ok(())
    }
}

const DRAWN: &str = r"
             508    4 - #ffffff
             292   20 - #ff3b30
             100   48 - #0a84ff
             592  112 - #ffffff
              48  136 - #ffffff
             464  164 - #0a84ff
             272  188 - #0a84ff
             200  216 - #7f7f83
              68  220 - #e0e0e1
             436  220 - #e0e0e1
             564  296 - #929292
              48  304 - #4a4a4c
             268  328 - #0a84ff
             580  372 - #ffffff
             184  408 - #bababa
             432  408 - #474748
             528  412 - #29292b
             368  428 - #79a0df
              12  436 - #9e9e9f
             304  436 - #bed3f3
             416  436 - #79a0df
             476  452 - #88abe3
             256  464 - #4278cf
             196  476 - #b0c9ef
             308  476 - #4278cf
             184  480 - #a1b8db
             532  480 - #95b4e7
             404  488 - #dae7fb
              24  512 - #c2c2c2
             320  512 - #212123
             588  520 - #ccddf7
             472  524 - #0a4fbf
            ";

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.01
}
