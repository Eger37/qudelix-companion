package com.qudelixwidget.app;

import android.content.ComponentName;
import android.content.Context;
import android.content.pm.PackageManager;

/**
 * Launcher icon background (Settings → Logo background). A launcher icon can't be
 * recoloured at run time, so the manifest has one activity-alias per prebuilt icon
 * (res/drawable/ic_logo_*.xml from tools/make_icon.py) and exactly one is enabled.
 */
public final class Logo {
    private Logo() {}

    /** Same order and colours as PRESETS in tools/make_icon.py. */
    static final String[] NAMES = {"White", "Red", "Black", "Orange", "Amber", "Green", "Blue", "Purple"};
    static final int[] COLORS = {0xFFFFFFFF, 0xFFE53935, 0xFF121214, 0xFFF97316,
            0xFFF59E0B, 0xFF10B981, 0xFF2F6FEB, 0xFF8B5CF6};

    /**
     * Enable the alias for `argb` (one of COLORS) first, then disable the others —
     * so there is never a moment with no launcher entry at all.
     */
    static void apply(Context c, int argb) {
        int pick = -1;
        for (int i = 0; i < COLORS.length; i++) {
            if ((COLORS[i] & 0xFFFFFF) == (argb & 0xFFFFFF)) pick = i;
        }
        if (pick < 0) return; // unknown colour: keep the current icon
        set(c, pick, PackageManager.COMPONENT_ENABLED_STATE_ENABLED);
        for (int i = 0; i < COLORS.length; i++) {
            if (i != pick) set(c, i, PackageManager.COMPONENT_ENABLED_STATE_DISABLED);
        }
        Prefs.setLogo(c, argb);
        Dbg.log(c, "logo background -> " + Integer.toHexString(argb));
    }

    private static void set(Context c, int i, int state) {
        c.getPackageManager().setComponentEnabledSetting(
                new ComponentName(c, c.getPackageName() + ".Logo" + NAMES[i]), state, PackageManager.DONT_KILL_APP);
    }
}
