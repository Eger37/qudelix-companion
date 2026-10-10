package com.qudelixwidget.app;

import android.app.Notification;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.app.PendingIntent;
import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.Intent;
import android.content.pm.PackageInstaller;

/** Result of a self-update install session. Asks the user only when Android requires it. */
public class InstallReceiver extends BroadcastReceiver {
    private static final String CHANNEL_ID = "qudelix-update";

    @Override
    public void onReceive(Context c, Intent intent) {
        int status = intent.getIntExtra(PackageInstaller.EXTRA_STATUS, PackageInstaller.STATUS_FAILURE);
        Dbg.log(c, "update: install status " + status + " "
                + intent.getStringExtra(PackageInstaller.EXTRA_STATUS_MESSAGE));
        if (status != PackageInstaller.STATUS_PENDING_USER_ACTION) return;

        Intent confirm = intent.getParcelableExtra(Intent.EXTRA_INTENT);
        if (confirm == null) return;
        confirm.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
        // From the background Android may block the activity start — so also
        // post a notification the user can tap to confirm.
        try {
            c.startActivity(confirm);
        } catch (Exception e) {
            Dbg.log(c, "update: confirm activity blocked: " + e);
        }
        NotificationManager nm = (NotificationManager) c.getSystemService(Context.NOTIFICATION_SERVICE);
        if (nm == null) return;
        if (nm.getNotificationChannel(CHANNEL_ID) == null) {
            nm.createNotificationChannel(new NotificationChannel(
                    CHANNEL_ID, "Updates", NotificationManager.IMPORTANCE_DEFAULT));
        }
        PendingIntent pi = PendingIntent.getActivity(c, 0, confirm,
                PendingIntent.FLAG_UPDATE_CURRENT | PendingIntent.FLAG_IMMUTABLE);
        nm.notify(2, new Notification.Builder(c, CHANNEL_ID)
                .setContentTitle("Qudelix Widget update")
                .setContentText("Tap to install the new version")
                .setSmallIcon(R.drawable.ic_widget)
                .setContentIntent(pi)
                .setAutoCancel(true)
                .build());
    }
}
