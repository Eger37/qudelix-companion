//! Tiny on-disk store for the widget's window position and settings.
//!
//! Format is a trivial `key=value` text file at
//! `%APPDATA%\qudelix-widget\widget.ini` (falls back to the system temp dir).
//! Kept deliberately dependency-free.

use std::path::PathBuf;

/// Saved top-left position of the widget, in parent (desktop) coordinates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pos {
    pub x: i32,
    pub y: i32,
}

fn config_path() -> PathBuf {
    let dir = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("qudelix-widget");
    let _ = std::fs::create_dir_all(&dir);
    dir.join("widget.ini")
}

fn read_map() -> std::collections::BTreeMap<String, String> {
    std::fs::read_to_string(config_path())
        .unwrap_or_default()
        .lines()
        .filter_map(|l| l.split_once('='))
        .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        .collect()
}

/// Update some keys, keeping the rest (best-effort; errors are ignored — losing
/// a setting is not worth bothering the user about).
fn write_keys(kv: &[(&str, String)]) {
    let mut m = read_map();
    for (k, v) in kv {
        m.insert(k.to_string(), v.clone());
    }
    let text: String = m.iter().map(|(k, v)| format!("{k}={v}\n")).collect();
    let _ = std::fs::write(config_path(), text);
}

/// Load the saved position, if any.
pub fn load_pos() -> Option<Pos> {
    let m = read_map();
    Some(Pos { x: m.get("x")?.parse().ok()?, y: m.get("y")?.parse().ok()? })
}

pub fn save_pos(pos: Pos) {
    write_keys(&[("x", pos.x.to_string()), ("y", pos.y.to_string())]);
}

/// One-shot flags (e.g. first-run defaults already applied).
pub fn flag_set(key: &str) -> bool {
    read_map().get(key).is_some_and(|v| v == "1")
}

pub fn set_flag(key: &str) {
    write_keys(&[(key, "1".to_string())]);
}

/// A plain string setting; None if never saved.
pub fn get(key: &str) -> Option<String> {
    read_map().get(key).cloned()
}

pub fn set(key: &str, value: &str) {
    write_keys(&[(key, value.to_string())]);
}

/// Device picked by name in the settings menu; None = auto (first "Qudelix-5K…").
pub fn load_device() -> Option<String> {
    read_map().get("device").filter(|v| !v.is_empty()).cloned()
}

pub fn save_device(name: Option<&str>) {
    write_keys(&[("device", name.unwrap_or("").to_string())]);
}

/// A theme is background AND accent together. Light = red & grey, dark = red &
/// black, System = light/dark like Windows (red), WindowsColors = like Windows
/// incl. its accent colour, Custom = user-picked background + accent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeMode {
    System,
    WindowsColors,
    Light,
    Dark,
    /// User-picked background (`Settings::custom_bg`); text/surfaces derived from it.
    Custom,
}

/// Accent of the Custom theme (the presets fix their own accent).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Accent {
    Red,
    Windows,
    /// User-picked colour, 0xRRGGBB.
    Custom(u32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    pub theme: ThemeMode,
    pub accent: Accent,
    /// Volume −/+ step, n = dB*60 (30 = 0.5 dB, 60 = 1 dB, 120 = 2 dB).
    pub step_n: i16,
    /// Background for ThemeMode::Custom, 0xRRGGBB.
    pub custom_bg: u32,
    /// Check GitHub Releases for a new version (at start + every 6 h).
    pub auto_update: bool,
    /// Background of the app icon (.exe files), 0xRRGGBB. See `logo.rs`.
    pub logo_bg: u32,
}

pub fn load_settings() -> Settings {
    let m = read_map();
    Settings {
        theme: match m.get("theme").map(String::as_str) {
            Some("light") => ThemeMode::Light,
            Some("dark") => ThemeMode::Dark,
            Some("custom") => ThemeMode::Custom,
            Some("windows") => ThemeMode::WindowsColors,
            // ≤1.3.0 stored the Windows accent separately.
            _ if m.get("accent").map(String::as_str) == Some("windows") => ThemeMode::WindowsColors,
            _ => ThemeMode::System,
        },
        accent: match m.get("accent").map(String::as_str) {
            Some("windows") => Accent::Windows,
            Some(hex) if hex.starts_with('#') => {
                u32::from_str_radix(&hex[1..], 16).map(Accent::Custom).unwrap_or(Accent::Red)
            }
            _ => Accent::Red,
        },
        custom_bg: m
            .get("custom_bg")
            .and_then(|v| u32::from_str_radix(v.trim_start_matches('#'), 16).ok())
            .unwrap_or(0x1B_1F3B),
        auto_update: m.get("auto_update").map(String::as_str) != Some("0"),
        logo_bg: m
            .get("logo_bg")
            .and_then(|v| u32::from_str_radix(v.trim_start_matches('#'), 16).ok())
            .unwrap_or(0xFF_FFFF),
        step_n: m.get("step_n").and_then(|v| v.parse().ok()).filter(|n| [30, 60, 120].contains(n)).unwrap_or(60),
    }
}

pub fn save_settings(s: &Settings) {
    let theme = match s.theme {
        ThemeMode::System => "system",
        ThemeMode::WindowsColors => "windows",
        ThemeMode::Light => "light",
        ThemeMode::Dark => "dark",
        ThemeMode::Custom => "custom",
    };
    let accent = match s.accent {
        Accent::Red => "red".to_string(),
        Accent::Windows => "windows".to_string(),
        Accent::Custom(rgb) => format!("#{rgb:06X}"),
    };
    write_keys(&[
        ("theme", theme.into()),
        ("accent", accent),
        ("step_n", s.step_n.to_string()),
        ("custom_bg", format!("#{:06X}", s.custom_bg)),
        ("auto_update", if s.auto_update { "1" } else { "0" }.to_string()),
        ("logo_bg", format!("#{:06X}", s.logo_bg)),
    ]);
}

/// Which exe file + colour the icon was last written for (see `logo::ensure`).
pub fn load_logo_stamp() -> Option<String> {
    read_map().get("logo_stamp").filter(|v| !v.is_empty()).cloned()
}

pub fn save_logo_stamp(stamp: Option<&str>) {
    write_keys(&[("logo_stamp", stamp.unwrap_or("").to_string())]);
}
