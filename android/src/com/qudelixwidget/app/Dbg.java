package com.qudelixwidget.app;

import android.content.Context;

import java.io.File;
import java.io.FileOutputStream;
import java.io.PrintWriter;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.text.SimpleDateFormat;
import java.util.Date;
import java.util.Locale;

/** Append-only debug log (filesDir/log.txt) shown in MainActivity and mirrored to the PC (RemoteLog). */
public final class Dbg {
    private Dbg() {}
    private static final SimpleDateFormat TS = new SimpleDateFormat("HH:mm:ss.SSS", Locale.US);
    private static final long MAX_LOG = 24_000;

    public static synchronized void log(Context c, String msg) {
        File f = new File(c.getFilesDir(), "log.txt");
        try {
            if (f.length() > MAX_LOG) { // keep it small: the full file becomes log.1 (one generation)
                File old = new File(c.getFilesDir(), "log.1");
                old.delete();
                if (!f.renameTo(old)) f.delete();
            }
            try (PrintWriter w = new PrintWriter(new FileOutputStream(f, true))) {
                w.println(TS.format(new Date()) + "  " + msg);
            }
        } catch (Exception ignored) {}
        RemoteLog.add(c, TS.format(new Date()) + "  " + msg);
    }

    /** The previous log generation followed by the current one, or null if there is none. */
    public static synchronized String read(Context c) {
        String prev = readText(new File(c.getFilesDir(), "log.1"));
        String cur = readText(new File(c.getFilesDir(), "log.txt"));
        if (prev == null) return cur;
        return cur == null ? prev : prev + cur;
    }

    /** A whole (small) text file, or null if it is missing or unreadable. */
    static String readText(File f) {
        if (!f.exists()) return null;
        try {
            return new String(Files.readAllBytes(f.toPath()), StandardCharsets.UTF_8);
        } catch (Exception e) { return null; }
    }
}
