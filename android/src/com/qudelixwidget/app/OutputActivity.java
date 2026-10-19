package com.qudelixwidget.app;

import android.app.Activity;
import android.content.Intent;
import android.media.MediaRouter2;
import android.os.Build;
import android.os.Bundle;
import android.provider.Settings;

/**
 * Widget's 🔊 button: opens Android's own "play media on" output picker. Apps can't
 * switch the phone's sound output themselves; this is the one-tap way there.
 * Invisible: it only exists to launch the picker from a widget tap, then finishes.
 */
public class OutputActivity extends Activity {

    @Override
    protected void onCreate(Bundle b) {
        super.onCreate(b);
        boolean shown = false;
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
            try {
                shown = MediaRouter2.getInstance(this).showSystemOutputSwitcher();
            } catch (Exception e) {
                Dbg.log(this, "output switcher failed: " + e);
            }
        }
        Dbg.log(this, "output picker " + (shown ? "shown" : "unavailable → Bluetooth settings"));
        if (!shown) {
            startActivity(new Intent(Settings.ACTION_BLUETOOTH_SETTINGS));
        }
        finish();
    }
}
