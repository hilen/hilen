# UI benchmark

Measures how many views the engine can render before a frame costs more than 16 ms (the
60 fps budget). Deterministic: no randomness, no wall-clock dependence — every run builds
the identical scene in the identical order.

## Run

The benchmark is behind the `bench` cargo feature and is compiled out of normal
builds (so a shipped game carries none of it, not even the `sysinfo` guard).
`make bench` turns the feature on for you; a manual run must pass it.

```bash
make bench                                                       # full suite, results saved to bench/
BENCH_RUNS=3 make bench                                          # fewer runs per mode
UI_BENCHMARK=1 cargo run -p demo --release --features bench # single run, prints and exits
HILEN_HEADLESS=1                                                    # add to run without a window
-- --no-guard                                                    # run even on a busy machine (see Consistency)
```

Also launchable from the demo menu ("ui bench"), in a build made with
`--features bench` — same run, shows an Alert and keeps the app alive.

## How it works

Every rendered frame adds one `BenchPanel` (~40 views: labels, button, image, switch,
checkbox, slider, progress, circle, number view, drawing, scroll view) placed with every
layout rule family (sides, anchors, same, relative, between, tiling, distribute, custom).
The metric is `Window::frame_work_time` — CPU time of update + render encoding, excluding
the wait for a drawable — so results are not capped by vsync or the compositor. Frames the
window did not actually render (occluded) are skipped. The run stops when the rolling
average over 10 frames crosses 16 ms and reports panels, views and ms.

Each step also records `Window::frame_gpu_time` — the render pass GPU execution time from
timestamp queries (`TIMESTAMP_QUERY`), resolved with a blocking poll after the work timer
closes so it never inflates the CPU number. It is advisory: the stop condition stays on CPU
work, and `gpu_ms` is reported next to `cpu_ms` so you can see whether the engine is CPU- or
GPU-bound at the limit. Unlike the CPU metric it is not deterministic — it carries GPU clock
and thermal noise the guard cannot catch, so do not treat a single `gpu_ms` as evidence.

## Consistency

A run refuses to start (exit code 75) when the system is busy or hot. Thresholds in
`benchmark_view.rs`: cpu usage 15%, load average 0.6 per core, cpu temperature 55 C.
A running browser is rejected outright regardless of load — browsers do bursty
background work even when idle. The suite fails immediately on rejection and names
the heavy processes — close them and rerun.

Pass `--no-guard` (a binary argument: `cargo run ... --features bench -- --no-guard`) to
run anyway. The load is still measured and recorded in the JSON, and the run prints that
its numbers are not evidence. The `bench` harness never passes it — committed history is
always guarded.

The strictness is deliberate. Real optimizations move the result by 3-10%, and a single
background process or a throttling chip shifts it by more than that — numbers collected
on a loaded machine are worthless as evidence and worse than no numbers. Known saboteurs
caught by the guard: Docker's VM, a browser, Spotlight indexing fresh build artifacts,
the chip still hot from a cargo build.

## Acceptance criteria

A performance change is accepted only with an A/B proof:

- both sides (with and without the change) measured in one session, on an idle machine,
  5+ runs per mode
- every run with the change beats every run without it — the ranges must not overlap
- the delta is reported per mode (debug, release, headless)

A median improvement with overlapping ranges, a single-run comparison, or numbers from
different sessions prove nothing. No A/B — no merge.

## History

`make bench` (the `bench` crate) runs all modes — debug, release, headless release —
`BENCH_RUNS` times each (default 5) and writes `bench/<date>-<commit>.json`: every run's
result (panels, views, cpu `ms`, `gpu_ms`) with the cpu usage, load and temperature it
started under, plus a median per mode.
Runs more than 15% below their mode's median are marked `"suspect": true` and excluded
from the median. The JSON files are committed — that is the engine's performance history.
Always benchmark a clean committed state, the file is named after the commit.
