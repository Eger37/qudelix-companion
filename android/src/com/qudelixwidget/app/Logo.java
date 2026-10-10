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

    /** Enable the alias for `argb` (one of COLORS), disable the others. */
    static void apply(Context c, int argb) {
        PackageManager pm = c.getPackageManager();
        for (int i = 0; i < COLORS.length; i++) {
            boolean on = (COLORS[i] & 0xFFFFFF) == (argb & 0xFFFFFF);
            pm.setComponentEnabledSetting(new ComponentName(c, c.getPackageName() + ".Logo" + NAMES[i]),
                    on ? PackageManager.COMPONENT_ENABLED_STATE_ENABLED : PackageManager.COMPONENT_ENABLED_STATE_DISABLED,
                    PackageManager.DONT_KILL_APP);
        }
        Prefs.setLogo(c, argb);
        Dbg.log(c, "logo background -> " + Integer.toHexString(argb));
    }
}
