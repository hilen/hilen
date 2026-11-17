//! The Windows theme of an app under WSL. `WSLg` carries the window over X11,
//! which has no theme, so winit reports none and the engine would stay
//! light whatever Windows is set to. Windows keeps the theme in the
//! registry and interop runs Windows programs from WSL, so `reg.exe` reads
//! it once and a `powershell.exe` child follows it: it waits on the
//! registry change event and prints the new value on every change, no
//! polling.

use std::{
    io::{BufRead, BufReader},
    process::{Child, Command, Stdio},
    thread,
};

use anyhow::{Result, anyhow};
use base64::{Engine, engine::general_purpose::STANDARD};
use log::{info, warn};
use parking_lot::Mutex;

use crate::{deps::hreads::on_main, ui::Theme};

const KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize";
const VALUE: &str = "AppsUseLightTheme";

// The registry event provider takes HKEY_USERS with the user's SID, it
// does not know HKEY_CURRENT_USER. Windows PowerShell 5.1 runs it, the
// one every Windows has.
const WATCH_SCRIPT: &str = r#"
$sid = [System.Security.Principal.WindowsIdentity]::GetCurrent().User.Value
$key = "$sid\\Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"
$query = "SELECT * FROM RegistryValueChangeEvent WHERE Hive='HKEY_USERS' AND KeyPath='$key' AND ValueName='AppsUseLightTheme'"
Register-CimIndicationEvent -Query $query -SourceIdentifier theme | Out-Null
while ($true) {
  if ($null -eq (Wait-Event -SourceIdentifier theme)) {
    Start-Sleep -Seconds 1
    continue
  }
  Remove-Event -SourceIdentifier theme
  (Get-ItemProperty HKCU:\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize).AppsUseLightTheme
}
"#;

static WATCHER: Mutex<Option<Child>> = Mutex::new(None);

/// Reads the Windows theme once and follows its changes. Off the main
/// thread, every interop call launches a Windows process. Nothing
/// outside WSL.
pub(crate) fn start() {
    if !super::active() {
        return;
    }
    thread::spawn(|| {
        match read() {
            Ok(theme) => apply(theme),
            Err(err) => warn!("WSL: Windows theme not read: {err}"),
        }
        if let Err(err) = watch() {
            warn!("WSL: Windows theme not followed: {err}");
        }
    });
}

/// Ends the watcher. Killing the interop stub ends the Windows process
/// with it, checked with `tasklist.exe`.
pub(crate) fn stop() {
    let Some(mut child) = WATCHER.lock().take() else {
        return;
    };
    if let Err(err) = child.kill().and_then(|()| child.wait()) {
        warn!("WSL: theme watcher not stopped: {err}");
    }
}

fn read() -> Result<Theme> {
    let output = Command::new("reg.exe").args(["query", KEY, "/v", VALUE]).output()?;
    let text = String::from_utf8_lossy(&output.stdout);
    parse_query(&text).ok_or_else(|| anyhow!("no {VALUE} in: {text}"))
}

fn watch() -> Result<()> {
    let mut child = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-EncodedCommand",
            &encode(WATCH_SCRIPT),
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let stdout = child.stdout.take().ok_or_else(|| anyhow!("no stdout pipe"))?;
    *WATCHER.lock() = Some(child);
    info!("WSL: following the Windows theme");
    for line in BufReader::new(stdout).lines() {
        let line = line?;
        match parse_value(&line) {
            Some(theme) => apply(theme),
            None => warn!("WSL: theme watcher printed: {line}"),
        }
    }
    Ok(())
}

fn apply(theme: Theme) {
    info!("WSL: Windows theme is {theme:?}");
    on_main(move || Theme::set_system(theme));
}

/// `-EncodedCommand` takes the script as base64 of UTF-16LE. A script file
/// on a WSL path is refused by the execution policy, and a script on stdin
/// is dropped at end of input before its loop runs.
fn encode(script: &str) -> String {
    let utf16: Vec<u8> = script.encode_utf16().flat_map(u16::to_le_bytes).collect();
    STANDARD.encode(utf16)
}

/// The value line of a `reg.exe query`:
/// `    AppsUseLightTheme    REG_DWORD    0x1`
fn parse_query(output: &str) -> Option<Theme> {
    let line = output.lines().find(|line| line.contains(VALUE))?;
    let hex = line.split_whitespace().last()?.strip_prefix("0x")?;
    Some(from_flag(u32::from_str_radix(hex, 16).ok()?))
}

/// A line the watcher prints, the value as a plain number.
fn parse_value(line: &str) -> Option<Theme> {
    Some(from_flag(line.trim().parse().ok()?))
}

fn from_flag(light: u32) -> Theme {
    if light == 0 { Theme::Dark } else { Theme::Light }
}

#[cfg(test)]
mod tests {
    use super::*;

    const QUERY: &str = "\r\nHKEY_CURRENT_USER\\Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize\r\n    AppsUseLightTheme    REG_DWORD    0x1\r\n\r\n";

    #[test]
    fn query_value() {
        assert_eq!(parse_query(QUERY), Some(Theme::Light));
        assert_eq!(parse_query(&QUERY.replace("0x1", "0x0")), Some(Theme::Dark));
        assert_eq!(
            parse_query("ERROR: The system was unable to find the specified registry key or value."),
            None
        );
    }

    #[test]
    fn watcher_line() {
        assert_eq!(parse_value("0\r"), Some(Theme::Dark));
        assert_eq!(parse_value("1"), Some(Theme::Light));
        assert_eq!(parse_value("#< CLIXML"), None);
    }

    #[test]
    fn utf16_base64() {
        assert_eq!(encode("hi"), "aABpAA==");
    }
}
