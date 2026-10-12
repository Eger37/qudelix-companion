package com.qudelixwidget.app;

import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.Intent;
import android.os.Build;

/**
 * Receives the widget's button taps and hands them to {@link RfcommService}.
 * Not exported: only this app's own PendingIntents can reach it, so another app
 * on the phone can't change the priority or volume. (The exported
 * {@link QudelixWidget} provider only takes system widget broadcasts.)
 */
public class TapReceiver extends BroadcastReceiver {

    @Override
    public void onReceive(Context context, Intent intent) {
        String action = intent.getAction();
        if (!RfcommService.ACTION_PRIORITY.equals(action) && !RfcommService.ACTION_VOLUME.equals(action)
                && !RfcommService.ACTION_REFRESH.equals(action)
                && !RfcommService.ACTION_VOLUME_SET.equals(action)) {
            return;
        }
        Dbg.log(context, "widget tap " + action + " value=" + intent.getIntExtra(RfcommService.EXTRA_VALUE, 0));
        if (!RfcommService.hasBtPermission(context)) {
            Dbg.log(context, "no BLUETOOTH_CONNECT permission — not starting service");
            QudelixWidget.pushStatus(context, "open app → grant Bluetooth");
            return;
        }
        Intent svc = new Intent(context, RfcommService.class)
                .setAction(action)
                .putExtra(RfcommService.EXTRA_VALUE, intent.getIntExtra(RfcommService.EXTRA_VALUE, 0));
        try {
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                context.startForegroundService(svc);
            } else {
                context.startService(svc);
            }
        } catch (Exception e) {
            Dbg.log(context, "startForegroundService failed: " + e);
            QudelixWidget.pushStatus(context, "start failed");
        }
    }
}
