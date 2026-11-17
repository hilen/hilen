//! The speed of the link to an app, `hilen-inspect speed`: the time of a
//! round trip, then raw bytes to the app and from the app, the way iperf
//! measures. See "Speed test" in `docs/inspect.md` of hilen.

use std::time::Duration;

use anyhow::{Context, Result};
use hilen::inspect::protocol::{Client, InspectorCommand, SpeedRun, megabytes};
use tokio::time::Instant;

use super::send;

/// So many small questions give the time of a round trip.
const PINGS: u32 = 10;
const OLD_APP: &str =
    "The speed test did not run. An app built before 2026-10-13 has none and closes the connection";

pub(super) async fn speed(client: &Client, seconds: u32) -> Result<()> {
    let (least, average) = round_trip(client).await?;
    println!("round trip  least {least:.1} ms, average {average:.1} ms");

    let time = Duration::from_secs(seconds.into());
    println!("to the app");
    let mut meter = Meter::new();
    let up = client.speed_to_app(time, |total| meter.seen(total)).await.context(OLD_APP)?;
    println!("from the app");
    let mut meter = Meter::new();
    let down = client.speed_from_app(time, |total| meter.seen(total)).await.context(OLD_APP)?;

    println!("to the app    {}", summary(&up));
    println!("from the app  {}", summary(&down));
    Ok(())
}

fn summary(run: &SpeedRun) -> String {
    format!(
        "{:.1} MB/s, {:.0} Mbit/s, {:.0} MB in {:.1} s",
        run.mb_per_second(),
        run.mb_per_second() * 8.0,
        megabytes(run.bytes),
        run.time.as_secs_f64()
    )
}

/// The least and the average time of a small question and its answer, in
/// milliseconds.
async fn round_trip(client: &Client) -> Result<(f64, f64)> {
    let mut times = vec![];
    for _ in 0..PINGS {
        let started = Instant::now();
        send(client, InspectorCommand::GetStartTime).await?;
        times.push(started.elapsed().as_secs_f64() * 1000.0);
    }
    let least = times.iter().copied().fold(f64::MAX, f64::min);
    Ok((least, times.iter().sum::<f64>() / f64::from(PINGS)))
}

/// Prints the speed of each second of a run that is under way.
struct Meter {
    started: Instant,
    second:  u64,
    /// The bytes at the end of that second.
    bytes:   u64,
}

impl Meter {
    fn new() -> Self {
        Self {
            started: Instant::now(),
            second:  0,
            bytes:   0,
        }
    }

    fn seen(&mut self, total: u64) {
        let second = self.started.elapsed().as_secs();
        if second == self.second {
            return;
        }
        let run = SpeedRun {
            bytes: total - self.bytes,
            time:  Duration::from_secs(second - self.second),
        };
        println!("  {second:>2} s  {:6.1} MB/s", run.mb_per_second());
        self.second = second;
        self.bytes = total;
    }
}
