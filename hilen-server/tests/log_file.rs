//! `tracing_init::init` sets the global subscriber and the panic hook, and
//! reads the env. All 3 belong to a whole process, so each case starts this
//! test binary again as a child with its own env and reads what it left.

use std::{
    env::{current_exe, temp_dir, var},
    fs::{read_dir, read_to_string, remove_dir_all},
    path::{Path, PathBuf},
    process::{Command, Output, id},
};

use anyhow::Result;
use hilen_server::{
    log_file::{LOG_DIR_ENV, LOG_FILE_ENV},
    log_file_path,
    tracing::info,
    tracing_init,
};

const CHILD_ENV: &str = "HILEN_LOG_TEST_CHILD";
const NAME: &str = "log_test";

fn fresh_dir(test: &str) -> PathBuf {
    temp_dir().join(format!("hilen-server-log-{test}-{}", id()))
}

fn run_child(mode: &str, dir: &Path, log_file: Option<&str>) -> Result<Output> {
    let mut command = Command::new(current_exe()?);
    command
        .args(["--exact", "child", "--nocapture"])
        .env(CHILD_ENV, mode)
        .env(LOG_DIR_ENV, dir)
        .env_remove(LOG_FILE_ENV)
        .env_remove("RUST_LOG");
    if let Some(value) = log_file {
        command.env(LOG_FILE_ENV, value);
    }
    Ok(command.output()?)
}

fn logs(dir: &Path) -> Result<Vec<PathBuf>> {
    Ok(read_dir(dir)?.filter_map(Result::ok).map(|entry| entry.path()).collect())
}

/// Not a test by itself. It does nothing in a normal run and plays the
/// backend when a test below starts it with `CHILD_ENV` set.
#[test]
fn child() {
    let Ok(mode) = var(CHILD_ENV) else {
        return;
    };
    tracing_init::init(NAME);
    info!(films = 3, "hello from the child");
    println!("path: {:?}", log_file_path());
    assert!(mode != "panic", "child panic");
}

#[test]
fn init_writes_the_lines_and_a_panic_to_the_file() -> Result<()> {
    let dir = fresh_dir("init");

    let output = run_child("panic", &dir, None)?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let files = logs(&dir)?;
    assert_eq!(files.len(), 1, "{files:?}");
    let path = &files[0];
    let text = read_to_string(path)?;
    remove_dir_all(&dir)?;

    assert!(!output.status.success());
    let file_name = path.file_name().unwrap().to_string_lossy().to_string();
    assert!(
        file_name.starts_with("log_test-") && path.extension().is_some_and(|ext| ext == "log"),
        "{file_name}"
    );

    let first = text.lines().next().unwrap();
    assert!(
        first.ends_with(&format!("log file: {}", path.display())),
        "{text}"
    );
    assert!(text.contains("hello from the child films=3"), "{text}");
    assert!(text.contains("ERROR") && text.contains("child panic"), "{text}");
    assert!(text.contains("Backtrace:"), "{text}");
    assert!(!text.contains('\u{1b}'), "{text}");

    // Stdout still gets every line, and the backend can ask for the path.
    assert!(stdout.contains("hello from the child"), "{stdout}");
    assert!(stdout.contains(&format!("path: Some({path:?})")), "{stdout}");
    Ok(())
}

#[test]
fn the_env_turns_the_file_off() -> Result<()> {
    let dir = fresh_dir("off");

    let output = run_child("quiet", &dir, Some("off"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    assert!(output.status.success(), "{stdout}");
    assert!(!dir.exists());
    assert!(stdout.contains("hello from the child"), "{stdout}");
    assert!(stdout.contains("path: None"), "{stdout}");
    assert!(!stdout.contains("log file"), "{stdout}");
    Ok(())
}
