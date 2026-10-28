//! The desktop-gadget window (Win32) and its event loop.
//!
//! This is a **desktop gadget**, not an always-on-top panel (Vista/7 gadget /
//! Rainmeter "stay on desktop" style): normal windows cover it and it is NOT
//! hidden by Win+D / "Show desktop".
//!
//! User-verified against Win+D: a top-level tool window kept directly ABOVE the
//! desktop window (`desktop_slot` — NOT `HWND_BOTTOM`, which on current
//! Windows 11 lands *under* Progman: invisible until the shell reshuffles, e.g.
//! on the Win key), OWNED by the desktop (`Progman`), plus guards that re-show
//! it whenever the shell hides/minimizes it (WM_SHOWWINDOW / WM_SIZE /
//! SC_MINIMIZE / SWP_HIDEWINDOW) and a 700 ms watchdog. (Parenting into the
//! shell's icon view was tried and dropped: worse rendering, Explorer hangs.)
//!
//! UI (custom-drawn, see `ui.rs`; follows the Windows light/dark theme): a
//! segmented source-priority control (USB / A2DP 1 / A2DP 2), volume −/+ that
//! repeat while held, a dB readout + bar, and what is playing — read back from
//! the 5K over the RFCOMM/GAIA session run by `worker.rs`.

use std::ffi::c_void;

use windows::core::{w, HSTRING, PCWSTR};
use windows::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWINDOWATTRIBUTE};
use windows::Win32::Graphics::Gdi::{BeginPaint, EndPaint, InvalidateRect, ScreenToClient, UpdateWindow, PAINTSTRUCT};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    ReleaseCapture, SetCapture, TrackMouseEvent, TME_LEAVE, TRACKMOUSEEVENT,
};
use windows::Win32::UI::Controls::WM_MOUSELEAVE;
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::config::{self, Accent, Pos, Settings, ThemeMode};
use crate::ui::{self, Hit};
use crate::worker::Worker;

const TIMER_REFRESH: usize = 1;
/// One-shot: read the 5K's state once after start.
const TIMER_STARTUP_READ: usize = 3;
/// Hold-to-repeat for volume −/+: first repeat after REPEAT_DELAY, then every REPEAT_EVERY.
const TIMER_REPEAT: usize = 2;
const REPEAT_DELAY_MS: u32 = 380;
const REPEAT_EVERY_MS: u32 = 90;
const WM_APP_RESHOW: u32 = WM_APP + 1; // undo a shell hide/minimize
/// Quit (Close, Uninstall, or the updated exe is starting). Always posted, never
/// handled inline: `DestroyWindow` frees `AppState`, which the caller may still
/// be using. Deferred while the settings menu or one of its dialogs is open.
const WM_APP_QUIT: u32 = WM_APP + 6;

/// Depth of the settings menu and its dialogs (modal loops that still hold
/// `&mut AppState`), and a quit that arrived meanwhile. See `ModalGuard`.
static MODAL: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
static QUIT_PENDING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Marks a modal section; on leaving the last one, a deferred quit is re-posted.
struct ModalGuard(HWND);

impl ModalGuard {
    fn new(hwnd: HWND) -> Self {
        MODAL.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        ModalGuard(hwnd)
    }
}

impl Drop for ModalGuard {
    fn drop(&mut self) {
        use std::sync::atomic::Ordering::SeqCst;
        if MODAL.fetch_sub(1, SeqCst) == 1 && QUIT_PENDING.swap(false, SeqCst) {
            post_quit(self.0);
        }
    }
}

/// Paired Bluetooth device names for ⚙ → Device, kept fresh in the background.
static PAIRED: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());
static PAIRED_BUSY: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn paired_names() -> Vec<String> {
    PAIRED.lock().map(|v| v.clone()).unwrap_or_default()
}

/// Re-enumerate on a helper thread (WinRT needs a COM apartment); one at a time.
fn refresh_paired_names() {
    use std::sync::atomic::Ordering::SeqCst;
    if PAIRED_BUSY.swap(true, SeqCst) {
        return;
    }
    std::thread::spawn(|| {
        crate::worker::init_com();
        if let Ok(v) = crate::ble::paired_devices() {
            if let Ok(mut g) = PAIRED.lock() {
                *g = v.into_iter().map(|(n, _)| n).collect();
            }
        }
        PAIRED_BUSY.store(false, SeqCst);
    });
}

fn post_quit(hwnd: HWND) {
    unsafe {
        let _ = PostMessageW(hwnd, WM_APP_QUIT, WPARAM(0), LPARAM(0));
    }
}
/// CLI → running widget: set priority (wParam = v). See `forward_to_widget`.
pub const WM_APP_CLI_PRIORITY: u32 = WM_APP + 2;
/// CLI → running widget: set absolute volume (wParam = n as u16, dB*60 signed).
pub const WM_APP_CLI_VOLUME: u32 = WM_APP + 3;
/// CLI → running widget: Refresh (read state from the 5K).
pub const WM_APP_CLI_REFRESH: u32 = WM_APP + 5;
/// Window class the CLI looks up to find a running widget.
pub const CLASS_NAME: PCWSTR = w!("QudelixWidgetClass");

// Volume stepping (the live level is read back from the 5K; see refresh_status).
const VOLUME_MIN_N: i16 = -3600; // -60 dB
const VOLUME_MAX_N: i16 = 0; // 0 dB (official app slider: -60…0)

struct AppState {
    /// The transport lives on a background thread; the UI only sends commands and
    /// polls status. This keeps blocking BLE/HID off the message loop (no freeze).
    worker: Worker,
    settings: Settings,
    theme: ui::Theme,
    fonts: ui::Fonts,
    /// DPI scale (1.0 = 96 dpi).
    scale: f32,
    hover: Option<Hit>,
    pressed: Option<Hit>,
    status: String,
    /// Last device state read back from the 5K.
    shown: crate::state::DevState,
    selected: Option<u8>,
    volume_n: i16,
    volume_known: bool,
    /// The 5K isn't this PC's default output (button shows "Link").
    pc_audio_off: bool,
    /// Music and calls are both down (button shows "Connect").
    pc_disconnected: bool,
    /// When the status last changed, and whether the bottom line still shows it.
    status_at: std::time::Instant,
    status_shown: bool,
}

/// How long a new status replaces "Playing" in the bottom line.
const STATUS_SHOW: std::time::Duration = std::time::Duration::from_secs(4);

impl AppState {
    /// Poll the worker; repaint only when the status line or device state changed.
    fn refresh_status(&mut self, hwnd: HWND) {
        let status = self.worker.status();
        let dev = self.worker.dev_state();
        // The bottom line shows a fresh status for STATUS_SHOW, then "Playing" again.
        if self.status_shown && self.status_at.elapsed() >= STATUS_SHOW {
            self.status_shown = false;
            invalidate(hwnd);
        }
        // Unlink/Link and Disconnect/Connect follow Windows' real state (checked by
        // the worker; unknown while an action runs), so a connect made elsewhere —
        // e.g. in Bluetooth settings — shows up too.
        // Link = the 5K isn't Windows' default output (Unlink moves it away).
        if let Some((music, calls, radio, on5k)) = self.worker.pc_live() {
            let (off, gone) = (!on5k, !music && !calls && !radio);
            if (off, gone) != (self.pc_audio_off, self.pc_disconnected) {
                self.pc_audio_off = off;
                self.pc_disconnected = gone;
                invalidate(hwnd);
            }
        }
        if status == self.status && dev == self.shown {
            return;
        }
        if status != self.status {
            self.status_at = std::time::Instant::now();
            self.status_shown = !status.is_empty() && status != "Ready";
        }
        if dev != self.shown {
            if let Some(v) = dev.priority {
                self.selected = Some(v);
            }
            // Don't fight the user's thumb while −/+ is held.
            if let (Some(n), false) = (dev.volume_n, self.is_repeating()) {
                self.volume_n = n;
                self.volume_known = true;
            }
            config::save_last_state(dev.priority, dev.volume_n);
            self.shown = dev;
        }
        self.status = status;
        invalidate(hwnd);
    }

    fn is_repeating(&self) -> bool {
        matches!(self.pressed, Some(Hit::VolDown | Hit::VolUp | Hit::Bar))
    }

    /// Priority click: update the UI optimistically and hand the command to the
    /// worker thread (non-blocking). The worker connects on demand.
    fn on_priority(&mut self, hwnd: HWND, v: u8) {
        crate::log::line(&format!("[click] priority {v}"));
        self.selected = Some(v);
        self.worker.priority(v);
        invalidate(hwnd);
    }

    /// One volume step (a click, or one tick of hold-to-repeat).
    fn on_volume(&mut self, hwnd: HWND, delta_n: i16) {
        self.volume_n = (self.volume_n as i32 + delta_n as i32)
            .clamp(VOLUME_MIN_N as i32, VOLUME_MAX_N as i32) as i16;
        self.volume_known = true;
        self.worker.volume(self.volume_n);
        invalidate(hwnd);
    }

    /// Release of a pressed element (click semantics: only if still over it).
    fn activate(&mut self, hwnd: HWND, hit: Hit) {
        match hit {
            Hit::Priority(v) => self.on_priority(hwnd, v),
            Hit::Refresh => {
                crate::log::line("[click] refresh");
                self.worker.refresh();
            }
            Hit::Close => post_quit(hwnd),
            Hit::Settings => self.settings_menu(hwnd),
            Hit::Output => self.output_menu(hwnd),
            Hit::PcAudio | Hit::Disconnect => {
                use crate::worker::PcLink;
                let mode = match hit {
                    Hit::Disconnect if self.pc_disconnected => PcLink::Connect,
                    Hit::Disconnect => PcLink::Disconnect,
                    _ if self.pc_audio_off => PcLink::Link,
                    _ => PcLink::Unlink,
                };
                crate::log::line(&format!("[click] pc {mode:?}"));
                // Optimistic; the worker's check of Windows' state corrects it.
                (self.pc_audio_off, self.pc_disconnected) = match mode {
                    PcLink::Unlink => (true, self.pc_disconnected),
                    PcLink::Disconnect => (true, true),
                    PcLink::Link | PcLink::Connect => (false, false),
                };
                self.worker.pc_audio(mode);
                invalidate(hwnd);
            }
            Hit::VolDown | Hit::VolUp | Hit::Bar => {} // acted on press + repeat/drag
        }
    }

    fn view(&self) -> ui::View<'_> {
        ui::View {
            version: crate::VERSION.trim(),
            status: &self.status,
            show_status: self.status_shown,
            priority: self.selected,
            volume_n: self.volume_n,
            volume_known: self.volume_known,
            playing: self.shown.playing.as_deref(),
            pc_audio_off: self.pc_audio_off,
            pc_disconnected: self.pc_disconnected,
            hover: self.hover,
            pressed: self.pressed,
        }
    }

    /// Output button: the PC's active playback devices, the default one checked; a
    /// pick makes it the default (plain Windows switch — nothing is sent to the 5K).
    /// Submenu: which output the PC falls back to on Unlink / Disconnect.
    fn output_menu(&mut self, hwnd: HWND) {
        const FIRST: usize = 1;
        const FB_AUTO: usize = 500;
        const FB_FIRST: usize = 501;
        let outputs = match crate::audio_out::list() {
            Ok(o) if !o.is_empty() => o,
            Ok(_) => {
                self.worker.note("no sound outputs found");
                return;
            }
            Err(e) => {
                crate::log::line(&format!("[output] list failed: {e}"));
                self.worker.note("sound outputs unavailable");
                return;
            }
        };
        let _modal = ModalGuard::new(hwnd);
        unsafe {
            let Ok(m) = CreatePopupMenu() else { return };
            for (i, o) in outputs.iter().enumerate() {
                let w = crate::win::wide(&o.name);
                let flags = MF_STRING | if o.default { MF_CHECKED } else { MF_UNCHECKED };
                let _ = AppendMenuW(m, flags, FIRST + i, PCWSTR(w.as_ptr()));
            }
            let fallback = config::load_fallback_output();
            let fb = CreatePopupMenu().unwrap_or_default();
            let add = |id: usize, on: bool, text: &str| {
                let w = crate::win::wide(text);
                let _ = AppendMenuW(fb, MF_STRING | if on { MF_CHECKED } else { MF_UNCHECKED }, id, PCWSTR(w.as_ptr()));
            };
            add(FB_AUTO, fallback.is_none(), "Automatic (the output before the 5K)");
            let _ = AppendMenuW(fb, MF_SEPARATOR, 0, PCWSTR::null());
            for (i, o) in outputs.iter().enumerate() {
                add(FB_FIRST + i, fallback.as_deref() == Some(o.id.as_str()), &o.name);
            }
            let _ = AppendMenuW(m, MF_SEPARATOR, 0, PCWSTR::null());
            let w = crate::win::wide("On Unlink / Disconnect, switch to");
            let _ = AppendMenuW(m, MF_POPUP, fb.0 as usize, PCWSTR(w.as_ptr()));
            let mut cp = POINT::default();
            let _ = GetCursorPos(&mut cp);
            let _ = SetForegroundWindow(hwnd); // so the menu closes on an outside click
            let cmd = TrackPopupMenu(m, TPM_RETURNCMD | TPM_RIGHTBUTTON, cp.x, cp.y, 0, hwnd, None).0 as usize;
            let _ = PostMessageW(hwnd, WM_NULL, WPARAM(0), LPARAM(0));
            let _ = DestroyMenu(m); // also destroys the submenu
            if cmd == FB_AUTO {
                config::save_fallback_output(None);
                self.worker.note("fallback output: automatic");
                return;
            }
            if let Some(o) = cmd.checked_sub(FB_FIRST).and_then(|i| outputs.get(i)) {
                crate::log::line(&format!("[output] fallback set: {}", o.name));
                config::save_fallback_output(Some(&o.id));
                self.worker.note(&format!("fallback → {}", o.name));
                return;
            }
            let Some(o) = cmd.checked_sub(FIRST).and_then(|i| outputs.get(i)) else { return };
            crate::log::line(&format!("[output] → {}", o.name));
            match crate::audio_out::set_default(&o.id) {
                Ok(()) => self.worker.note(&format!("output → {}", o.name)),
                Err(e) => {
                    crate::log::line(&format!("[output] set failed: {e}"));
                    self.worker.note("couldn't switch output");
                }
            }
        }
    }

    /// Settings menu (gear or right-click): theme, accent, volume step, logo background. Saved to widget.ini.
    fn settings_menu(&mut self, hwnd: HWND) {
        const T_SYSTEM: usize = 1;
        const T_LIGHT: usize = 2;
        const T_DARK: usize = 3;
        const T_CUSTOM: usize = 4;
        const A_CUSTOM: usize = 13;
        const A_WIN: usize = 12;
        const S_05: usize = 21;
        const S_1: usize = 22;
        const S_2: usize = 23;
        const CLOSE: usize = 99;
        const U_CHECK: usize = 31;
        const U_AUTO: usize = 32;
        const START_WIN: usize = 33;
        const UNINSTALL: usize = 34;
        const PHONE_DEBUG: usize = 35;
        const INSTALL: usize = 36;
        const D_AUTO: usize = 200;
        const D_FIRST: usize = 201;
        const L_FIRST: usize = 300;
        const L_CUSTOM: usize = 320;
        // Menu, colour picker, message boxes: other messages are dispatched while
        // they're open, and we hold &mut self — a quit must wait until we return.
        let _modal = ModalGuard::new(hwnd);
        unsafe {
            let check = |on: bool| if on { MF_CHECKED } else { MF_UNCHECKED };
            let add = |m: HMENU, id: usize, on: bool, text: &str| {
                let w = crate::win::wide(text);
                let _ = AppendMenuW(m, MF_STRING | check(on), id, PCWSTR(w.as_ptr()));
            };
            let sub = |root: HMENU, m: HMENU, text: &str| {
                let w = crate::win::wide(text);
                let _ = AppendMenuW(root, MF_POPUP, m.0 as usize, PCWSTR(w.as_ptr()));
            };
            let s = self.settings;
            let Ok(root) = CreatePopupMenu() else { return };
            // A theme = background + accent together.
            let theme = CreatePopupMenu().unwrap_or_default();
            add(theme, T_SYSTEM, s.theme == ThemeMode::System, "As in Windows — red");
            add(theme, A_WIN, s.theme == ThemeMode::WindowsColors, "Windows colours (incl. accent)");
            add(theme, T_LIGHT, s.theme == ThemeMode::Light, "Light — red && grey");
            add(theme, T_DARK, s.theme == ThemeMode::Dark, "Dark — red && black");
            let _ = AppendMenuW(theme, MF_SEPARATOR, 0, PCWSTR::null());
            add(theme, T_CUSTOM, s.theme == ThemeMode::Custom, "Custom: background colour…");
            add(theme, A_CUSTOM, false, "Custom: accent colour…");
            sub(root, theme, "Theme");
            // Which paired device to control: the list found in the background
            // (enumeration can take seconds — the menu must open at once).
            let devices = paired_names();
            refresh_paired_names();
            let chosen = config::load_device();
            let dev = CreatePopupMenu().unwrap_or_default();
            add(dev, D_AUTO, chosen.is_none(), "Auto — first “Qudelix-5K…”");
            let _ = AppendMenuW(dev, MF_SEPARATOR, 0, PCWSTR::null());
            for (i, name) in devices.iter().enumerate().take(40) {
                add(dev, D_FIRST + i, chosen.as_deref() == Some(name.as_str()), &name.replace('&', "&&"));
            }
            sub(root, dev, "Device");
            let step = CreatePopupMenu().unwrap_or_default();
            add(step, S_05, s.step_n == 30, "0.5 dB");
            add(step, S_1, s.step_n == 60, "1 dB");
            add(step, S_2, s.step_n == 120, "2 dB");
            sub(root, step, "Volume step");
            // Background of the app icon on the .exe files (logo.rs).
            let logo = CreatePopupMenu().unwrap_or_default();
            for (i, (name, rgb)) in crate::logo::PRESETS.iter().enumerate() {
                add(logo, L_FIRST + i, s.logo_bg == *rgb, name);
            }
            let _ = AppendMenuW(logo, MF_SEPARATOR, 0, PCWSTR::null());
            let custom_logo = !crate::logo::PRESETS.iter().any(|(_, rgb)| *rgb == s.logo_bg);
            add(logo, L_CUSTOM, custom_logo, "Custom colour…");
            sub(root, logo, "Logo background");
            let _ = AppendMenuW(root, MF_SEPARATOR, 0, PCWSTR::null());
            add(root, U_CHECK, false, &format!("Check for updates now (v{})", crate::VERSION.trim()));
            add(root, U_AUTO, s.auto_update, "Automatic updates");
            add(root, START_WIN, crate::autostart::is_enabled(), "Start with Windows");
            add(root, PHONE_DEBUG, crate::update_server::enabled(), "Phone debug over Tailscale…");
            if crate::installer::is_installed_copy() {
                add(root, UNINSTALL, false, "Uninstall…");
            } else {
                add(root, INSTALL, false, "Install for this user…");
            }
            let _ = AppendMenuW(root, MF_SEPARATOR, 0, PCWSTR::null());
            add(root, CLOSE, false, "Close widget");

            let mut cp = POINT::default();
            let _ = GetCursorPos(&mut cp);
            let _ = SetForegroundWindow(hwnd); // so the menu closes on an outside click
            let cmd = TrackPopupMenu(root, TPM_RETURNCMD | TPM_RIGHTBUTTON, cp.x, cp.y, 0, hwnd, None).0 as usize;
            let _ = PostMessageW(hwnd, WM_NULL, WPARAM(0), LPARAM(0));
            let _ = DestroyMenu(root); // also destroys the submenus

            let mut n = self.settings;
            match cmd {
                T_SYSTEM => n.theme = ThemeMode::System,
                T_LIGHT => n.theme = ThemeMode::Light,
                T_DARK => n.theme = ThemeMode::Dark,
                T_CUSTOM => match pick_color(hwnd, n.custom_bg) {
                    Some(rgb) => {
                        n.theme = ThemeMode::Custom;
                        n.custom_bg = rgb;
                    }
                    None => return,
                },
                A_CUSTOM => {
                    let cur = if let Accent::Custom(rgb) = n.accent { rgb } else { 0xE5_3935 };
                    match pick_color(hwnd, cur) {
                        Some(rgb) => {
                            n.theme = ThemeMode::Custom;
                            n.accent = Accent::Custom(rgb);
                        }
                        None => return,
                    }
                }
                A_WIN => n.theme = ThemeMode::WindowsColors,
                S_05 => n.step_n = 30,
                S_1 => n.step_n = 60,
                S_2 => n.step_n = 120,
                CLOSE => {
                    post_quit(hwnd);
                    return;
                }
                U_CHECK => {
                    spawn_update_check(hwnd, self.worker.status_arc(), true);
                    return;
                }
                U_AUTO => n.auto_update = !n.auto_update,
                UNINSTALL => {
                    if crate::installer::uninstall(hwnd) {
                        post_quit(hwnd);
                    }
                    return;
                }
                INSTALL => {
                    if crate::installer::install_from_menu(hwnd) {
                        post_quit(hwnd); // the installer waits for us, then starts the installed copy
                    }
                    return;
                }
                PHONE_DEBUG => {
                    phone_debug_toggle(hwnd);
                    return;
                }
                START_WIN => {
                    crate::autostart::set(!crate::autostart::is_enabled());
                    return;
                }
                c if (L_FIRST..L_FIRST + crate::logo::PRESETS.len()).contains(&c) => n.logo_bg = crate::logo::PRESETS[c - L_FIRST].1,
                L_CUSTOM => match pick_color(hwnd, n.logo_bg) {
                    Some(rgb) => n.logo_bg = rgb,
                    None => return,
                },
                D_AUTO => {
                    crate::log::line("[settings] device: auto");
                    config::save_device(None);
                    return;
                }
                c if (D_FIRST..D_FIRST + devices.len()).contains(&c) => {
                    let name = &devices[c - D_FIRST];
                    crate::log::line(&format!("[settings] device: {name:?}"));
                    config::save_device(Some(name));
                    return;
                }
                _ => return,
            }
            if n != self.settings {
                crate::log::line(&format!("[settings] {n:?}"));
                let logo_changed = n.logo_bg != self.settings.logo_bg;
                self.settings = n;
                config::save_settings(&n);
                if logo_changed {
                    let bg = n.logo_bg;
                    std::thread::spawn(move || crate::logo::apply(bg));
                }
                self.reload_theme(hwnd);
            }
        }
    }

    /// Click/drag on the volume bar: absolute level from the pointer's x.
    fn on_bar(&mut self, hwnd: HWND, x: i32) {
        let n = ui::bar_value(x, self.scale);
        if n != self.volume_n || !self.volume_known {
            self.volume_n = n;
            self.volume_known = true;
            self.worker.volume(n);
            invalidate(hwnd);
        }
    }

    /// Theme changed in Windows settings: recolour, including the DWM border.
    fn reload_theme(&mut self, hwnd: HWND) {
        self.theme = ui::Theme::new(&self.settings);
        apply_dwm_frame(hwnd, &self.theme);
        invalidate(hwnd);
    }

    /// Absolute volume forwarded from the CLI.
    fn set_volume_abs(&mut self, hwnd: HWND, n: i16) {
        crate::log::line(&format!("[cli] volume n={n}"));
        self.volume_n = n.clamp(VOLUME_MIN_N, VOLUME_MAX_N);
        self.volume_known = true;
        self.worker.volume(self.volume_n);
        invalidate(hwnd);
    }

    /// Save the current window position (screen coordinates) to disk.
    fn save_position(&self, hwnd: HWND) {
        unsafe {
            let mut wr = RECT::default();
            if GetWindowRect(hwnd, &mut wr).is_ok() {
                config::save_pos(Pos { x: wr.left, y: wr.top });
            }
        }
    }

    /// Moved to a monitor with another scale (or the scale changed): re-scale
    /// fonts and layout and take the size Windows suggests at the new position.
    fn on_dpi_changed(&mut self, hwnd: HWND, dpi: u32, suggested: &RECT) {
        self.scale = dpi.max(96) as f32 / 96.0;
        self.fonts = ui::Fonts::new(self.scale);
        let px = |v: i32| (v as f32 * self.scale).round() as i32;
        crate::log::line(&format!("[dpi] {dpi} dpi (scale {:.2})", self.scale));
        unsafe {
            let _ = SetWindowPos(
                hwnd,
                None,
                suggested.left,
                suggested.top,
                px(ui::WIDTH),
                px(ui::HEIGHT),
                SWP_NOZORDER | SWP_NOACTIVATE,
            );
        }
        invalidate(hwnd);
    }
}

/// Windows' standard colour dialog (full spectrum). `initial`/result are 0xRRGGBB.
fn pick_color(owner: HWND, initial: u32) -> Option<u32> {
    use windows::Win32::Foundation::COLORREF;
    use windows::Win32::UI::Controls::Dialogs::{ChooseColorW, CC_ANYCOLOR, CC_FULLOPEN, CC_RGBINIT, CHOOSECOLORW};
    let to_ref = |rgb: u32| COLORREF(((rgb & 0xFF) << 16) | (rgb & 0xFF00) | ((rgb >> 16) & 0xFF));
    // Preset swatches in the dialog's "custom colours" row: the two built-in themes + accents.
    let mut custom = [0xE5_3935u32, 0x0B_0B0C, 0xFF_FFFF, 0x1B_1F3B, 0x2F_6FEB, 0x10_B981, 0x8B_5CF6, 0xF5_9E0B,
        0xEC_4899, 0x06_B6D4, 0x37_4151, 0xF4_F4F5, 0x7F_1D1D, 0x14_532D, 0x1E_3A8A, 0x00_0000]
        .map(to_ref);
    let mut cc = CHOOSECOLORW {
        lStructSize: std::mem::size_of::<CHOOSECOLORW>() as u32,
        hwndOwner: owner,
        rgbResult: to_ref(initial),
        lpCustColors: custom.as_mut_ptr(),
        Flags: CC_RGBINIT | CC_FULLOPEN | CC_ANYCOLOR,
        ..Default::default()
    };
    unsafe { ChooseColorW(&mut cc) }.as_bool().then(|| {
        let c = cc.rgbResult.0;
        ((c & 0xFF) << 16) | (c & 0xFF00) | ((c >> 16) & 0xFF)
    })
}

/// Check GitHub Releases on a background thread; on a newer version the exe is
/// swapped and relaunched (see self_update.rs) and this window is told to quit.
fn spawn_update_check(hwnd: HWND, status: std::sync::Arc<std::sync::Mutex<String>>, manual: bool) {
    let raw = hwnd.0 as isize;
    std::thread::spawn(move || run_update_check(raw, &status, manual));
}

/// One update check at a time (manual + automatic would download twice and race
/// on the exe swap).
static UPDATE_BUSY: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn run_update_check(hwnd_raw: isize, status: &std::sync::Mutex<String>, manual: bool) {
    use std::sync::atomic::Ordering::SeqCst;
    let note = |s: String| {
        if let Ok(mut g) = status.lock() {
            *g = s;
        }
    };
    if UPDATE_BUSY.swap(true, SeqCst) {
        if manual {
            note("update check already running…".into());
        }
        return;
    }
    if manual {
        note("checking for updates…".into());
    }
    let result = crate::self_update::check_and_apply();
    UPDATE_BUSY.store(false, SeqCst);
    match result {
        Ok(crate::self_update::Outcome::UpToDate(v)) => {
            if manual {
                note(format!("up to date (v{v})"));
            }
        }
        Ok(crate::self_update::Outcome::Restarting(v)) => {
            crate::log::line(&format!("[self-update] installed v{v}, restarting"));
            note(format!("updating to v{v}…"));
            unsafe {
                let _ = PostMessageW(HWND(hwnd_raw as *mut c_void), WM_APP_QUIT, WPARAM(0), LPARAM(0));
            }
        }
        Err(e) => {
            crate::log::line(&format!("[self-update] check failed: {e}"));
            if manual {
                note("update check failed (offline?)".into());
            }
        }
    }
}

fn invalidate(hwnd: HWND) {
    unsafe {
        let _ = InvalidateRect(hwnd, None, false);
    }
}

/// Win11: rounded corners + a theme-coloured 1-px border (ignored on Win10).
fn apply_dwm_frame(hwnd: HWND, t: &ui::Theme) {
    const DWMWA_WINDOW_CORNER_PREFERENCE: i32 = 33;
    const DWMWA_BORDER_COLOR: i32 = 34;
    const DWMWCP_ROUND: u32 = 2;
    unsafe {
        let round = DWMWCP_ROUND;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWINDOWATTRIBUTE(DWMWA_WINDOW_CORNER_PREFERENCE),
            &round as *const u32 as *const c_void,
            4,
        );
        let border = t.border_colorref();
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWINDOWATTRIBUTE(DWMWA_BORDER_COLOR),
            &border as *const u32 as *const c_void,
            4,
        );
    }
}

/// ⚙ → "Phone debug over Tailscale…": explain, then switch the phone's update/log server.
fn phone_debug_toggle(hwnd: HWND) {
    let was_on = crate::update_server::enabled();
    let name = crate::update_server::tailnet_name();
    let setup = format!(
        "On the phone: settings → \"Debug over Tailscale\" → tick it and enter\n\n    \
         PC name:  {}\n    PC key:     {}\n\n",
        name.as_deref().unwrap_or("(Tailscale not running — start it and try again)"),
        crate::update_server::phone_key(),
    );
    let text = if was_on {
        format!("Phone debug over Tailscale is ON.\n\n{setup}Turn it off?")
    } else {
        format!(
            "Phone debug over Tailscale\n\n\
             What it does: the Android widget sends its log and crashes to this PC \
             (%APPDATA%\\qudelix-widget\\phone.log) and can install test builds from it \
             (the folder %APPDATA%\\qudelix-widget\\updates, filled by android/publish_update.sh).\n\n\
             Needs: Tailscale on this PC and on the phone, in the same tailnet. {setup}\
             How it works: Tailscale forwards port {} of this PC inside the tailnet only to the \
             widget (127.0.0.1:8788). Nothing is opened to the internet or the local network; no admin rights. \
             Requests without the key are refused, so other devices in the tailnet can't use it.\n\n\
             AI assistants can read phone.log to debug the phone without a cable.\n\n\
             Turn it on?",
            crate::update_server::PORT,
        )
    };
    let yes = unsafe {
        MessageBoxW(hwnd, &HSTRING::from(text), w!("Qudelix Widget"), MB_YESNO | MB_ICONINFORMATION) == IDYES
    };
    if yes {
        crate::update_server::set_enabled(!was_on);
    }
}

/// Create the window, pin it to the desktop, and run the message loop.
pub fn run() -> windows::core::Result<()> {
    unsafe {
        // Single instance: if a mutex of this name already exists, another copy
        // is running — exit immediately instead of stacking up processes.
        let _singleton = CreateMutexW(None, false, w!("Local\\QudelixWidgetSingleton")).ok();
        if GetLastError() == ERROR_ALREADY_EXISTS {
            return Ok(());
        }
        crate::log::line(&format!("[start] qudelix-widget v{} launched", crate::VERSION.trim()));
        // First run: start with Windows by default (toggle in the ⚙ menu).
        crate::autostart::init_default();
        // Phone debug over Tailscale (opt-in): serve the phone's test builds + log upload.
        crate::update_server::start_if_enabled();

        let hinstance = GetModuleHandleW(None)?;
        let class_name = CLASS_NAME;

        let wc = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wndproc),
            hInstance: hinstance.into(),
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            // Resource 101 = icon.ico, embedded by build.rs.
            hIcon: LoadIconW(hinstance, PCWSTR(101 as *const u16)).unwrap_or_default(),
            lpszClassName: class_name,
            ..Default::default()
        };
        RegisterClassW(&wc);

        // Never WS_EX_TOPMOST. Tool window => no taskbar button / Alt-Tab entry;
        // no-activate => never steals focus from the real foreground window.
        let ex_style = WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE;
        let style = WS_POPUP;

        let hwnd = CreateWindowExW(
            ex_style,
            class_name,
            w!("Qudelix 5K"),
            style,
            100,
            100,
            240,
            232,
            None,
            None,
            hinstance,
            None,
        )?;

        let dpi = GetDpiForWindow(hwnd).max(96);
        let scale = dpi as f32 / 96.0;
        let px = |v: i32| (v as f32 * scale).round() as i32;
        let _gdiplus = ui::gdiplus_start(); // kept for the process lifetime

        // ── State ─────────────────────────────────────────────────────────────
        // The transport runs on its own thread; the UI never blocks on BLE/HID.
        let worker = Worker::start();
        let settings = config::load_settings();
        crate::logo::ensure(settings.logo_bg);
        let theme = ui::Theme::new(&settings);
        apply_dwm_frame(hwnd, &theme);
        let (last_priority, last_volume) = config::load_last_state();
        let state = Box::new(AppState {
            worker,
            settings,
            theme,
            fonts: ui::Fonts::new(scale),
            scale,
            hover: None,
            pressed: None,
            status: String::new(),
            status_at: std::time::Instant::now(),
            status_shown: false,
            shown: crate::state::DevState::default(),
            selected: last_priority,
            volume_n: last_volume.unwrap_or(-1200),
            volume_known: last_volume.is_some(),
            pc_audio_off: false,
            pc_disconnected: false,
        });
        let state_ptr = Box::into_raw(state);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, state_ptr as isize);

        // ── Place + show (never activate) ─────────────────────────────────────
        let saved = config::load_pos().unwrap_or(Pos { x: 120, y: 120 });
        let (x, y, w_, h_) = (saved.x, saved.y, px(ui::WIDTH), px(ui::HEIGHT));
        if let Err(e) = SetWindowPos(hwnd, desktop_slot(hwnd), x, y, w_, h_, SWP_NOACTIVATE) {
            // The window just above the desktop can belong to an elevated process:
            // Windows then refuses to insert after it (Access denied). Place without
            // touching the z-order instead of failing to start; the shell guards
            // re-slot the gadget later.
            crate::log::line(&format!("[start] z-order slot refused ({e}) — placing without it"));
            SetWindowPos(hwnd, None, x, y, w_, h_, SWP_NOACTIVATE | SWP_NOZORDER)?;
        }
        // Win+D doesn't hide windows on Win10/11 — it raises the desktop window over
        // every normal window, so a bottom-pinned gadget would end up *under* it.
        // Make the DESKTOP (Progman) the window's OWNER — user-verified that this
        // plus the watchdog survives Win+D (2026-10-09).
        if let Ok(progman) = FindWindowW(w!("Progman"), PCWSTR::null()) {
            if !progman.0.is_null() {
                SetWindowLongPtrW(hwnd, GWLP_HWNDPARENT, progman.0 as isize);
                // Changing the owner doesn't reorder: put us right above the desktop now.
                let _ = SetWindowPos(
                    hwnd,
                    desktop_slot(hwnd),
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                );
            }
        }

        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        let _ = UpdateWindow(hwnd);
        // Self-update from GitHub Releases: shortly after start, then every 6 h.
        {
            let raw = hwnd.0 as isize;
            let status = (*state_ptr).worker.status_arc();
            std::thread::spawn(move || loop {
                std::thread::sleep(std::time::Duration::from_secs(30));
                if config::load_settings().auto_update {
                    run_update_check(raw, &status, false);
                }
                std::thread::sleep(std::time::Duration::from_secs(6 * 3600 - 30));
            });
        }
        // Poll the worker's status/connection for the status line (never blocks).
        SetTimer(hwnd, TIMER_REFRESH, 700, None);
        // Read the 5K's real state shortly after start (Bluetooth may still be
        // coming up right after logon).
        SetTimer(hwnd, TIMER_STARTUP_READ, 3000, None);
        refresh_paired_names(); // for ⚙ → Device

        // ── Message loop ──────────────────────────────────────────────────────
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        Ok(())
    }
}

/// Window procedure. `AppState` lives in `GWLP_USERDATA`; messages that arrive
/// before it is set fall through to `DefWindowProcW`.
unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    let state_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut AppState;

    match msg {
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let hdc = BeginPaint(hwnd, &mut ps);
            if !state_ptr.is_null() {
                let st = &*state_ptr;
                let mut rc = RECT::default();
                let _ = GetClientRect(hwnd, &mut rc);
                ui::paint(hdc, rc, st.scale, &st.theme, &st.fonts, &st.view());
            }
            let _ = EndPaint(hwnd, &ps);
            LRESULT(0)
        }
        WM_ERASEBKGND => LRESULT(1), // everything is painted in WM_PAINT (no flicker)
        // Press on an element: capture the mouse; volume acts at once and repeats
        // while held. Press on the background: drag the gadget (system move loop);
        // a press that doesn't move it is a Refresh.
        WM_LBUTTONDOWN => {
            if state_ptr.is_null() {
                return LRESULT(0);
            }
            let st = &mut *state_ptr;
            let (x, y) = (lp.0 as i16 as i32, (lp.0 >> 16) as i16 as i32);
            if let Some(hit) = ui::hit_test(x, y, st.scale) {
                st.pressed = Some(hit);
                SetCapture(hwnd);
                match hit {
                    Hit::VolDown => st.on_volume(hwnd, -st.settings.step_n),
                    Hit::VolUp => st.on_volume(hwnd, st.settings.step_n),
                    Hit::Bar => st.on_bar(hwnd, x),
                    _ => {}
                }
                if matches!(hit, Hit::VolDown | Hit::VolUp) {
                    crate::log::line(&format!("[click] volume {}", if hit == Hit::VolUp { "+" } else { "-" }));
                    SetTimer(hwnd, TIMER_REPEAT, REPEAT_DELAY_MS, None);
                }
                invalidate(hwnd);
            } else {
                let mut before = RECT::default();
                let _ = GetWindowRect(hwnd, &mut before);
                let _ = ReleaseCapture();
                // Modal move loop: other messages run meanwhile (even a quit), so
                // `st` is not used after it — re-read the state pointer instead.
                SendMessageW(hwnd, WM_NCLBUTTONDOWN, WPARAM(HTCAPTION as usize), LPARAM(0));
                let mut after = RECT::default();
                let p = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut AppState;
                if !p.is_null() && GetWindowRect(hwnd, &mut after).is_ok() && (after.left, after.top) == (before.left, before.top) {
                    crate::log::line("[click] background → refresh");
                    (*p).worker.refresh();
                }
            }
            LRESULT(0)
        }
        WM_MOUSEMOVE => {
            if !state_ptr.is_null() {
                let st = &mut *state_ptr;
                if st.pressed == Some(Hit::Bar) {
                    st.on_bar(hwnd, lp.0 as i16 as i32); // dragging the volume bar
                } else {
                    let (x, y) = (lp.0 as i16 as i32, (lp.0 >> 16) as i16 as i32);
                    let hover = ui::hit_test(x, y, st.scale);
                    if hover != st.hover {
                        st.hover = hover;
                        invalidate(hwnd);
                    }
                    let mut tme = TRACKMOUSEEVENT {
                        cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                        dwFlags: TME_LEAVE,
                        hwndTrack: hwnd,
                        dwHoverTime: 0,
                    };
                    let _ = TrackMouseEvent(&mut tme);
                }
            }
            LRESULT(0)
        }
        WM_MOUSELEAVE => {
            if !state_ptr.is_null() && (*state_ptr).hover.take().is_some() {
                invalidate(hwnd);
            }
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            if state_ptr.is_null() {
                return LRESULT(0);
            }
            let st = &mut *state_ptr;
            if let Some(hit) = st.pressed.take() {
                let _ = KillTimer(hwnd, TIMER_REPEAT);
                let _ = ReleaseCapture();
                let (x, y) = (lp.0 as i16 as i32, (lp.0 >> 16) as i16 as i32);
                if ui::hit_test(x, y, st.scale) == Some(hit) {
                    st.activate(hwnd, hit);
                }
                invalidate(hwnd);
            }
            LRESULT(0)
        }
        // Right-click anywhere: the settings menu.
        WM_RBUTTONUP => {
            if !state_ptr.is_null() {
                (*state_ptr).settings_menu(hwnd);
            }
            LRESULT(0)
        }
        WM_CAPTURECHANGED => {
            // Capture lost (e.g. another window grabbed it): stop any repeat.
            if !state_ptr.is_null() && (*state_ptr).pressed.take().is_some() {
                let _ = KillTimer(hwnd, TIMER_REPEAT);
                invalidate(hwnd);
            }
            LRESULT(0)
        }
        WM_SETTINGCHANGE => {
            // Light/dark switch in Windows settings arrives as "ImmersiveColorSet".
            if !state_ptr.is_null() && lp.0 != 0 {
                let name = PCWSTR(lp.0 as *const u16).to_string().unwrap_or_default();
                if name == "ImmersiveColorSet" {
                    (*state_ptr).reload_theme(hwnd);
                }
            }
            DefWindowProcW(hwnd, msg, wp, lp)
        }
        // Safety net: if Show Desktop / the shell hides, minimizes or refuses us,
        // bounce back. Re-show is posted, not done inline.
        WM_SHOWWINDOW => {
            if !state_ptr.is_null() && wp.0 == 0 && lp.0 != 0 {
                let _ = PostMessageW(hwnd, WM_APP_RESHOW, WPARAM(0), LPARAM(0));
            }
            DefWindowProcW(hwnd, msg, wp, lp)
        }
        WM_SIZE => {
            if !state_ptr.is_null() && wp.0 == SIZE_MINIMIZED as usize {
                let _ = PostMessageW(hwnd, WM_APP_RESHOW, WPARAM(0), LPARAM(0));
            }
            DefWindowProcW(hwnd, msg, wp, lp)
        }
        WM_SYSCOMMAND => {
            if (wp.0 & 0xFFF0) == SC_MINIMIZE as usize {
                return LRESULT(0);
            }
            DefWindowProcW(hwnd, msg, wp, lp)
        }
        // A `--priority`/`--volume` CLI call while the widget runs: the widget owns
        // the single RFCOMM session, so the CLI hands the command over instead of
        // opening a competing connection (which the 5K refuses with WSA 10064).
        WM_APP_CLI_PRIORITY => {
            if !state_ptr.is_null() && wp.0 <= 2 {
                (*state_ptr).on_priority(hwnd, wp.0 as u8);
            }
            LRESULT(0)
        }
        WM_APP_CLI_REFRESH => {
            if !state_ptr.is_null() {
                crate::log::line("[cli] refresh");
                (*state_ptr).worker.refresh();
            }
            LRESULT(0)
        }
        WM_APP_CLI_VOLUME => {
            if !state_ptr.is_null() {
                (*state_ptr).set_volume_abs(hwnd, wp.0 as u16 as i16);
            }
            LRESULT(0)
        }
        WM_APP_QUIT => {
            if MODAL.load(std::sync::atomic::Ordering::SeqCst) > 0 {
                // Settings menu / dialog open: close the menu, quit when it returns.
                QUIT_PENDING.store(true, std::sync::atomic::Ordering::SeqCst);
                let _ = EndMenu();
            } else {
                let _ = DestroyWindow(hwnd); // after an update the new exe is waiting for us to exit
            }
            LRESULT(0)
        }
        WM_DPICHANGED => {
            if !state_ptr.is_null() && lp.0 != 0 {
                (*state_ptr).on_dpi_changed(hwnd, (wp.0 & 0xFFFF) as u32, &*(lp.0 as *const RECT));
            }
            LRESULT(0)
        }
        WM_APP_RESHOW => {
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            let _ = SetWindowPos(
                hwnd,
                desktop_slot(hwnd),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
            LRESULT(0)
        }
        WM_EXITSIZEMOVE => {
            if !state_ptr.is_null() {
                (*state_ptr).save_position(hwnd);
            }
            LRESULT(0)
        }
        WM_TIMER => {
            // Hold-to-repeat: step while the pointer is still on the pressed button.
            if !state_ptr.is_null() && wp.0 == TIMER_REPEAT {
                let st = &mut *state_ptr;
                let mut cp = POINT::default();
                let over = GetCursorPos(&mut cp).is_ok()
                    && ScreenToClient(hwnd, &mut cp).as_bool()
                    && ui::hit_test(cp.x, cp.y, st.scale) == st.pressed;
                match st.pressed {
                    Some(Hit::VolDown) if over => st.on_volume(hwnd, -st.settings.step_n),
                    Some(Hit::VolUp) if over => st.on_volume(hwnd, st.settings.step_n),
                    _ => {}
                }
                SetTimer(hwnd, TIMER_REPEAT, REPEAT_EVERY_MS, None);
                return LRESULT(0);
            }
            if !state_ptr.is_null() && wp.0 == TIMER_STARTUP_READ {
                let _ = KillTimer(hwnd, TIMER_STARTUP_READ);
                crate::log::line("[start] reading the 5K's state");
                (*state_ptr).worker.refresh();
                return LRESULT(0);
            }
            if !state_ptr.is_null() && wp.0 == TIMER_REFRESH {
                (*state_ptr).refresh_status(hwnd);
                // Desktop-gadget watchdog: whatever "Show desktop" (Win+D) did —
                // hide or minimize us — put the gadget back on the desktop. This
                // catches every hide path the event-based guards miss.
                if IsIconic(hwnd).as_bool() || !IsWindowVisible(hwnd).as_bool() {
                    let _ = PostMessageW(hwnd, WM_APP_RESHOW, WPARAM(0), LPARAM(0));
                }
            }
            LRESULT(0)
        }
        // Safety net: stay at the bottom, and REFUSE to be hidden.
        // "Show desktop" (Win+D) hides top-level windows via SWP_HIDEWINDOW — strip
        // that flag so the gadget stays visible on the desktop. (The WM_SHOWWINDOW/
        // WM_SIZE re-show guard doesn't catch this path.)
        WM_WINDOWPOSCHANGING => {
            if !state_ptr.is_null() {
                let wpos = lp.0 as *mut WINDOWPOS;
                if !wpos.is_null() && ((*wpos).flags & SWP_NOZORDER).0 == 0 {
                    (*wpos).hwndInsertAfter = desktop_slot(hwnd);
                }
                if !wpos.is_null() {
                    (*wpos).flags &= !SWP_HIDEWINDOW;
                }
            }
            DefWindowProcW(hwnd, msg, wp, lp)
        }
        WM_DESTROY => {
            if !state_ptr.is_null() {
                drop(Box::from_raw(state_ptr));
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            }
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

/// `hWndInsertAfter` that puts `me` directly above the desktop (Progman): after
/// the lowest normal window. `HWND_BOTTOM` would put us UNDER Progman — hidden.
unsafe fn desktop_slot(me: HWND) -> HWND {
    let Ok(progman) = FindWindowW(w!("Progman"), PCWSTR::null()) else { return HWND_BOTTOM };
    if progman.0.is_null() {
        return HWND_BOTTOM;
    }
    let mut above = GetWindow(progman, GW_HWNDPREV).unwrap_or_default();
    if above == me {
        // Already in place: "after the window above us" keeps the position.
        above = GetWindow(me, GW_HWNDPREV).unwrap_or_default();
    }
    // Nothing (normal) above the desktop → just be the top normal window.
    if above.0.is_null() || (GetWindowLongW(above, GWL_EXSTYLE) as u32) & WS_EX_TOPMOST.0 != 0 {
        return HWND_TOP;
    }
    above
}

// ── Small Win32 helpers ────────────────────────────────────────────────────────

/// If a widget instance is running, hand it a CLI command and return true.
pub fn forward_to_widget(msg: u32, wparam: usize) -> bool {
    unsafe {
        match FindWindowW(CLASS_NAME, PCWSTR::null()) {
            Ok(h) if !h.is_invalid() => PostMessageW(h, msg, WPARAM(wparam), LPARAM(0)).is_ok(),
            _ => false,
        }
    }
}
