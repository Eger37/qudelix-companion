package com.qudelixwidget.app;

import android.content.Context;
import android.os.Build;

import java.io.OutputStream;
import java.net.HttpURLConnection;
import java.net.URL;
import java.util.ArrayList;
import java.util.List;
import java.util.concurrent.Executors;
import java.util.concurrent.ScheduledExecutorService;
import java.util.concurrent.TimeUnit;

/**
 * Ships the debug log to the PC widget (`POST /log` → %APPDATA%\qudelix-widget\phone.log)
 * over Tailscale, so the phone can be debugged from the PC without adb. Lines are
 * batched for ~1.5 s; on failure they are kept and retried with the next batch.
 * Never calls {@link Dbg} (that would recurse).
 */
public final class RemoteLog {
    private RemoteLog() {}

    private static final int MAX_PENDING = 2000;
    private static final List<String> pending = new ArrayList<>();
    private static final ScheduledExecutorService ex = Executors.newSingleThreadScheduledExecutor(r -> {
        Thread t = new Thread(r, "qudelix-remotelog");
        t.setDaemon(true);
        return t;
    });
    private static boolean scheduled;

    /** Device/app header so each upload is self-describing. */
    static String tag(Context c) {
        return Build.MODEL + " A" + Build.VERSION.RELEASE + " v" + Updater.installedVersionName(c);
    }

    static synchronized void add(Context c, String line) {
        pending.add(line);
        while (pending.size() > MAX_PENDING) pending.remove(0);
        if (!scheduled) {
            scheduled = true;
            final Context app = c.getApplicationContext();
            ex.schedule(() -> flush(app), 1500, TimeUnit.MILLISECONDS);
        }
    }

    private static void flush(Context c) {
        List<String> batch;
        synchronized (RemoteLog.class) {
            scheduled = false;
            if (pending.isEmpty()) return;
            batch = new ArrayList<>(pending);
            pending.clear();
        }
        StringBuilder sb = new StringBuilder();
        for (String l : batch) sb.append("[").append(tag(c)).append("] ").append(l).append('\n');
        if (!post(sb.toString(), 4000)) {
            synchronized (RemoteLog.class) {
                pending.addAll(0, batch);
                while (pending.size() > MAX_PENDING) pending.remove(0);
            }
        }
    }

    /** Blocking upload; true on success. Used directly for crash reports. */
    static boolean post(String text, int timeoutMs) {
        if (!Updater.enabled()) return true; // nowhere to send: drop silently
        try {
            byte[] body = text.getBytes("UTF-8");
            HttpURLConnection h = (HttpURLConnection) new URL(Updater.BASE_URL + "log").openConnection();
            h.setConnectTimeout(timeoutMs);
            h.setReadTimeout(timeoutMs);
            h.setRequestMethod("POST");
            h.setDoOutput(true);
            h.setFixedLengthStreamingMode(body.length);
            h.setRequestProperty("Content-Type", "text/plain; charset=utf-8");
            try (OutputStream out = h.getOutputStream()) {
                out.write(body);
            }
            int code = h.getResponseCode();
            h.disconnect();
            return code / 100 == 2;
        } catch (Exception e) {
            return false;
        }
    }
}
