//! Self-update from GitHub Releases.
//!
//! Asks `api.github.com/repos/<REPO>/releases/latest`; if its tag (`vX.Y.Z`) is
//! newer than our VERSION, downloads the `qudelix-widget.exe` asset, swaps it in
//! (a running .exe can be *renamed* but not overwritten: exe → exe.old, new → exe)
//! and starts it with `--wait-pid <our pid>`; the caller then exits. HTTPS via
//! WinHTTP (built into Windows) — no TLS crate, no admin rights.

use std::ffi::c_void;
use std::path::PathBuf;

use windows::core::{w, PCWSTR};
use windows::Win32::Networking::WinHttp::*;

use crate::release::{asset_url, json_str, semver, split_url};

/// GitHub repository the releases come from (owner/name).
pub const REPO: &str = "Eger37/qudelix-companion";
const ASSET: &str = "qudelix-widget.exe";

pub enum Outcome {
    UpToDate(String),
    /// New version installed and started; the caller must exit now.
    Restarting(String),
}

/// Check GitHub and, if newer, install + relaunch. Blocking (network).
pub fn check_and_apply() -> Result<Outcome, String> {
    let api = https_get("api.github.com", &format!("/repos/{REPO}/releases/latest"))?;
    let json = String::from_utf8_lossy(&api);
    let tag = json_str(&json, "tag_name").ok_or("no tag_name in release")?;
    let latest = tag.trim_start_matches('v').to_string();
    let current = crate::VERSION.trim();
    if semver(&latest) <= semver(current) {
        return Ok(Outcome::UpToDate(current.to_string()));
    }
    let url = asset_url(&json, ASSET).ok_or("release has no qudelix-widget.exe")?;
    crate::log::line(&format!("[self-update] {current} → {latest}: downloading"));
    let (host, path) = split_url(&url).ok_or("bad asset url")?;
    let exe_bytes = https_get(&host, &path)?;
    if exe_bytes.len() < 200_000 || !exe_bytes.starts_with(b"MZ") {
        return Err(format!("downloaded file is not an exe ({} bytes)", exe_bytes.len()));
    }
    install(&exe_bytes)?;
    Ok(Outcome::Restarting(latest))
}

/// Diagnostics (`--install-test <exe>`): run the swap+relaunch with a local file.
pub fn install_test(file: &str) -> Result<(), String> {
    install(&std::fs::read(file).map_err(|e| e.to_string())?)
}

/// exe → exe.old, new bytes → exe, start it waiting for us to exit.
fn install(bytes: &[u8]) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let new = exe.with_extension("exe.new");
    let old = old_path(&exe);
    std::fs::write(&new, bytes).map_err(|e| format!("write {}: {e}", new.display()))?;
    let _ = std::fs::remove_file(&old);
    std::fs::rename(&exe, &old).map_err(|e| format!("rename running exe: {e}"))?;
    if let Err(e) = std::fs::rename(&new, &exe) {
        let _ = std::fs::rename(&old, &exe); // put the old one back
        return Err(format!("swap in new exe: {e}"));
    }
    std::process::Command::new(&exe)
        .args(["--wait-pid", &std::process::id().to_string()])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("start new exe: {e}"))?;
    Ok(())
}

fn old_path(exe: &std::path::Path) -> PathBuf {
    exe.with_extension("exe.old")
}

/// Called at startup: after an update, wait for the previous process to exit
/// (it holds the single-instance mutex), then remove its renamed exe.
pub fn finish_update(wait_pid: Option<u32>) {
    if let Some(pid) = wait_pid {
        use windows::Win32::Foundation::CloseHandle;
        use windows::Win32::System::Threading::{OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE};
        unsafe {
            if let Ok(h) = OpenProcess(PROCESS_SYNCHRONIZE, false, pid) {
                let _ = WaitForSingleObject(h, 15_000);
                let _ = CloseHandle(h);
            }
        }
        crate::log::line(&format!("[self-update] now running v{}", crate::VERSION.trim()));
    }
    if let Ok(exe) = std::env::current_exe() {
        let _ = std::fs::remove_file(old_path(&exe));
    }
}

/// HTTPS GET (WinHTTP follows GitHub's redirects to its CDN). Returns the body
/// of a 200 response.
fn https_get(host: &str, path: &str) -> Result<Vec<u8>, String> {
    let wide = |s: &str| s.encode_utf16().chain(std::iter::once(0)).collect::<Vec<u16>>();
    let (h_host, h_path) = (wide(host), wide(path));
    unsafe {
        let session = WinHttpOpen(
            w!("qudelix-widget"),
            WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY,
            PCWSTR::null(),
            PCWSTR::null(),
            0,
        );
        if session.is_null() {
            return Err("WinHttpOpen failed".into());
        }
        let _ = WinHttpSetTimeouts(session, 10_000, 10_000, 15_000, 30_000);
        let close = |h: *mut c_void| {
            if !h.is_null() {
                let _ = WinHttpCloseHandle(h);
            }
        };
        let conn = WinHttpConnect(session, PCWSTR(h_host.as_ptr()), INTERNET_DEFAULT_HTTPS_PORT, 0);
        let req = if conn.is_null() {
            std::ptr::null_mut()
        } else {
            WinHttpOpenRequest(
                conn,
                w!("GET"),
                PCWSTR(h_path.as_ptr()),
                PCWSTR::null(),
                PCWSTR::null(),
                std::ptr::null(),
                WINHTTP_FLAG_SECURE,
            )
        };
        let result = (|| {
            if req.is_null() {
                return Err(format!("cannot open request to {host}"));
            }
            let headers: Vec<u16> = "Accept: application/vnd.github+json\r\n".encode_utf16().collect();
            WinHttpSendRequest(req, Some(&headers), None, 0, 0, 0).map_err(|e| format!("send: {e}"))?;
            WinHttpReceiveResponse(req, std::ptr::null_mut()).map_err(|e| format!("receive: {e}"))?;
            let mut status: u32 = 0;
            let mut len = std::mem::size_of::<u32>() as u32;
            let _ = WinHttpQueryHeaders(
                req,
                WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
                PCWSTR::null(),
                Some(&mut status as *mut u32 as *mut c_void),
                &mut len,
                std::ptr::null_mut(),
            );
            if status != 200 {
                return Err(format!("HTTP {status} from {host}"));
            }
            let mut body = Vec::new();
            let mut buf = vec![0u8; 64 * 1024];
            loop {
                let mut read = 0u32;
                WinHttpReadData(req, buf.as_mut_ptr() as *mut c_void, buf.len() as u32, &mut read)
                    .map_err(|e| format!("read: {e}"))?;
                if read == 0 {
                    break;
                }
                body.extend_from_slice(&buf[..read as usize]);
            }
            Ok(body)
        })();
        close(req);
        close(conn);
        close(session);
        result
    }
}
