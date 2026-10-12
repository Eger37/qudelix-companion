//! Self-update from GitHub Releases.
//!
//! Asks `api.github.com/repos/<REPO>/releases/latest`; if its tag (`vX.Y.Z`) is
//! newer than our VERSION, downloads the `qudelix-widget.exe` asset, checks its
//! Ed25519 signature (`qudelix-widget.exe.sig`, see release.rs), swaps it in
//! (a running .exe can be *renamed* but not overwritten: exe → exe.old, new → exe)
//! and starts it with `--wait-pid <our pid>`; the caller then exits. HTTPS via
//! WinHTTP (built into Windows) — no TLS crate, no admin rights.

use std::ffi::c_void;
use std::path::{Path, PathBuf};

use windows::core::{w, PCWSTR};
use windows::Win32::Networking::WinHttp::*;

use crate::release::{asset_url, json_str, semver, signed_by_trusted, split_url};

/// GitHub repository the releases come from (owner/name).
pub const REPO: &str = "Eger37/qudelix-companion";
const ASSET: &str = "qudelix-widget.exe";
/// Raw 64-byte Ed25519 signature of ASSET (tools/release.sh).
const SIG_ASSET: &str = "qudelix-widget.exe.sig";
/// Sanity cap on the download (the exe is ~1 MB).
const MAX_EXE: usize = 32 * 1024 * 1024;

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
    if exe_bytes.len() < 200_000 || exe_bytes.len() > MAX_EXE || !exe_bytes.starts_with(b"MZ") {
        return Err(format!("downloaded file is not an exe ({} bytes)", exe_bytes.len()));
    }
    // Only an exe signed with the owner's release key is installed: a hijacked
    // GitHub account or release can't push code to this PC.
    let sig_url = asset_url(&json, SIG_ASSET).ok_or("release is not signed (no qudelix-widget.exe.sig)")?;
    let (host, path) = split_url(&sig_url).ok_or("bad signature url")?;
    let sig = https_get(&host, &path)?;
    if !signed_by_trusted(&exe_bytes, &sig) {
        return Err("bad signature on the downloaded exe — not installing".into());
    }
    crate::log::line("[self-update] signature ok");
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
    std::fs::write(&new, bytes).map_err(|e| format!("write {}: {e}", new.display()))?;
    let old = move_aside(&exe).inspect_err(|_| {
        let _ = std::fs::remove_file(&new);
    })?;
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

/// Rename `exe` (possibly running) out of the way so a new file can take its
/// place; returns where it went. `exe.old` may itself still be a running image
/// (e.g. after a logo recolour swapped the exe), which can't be deleted or
/// overwritten — then a numbered name is used instead.
pub fn move_aside(exe: &Path) -> Result<PathBuf, String> {
    let mut last = String::new();
    for n in 0..20 {
        let old = if n == 0 { exe.with_extension("exe.old") } else { exe.with_extension(format!("exe.old{n}")) };
        let _ = std::fs::remove_file(&old);
        match std::fs::rename(exe, &old) {
            Ok(()) => return Ok(old),
            Err(e) => last = e.to_string(),
        }
    }
    Err(format!("rename running exe: {last}"))
}

/// Delete leftovers of `move_aside` next to `exe` (those still running stay).
pub fn remove_old_copies(exe: &Path) {
    let (Some(dir), Some(name)) = (exe.parent(), exe.file_name().and_then(|n| n.to_str())) else { return };
    let prefix = format!("{name}.old").to_ascii_lowercase();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            if e.file_name().to_string_lossy().to_ascii_lowercase().starts_with(&prefix) {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }
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
        remove_old_copies(&exe);
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
                if body.len() > MAX_EXE {
                    return Err(format!("response from {host} is too big"));
                }
            }
            Ok(body)
        })();
        close(req);
        close(conn);
        close(session);
        result
    }
}
