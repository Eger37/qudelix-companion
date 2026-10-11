//! Tiny HTTP server for the Android widget: self-update + remote debug log.
//!
//! * `GET` serves exactly two files from `%APPDATA%\qudelix-widget\updates` —
//!   `version.json` and `qudelix-widget.apk` (written by `android/publish_update.sh`).
//! * `POST /log` appends the phone's debug log to `%APPDATA%\qudelix-widget\phone.log`,
//!   so the phone can be debugged from the PC (readable from WSL).
//!
//! Opt-in: ⚙ → "Phone debug over Tailscale" (widget.ini `phone_debug`). Off = no
//! tailnet forward and every request refused. Default for a fresh install is off;
//! a PC that already has an `updates` folder (the owner's dev PC) starts with it on.
//!
//! Listens on 127.0.0.1 only; Tailscale forwards tailnet port 8787 to it
//! (`tailscale serve --http=8787 http://127.0.0.1:8788`, set up by us at start,
//! no admin needed). That way Windows Firewall — which blocks inbound to our exe
//! and needs admin to change — is never involved, and nothing outside the
//! tailnet can reach the server.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// Tailnet-facing port (what the phone uses) and our local backend port.
pub const PORT: u16 = 8787;
const LOCAL_PORT: u16 = 8788;
const TAILSCALE_EXE: &str = r"C:\Program Files\Tailscale\tailscale.exe";
/// Cap on one uploaded log batch.
const MAX_LOG_BODY: usize = 512 * 1024;
const FILES: [(&str, &str); 2] = [
    ("/version.json", "application/json"),
    ("/qudelix-widget.apk", "application/vnd.android.package-archive"),
];

static ENABLED: AtomicBool = AtomicBool::new(false);
static SPAWNED: AtomicBool = AtomicBool::new(false);

/// The saved setting (see module docs for the default).
pub fn enabled() -> bool {
    match crate::config::get("phone_debug").as_deref() {
        Some(v) => v == "1",
        None => updates_dir().is_dir(),
    }
}

/// At widget start: serve only if phone debug is on.
pub fn start_if_enabled() {
    if enabled() {
        ENABLED.store(true, Ordering::SeqCst);
        spawn();
    }
}

/// ⚙ menu toggle: save, then start serving or remove the tailnet forward.
pub fn set_enabled(on: bool) {
    crate::config::set("phone_debug", if on { "1" } else { "0" });
    ENABLED.store(on, Ordering::SeqCst);
    crate::log::line(&format!("[update] phone debug over Tailscale {}", if on { "on" } else { "off" }));
    if on {
        if SPAWNED.load(Ordering::SeqCst) {
            std::thread::spawn(ensure_tailscale_forward);
        } else {
            spawn();
        }
    } else {
        std::thread::spawn(|| tailscale(&["serve", &format!("--http={PORT}"), "off"]));
    }
}

/// This PC's Tailscale MagicDNS name (what to enter on the phone), if Tailscale runs.
pub fn tailnet_name() -> Option<String> {
    let out = tailscale_output(&["status", "--json"])?;
    let json = String::from_utf8_lossy(&out);
    let me = &json[json.find("\"Self\"")?..];
    let name = crate::release::json_str(me, "DNSName")?;
    Some(name.trim_end_matches('.').to_string()).filter(|n| !n.is_empty())
}

fn tailscale_output(args: &[&str]) -> Option<Vec<u8>> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let o = std::process::Command::new(TAILSCALE_EXE).args(args).creation_flags(CREATE_NO_WINDOW).output().ok()?;
    o.status.success().then_some(o.stdout)
}

fn tailscale(args: &[&str]) {
    match tailscale_output(args) {
        Some(_) => crate::log::line(&format!("[update] tailscale {}", args.join(" "))),
        None => crate::log::line(&format!("[update] tailscale {} failed", args.join(" "))),
    }
}

/// Start the server on a background thread (never blocks the UI). Once per process.
fn spawn() {
    if SPAWNED.swap(true, Ordering::SeqCst) {
        return;
    }
    std::thread::spawn(|| {
        ensure_tailscale_forward();
        loop {
            match TcpListener::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), LOCAL_PORT)) {
                Ok(l) => {
                    crate::log::line(&format!("[update] serving on 127.0.0.1:{LOCAL_PORT} (tailnet :{PORT})"));
                    for s in l.incoming().flatten() {
                        std::thread::spawn(move || {
                            let _ = s.set_read_timeout(Some(Duration::from_secs(10)));
                            if let Err(e) = handle(s) {
                                crate::log::line(&format!("[update] request failed: {e}"));
                            }
                        });
                    }
                }
                Err(e) => crate::log::line(&format!("[update] bind 127.0.0.1:{LOCAL_PORT} failed: {e}")),
            }
            std::thread::sleep(Duration::from_secs(60));
        }
    });
}

/// Point tailnet :PORT at our local backend. Idempotent; persists in tailscaled.
fn ensure_tailscale_forward() {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let target = format!("http://127.0.0.1:{LOCAL_PORT}");
    let r = std::process::Command::new(TAILSCALE_EXE)
        .args(["serve", "--bg", &format!("--http={PORT}"), &target])
        .creation_flags(CREATE_NO_WINDOW)
        .output();
    match r {
        Ok(o) if o.status.success() => crate::log::line(&format!("[update] tailnet :{PORT} → {target}")),
        Ok(o) => crate::log::line(&format!(
            "[update] tailscale serve failed: {}",
            String::from_utf8_lossy(&o.stderr).trim()
        )),
        Err(e) => crate::log::line(&format!("[update] tailscale not found: {e}")),
    }
}

fn app_dir() -> PathBuf {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("qudelix-widget")
}

fn updates_dir() -> PathBuf {
    app_dir().join("updates")
}

fn handle(mut s: TcpStream) -> std::io::Result<()> {
    let mut peer = String::from("?");
    let mut r = BufReader::new(s.try_clone()?);
    let mut line = String::new();
    r.read_line(&mut line)?;
    let mut parts = line.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    if !ENABLED.load(Ordering::SeqCst) {
        return s.write_all(b"HTTP/1.0 503 Service Unavailable\r\nContent-Length: 0\r\n\r\n");
    }
    let path = parts.next().unwrap_or("").to_string();
    let mut content_len = 0usize;
    loop {
        let mut h = String::new();
        if r.read_line(&mut h)? == 0 || h.trim().is_empty() {
            break;
        }
        if let Some((k, v)) = h.split_once(':') {
            if k.trim().eq_ignore_ascii_case("content-length") {
                content_len = v.trim().parse().unwrap_or(0);
            }
            // Set by tailscale serve: who on the tailnet is calling.
            if k.trim().eq_ignore_ascii_case("x-forwarded-for") {
                peer = v.trim().to_string();
            }
        }
    }
    if method == "POST" && path == "/log" {
        if content_len > MAX_LOG_BODY {
            return s.write_all(b"HTTP/1.0 413 Payload Too Large\r\nContent-Length: 0\r\n\r\n");
        }
        let mut body = vec![0u8; content_len];
        r.read_exact(&mut body)?;
        let mut f = std::fs::OpenOptions::new().create(true).append(true).open(app_dir().join("phone.log"))?;
        f.write_all(&body)?;
        if !body.ends_with(b"\n") {
            f.write_all(b"\n")?;
        }
        return s.write_all(b"HTTP/1.0 204 No Content\r\n\r\n");
    }
    crate::log::line(&format!("[update] {peer} {method} {path}"));
    let path = path.as_str();
    let Some(&(name, ctype)) = FILES.iter().find(|(p, _)| *p == path) else {
        return s.write_all(b"HTTP/1.0 404 Not Found\r\nContent-Length: 0\r\n\r\n");
    };
    match std::fs::read(updates_dir().join(&name[1..])) {
        Ok(body) => {
            write!(
                s,
                "HTTP/1.0 200 OK\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nCache-Control: no-store\r\n\r\n",
                body.len()
            )?;
            s.write_all(&body)
        }
        Err(_) => s.write_all(b"HTTP/1.0 404 Not Found\r\nContent-Length: 0\r\n\r\n"),
    }
}
