//! Built-in per-user installer — no separate setup program, no admin rights.
//!
//! First start of the exe from anywhere else (e.g. Downloads) asks once:
//! "Install?" → copy to `%LOCALAPPDATA%\Programs\Qudelix Widget\`, add a
//! Start-menu shortcut, start with Windows, launch the installed copy, exit.
//! "No" → run portable from where it is (remembered in widget.ini).
//! Not `Program Files`: writing there needs admin (UAC), which would break the
//! silent self-update (self_update.rs replaces the exe in place).
//! The ⚙ menu has "Uninstall" (see `uninstall`).

use std::path::{Path, PathBuf};

use windows::core::{w, Interface, HSTRING, PCWSTR};
use windows::Win32::Foundation::HWND;
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, IPersistFile, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED,
};
use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};
use windows::Win32::UI::WindowsAndMessaging::*;

const APP_DIR: &str = "Qudelix Widget";
const EXE_NAME: &str = "qudelix-widget.exe";

fn env_dir(var: &str) -> Option<PathBuf> {
    std::env::var_os(var).map(PathBuf::from)
}

pub fn install_dir() -> Option<PathBuf> {
    Some(env_dir("LOCALAPPDATA")?.join("Programs").join(APP_DIR))
}

fn shortcut_path() -> Option<PathBuf> {
    Some(env_dir("APPDATA")?.join("Microsoft\\Windows\\Start Menu\\Programs").join(format!("{APP_DIR}.lnk")))
}

fn same_dir(a: &Path, b: &Path) -> bool {
    a.to_string_lossy().eq_ignore_ascii_case(&b.to_string_lossy())
}

/// Running the installed copy?
pub fn is_installed_copy() -> bool {
    match (std::env::current_exe(), install_dir()) {
        (Ok(exe), Some(dir)) => exe.parent().is_some_and(|p| same_dir(p, &dir)),
        _ => false,
    }
}

/// At startup (no CLI args): offer to install once. Returns true when the
/// installed copy was launched and this process must exit.
pub fn maybe_offer_install() -> bool {
    if is_installed_copy()
        || crate::config::flag_set("install_declined")
        || std::env::var_os("QUDELIX_PORTABLE").is_some()
    {
        return false;
    }
    let Some(dir) = install_dir() else { return false };
    let text = format!(
        "Install Qudelix Widget for this Windows user?\n\n\
         • copies it to {}\n\
         • adds it to the Start menu\n\
         • starts it with Windows (can be switched off in ⚙)\n\
         • updates install themselves\n\n\
         No admin rights needed. Choose “No” to just run it from here.",
        dir.display()
    );
    let answer = unsafe {
        MessageBoxW(
            HWND::default(),
            &HSTRING::from(text),
            w!("Qudelix Widget"),
            MB_YESNO | MB_ICONQUESTION | MB_SETFOREGROUND,
        )
    };
    if answer != IDYES {
        crate::config::set_flag("install_declined");
        return false;
    }
    match install(&dir) {
        Ok(()) => true,
        Err(e) => {
            crate::log::line(&format!("[install] failed: {e}"));
            unsafe {
                MessageBoxW(
                    HWND::default(),
                    &HSTRING::from(format!("Installation failed:\n{e}\n\nRunning from here instead.")),
                    w!("Qudelix Widget"),
                    MB_OK | MB_ICONWARNING,
                );
            }
            false
        }
    }
}

/// `--install`: install without asking (scripts, deploy). Starts the installed copy.
pub fn install_silent() -> Result<(), String> {
    let dir = install_dir().ok_or("no LOCALAPPDATA")?;
    install(&dir)
}

fn install(dir: &Path) -> Result<(), String> {
    let me = std::env::current_exe().map_err(|e| e.to_string())?;
    let target = dir.join(EXE_NAME);
    close_running_widget();
    std::fs::create_dir_all(dir).map_err(|e| format!("create {}: {e}", dir.display()))?;
    // An older installed copy may still be locked for a moment: rename it away first.
    if target.exists() {
        let old = target.with_extension("exe.old");
        let _ = std::fs::remove_file(&old);
        let _ = std::fs::rename(&target, &old);
    }
    std::fs::copy(&me, &target).map_err(|e| format!("copy to {}: {e}", target.display()))?;
    if let Err(e) = create_shortcut(&target) {
        crate::log::line(&format!("[install] Start-menu shortcut failed: {e}"));
    }
    crate::autostart::set_for(&target, true);
    crate::config::set_flag("autostart_init");
    // Detached stdio: an inherited console/pipe would keep the caller (e.g. a
    // WSL shell running `--install`) waiting until the widget exits.
    std::process::Command::new(&target)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("start installed copy: {e}"))?;
    crate::log::line(&format!("[install] installed to {}", target.display()));
    Ok(())
}

/// Ask a running widget (any copy) to close, and give it a moment.
fn close_running_widget() {
    unsafe {
        if let Ok(h) = FindWindowW(crate::app::CLASS_NAME, PCWSTR::null()) {
            if !h.is_invalid() {
                let _ = PostMessageW(h, WM_CLOSE, None, None);
                for _ in 0..30 {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    if FindWindowW(crate::app::CLASS_NAME, PCWSTR::null()).map_or(true, |h| h.is_invalid()) {
                        break;
                    }
                }
            }
        }
    }
}

fn create_shortcut(target: &Path) -> Result<(), String> {
    let lnk = shortcut_path().ok_or("no APPDATA")?;
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).map_err(|e| e.to_string())?;
        let t = HSTRING::from(target.as_os_str());
        link.SetPath(&t).map_err(|e| e.to_string())?;
        if let Some(dir) = target.parent() {
            link.SetWorkingDirectory(&HSTRING::from(dir.as_os_str())).map_err(|e| e.to_string())?;
        }
        link.SetDescription(w!("Qudelix 5K source priority & volume")).map_err(|e| e.to_string())?;
        link.SetIconLocation(&t, 0).map_err(|e| e.to_string())?;
        let file: IPersistFile = link.cast().map_err(|e| e.to_string())?;
        file.Save(&HSTRING::from(lnk.as_os_str()), true).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// ⚙ → Uninstall: remove autostart + shortcut, delete the install folder after
/// we exit. Settings in %APPDATA%\qudelix-widget stay. Returns true if the
/// caller should close the widget.
pub fn uninstall(owner: HWND) -> bool {
    let answer = unsafe {
        MessageBoxW(
            owner,
            w!("Uninstall Qudelix Widget?\n\nRemoves it from the Start menu and from Windows start-up, and deletes the installed program. Your settings are kept."),
            w!("Qudelix Widget"),
            MB_YESNO | MB_ICONQUESTION,
        )
    };
    if answer != IDYES {
        return false;
    }
    crate::autostart::set(false);
    if let Some(lnk) = shortcut_path() {
        let _ = std::fs::remove_file(lnk);
    }
    if is_installed_copy() {
        if let Some(dir) = install_dir() {
            // The running exe can't delete itself: a hidden cmd waits, then removes the folder.
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            let _ = std::process::Command::new("cmd.exe")
                .raw_arg(format!("/c ping 127.0.0.1 -n 3 >nul & rmdir /s /q \"{}\"", dir.display()))
                .creation_flags(CREATE_NO_WINDOW)
                .spawn();
        }
    }
    crate::log::line("[install] uninstalled");
    true
}
