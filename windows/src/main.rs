//! Qudelix 5K desktop gadget — a small Windows widget that controls the user's
//! own Qudelix 5K DAC/amp over Bluetooth (a GAIA session on RFCOMM, the same
//! commands the official app sends). USB-HID / BLE GATT remain as CLI probes.
//!
//! The protocol core (`proto`, `state`, `release`, `log`) is platform-independent
//! and host-testable; the window and the transports compile only for Windows.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
// The host build only runs the tests: most of the shared core is unused there.
#![cfg_attr(not(windows), allow(dead_code))]

mod log;
mod proto;
mod release;
mod win;

/// Semantic version shared with the Android widget (repo-root `VERSION`).
#[allow(dead_code)]
pub const VERSION: &str = include_str!("../../VERSION");
mod state;

#[cfg(windows)]
mod app;
#[cfg(all(windows, feature = "ble"))]
mod ble;
#[cfg(windows)]
mod config;
#[cfg(windows)]
mod device;
#[cfg(windows)]
mod logo;
#[cfg(windows)]
mod worker;
#[cfg(windows)]
mod usb;
#[cfg(windows)]
mod rfcomm;
#[cfg(windows)]
mod update_server;
#[cfg(windows)]
mod ui;
#[cfg(windows)]
mod self_update;
#[cfg(windows)]
mod autostart;
#[cfg(windows)]
mod installer;

#[cfg(windows)]
fn main() {
    // A panic inside the window procedure can't unwind across FFI and aborts the
    // process silently (release has no console) — record it first.
    std::panic::set_hook(Box::new(|info| log::line(&format!("[panic] {info}"))));
    // One-shot CLI probes (no window) to validate the transport end-to-end:
    //   qudelix-widget.exe --set-priority 0|1|2
    //   qudelix-widget.exe --set-volume <n>        (n = dB*60, signed)
    //   qudelix-widget.exe --set-logo RRGGBB       (recolour this exe's icon only)
    let args: Vec<String> = std::env::args().collect();
    if let Some(i) = args.iter().position(|a| a == "--set-priority") {
        if let Some(v) = args.get(i + 1).and_then(|s| s.parse::<u8>().ok()) {
            worker::probe_priority(v);
            return;
        }
    }
    if let Some(i) = args.iter().position(|a| a == "--set-logo") {
        if let Some(rgb) = args.get(i + 1).and_then(|s| u32::from_str_radix(s.trim_start_matches('#'), 16).ok()) {
            logo::probe(rgb);
            return;
        }
    }
    if let Some(i) = args.iter().position(|a| a == "--set-volume") {
        if let Some(n) = args.get(i + 1).and_then(|s| s.parse::<i16>().ok()) {
            worker::probe_volume(n);
            return;
        }
    }
    #[cfg(feature = "ble")]
    if let Some(i) = args.iter().position(|a| a == "--raw") {
        if let Some(hex) = args.get(i + 1) {
            worker::probe_raw(hex);
            return;
        }
    }
    #[cfg(feature = "ble")]
    if let Some(i) = args.iter().position(|a| a == "--rfcomm") {
        if let Some(hex) = args.get(i + 1) {
            worker::probe_rfcomm(hex);
            return;
        }
    }
    if args.iter().any(|a| a == "--refresh") {
        app::forward_to_widget(app::WM_APP_CLI_REFRESH, 0);
        return;
    }
    #[cfg(feature = "ble")]
    if let Some(i) = args.iter().position(|a| a == "--collision-test") {
        let n = args.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(3);
        worker::collision_test(n);
        return;
    }
    #[cfg(feature = "ble")]
    if let Some(i) = args.iter().position(|a| a == "--cycle-test") {
        let n = args.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(3);
        worker::cycle_test(n);
        return;
    }
    // The real, confirmed priority switch (full GAIA session over RFCOMM).
    #[cfg(feature = "ble")]
    if let Some(i) = args.iter().position(|a| a == "--priority") {
        if let Some(v) = args.get(i + 1).and_then(|s| s.parse::<u8>().ok()) {
            // A running widget owns the RFCOMM session — hand the command over.
            if v <= 2 && app::forward_to_widget(app::WM_APP_CLI_PRIORITY, v as usize) {
                return;
            }
            let _ = worker::set_priority_via_rfcomm(v);
            return;
        }
    }
    // Volume over RFCOMM (tentative; n = dB*60 signed). e.g. --volume -2400 = -40 dB.
    #[cfg(feature = "ble")]
    if let Some(i) = args.iter().position(|a| a == "--volume") {
        if let Some(n) = args.get(i + 1).and_then(|s| s.parse::<i16>().ok()) {
            if app::forward_to_widget(app::WM_APP_CLI_VOLUME, n as u16 as usize) {
                return;
            }
            let _ = worker::set_volume_via_rfcomm(n);
            return;
        }
    }

    // Diagnostics: GitHub update check (result in the log) / local swap test.
    if args.iter().any(|a| a == "--update-check") {
        match self_update::check_and_apply() {
            Ok(self_update::Outcome::UpToDate(v)) => log::line(&format!("[self-update] up to date v{v}")),
            Ok(self_update::Outcome::Restarting(v)) => log::line(&format!("[self-update] installed v{v}")),
            Err(e) => log::line(&format!("[self-update] check: {e}")),
        }
        return;
    }
    if let Some(i) = args.iter().position(|a| a == "--install-test") {
        let r = args.get(i + 1).map(|f| self_update::install_test(f));
        log::line(&format!("[self-update] install-test: {r:?}"));
        return;
    }
    // After a self-update: wait for the previous process, delete its renamed exe.
    let wait_pid = args
        .iter()
        .position(|a| a == "--wait-pid")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse().ok());
    self_update::finish_update(wait_pid);
    // Uninstall's helper copy in %TEMP%: the widget has exited, delete its folder.
    if args.iter().any(|a| a == "--remove-install") {
        installer::remove_install();
        return;
    }
    if args.iter().any(|a| a == "--install") {
        let r = installer::install_silent();
        log::line(&format!("[install] --install: {r:?}"));
        return;
    }
    // Plain start from outside the install folder: offer the built-in installer once.
    if args.len() == 1 && installer::maybe_offer_install() {
        return;
    }

    if let Err(e) = app::run() {
        // No console in release; this path matters mainly for `--debug` runs.
        eprintln!("qudelix-widget: fatal: {e}");
        std::process::exit(1);
    }
}

// On non-Windows hosts there is no window/HID to build; `cargo test` still runs
// the `proto` byte-packing tests. This stub keeps `cargo build`/`check` green.
#[cfg(not(windows))]
fn main() {
    eprintln!("qudelix-widget is a Windows application; build for x86_64-pc-windows-gnu.");
}
