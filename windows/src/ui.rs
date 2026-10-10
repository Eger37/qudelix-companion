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
pub const HEIGHT: i32 = 184;

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

const PAD: i32 = 12;
const ICON: i32 = 24;
const SEG_Y: i32 = 58;
const SEG_H: i32 = 34;
const VOL_Y: i32 = 104;
const VOL_BTN: i32 = 36;

fn r(x: i32, y: i32, w: i32, h: i32) -> RECT {
    RECT { left: x, top: y, right: x + w, bottom: y + h }
}

fn close_rect() -> RECT {
    r(WIDTH - PAD - ICON + 4, 8, ICON, ICON)
}
fn refresh_rect() -> RECT {
    r(WIDTH - PAD - 2 * ICON + 2, 8, ICON, ICON)
}
fn settings_rect() -> RECT {
    r(WIDTH - PAD - 3 * ICON, 8, ICON, ICON)
}
fn seg_rect() -> RECT {
    r(PAD, SEG_Y, WIDTH - 2 * PAD, SEG_H)
}
fn seg_item(i: i32) -> RECT {
    let s = seg_rect();
    let w = (s.right - s.left) / 3;
    r(s.left + i * w, s.top, if i == 2 { s.right - s.left - 2 * w } else { w }, SEG_H)
}
fn vol_down_rect() -> RECT {
    r(PAD, VOL_Y, VOL_BTN, VOL_BTN)
}
fn vol_up_rect() -> RECT {
    r(WIDTH - PAD - VOL_BTN, VOL_Y, VOL_BTN, VOL_BTN)
}

fn bar_rect() -> RECT {
    r(PAD + VOL_BTN + 12, VOL_Y + 30, WIDTH - 2 * (PAD + VOL_BTN + 12), 4)
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
    if inside(&refresh_rect(), x, y) {
        return Some(Hit::Refresh);
    }
    if inside(&settings_rect(), x, y) {
        return Some(Hit::Settings);
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
    // Generous vertical grab area around the 4-px bar.
    let b = bar_rect();
    if inside(&RECT { top: b.top - 8, bottom: b.bottom + 8, ..b }, x, y) {
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
const G_MINUS: &str = "\u{E738}";
const G_PLUS: &str = "\u{E710}";

// ── Painting ─────────────────────────────────────────────────────────────────────

/// Everything the gadget shows.
pub struct View<'a> {
    pub version: &'a str,
    pub status: &'a str,
    pub priority: Option<u8>,
    pub volume_n: i16,
    pub volume_known: bool,
    pub playing: Option<&'a str>,
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

        // Header icon buttons: soft circle on hover/press.
        for (hit, rc) in [(Hit::Settings, settings_rect()), (Hit::Refresh, refresh_rect()), (Hit::Close, close_rect())] {
            let (hov, prs) = state(hit);
            if hov || prs {
                let (x, y, w, h) = s(rc);
                fill_round(g, x, y, w, h, w / 2.0, if prs { t.border } else { t.hover });
            }
        }

        // Segmented priority control.
        let (x, y, w, h) = s(seg_rect());
        fill_round(g, x, y, w, h, 9.0 * scale, t.surface);
        if !t.dark {
            stroke_round(g, x, y, w, h, 9.0 * scale, t.border, scale);
        }
        for i in 0..3u8 {
            let (x, y, w, h) = s(seg_item(i as i32));
            let inset = 3.0 * scale;
            let (hov, prs) = state(Hit::Priority(i));
            let color = if v.priority == Some(i) {
                Some(t.accent)
            } else if prs {
                Some(t.border)
            } else if hov {
                Some(t.hover)
            } else {
                None
            };
            if let Some(c) = color {
                fill_round(g, x + inset, y + inset, w - 2.0 * inset, h - 2.0 * inset, 7.0 * scale, c);
            }
        }

        // Volume buttons.
        for (hit, rc) in [(Hit::VolDown, vol_down_rect()), (Hit::VolUp, vol_up_rect())] {
            let (hov, prs) = state(hit);
            let (x, y, w, h) = s(rc);
            let c = if prs { t.accent } else if hov { t.hover } else { t.surface };
            fill_round(g, x, y, w, h, w / 2.0, c);
            if !t.dark && !prs {
                stroke_round(g, x, y, w, h, w / 2.0, t.border, scale);
            }
        }

        // Volume bar (-60…0 dB); a knob appears on hover/drag.
        let (x, y, w, h) = s(bar_rect());
        fill_round(g, x, y, w, h, h / 2.0, t.border);
        if v.volume_known {
            let frac = ((v.volume_n as f32 + 3600.0) / 3600.0).clamp(0.0, 1.0);
            if frac > 0.0 {
                fill_round(g, x, y, (w * frac).max(h), h, h / 2.0, t.accent);
            }
            if v.hover == Some(Hit::Bar) || v.pressed == Some(Hit::Bar) {
                let k = 12.0 * scale;
                fill_round(g, x + w * frac - k / 2.0, y + h / 2.0 - k / 2.0, k, k, k / 2.0, t.accent);
            }
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

        text(mem, fonts.title, t.text, sc(r(PAD, 8, 90, ICON)), "Qudelix 5K", left);
        text(mem, fonts.small, t.muted, sc(r(PAD + 76, 8, 54, ICON)), &format!("v{}", v.version), left);
        text(mem, fonts.icon, t.muted, sc(settings_rect()), G_SETTINGS, center);
        text(mem, fonts.icon, t.muted, sc(refresh_rect()), G_REFRESH, center);
        text(mem, fonts.icon, t.muted, sc(close_rect()), G_CLOSE, center);
        text(mem, fonts.small, t.muted, sc(r(PAD, 34, WIDTH - 2 * PAD, 18)), v.status, left);

        for i in 0..3u8 {
            let label = ["USB", "A2DP 1", "A2DP 2"][i as usize];
            let c = if v.priority == Some(i) { t.on_accent } else { t.text };
            text(mem, fonts.body, c, sc(seg_item(i as i32)), label, center);
        }

        for (hit, rc, glyph) in [(Hit::VolDown, vol_down_rect(), G_MINUS), (Hit::VolUp, vol_up_rect(), G_PLUS)] {
            let c = if v.pressed == Some(hit) { t.on_accent } else { t.text };
            text(mem, fonts.icon, c, sc(rc), glyph, center);
        }
        let db = if v.volume_known { format!("{:+.1} dB", proto::n_to_db(v.volume_n)) } else { "— dB".into() };
        text(mem, fonts.big, t.text, sc(r(PAD + VOL_BTN, VOL_Y - 2, WIDTH - 2 * (PAD + VOL_BTN), 30)), &db, center);

        let playing = match v.playing {
            Some("nothing") => "Nothing playing".to_string(),
            Some(p) => format!("\u{25B6}  {p}"),
            None => "\u{21BB} reads volume, priority and playback".to_string(),
        };
        text(mem, fonts.small, t.muted, sc(r(PAD, 152, WIDTH - 2 * PAD, 20)), &playing, left);

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
