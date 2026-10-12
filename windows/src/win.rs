//! Small Windows helpers shared by several modules.

use std::path::PathBuf;

/// `CreateProcess` flag: start a console program (cmd, tailscale) without a window.
pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// `%APPDATA%\qudelix-widget` (system temp dir as a fallback), created if missing:
/// settings, logs and the phone's update files live here.
pub fn app_dir() -> PathBuf {
    let dir = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("qudelix-widget");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// NUL-terminated UTF-16 for Win32 `PCWSTR` arguments.
pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}
