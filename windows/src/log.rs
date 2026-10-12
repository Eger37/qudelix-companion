//! Dead-simple append-only debug log at `%APPDATA%\qudelix-widget\qudelix.log`.
//! Lets us diagnose the Bluetooth path on the user's machine without a console
//! (the release build is a GUI subsystem app with no stdout). At MAX_SIZE the
//! file moves to `qudelix.log.1` (replacing the older one) and starts over.

use std::io::Write;
use std::path::PathBuf;

const MAX_SIZE: u64 = 2 * 1024 * 1024;

fn log_path() -> PathBuf {
    crate::win::app_dir().join("qudelix.log")
}

/// "2026-10-12 08:15:42.123Z" (UTC) for milliseconds since the Unix epoch.
fn stamp(ms: u128) -> String {
    let day = ms % 86_400_000;
    // Days → civil date (Howard Hinnant's algorithm), valid for any date after 1970.
    let z = (ms / 86_400_000) as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}-{m:02}-{d:02} {:02}:{:02}:{:02}.{:03}Z",
        day / 3_600_000,
        day / 60_000 % 60,
        day / 1000 % 60,
        day % 1000
    )
}

/// Serialises appends with the size check, so two threads never rotate at once.
static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Append one line prefixed with a UTC date + time stamp. Best-effort; never panics.
pub fn line(msg: &str) {
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let path = log_path();
    if std::fs::metadata(&path).is_ok_and(|m| m.len() > MAX_SIZE) {
        let old = path.with_extension("log.1");
        let _ = std::fs::remove_file(&old); // Windows rename won't replace
        let _ = std::fs::rename(&path, old);
    }
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "{} {msg}", stamp(ms));
    }
}

/// Format bytes as hex for the log.
pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02X}")).collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    #[test]
    fn stamp_has_the_date() {
        assert_eq!(super::stamp(0), "1970-01-01 00:00:00.000Z");
        // 2024-02-29 13:45:07.089 UTC (leap day)
        assert_eq!(super::stamp(1_709_214_307_089), "2024-02-29 13:45:07.089Z");
        assert_eq!(super::stamp(1_791_763_200_000), "2026-10-12 00:00:00.000Z");
    }
}
