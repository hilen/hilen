//! UI benchmark suite. Runs the benchmark RUNS times in each mode and saves
//! the results to bench/<date>-<commit>.json for history tracking.
//!
//! The benchmark app rejects a run with exit code 75 when the system is busy
//! or the CPU is hot (throttling skews results) - the suite fails immediately.
//! Runs that come out far below the mode's median are marked suspect and
//! excluded from it.

use std::{env, fs, path::Path, process::Command};

use anyhow::{Result, bail};
use chrono::Local;
use serde_json::{Value, json};

/// Must match `BENCH_REJECTED_EXIT_CODE` in `benchmark_view.rs`.
const REJECTED: i32 = 75;

/// Contention only makes results worse, so a run far below the median was
/// fought over the CPU by something else.
const SUSPECT_BELOW_MEDIAN_PERCENT: i64 = 85;

/// Mode name, binary to run and extra environment variables.
type Mode = (
    &'static str,
    &'static str,
    &'static [(&'static str, &'static str)],
);

const MODES: &[Mode] = &[
    ("debug", "target/debug/demo", &[]),
    ("release", "target/release/demo", &[]),
    ("headless", "target/release/demo", &[("HILEN_HEADLESS", "1")]),
];

fn main() -> Result<()> {
    let runs_per_mode: usize = env::var("BENCH_RUNS").unwrap_or_else(|_| "5".to_string()).parse()?;

    let commit = git_commit()?;

    println!("building debug and release");
    build(&[])?;
    build(&["--release"])?;

    let result_path = env::temp_dir().join("ui_benchmark_run.json");
    let mut modes = serde_json::Map::new();

    for (name, binary, envs) in MODES {
        let mut runs = vec![];

        for i in 0..runs_per_mode {
            let run = run_once(binary, envs, &result_path)?;
            println!(
                "{name} {}/{runs_per_mode}: {} panels, {} views, {} ms cpu, {} ms gpu",
                i + 1,
                run["panels"],
                run["views"],
                run["ms"],
                run["gpu_ms"],
            );
            runs.push(run);
        }

        let median_views = mark_suspects(&mut runs);

        modes.insert(
            (*name).to_string(),
            json!({
                "median_views": median_views,
                "runs": runs,
            }),
        );
    }

    let _ = fs::remove_file(&result_path);

    let date = Local::now().format("%Y-%m-%d").to_string();

    let suite = json!({
        "date": date,
        "commit": commit,
        "host": format!("{} {}", env::consts::OS, env::consts::ARCH),
        "runs_per_mode": runs_per_mode,
        "modes": modes,
    });

    let out = Path::new("bench").join(format!("{date}-{commit}.json"));
    fs::write(&out, serde_json::to_string_pretty(&suite)? + "\n")?;

    println!();
    for (name, mode) in suite["modes"].as_object().expect("modes is an object") {
        println!("{name} median views before lag: {}", mode["median_views"]);
    }
    println!("saved to {}", out.display());

    Ok(())
}

fn git_commit() -> Result<String> {
    let output = Command::new("git").args(["rev-parse", "--short", "HEAD"]).output()?;
    if !output.status.success() {
        bail!("git rev-parse failed");
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

fn build(args: &[&str]) -> Result<()> {
    let status = Command::new("cargo")
        .args(["build", "-p", "demo", "--features", "bench"])
        .args(args)
        .status()?;
    if !status.success() {
        bail!("cargo build failed");
    }
    Ok(())
}

fn run_once(binary: &str, envs: &[(&str, &str)], result_path: &Path) -> Result<Value> {
    let mut command = Command::new(binary);

    command
        .env("UI_BENCHMARK", "1")
        .env("UI_BENCHMARK_JSON", result_path)
        .stdout(std::process::Stdio::null());

    for (key, value) in envs {
        command.env(key, value);
    }

    let output = command.output()?;

    if output.status.success() {
        return Ok(serde_json::from_str(&fs::read_to_string(result_path)?)?);
    }

    let stderr = String::from_utf8_lossy(&output.stderr);

    if output.status.code() == Some(REJECTED) {
        bail!("benchmark rejected\n{}", stderr.trim());
    }

    bail!("benchmark failed with {}\n{stderr}", output.status);
}

/// Marks runs far below the median as suspect. Returns the median over the
/// clean runs.
fn mark_suspects(runs: &mut [Value]) -> i64 {
    let all = median(runs.iter().map(|r| r["views"].as_i64().expect("views is a number")));
    let threshold = all * SUSPECT_BELOW_MEDIAN_PERCENT / 100;

    for run in &mut *runs {
        if run["views"].as_i64().expect("views is a number") < threshold {
            run["suspect"] = json!(true);
            println!("suspect run: {} views vs median {all}", run["views"]);
        }
    }

    let clean: Vec<i64> = runs
        .iter()
        .filter(|r| r["suspect"] != json!(true))
        .map(|r| r["views"].as_i64().expect("views is a number"))
        .collect();

    if clean.is_empty() {
        all
    } else {
        median(clean.into_iter())
    }
}

fn median(values: impl Iterator<Item = i64>) -> i64 {
    let mut values: Vec<i64> = values.collect();
    assert!(!values.is_empty(), "median of empty values");
    values.sort_unstable();

    let mid = values.len() / 2;
    if values.len().is_multiple_of(2) {
        i64::midpoint(values[mid - 1], values[mid])
    } else {
        values[mid]
    }
}
