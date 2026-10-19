//! Custom-drawn gadget UI: themes (light = red & grey, dark = red & black;
//! System follows Windows; accent = Qudelix red or the Windows accent),
//! layout + hit-testing, and painting.
//!
//! Plain Win32: anti-aliased shapes via GDI+ (flat API), ClearType text via GDI,
//! double-buffered. No browser engine, no extra runtime — the widget stays a few MB.

use std::ffi::c_void;

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{COLORREF, RECT};
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::Graphics::GdiPlus::*;
use windows::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};

use crate::config::{Accent, Settings, ThemeMode};
use crate::proto;

/// Logical (96-dpi) size of the gadget.
pub const WIDTH: i32 = 232;
pub const HEIGHT: i32 = 162;

/// Clickable elements.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Hit {
    Close,
    Refresh,
    Priority(u8),
    VolDown,
    VolUp,
    Settings,
    /// The volume bar: click or drag to set the level.
    Bar,
    /// Unlink (music off) / Link (all back) this PC's Bluetooth links to the 5K.
    PcAudio,
    /// Drop this PC's whole Bluetooth connection to the 5K (music + calls).
    Disconnect,
    /// Pick the PC's sound output device (menu).
    Output,
}

/// Colours as 0xAARRGGBB.
#[derive(Clone, Copy)]
pub struct Theme {
    pub dark: bool,
    bg: u32,
    surface: u32,
    hover: u32,
    border: u32,
    text: u32,
    muted: u32,
    accent: u32,
    on_accent: u32,
}

impl Theme {
    /// Qudelix red, used as the default accent in both themes.
    const RED: u32 = 0xFFE5_3935;

    pub fn new(cfg: &Settings) -> Self {
        let dark = match cfg.theme {
            ThemeMode::Light => false,
            ThemeMode::Dark => true,
            ThemeMode::Custom => luminance(cfg.custom_bg) < 0.5,
            ThemeMode::System | ThemeMode::WindowsColors => reg_dword(
                w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"),
                w!("AppsUseLightTheme"),
            )
            .map(|v| v == 0)
            .unwrap_or(false),
        };
        // The accent belongs to the theme: presets are red, WindowsColors uses the
        // Windows accent, Custom uses the user's pick.
        let accent = match (cfg.theme, cfg.accent) {
            (ThemeMode::Custom, Accent::Custom(rgb)) => 0xFF00_0000 | rgb,
            // DWM AccentColor is 0xAABBGGRR.
            (ThemeMode::WindowsColors, _) | (ThemeMode::Custom, Accent::Windows) => {
                reg_dword(w!("Software\\Microsoft\\Windows\\DWM"), w!("AccentColor"))
                    .map(|abgr| 0xFF00_0000 | ((abgr & 0xFF) << 16) | (abgr & 0xFF00) | ((abgr >> 16) & 0xFF))
                    .unwrap_or(Self::RED)
            }
            _ => Self::RED,
        };
        if cfg.theme == ThemeMode::Custom {
            // Everything derived from the picked background so it stays readable.
            let bg = 0xFF00_0000 | cfg.custom_bg;
            let text = if dark { 0xFFF2_F2F2 } else { 0xFF11_1111 };
            return Theme {
                dark,
                bg,
                surface: mix(bg, text, 0.07),
                hover: mix(bg, text, 0.13),
                border: mix(bg, text, 0.16),
                text,
                muted: mix(bg, text, 0.60),
                accent,
                on_accent: if luminance(accent & 0xFF_FFFF) > 0.6 { 0xFF11_1111 } else { 0xFFFF_FFFF },
            };
        }
        if dark {
            // Red & black.
            Theme {
                dark,
                bg: 0xFF0B_0B0C,
                surface: 0xFF1A_1A1C,
                hover: 0xFF26_262A,
                border: 0xFF2E_2E33,
                text: 0xFFF2_F2F2,
                muted: 0xFF9A_9A9E,
                accent,
                on_accent: 0xFFFF_FFFF,
            }
        } else {
            // Red & light grey (user: grey, not white).
            Theme {
                dark,
                bg: 0xFFE4_E4E7,
                surface: 0xFFF4_F4F6,
                hover: 0xFFFF_FFFF,
                border: 0xFFCF_CFD4,
                text: 0xFF11_1111,
                muted: 0xFF5E_5E63,
                accent,
                on_accent: 0xFFFF_FFFF,
            }
        }
    }

    /// Window-border colour for DWM (COLORREF 0x00BBGGRR).
    pub fn border_colorref(&self) -> u32 {
        colorref(self.border).0
    }
}

/// Relative luminance (0..1) of 0xRRGGBB.
fn luminance(rgb: u32) -> f32 {
    let c = |s: u32| ((rgb >> s) & 0xFF) as f32 / 255.0;
    0.2126 * c(16) + 0.7152 * c(8) + 0.0722 * c(0)
}

/// Blend ARGB `a` toward `b` by `t` (0..1), opaque.
fn mix(a: u32, b: u32, t: f32) -> u32 {
    let ch = |s: u32| {
        let (x, y) = (((a >> s) & 0xFF) as f32, ((b >> s) & 0xFF) as f32);
        ((x + (y - x) * t).round() as u32) << s
    };
    0xFF00_0000 | ch(16) | ch(8) | ch(0)
}

fn reg_dword(key: PCWSTR, value: PCWSTR) -> Option<u32> {
    let mut v: u32 = 0;
    let mut size = std::mem::size_of::<u32>() as u32;
    let r = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            key,
            value,
            RRF_RT_REG_DWORD,
            None,
            Some(&mut v as *mut u32 as *mut c_void),
            Some(&mut size),
        )
    };
    (r.0 == 0).then_some(v)
}

fn colorref(argb: u32) -> COLORREF {
    COLORREF(((argb & 0xFF) << 16) | (argb & 0xFF00) | ((argb >> 16) & 0xFF))
}

// ── Layout (logical px) ──────────────────────────────────────────────────────────

// Same layout as the phone widget (android/res/layout/widget.xml): "QDLX ⚙ ✕" over
// "5K v…" (✕ is PC only), the X-cut button block top right, then priority,
// volume, the 12-segment bar, and one bottom line (status, else "Playing").

const PAD: i32 = 12;
const TOP: i32 = 8;
const HEADER_H: i32 = 48;
const LINE_H: i32 = HEADER_H / 2;
const ICON: i32 = 18;
const ROW_H: i32 = 26;
const SEG_Y: i32 = TOP + HEADER_H + 6;
const VOL_Y: i32 = SEG_Y + ROW_H + 5;
const VOL_BTN_W: i32 = 60;
const BAR_Y: i32 = VOL_Y + ROW_H + 6;
const BAR_H: i32 = 10;
const BAR_SEGS: i32 = 12;
const FOOT_Y: i32 = BAR_Y + BAR_H + 4;
const FOOT_H: i32 = 16;
/// Left edge of the X-cut block.
const XB_LEFT: i32 = 98;

fn r(x: i32, y: i32, w: i32, h: i32) -> RECT {
    RECT { left: x, top: y, right: x + w, bottom: y + h }
}

/// The 2×2 buttons top right: Output ↻ / Unlink Disconnect.
const GRID: [Hit; 4] = [Hit::Output, Hit::Refresh, Hit::PcAudio, Hit::Disconnect];
const GAP: i32 = 3;

fn grid_rect(hit: Hit) -> RECT {
    let i = GRID.iter().position(|&h| h == hit).unwrap_or(0) as i32;
    let (col, row) = (i % 2, i / 2);
    let w = (WIDTH - PAD - XB_LEFT - GAP) / 2;
    let h = (HEADER_H - GAP) / 2;
    r(XB_LEFT + col * (w + GAP), TOP + row * (h + GAP), w, h)
}

fn settings_rect() -> RECT {
    r(PAD + 40, TOP + (LINE_H - ICON) / 2, ICON, ICON)
}
fn close_rect() -> RECT {
    r(PAD + 40 + ICON + 2, TOP + (LINE_H - ICON) / 2, ICON, ICON)
}
fn seg_item(i: i32) -> RECT {
    let w = (WIDTH - 2 * PAD - 2 * 4) / 3;
    r(PAD + i * (w + 4), SEG_Y, w, ROW_H)
}
fn vol_down_rect() -> RECT {
    r(PAD, VOL_Y, VOL_BTN_W, ROW_H)
}
fn vol_up_rect() -> RECT {
    r(WIDTH - PAD - VOL_BTN_W, VOL_Y, VOL_BTN_W, ROW_H)
}

fn bar_rect() -> RECT {
    r(PAD, BAR_Y, WIDTH - 2 * PAD, BAR_H)
}

/// Volume (n = dB*60, 0.5-dB steps) for a pointer at client x (physical px) on the bar.
pub fn bar_value(x: i32, scale: f32) -> i16 {
    let b = bar_rect();
    let frac = ((x as f32 / scale - b.left as f32) / (b.right - b.left) as f32).clamp(0.0, 1.0);
    let n = -3600.0 + frac * 3600.0;
    ((n / 30.0).round() * 30.0) as i16
}

fn inside(rc: &RECT, x: i32, y: i32) -> bool {
    x >= rc.left && x < rc.right && y >= rc.top && y < rc.bottom
}

/// Which element is at client point (x, y) (physical px).
pub fn hit_test(x: i32, y: i32, scale: f32) -> Option<Hit> {
    let (x, y) = ((x as f32 / scale) as i32, (y as f32 / scale) as i32);
    if inside(&close_rect(), x, y) {
        return Some(Hit::Close);
    }
    if inside(&settings_rect(), x, y) {
        return Some(Hit::Settings);
    }
    if let Some(&hit) = GRID.iter().find(|&&h| inside(&grid_rect(h), x, y)) {
        return Some(hit);
    }
    for i in 0..3 {
        if inside(&seg_item(i), x, y) {
            return Some(Hit::Priority(i as u8));
        }
    }
    if inside(&vol_down_rect(), x, y) {
        return Some(Hit::VolDown);
    }
    if inside(&vol_up_rect(), x, y) {
        return Some(Hit::VolUp);
    }
    // A little vertical grab area around the bar.
    let b = bar_rect();
    if inside(&RECT { top: b.top - 3, bottom: b.bottom + 3, ..b }, x, y) {
        return Some(Hit::Bar);
    }
    None
}

// ── Fonts ────────────────────────────────────────────────────────────────────────

pub struct Fonts {
    title: HFONT,
    body: HFONT,
    small: HFONT,
    big: HFONT,
    icon: HFONT,
}

impl Fonts {
    pub fn new(scale: f32) -> Self {
        let f = |px: f32, weight: i32, face: PCWSTR| unsafe {
            CreateFontW(
                -(px * scale).round() as i32,
                0,
                0,
                0,
                weight,
                0,
                0,
                0,
                DEFAULT_CHARSET.0 as u32,
                OUT_DEFAULT_PRECIS.0 as u32,
                CLIP_DEFAULT_PRECIS.0 as u32,
                CLEARTYPE_QUALITY.0 as u32,
                0,
                face,
            )
        };
        Fonts {
            title: f(13.0, 600, w!("Segoe UI")),
            body: f(12.5, 400, w!("Segoe UI")),
            small: f(11.0, 400, w!("Segoe UI")),
            big: f(20.0, 600, w!("Segoe UI")),
            // Segoe MDL2 Assets ships with Windows 10 and 11.
            icon: f(11.0, 400, w!("Segoe MDL2 Assets")),
        }
    }
}

impl Drop for Fonts {
    fn drop(&mut self) {
        unsafe {
            for f in [self.title, self.body, self.small, self.big, self.icon] {
                let _ = DeleteObject(f);
            }
        }
    }
}

// MDL2 glyphs.
const G_REFRESH: &str = "\u{E72C}";
const G_SETTINGS: &str = "\u{E713}";
const G_CLOSE: &str = "\u{E711}";
const G_SPEAKERS: &str = "\u{E7F5}";
const G_BLUETOOTH: &str = "\u{E702}";

// ── Painting ─────────────────────────────────────────────────────────────────────

/// Everything the gadget shows.
pub struct View<'a> {
    pub version: &'a str,
    pub status: &'a str,
    /// Show `status` in the bottom line instead of "Playing" (it just changed).
    pub show_status: bool,
    pub priority: Option<u8>,
    pub volume_n: i16,
    pub volume_known: bool,
    pub playing: Option<&'a str>,
    /// This PC's audio link was dropped by the PcAudio button.
    pub pc_audio_off: bool,
    /// Music and calls both down: Disconnect reads "Connect".
    pub pc_disconnected: bool,
    pub hover: Option<Hit>,
    pub pressed: Option<Hit>,
}

/// Start GDI+ once per process; returns the token (kept for the process lifetime).
pub fn gdiplus_start() -> usize {
    let mut token = 0usize;
    let input = GdiplusStartupInput { GdiplusVersion: 1, ..Default::default() };
    unsafe {
        let _ = GdiplusStartup(&mut token, &input, std::ptr::null_mut());
    }
    token
}

/// Paint the whole gadget into `hdc` (double-buffered).
pub fn paint(hdc: HDC, client: RECT, scale: f32, t: &Theme, fonts: &Fonts, v: &View) {
    unsafe {
        let (w_, h_) = (client.right - client.left, client.bottom - client.top);
        let mem = CreateCompatibleDC(hdc);
        let bmp = CreateCompatibleBitmap(hdc, w_, h_);
        let old = SelectObject(mem, bmp);

        // Background.
        let bg = CreateSolidBrush(colorref(t.bg));
        FillRect(mem, &client, bg);
        let _ = DeleteObject(bg);

        // ── Shapes (GDI+, anti-aliased) ──
        let mut g: *mut GpGraphics = std::ptr::null_mut();
        GdipCreateFromHDC(mem, &mut g);
        GdipSetSmoothingMode(g, SmoothingModeAntiAlias);
        let s = |rc: RECT| {
            (
                rc.left as f32 * scale,
                rc.top as f32 * scale,
                (rc.right - rc.left) as f32 * scale,
                (rc.bottom - rc.top) as f32 * scale,
            )
        };
        let state = |hit: Hit| -> (bool, bool) { (v.hover == Some(hit), v.pressed == Some(hit)) };

        // ⚙ / ✕: soft circle on hover/press.
        for (hit, rc) in [(Hit::Settings, settings_rect()), (Hit::Close, close_rect())] {
            let (hov, prs) = state(hit);
            if hov || prs {
                let (x, y, w, h) = s(rc);
                fill_round(g, x, y, w, h, w / 2.0, if prs { t.border } else { t.hover });
            }
        }

        // Every button is the same rounded pill as on the phone: accent when
        // selected / pressed (Unlink: while this PC is cut off), else surface.
        let pill = |hit: Hit, rc: RECT, on: bool| {
            let (hov, prs) = state(hit);
            let (x, y, w, h) = s(rc);
            let c = if on || prs { t.accent } else if hov { t.hover } else { t.surface };
            fill_round(g, x, y, w, h, 8.0 * scale, c);
            if !t.dark && c == t.surface {
                stroke_round(g, x, y, w, h, 8.0 * scale, t.border, scale);
            }
        };
        for hit in GRID {
            let on = (hit == Hit::PcAudio && v.pc_audio_off) || (hit == Hit::Disconnect && v.pc_disconnected);
            pill(hit, grid_rect(hit), on);
        }
        for i in 0..3u8 {
            pill(Hit::Priority(i), seg_item(i as i32), v.priority == Some(i));
        }
        pill(Hit::VolDown, vol_down_rect(), false);
        pill(Hit::VolUp, vol_up_rect(), false);

        // Volume bar: 12 segments of 5 dB like the phone (click / drag still sets
        // any 0.5-dB level). A segment is lit when the volume reaches it.
        let b = bar_rect();
        let seg_gap = 3;
        let seg_w = (b.right - b.left - (BAR_SEGS - 1) * seg_gap) as f32 / BAR_SEGS as f32;
        for i in 0..BAR_SEGS {
            let seg_n = -3600 + (i + 1) * 300;
            let lit = v.volume_known && v.volume_n as i32 >= seg_n - 150;
            let x = (b.left as f32 + i as f32 * (seg_w + seg_gap as f32)) * scale;
            let (y, w, h) = (b.top as f32 * scale, seg_w * scale, BAR_H as f32 * scale);
            fill_round(g, x, y, w, h, 4.0 * scale, if lit { t.accent } else { t.border });
        }
        GdipDeleteGraphics(g);

        // ── Text (GDI, ClearType) ──
        SetBkMode(mem, TRANSPARENT);
        let left = DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX;
        let center = DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX;
        let sc = |rc: RECT| RECT {
            left: (rc.left as f32 * scale) as i32,
            top: (rc.top as f32 * scale) as i32,
            right: (rc.right as f32 * scale) as i32,
            bottom: (rc.bottom as f32 * scale) as i32,
        };

        // Two-line title: "QDLX ⚙ ✕" / "5K  v…" (the version as small as the bottom line).
        text(mem, fonts.title, t.text, sc(r(PAD, TOP, 40, LINE_H)), "QDLX", left);
        text(mem, fonts.icon, t.muted, sc(settings_rect()), G_SETTINGS, center);
        text(mem, fonts.icon, t.muted, sc(close_rect()), G_CLOSE, center);
        text(mem, fonts.small, t.text, sc(r(PAD, TOP + LINE_H, XB_LEFT - PAD - 4, LINE_H)), &format!("5K  v{}", v.version), left);

        let ink = |hit: Hit, on: bool| if on || v.pressed == Some(hit) { t.on_accent } else { t.text };
        text(mem, fonts.icon, ink(Hit::Output, false), sc(grid_rect(Hit::Output)), G_SPEAKERS, center);
        text(mem, fonts.icon, ink(Hit::Refresh, false), sc(grid_rect(Hit::Refresh)), G_REFRESH, center);
        let label = if v.pc_audio_off { "Link" } else { "Unlink" };
        text(mem, fonts.small, ink(Hit::PcAudio, v.pc_audio_off), sc(grid_rect(Hit::PcAudio)), label, center);
        let label = if v.pc_disconnected { "Connect" } else { "Disconnect" };
        text(mem, fonts.small, ink(Hit::Disconnect, v.pc_disconnected), sc(grid_rect(Hit::Disconnect)), label, center);

        for i in 0..3u8 {
            let c = ink(Hit::Priority(i), v.priority == Some(i));
            let rc = seg_item(i as i32);
            if i == 0 {
                text(mem, fonts.body, c, sc(rc), "USB", center);
            } else {
                // Bluetooth mark + slot number, centred as a pair (like the phone).
                let cx = (rc.left + rc.right) / 2;
                let h = rc.bottom - rc.top;
                text(mem, fonts.icon, c, sc(r(cx - 12, rc.top, 12, h)), G_BLUETOOTH, center);
                text(mem, fonts.body, c, sc(r(cx + 1, rc.top, 10, h)), if i == 1 { "1" } else { "2" }, center);
            }
        }

        // −, +, and the dB value in the same font as the priority buttons (like the phone).
        for (hit, rc, sign) in [(Hit::VolDown, vol_down_rect(), "\u{2212}"), (Hit::VolUp, vol_up_rect(), "+")] {
            text(mem, fonts.body, ink(hit, false), sc(rc), sign, center);
        }
        let db = if v.volume_known { format!("{:+.1} dB", proto::n_to_db(v.volume_n)) } else { "— dB".into() };
        let mid = vol_down_rect().right;
        text(mem, fonts.body, t.text, sc(r(mid, VOL_Y, vol_up_rect().left - mid, ROW_H)), &db, center);

        // Bottom line: the status while something happens (and a few seconds after),
        // otherwise what is playing — one line, like the phone.
        let bottom = match (v.show_status, v.playing) {
            (true, _) => v.status.to_string(),
            (false, Some(p)) => format!("Playing: {p}"),
            (false, None) => "Playing: ?".to_string(),
        };
        text(mem, fonts.small, t.muted, sc(r(PAD, FOOT_Y, WIDTH - 2 * PAD, FOOT_H)), &bottom, left);

        let _ = BitBlt(hdc, 0, 0, w_, h_, mem, 0, 0, SRCCOPY);
        SelectObject(mem, old);
        let _ = DeleteObject(bmp);
        let _ = DeleteDC(mem);
    }
}

unsafe fn text(hdc: HDC, font: HFONT, color: u32, mut rc: RECT, s: &str, fmt: DRAW_TEXT_FORMAT) {
    // An empty slice hands DrawTextW a dangling pointer with length 0 — it
    // crashes on that (seen: status line empty at start). Nothing to draw anyway.
    if s.is_empty() {
        return;
    }
    let old = SelectObject(hdc, font);
    SetTextColor(hdc, colorref(color));
    let mut wide: Vec<u16> = s.encode_utf16().collect();
    DrawTextW(hdc, &mut wide, &mut rc, fmt);
    SelectObject(hdc, old);
}

unsafe fn round_path(x: f32, y: f32, w: f32, h: f32, rad: f32) -> *mut GpPath {
    let mut p: *mut GpPath = std::ptr::null_mut();
    GdipCreatePath(FillModeAlternate, &mut p);
    let d = (rad * 2.0).min(w).min(h);
    GdipAddPathArc(p, x, y, d, d, 180.0, 90.0);
    GdipAddPathArc(p, x + w - d, y, d, d, 270.0, 90.0);
    GdipAddPathArc(p, x + w - d, y + h - d, d, d, 0.0, 90.0);
    GdipAddPathArc(p, x, y + h - d, d, d, 90.0, 90.0);
    GdipClosePathFigure(p);
    p
}

unsafe fn fill_round(g: *mut GpGraphics, x: f32, y: f32, w: f32, h: f32, rad: f32, argb: u32) {
    let p = round_path(x, y, w, h, rad);
    let mut b: *mut GpSolidFill = std::ptr::null_mut();
    GdipCreateSolidFill(argb, &mut b);
    GdipFillPath(g, b as *mut GpBrush, p);
    GdipDeleteBrush(b as *mut GpBrush);
    GdipDeletePath(p);
}

unsafe fn stroke_round(g: *mut GpGraphics, x: f32, y: f32, w: f32, h: f32, rad: f32, argb: u32, scale: f32) {
    let half = 0.5 * scale;
    let p = round_path(x + half, y + half, w - 2.0 * half, h - 2.0 * half, rad);
    let mut pen: *mut GpPen = std::ptr::null_mut();
    GdipCreatePen1(argb, scale, UnitPixel, &mut pen);
    GdipDrawPath(g, pen, p);
    GdipDeletePen(pen);
    GdipDeletePath(p);
}

/// Layout checks (Windows target only — the UI is Win32): every element sits inside
/// the gadget, none overlap, and a click on an element's centre hits that element.
/// Run from WSL: `cargo test --target x86_64-pc-windows-gnu --features ble` (the
/// test .exe runs through WSL interop).
#[cfg(test)]
mod tests {
    use super::*;

    fn elements() -> Vec<(Hit, RECT)> {
        let mut v = vec![(Hit::Settings, settings_rect()), (Hit::Close, close_rect())];
        v.extend(GRID.iter().map(|&h| (h, grid_rect(h))));
        v.extend((0..3).map(|i| (Hit::Priority(i as u8), seg_item(i))));
        v.push((Hit::VolDown, vol_down_rect()));
        v.push((Hit::VolUp, vol_up_rect()));
        v.push((Hit::Bar, bar_rect()));
        v
    }

    #[test]
    fn every_element_inside_the_gadget() {
        for (hit, rc) in elements() {
            assert!(rc.left >= 0 && rc.top >= 0 && rc.right <= WIDTH && rc.bottom <= HEIGHT, "{hit:?} {rc:?}");
        }
        // The bottom line too.
        assert!(FOOT_Y + FOOT_H <= HEIGHT);
    }

    #[test]
    fn no_elements_overlap() {
        let els = elements();
        for (i, (a, ra)) in els.iter().enumerate() {
            for (b, rb) in &els[i + 1..] {
                let apart = ra.right <= rb.left || rb.right <= ra.left || ra.bottom <= rb.top || rb.bottom <= ra.top;
                assert!(apart, "{a:?} {ra:?} overlaps {b:?} {rb:?}");
            }
        }
    }

    #[test]
    fn clicking_an_elements_centre_hits_it() {
        for scale in [1.0f32, 1.5, 2.0] {
            for (hit, rc) in elements() {
                let (cx, cy) = ((rc.left + rc.right) / 2, (rc.top + rc.bottom) / 2);
                let (px, py) = ((cx as f32 * scale) as i32, (cy as f32 * scale) as i32);
                assert_eq!(hit_test(px, py, scale), Some(hit), "scale {scale}");
            }
        }
    }
}
