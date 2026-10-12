//! App icon with a user-chosen background colour (Settings → Logo background).
//!
//! The widget is a tool window (no taskbar button), so the logo is only seen on
//! the .exe files themselves. `tools/make_icon.py` exports the icon as two
//! layers per size (`logo-layers.bin`): the rounded square's coverage and the
//! device on a transparent background. Here they are composited over the chosen
//! colour and written into the icon resources of this exe: a running .exe can
//! be renamed but not rewritten, so it is patched as a copy and swapped in
//! (`self_update::move_aside`, the same swap as an update).

use std::path::{Path, PathBuf};

use windows::core::{HSTRING, PCWSTR};
use windows::Win32::Foundation::BOOL;
use windows::Win32::System::LibraryLoader::{BeginUpdateResourceW, EndUpdateResourceW, UpdateResourceW};
use windows::Win32::UI::Shell::{SHChangeNotify, SHCNE_ASSOCCHANGED, SHCNE_UPDATEITEM, SHCNF_IDLIST, SHCNF_PATHW};

use crate::config;

/// Must match ICO_SIZES in tools/make_icon.py (and the order windres numbered them).
const SIZES: [usize; 8] = [16, 20, 24, 32, 40, 48, 64, 256];
static LAYERS: &[u8] = include_bytes!("../logo-layers.bin");
/// 5 bytes per pixel and size (1 coverage + 4 RGBA). A stale logo-layers.bin
/// fails the build instead of panicking (= aborting) in `images` at run time.
const _: () = {
    let mut n = 0;
    let mut i = 0;
    while i < SIZES.len() {
        n += 5 * SIZES[i] * SIZES[i];
        i += 1;
    }
    assert!(include_bytes!("../logo-layers.bin").len() == n, "logo-layers.bin doesn't match SIZES");
};
/// build.rs embeds icon.ico as group 101; windres numbers its images 1…N.
const GROUP_ID: u16 = 101;
const RT_ICON: u16 = 3;
const RT_GROUP_ICON: u16 = 14;
const LANG_NEUTRAL: u16 = 0;

/// Background presets offered in the menu (same as tools/make_icon.py PRESETS).
pub const PRESETS: [(&str, u32); 8] = [
    ("White", 0xFF_FFFF),
    ("Red", 0xE5_3935),
    ("Black", 0x12_1214),
    ("Orange", 0xF9_7316),
    ("Amber", 0xF5_9E0B),
    ("Green", 0x10_B981),
    ("Blue", 0x2F_6FEB),
    ("Purple", 0x8B_5CF6),
];
pub const DEFAULT_BG: u32 = 0xFF_FFFF;

/// One icon image as a 32-bit DIB (what RT_ICON holds): header, bottom-up BGRA,
/// then an all-zero AND mask (the alpha channel does the masking).
fn dib(size: usize, bgra_top_down: &[u8]) -> Vec<u8> {
    let mask_row = size.div_ceil(32) * 4;
    let mut v = Vec::with_capacity(40 + size * size * 4 + mask_row * size);
    let h = |v: &mut Vec<u8>, x: u32| v.extend_from_slice(&x.to_le_bytes());
    h(&mut v, 40); // biSize
    h(&mut v, size as u32); // biWidth
    h(&mut v, (size * 2) as u32); // biHeight: XOR + AND
    v.extend_from_slice(&1u16.to_le_bytes()); // biPlanes
    v.extend_from_slice(&32u16.to_le_bytes()); // biBitCount
    h(&mut v, 0); // BI_RGB
    h(&mut v, (size * size * 4 + mask_row * size) as u32);
    for _ in 0..4 {
        h(&mut v, 0);
    }
    for row in (0..size).rev() {
        v.extend_from_slice(&bgra_top_down[row * size * 4..(row + 1) * size * 4]);
    }
    v.resize(v.len() + mask_row * size, 0);
    v
}

/// The icon images (size, DIB) with the device over a `bg` (0xRRGGBB) square.
fn images(bg: u32) -> Vec<(usize, Vec<u8>)> {
    let (br, bgc, bb) = ((bg >> 16) as f32, ((bg >> 8) & 0xFF) as f32, (bg & 0xFF) as f32);
    let mut at = 0;
    let mut out = Vec::new();
    for &s in &SIZES {
        let sq = &LAYERS[at..at + s * s];
        let fg = &LAYERS[at + s * s..at + s * s * 5];
        at += s * s * 5;
        let mut px = Vec::with_capacity(s * s * 4);
        for i in 0..s * s {
            let a_sq = sq[i] as f32 / 255.0;
            let a_fg = fg[i * 4 + 3] as f32 / 255.0;
            let a = a_fg + a_sq * (1.0 - a_fg);
            let mix = |f: u8, b: f32| {
                if a <= 0.0 { 0 } else { ((f as f32 * a_fg + b * a_sq * (1.0 - a_fg)) / a).round() as u8 }
            };
            px.extend_from_slice(&[mix(fg[i * 4 + 2], bb), mix(fg[i * 4 + 1], bgc), mix(fg[i * 4], br), (a * 255.0).round() as u8]);
        }
        out.push((s, dib(s, &px)));
    }
    out
}

/// Replace the icon resources of `exe` (must not be running) with `bg`'s icon.
fn patch(exe: &Path, bg: u32) -> Result<(), String> {
    let imgs = images(bg);
    let mut group = Vec::new();
    group.extend_from_slice(&[0, 0, 1, 0]);
    group.extend_from_slice(&(imgs.len() as u16).to_le_bytes());
    for (i, (s, d)) in imgs.iter().enumerate() {
        let dim = if *s >= 256 { 0 } else { *s as u8 };
        group.extend_from_slice(&[dim, dim, 0, 0]);
        group.extend_from_slice(&1u16.to_le_bytes());
        group.extend_from_slice(&32u16.to_le_bytes());
        group.extend_from_slice(&(d.len() as u32).to_le_bytes());
        group.extend_from_slice(&(i as u16 + 1).to_le_bytes());
    }
    let id = |n: u16| PCWSTR(n as usize as *const u16);
    unsafe {
        let h = BeginUpdateResourceW(&HSTRING::from(exe.as_os_str()), BOOL(0)).map_err(|e| format!("open: {e}"))?;
        let mut res = Ok(());
        for (i, (_, d)) in imgs.iter().enumerate() {
            res = res.and(UpdateResourceW(h, id(RT_ICON), id(i as u16 + 1), LANG_NEUTRAL, Some(d.as_ptr().cast()), d.len() as u32));
        }
        res = res.and(UpdateResourceW(h, id(RT_GROUP_ICON), id(GROUP_ID), LANG_NEUTRAL, Some(group.as_ptr().cast()), group.len() as u32));
        if let Err(e) = res {
            let _ = EndUpdateResourceW(h, BOOL(1));
            return Err(format!("write: {e}"));
        }
        EndUpdateResourceW(h, BOOL(0)).map_err(|e| format!("commit: {e}"))
    }
}

/// Patch `exe` even while it runs: patch a copy, then swap it in.
fn patch_in_place(exe: &Path, bg: u32) -> Result<(), String> {
    let tmp = exe.with_extension("exe.logo");
    std::fs::copy(exe, &tmp).map_err(|e| format!("copy: {e}"))?;
    if let Err(e) = patch(&tmp, bg) {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
    if std::fs::rename(&tmp, exe).is_err() {
        // Running: move it aside (allowed for a running exe), then put the copy in its place.
        let old = crate::self_update::move_aside(exe).inspect_err(|_| {
            let _ = std::fs::remove_file(&tmp);
        })?;
        if let Err(e) = std::fs::rename(&tmp, exe) {
            let _ = std::fs::rename(&old, exe);
            return Err(format!("swap: {e}"));
        }
    }
    unsafe { SHChangeNotify(SHCNE_UPDATEITEM, SHCNF_PATHW, Some(HSTRING::from(exe.as_os_str()).as_ptr().cast()), None) };
    Ok(())
}

/// The exe files to recolour: just this one (installed or portable).
fn targets() -> Vec<PathBuf> {
    std::env::current_exe().into_iter().collect()
}

/// Identifies this exe file + colour, so a fresh (white) copy from deploy/update gets recoloured.
fn stamp(bg: u32) -> Option<String> {
    let exe = std::env::current_exe().ok()?;
    let m = std::fs::metadata(&exe).ok()?;
    let t = m.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs();
    Some(format!("{bg:06X}:{}:{t}", m.len()))
}

/// Recolour the .exe icons now (blocking; call off the UI thread).
pub fn apply(bg: u32) {
    let mut ok = true;
    for exe in targets() {
        match patch_in_place(&exe, bg) {
            Ok(()) => crate::log::line(&format!("[logo] {} → #{bg:06X}", exe.display())),
            Err(e) => {
                ok = false;
                crate::log::line(&format!("[logo] {} failed: {e}", exe.display()));
            }
        }
    }
    unsafe { SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, None, None) }; // refresh Explorer's icon cache
    if ok {
        config::save_logo_stamp(stamp(bg).as_deref());
    }
}

/// At start: drop a leftover exe.old and re-apply the colour if this exe is a
/// fresh copy (deploy or self-update put back the default white icon).
pub fn ensure(bg: u32) {
    if let Ok(exe) = std::env::current_exe() {
        crate::self_update::remove_old_copies(&exe);
    }
    let saved = config::load_logo_stamp();
    if bg == DEFAULT_BG && saved.is_none() {
        return; // never recoloured: the embedded white icon is right
    }
    if saved != stamp(bg) {
        std::thread::spawn(move || apply(bg));
    }
}

/// CLI probe `--set-logo RRGGBB`: recolour only this exe (no settings written).
pub fn probe(bg: u32) {
    if let Ok(exe) = std::env::current_exe() {
        match patch_in_place(&exe, bg) {
            Ok(()) => crate::log::line(&format!("[logo] probe {} → #{bg:06X}", exe.display())),
            Err(e) => crate::log::line(&format!("[logo] probe failed: {e}")),
        }
    }
}
