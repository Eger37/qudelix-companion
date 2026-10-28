package com.qudelixwidget.app;

import android.bluetooth.BluetoothDevice;
import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.Intent;

/**
 * Android's own Bluetooth link changes (ACL, A2DP, hands-free) for the 5K: re-read what
 * is live and redraw the Unlink/Link and Disconnect/Connect labels — so a connect made in
 * Android settings shows up too, like the PC widget's ~3 s check. Exported only because
 * these are system broadcasts (protected: no other app can send them).
 */
public class BtStateReceiver extends BroadcastReceiver {

    @Override
    public void onReceive(Context context, Intent intent) {
        if (!RfcommService.hasBtPermission(context)) return;
        BluetoothDevice dev = intent.getParcelableExtra(BluetoothDevice.EXTRA_DEVICE);
        BluetoothDevice target;
        try {
            target = PhoneLink.target(context);
        } catch (Exception e) {
            return;
        }
        if (dev == null || target == null || !dev.getAddress().equals(target.getAddress())) return;
        final PendingResult done = goAsync();
        new Thread(() -> {
            try {
                Thread.sleep(800); // the profiles settle just after the ACL event
                PhoneLink.show(context, PhoneLink.read(context, target));
            } catch (Exception e) {
                Dbg.log(context, "link state: " + e);
            } finally {
                done.finish();
            }
        }, "qudelix-link").start();
    }
}
