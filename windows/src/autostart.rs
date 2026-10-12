//! "Start with Windows": a per-user `HKCU\…\CurrentVersion\Run` entry pointing
//! at this exe (no admin rights). The registry is the source of truth; the
//! entry is created once by default on first run (widget.ini `autostart_init`).

use std::ffi::c_void;

use windows::core::{w, PCWSTR};
use windows::Win32::System::Registry::{
    RegDeleteKeyValueW, RegGetValueW, RegSetKeyValueW, HKEY_CURRENT_USER, REG_SZ, RRF_RT_REG_SZ,
};

const RUN_KEY: PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
const VALUE: PCWSTR = w!("QudelixWidget");

/// The command line we register: the current exe, quoted.
fn command() -> Option<String> {
    let exe = std::env::current_exe().ok()?;
    Some(quoted(&exe))
}

fn quoted(exe: &std::path::Path) -> String {
    format!("\"{}\"", exe.display())
}

/// What is registered right now (any exe), if anything.
fn registered() -> Option<String> {
    let mut buf = [0u16; 1024];
    let mut size = (buf.len() * 2) as u32;
    let r = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            RUN_KEY,
            VALUE,
            RRF_RT_REG_SZ,
            None,
            Some(buf.as_mut_ptr() as *mut c_void),
            Some(&mut size),
        )
    };
    if r.0 != 0 {
        return None;
    }
    let n = (size as usize / 2).saturating_sub(1);
    Some(String::from_utf16_lossy(&buf[..n.min(buf.len())]))
}

/// Registered AND pointing at this exe (a moved exe counts as off).
pub fn is_enabled() -> bool {
    match (registered(), command()) {
        (Some(reg), Some(cmd)) => reg.eq_ignore_ascii_case(&cmd),
        _ => false,
    }
}

pub fn set(enable: bool) {
    match std::env::current_exe() {
        Ok(exe) => set_for(&exe, enable),
        Err(_) => {}
    }
}

/// Register / remove autostart for a specific exe (the installer uses this).
pub fn set_for(exe: &std::path::Path, enable: bool) {
    unsafe {
        if enable {
            let cmd = quoted(exe);
            let wide = crate::win::wide(&cmd);
            let r = RegSetKeyValueW(
                HKEY_CURRENT_USER,
                RUN_KEY,
                VALUE,
                REG_SZ.0,
                Some(wide.as_ptr() as *const c_void),
                (wide.len() * 2) as u32,
            );
            crate::log::line(&format!("[autostart] on → {cmd} (rc {})", r.0));
        } else {
            let r = RegDeleteKeyValueW(HKEY_CURRENT_USER, RUN_KEY, VALUE);
            crate::log::line(&format!("[autostart] off (rc {})", r.0));
        }
    }
}

/// First run of the installed copy: turn autostart on once (the user can switch
/// it off in the menu). A portable copy (run from Downloads etc.) is never
/// registered on its own — "Start with Windows" in ⚙ still works for it.
/// The installed copy also takes over an entry that points at an older location.
pub fn init_default() {
    if !crate::config::flag_set("autostart_init") {
        if crate::installer::is_installed_copy() {
            set(true);
        }
        crate::config::set_flag("autostart_init");
    } else if crate::installer::is_installed_copy() && registered().is_some() && !is_enabled() {
        set(true);
    }
}
