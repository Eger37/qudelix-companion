package com.qudelixwidget.app;

import android.app.Application;
import android.app.NotificationChannel;
import android.app.NotificationManager;

import java.io.File;
import java.io.PrintWriter;

/** Writes any uncaught crash to filesDir/crash.txt so it can be inspected without adb. */
public class App extends Application {
    @Override
    public void onCreate() {
        super.onCreate();
        final Thread.UncaughtExceptionHandler prev = Thread.getDefaultUncaughtExceptionHandler();
        Thread.setDefaultUncaughtExceptionHandler((t, e) -> {
            java.io.StringWriter sw = new java.io.StringWriter();
            try (PrintWriter w = new PrintWriter(sw)) {
                w.println("thread: " + t.getName());
                e.printStackTrace(w);
            }
            try (PrintWriter w = new PrintWriter(new File(getFilesDir(), "crash.txt"))) {
                w.print(sw);
            } catch (Exception ignored) {}
            // Best effort: get the crash to the PC before the process dies.
            Thread up = new Thread(() -> RemoteLog.post(this, "[" + RemoteLog.tag(this) + "] CRASH\n" + sw, 2500));
            up.start();
            try { up.join(3000); } catch (InterruptedException ignored) {}
            if (prev != null) prev.uncaughtException(t, e);
        });
        Prefs.seedDebug(this);
        Dbg.log(this, "app start (process " + android.os.Process.myPid() + ")");
        // Report each crash once (it stays readable in the settings for CRASH_KEEP_MS).
        String crash = readCrash(this);
        long crashTime = new File(getFilesDir(), "crash.txt").lastModified();
        if (crash != null && getSharedPreferences("qudelix", MODE_PRIVATE).getLong("crashReported", 0) != crashTime) {
            Dbg.log(this, "previous crash:\n" + crash);
            getSharedPreferences("qudelix", MODE_PRIVATE).edit().putLong("crashReported", crashTime).apply();
        }
        // Nothing optional may take the app down at start (that kills the widget too).
        try {
            UpdateJob.schedule(this);
        } catch (Exception e) {
            Dbg.log(this, "update job not scheduled: " + e);
        }
        try {
            QudelixWidget.update(this); // any app start redraws the widget (version, last state)
        } catch (Exception e) {
            Dbg.log(this, "widget redraw failed: " + e);
        }
    }

    /** A crash report older than this is deleted instead of shown again. */
    private static final long CRASH_KEEP_MS = 3L * 24 * 3600 * 1000;

    static String readCrash(android.content.Context c) {
        File f = new File(c.getFilesDir(), "crash.txt");
        if (!f.exists()) return null;
        if (System.currentTimeMillis() - f.lastModified() > CRASH_KEEP_MS) {
            f.delete();
            return null;
        }
        return Dbg.readText(f);
    }

    /** Create a notification channel once (an existing one keeps the user's settings). */
    static void ensureChannel(NotificationManager nm, String id, String name, int importance, boolean badge) {
        if (nm == null || nm.getNotificationChannel(id) != null) return;
        NotificationChannel ch = new NotificationChannel(id, name, importance);
        ch.setShowBadge(badge);
        nm.createNotificationChannel(ch);
    }
}
