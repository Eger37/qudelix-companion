//! Tiny HTTP server for the Android widget: self-update + remote debug log.
//!
//! * `GET` serves exactly two files from `%APPDATA%\qudelix-widget\updates` —
//!   `version.json` and `qudelix-widget.apk` (written by `android/publish_update.sh`).
//! * `POST /log` appends the phone's debug log to `%APPDATA%\qudelix-widget\phone.log`,
//!   so the phone can be debugged from the PC (readable from WSL).
//!
//! Opt-in: ⚙ → "Phone debug over Tailscale" (widget.ini `phone_debug`). Off = no
//! tailnet forward and every request refused. The forward is kept by tailscaled
//! and outlives the widget (requests then just fail); Uninstall removes it. Default for a fresh install is off;
//! a PC that already has an `updates` folder (the owner's dev PC) starts with it on.
//!
//! Listens on 127.0.0.1 only; Tailscale forwards tailnet port 8787 to it
//! (`tailscale serve --http=8787 http://127.0.0.1:8788`, set up by us at start,
//! no admin needed). That way Windows Firewall — which blocks inbound to our exe
//! and needs admin to change — is never involved, and nothing outside the
//! tailnet can reach the server.
//!
//! Every request must carry the PC key (`X-Qudelix-Key`, widget.ini `phone_key`,
//! made on first use and shown in the ⚙ dialog), so other devices in the tailnet
//! can neither write to phone.log nor fetch the owner's APK. Limits: a few
//! connections at a time, small headers, capped log batches and phone.log size,
//! and a pause after repeated wrong keys.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::win::app_dir;

/// Tailnet-facing port (what the phone uses) and our local backend port.
pub const PORT: u16 = 8787;
const LOCAL_PORT: u16 = 8788;
const TAILSCALE_EXE: &str = r"C:\Program Files\Tailscale\tailscale.exe";
/// Cap on one uploaded log batch.
const MAX_LOG_BODY: usize = 512 * 1024;
/// Cap on the request line + headers.
const MAX_HEADER: u64 = 8 * 1024;
/// phone.log is moved to phone.log.1 (replacing it) when it grows past this.
const MAX_LOG_FILE: u64 = 8 * 1024 * 1024;
/// Connections handled at once; more are closed right away.
const MAX_CONNS: usize = 4;
/// Wrong keys allowed per minute before every request is refused for that minute.
const MAX_BAD_KEYS: u32 = 10;
const FILES: [(&str, &str); 2] = [
    ("/version.json", "application/json"),
    ("/qudelix-widget.apk", "application/vnd.android.package-archive"),
];

static ENABLED: AtomicBool = AtomicBool::new(false);
static SPAWNED: AtomicBool = AtomicBool::new(false);
static CONNS: AtomicUsize = AtomicUsize::new(0);
/// (start of the current minute, wrong keys in it)
static BAD_KEYS: Mutex<Option<(Instant, u32)>> = Mutex::new(None);

/// The key the phone must send (created on first use). Grouped for typing:
/// "abcd-efgh-jkmn-pqrs" — 16 chars from a 31-letter alphabet, ~79 bits.
pub fn phone_key() -> String {
    if let Some(k) = crate::config::get("phone_key").filter(|k| norm_key(k).len() == 16) {
        return k;
    }
    let k = new_key();
    crate::config::set("phone_key", &k);
    k
}

fn new_key() -> String {
    use windows::Win32::Security::Cryptography::{BCryptGenRandom, BCRYPT_USE_SYSTEM_PREFERRED_RNG};
    const ALPHABET: &[u8] = b"abcdefghjkmnpqrstuvwxyz23456789";
    let mut out = String::new();
    while out.len() < 19 {
        let mut b = [0u8; 32];
        // SAFETY: plain buffer fill by the system RNG.
        let ok = unsafe { BCryptGenRandom(None, &mut b, BCRYPT_USE_SYSTEM_PREFERRED_RNG) }.is_ok();
        assert!(ok, "system RNG failed");
        // 248 = 8 × 31: dropping larger bytes keeps every letter equally likely.
        for x in b.iter().filter(|&&x| x < 248) {
            if out.len() == 19 {
                break;
            }
            if out.len() % 5 == 4 {
                out.push('-');
            }
            out.push(ALPHABET[(*x % 31) as usize] as char);
        }
    }
    out
}

/// Lowercase, without dashes/spaces: what the phone typed vs what we stored.
fn norm_key(k: &str) -> String {
    k.chars().filter(|c| c.is_ascii_alphanumeric()).map(|c| c.to_ascii_lowercase()).collect()
}

/// Constant-time compare (no early exit on the first wrong letter).
fn key_matches(sent: &str, want: &str) -> bool {
    let (a, b) = (norm_key(sent), norm_key(want));
    a.len() == b.len() && !b.is_empty() && a.bytes().zip(b.bytes()).fold(0u8, |d, (x, y)| d | (x ^ y)) == 0
}

/// True while too many wrong keys arrived this minute; `bad` records one more.
fn locked_out(bad: bool) -> bool {
    let mut g = BAD_KEYS.lock().unwrap_or_else(|e| e.into_inner());
    let now = Instant::now();
    let (start, n) = g.get_or_insert((now, 0));
    if now.duration_since(*start) > Duration::from_secs(60) {
        *start = now;
        *n = 0;
    }
    if bad {
        *n += 1;
    }
    *n >= MAX_BAD_KEYS
}

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
        std::thread::spawn(remove_forward);
    }
}

/// Drop the tailnet forward. It lives in tailscaled, not in this process, so it
/// outlasts the widget: switching phone debug off and uninstalling remove it.
pub fn remove_forward() {
    tailscale(&["serve", &format!("--http={PORT}"), "off"]);
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
    let o = std::process::Command::new(TAILSCALE_EXE)
        .args(args)
        .creation_flags(crate::win::CREATE_NO_WINDOW)
        .output()
        .ok()?;
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
                        if CONNS.fetch_add(1, Ordering::SeqCst) >= MAX_CONNS {
                            CONNS.fetch_sub(1, Ordering::SeqCst);
                            continue; // busy: drop the connection
                        }
                        std::thread::spawn(move || {
                            let _ = s.set_read_timeout(Some(Duration::from_secs(10)));
                            let _ = s.set_write_timeout(Some(Duration::from_secs(30)));
                            if let Err(e) = handle(s) {
                                crate::log::line(&format!("[update] request failed: {e}"));
                            }
                            CONNS.fetch_sub(1, Ordering::SeqCst);
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
    let target = format!("http://127.0.0.1:{LOCAL_PORT}");
    let r = std::process::Command::new(TAILSCALE_EXE)
        .args(["serve", "--bg", &format!("--http={PORT}"), &target])
        .creation_flags(crate::win::CREATE_NO_WINDOW)
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

fn updates_dir() -> PathBuf {
    app_dir().join("updates")
}

fn handle(mut s: TcpStream) -> std::io::Result<()> {
    let mut peer = String::from("?");
    // Headers are read through a hard byte cap: a client can't feed us endless lines.
    let mut r = BufReader::new(s.try_clone()?.take(MAX_HEADER));
    let mut line = String::new();
    r.read_line(&mut line)?;
    let mut parts = line.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    if !ENABLED.load(Ordering::SeqCst) {
        return s.write_all(b"HTTP/1.0 503 Service Unavailable\r\nContent-Length: 0\r\n\r\n");
    }
    let path = parts.next().unwrap_or("").to_string();
    let mut content_len = 0usize;
    let mut key = String::new();
    loop {
        let mut h = String::new();
        if r.read_line(&mut h)? == 0 || h.trim().is_empty() {
            break;
        }
        if let Some((k, v)) = h.split_once(':') {
            if k.trim().eq_ignore_ascii_case("content-length") {
                content_len = v.trim().parse().unwrap_or(usize::MAX);
            }
            if k.trim().eq_ignore_ascii_case("x-qudelix-key") {
                key = v.trim().to_string();
            }
            // Set by tailscale serve: who on the tailnet is calling.
            if k.trim().eq_ignore_ascii_case("x-forwarded-for") {
                peer = v.trim().to_string();
            }
        }
    }
    if locked_out(false) {
        return s.write_all(b"HTTP/1.0 429 Too Many Requests\r\nContent-Length: 0\r\n\r\n");
    }
    if !key_matches(&key, &phone_key()) {
        locked_out(true);
        // {:?}: client-supplied text can't forge extra log lines.
        crate::log::line(&format!("[update] {peer:?} {method:?} {path:?}: wrong or missing key"));
        return s.write_all(b"HTTP/1.0 401 Unauthorized\r\nContent-Length: 0\r\n\r\n");
    }
    if method == "POST" && path == "/log" {
        if content_len > MAX_LOG_BODY {
            return s.write_all(b"HTTP/1.0 413 Payload Too Large\r\nContent-Length: 0\r\n\r\n");
        }
        let mut body = vec![0u8; content_len];
        // The body is outside the header cap: read it from a fresh limit.
        r.get_mut().set_limit(content_len as u64);
        r.read_exact(&mut body)?;
        let log = app_dir().join("phone.log");
        if std::fs::metadata(&log).is_ok_and(|m| m.len() > MAX_LOG_FILE) {
            let old = app_dir().join("phone.log.1");
            let _ = std::fs::remove_file(&old); // Windows rename won't replace
            let _ = std::fs::rename(&log, old);
        }
        let mut f = std::fs::OpenOptions::new().create(true).append(true).open(&log)?;
        f.write_all(&body)?;
        if !body.ends_with(b"\n") {
            f.write_all(b"\n")?;
        }
        return s.write_all(b"HTTP/1.0 204 No Content\r\n\r\n");
    }
    crate::log::line(&format!("[update] {peer:?} {method:?} {path:?}"));
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
