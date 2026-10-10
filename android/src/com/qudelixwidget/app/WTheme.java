package com.qudelixwidget.app;

import android.content.Context;
import android.content.res.Configuration;

/**
 * Light = red & light grey, dark = red & black. "system" follows the phone's dark mode.
 * For the home-screen widget the "auto" resource set (values / values-night) lets
 * the launcher pick light or dark itself; forced themes use the explicit sets.
 */
public final class WTheme {
    private WTheme() {}

    public static final String SYSTEM = "system", LIGHT = "light", DARK = "dark", CUSTOM = "custom";
    public static final int RED = 0xFFE53935;

    /** Resource set for the widget: 'a' (auto), 'l' or 'd'. */
    static char widgetVariant(Context c) {
        switch (Prefs.getTheme(c)) {
            case LIGHT: return 'l';
            case DARK: return 'd';
            case CUSTOM: return 'c';
            default: return 'a';
        }
    }

    /** Whether in-app screens (settings, volume popup) should be dark right now. */
    public static boolean isDark(Context c) {
        switch (Prefs.getTheme(c)) {
            case LIGHT: return false;
            case DARK: return true;
            case CUSTOM: return luminance(Prefs.getCustomBg(c)) < 0.5;
            default:
                return (c.getResources().getConfiguration().uiMode & Configuration.UI_MODE_NIGHT_MASK)
                        == Configuration.UI_MODE_NIGHT_YES;
        }
    }

    // Palette for in-app screens (same values as res/values*/colors.xml); the
    // custom theme derives everything from its background so text stays readable.
    private static boolean custom(Context c) { return CUSTOM.equals(Prefs.getTheme(c)); }
    public static int bg(Context c) {
        return custom(c) ? Prefs.getCustomBg(c) : isDark(c) ? 0xFF0B0B0C : 0xFFE4E4E7;
    }
    public static int text(Context c)    { return isDark(c) ? 0xFFF2F2F2 : 0xFF111111; }
    public static int surface(Context c) { return custom(c) ? mix(bg(c), text(c), 0.07f) : isDark(c) ? 0xFF1A1A1C : 0xFFF4F4F6; }
    public static int border(Context c)  { return custom(c) ? mix(bg(c), text(c), 0.16f) : isDark(c) ? 0xFF2E2E33 : 0xFFCFCFD4; }
    public static int muted(Context c)   { return custom(c) ? mix(bg(c), text(c), 0.60f) : isDark(c) ? 0xFF9A9A9E : 0xFF5E5E63; }
    /** The accent belongs to the theme: presets are red, Custom uses the user's pick. */
    public static int accent(Context c)  { return custom(c) ? Prefs.getAccent(c) : RED; }
    /** Text on an accent fill: dark on very light accents. */
    public static int onAccent(Context c) { return luminance(accent(c)) > 0.6 ? 0xFF111111 : 0xFFFFFFFF; }

    static double luminance(int argb) {
        return (0.2126 * ((argb >> 16) & 0xFF) + 0.7152 * ((argb >> 8) & 0xFF) + 0.0722 * (argb & 0xFF)) / 255.0;
    }

    static int mix(int a, int b, float t) {
        int r = 0xFF000000;
        for (int sh = 0; sh <= 16; sh += 8) {
            int x = (a >> sh) & 0xFF, y = (b >> sh) & 0xFF;
            r |= Math.round(x + (y - x) * t) << sh;
        }
        return r;
    }
}
