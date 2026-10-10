package com.qudelixwidget.app;

import android.app.Application;

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
            Thread up = new Thread(() -> RemoteLog.post("[" + RemoteLog.tag(this) + "] CRASH\n" + sw, 2500));
            up.start();
            try { up.join(3000); } catch (InterruptedException ignored) {}
            if (prev != null) prev.uncaughtException(t, e);
        });
        Dbg.log(this, "app start (process " + android.os.Process.myPid() + ")");
        String crash = readCrash(this);
        if (crash != null) Dbg.log(this, "previous crash:\n" + crash);
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

    static String readCrash(android.content.Context c) {
        File f = new File(c.getFilesDir(), "crash.txt");
        if (!f.exists()) return null;
        try {
            byte[] b = new byte[(int) f.length()];
            try (java.io.FileInputStream in = new java.io.FileInputStream(f)) { in.read(b); }
            return new String(b);
        } catch (Exception e) { return null; }
    }
}
