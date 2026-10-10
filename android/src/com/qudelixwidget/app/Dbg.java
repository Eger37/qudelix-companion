package com.qudelixwidget.app;

import android.content.Context;

import java.io.File;
import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.io.PrintWriter;
import java.text.SimpleDateFormat;
import java.util.Date;
import java.util.Locale;

/** Append-only debug log (filesDir/log.txt) shown in MainActivity and mirrored to the PC (RemoteLog). */
public final class Dbg {
    private Dbg() {}
    private static final SimpleDateFormat TS = new SimpleDateFormat("HH:mm:ss.SSS", Locale.US);

    public static synchronized void log(Context c, String msg) {
        File f = new File(c.getFilesDir(), "log.txt");
        try {
            if (f.length() > 24_000) { // keep it small: drop the file when it grows
                f.delete();
            }
            try (PrintWriter w = new PrintWriter(new FileOutputStream(f, true))) {
                w.println(TS.format(new Date()) + "  " + msg);
            }
        } catch (Exception ignored) {}
        RemoteLog.add(c, TS.format(new Date()) + "  " + msg);
    }

    public static String read(Context c) {
        File f = new File(c.getFilesDir(), "log.txt");
        if (!f.exists()) return null;
        try {
            byte[] b = new byte[(int) f.length()];
            try (FileInputStream in = new FileInputStream(f)) { in.read(b); }
            return new String(b);
        } catch (Exception e) { return null; }
    }

    public static void clear(Context c) {
        new File(c.getFilesDir(), "log.txt").delete();
    }

    public static String hex(byte[] b) {
        StringBuilder s = new StringBuilder();
        for (byte x : b) s.append(String.format("%02X ", x));
        return s.toString().trim();
    }
}
