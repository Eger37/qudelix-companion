package com.qudelixwidget.app;

import android.content.Context;
import android.content.SharedPreferences;

/** Widget state: last known device state (read back from the 5K) + status line. */
public final class Prefs {
    private Prefs() {}
    private static final String FILE = "qudelix";

    private static SharedPreferences p(Context c) {
        return c.getSharedPreferences(FILE, Context.MODE_PRIVATE);
    }

    /** Source-priority preference: 0 USB / 1 A2DP#1 / 2 A2DP#2, -1 unknown. */
    public static int getPriority(Context c) { return p(c).getInt("priority", -1); }
    public static void setPriority(Context c, int v) { p(c).edit().putInt("priority", v).apply(); }

    /** Main volume, n = dB*60. */
    public static int getVolumeN(Context c) { return p(c).getInt("volN", -1200); }
    public static void setVolumeN(Context c, int n) { p(c).edit().putInt("volN", n).apply(); }
    public static boolean hasVolume(Context c) { return p(c).contains("volN"); }

    /** What is actually playing right now — NOT the priority. */
    public static String getPlaying(Context c) { return p(c).getString("playing", null); }
    public static void setPlaying(Context c, String s) { p(c).edit().putString("playing", s).apply(); }

    /** Theme: WTheme.SYSTEM / LIGHT (red & grey) / DARK (red & black). */
    public static String getTheme(Context c) { return p(c).getString("theme", WTheme.SYSTEM); }
    public static void setTheme(Context c, String t) { p(c).edit().putString("theme", t).apply(); }

    /** Custom theme background and accent, 0xAARRGGBB. */
    public static int getCustomBg(Context c) { return p(c).getInt("customBg", 0xFF1B1F3B); }
    public static void setCustomBg(Context c, int argb) { p(c).edit().putInt("customBg", argb).apply(); }
    public static int getAccent(Context c) { return p(c).getInt("accent", WTheme.RED); }
    public static void setAccent(Context c, int argb) { p(c).edit().putInt("accent", argb).apply(); }

    /** Launcher icon background, 0xAARRGGBB (one of Logo.COLORS). */
    public static int getLogo(Context c) { return p(c).getInt("logo", 0xFFFFFFFF); }
    public static void setLogo(Context c, int argb) { p(c).edit().putInt("logo", argb).apply(); }

    /** Volume −/+ step, n = dB*60: 30 (0.5 dB), 60 (1 dB) or 120 (2 dB). */
    public static int getStepN(Context c) { return p(c).getInt("stepN", 60); }
    public static void setStepN(Context c, int n) { p(c).edit().putInt("stepN", n).apply(); }

    /** Bluetooth address of the device to control; "" = auto (first "Qudelix-5K…"). */
    public static String getDevice(Context c) { return p(c).getString("device", ""); }
    public static void setDevice(Context c, String addr) { p(c).edit().putString("device", addr).apply(); }

    /** Debug over Tailscale: send the log to the PC widget and also take test builds from it. */
    public static boolean getDebugOn(Context c) { return p(c).getBoolean("debugOn", false); }
    public static void setDebugOn(Context c, boolean on) { p(c).edit().putBoolean("debugOn", on).apply(); }
    /** The PC's Tailscale MagicDNS name, e.g. "my-pc.tailnet-name.ts.net"; "" = not set. */
    public static String getDebugHost(Context c) { return p(c).getString("debugHost", ""); }
    public static void setDebugHost(Context c, String h) { p(c).edit().putString("debugHost", h.trim()).apply(); }
    /** The PC widget's key (⚙ dialog on the PC), sent with every request to it; "" = not set. */
    public static String getDebugKey(Context c) { return p(c).getString("debugKey", ""); }
    public static void setDebugKey(Context c, String k) { p(c).edit().putString("debugKey", k.trim()).apply(); }

    /** Builds made with UPDATE_HOST / DEBUG_KEY (the owner's tailnet builds) switch debug on
     *  once, so the setting survives a later update to the public GitHub build. */
    static void seedDebug(Context c) {
        SharedPreferences.Editor e = p(c).edit();
        if (!BuildInfo.UPDATE_HOST.isEmpty() && !p(c).contains("debugHost")) {
            e.putString("debugHost", BuildInfo.UPDATE_HOST).putBoolean("debugOn", true);
        }
        if (!BuildInfo.DEBUG_KEY.isEmpty() && !p(c).contains("debugKey")) {
            e.putString("debugKey", BuildInfo.DEBUG_KEY);
        }
        e.apply();
    }

    public static String getStatus(Context c) { return p(c).getString("status", "tap ↻"); }
    public static void setStatus(Context c, String s) { p(c).edit().putString("status", s).apply(); }
}
