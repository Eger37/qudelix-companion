//! Dead-simple append-only debug log at `%APPDATA%\qudelix-widget\qudelix.log`.
//! Lets us diagnose the HID/BLE path on the user's machine without a console
//! (the release build is a GUI subsystem app with no stdout).

use std::io::Write;
use std::path::PathBuf;

fn log_path() -> PathBuf {
    let dir = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("qudelix-widget");
    let _ = std::fs::create_dir_all(&dir);
    dir.join("qudelix.log")
}

/// Append one line prefixed with a UTC `HH:MM:SS.mmm` stamp. Best-effort; never panics.
pub fn line(msg: &str) {
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let day = ms % 86_400_000;
    let stamp = format!(
        "{:02}:{:02}:{:02}.{:03}Z",
        day / 3_600_000,
        day / 60_000 % 60,
        day / 1000 % 60,
        day % 1000
    );
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(log_path()) {
        let _ = writeln!(f, "{stamp} {msg}");
    }
}

/// Format bytes as hex for the log.
pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02X}")).collect::<Vec<_>>().join(" ")
}
