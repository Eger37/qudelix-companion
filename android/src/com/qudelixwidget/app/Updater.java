package com.qudelixwidget.app;

import android.app.PendingIntent;
import android.content.Context;
import android.content.Intent;
import android.content.pm.PackageInstaller;
import android.os.Build;

import org.json.JSONObject;

import java.io.ByteArrayOutputStream;
import java.io.File;
import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.io.InputStream;
import java.io.OutputStream;
import java.net.HttpURLConnection;
import java.net.URL;

/**
 * Self-update: from GitHub Releases (latest release's qudelix-widget.apk) and,
 * on the owner's builds, also from the PC over Tailscale. The Windows widget serves `version.json` + the APK
 * on its Tailscale address (windows/src/update_server.rs, filled by
 * android/publish_update.sh); this checks it and installs
 * newer builds through PackageInstaller. The first update asks the user once
 * (the APK was sideloaded by a file manager); after that this app is the
 * installer of record and Android 12+ installs updates without a prompt.
 */
public final class Updater {
    private Updater() {}

    /** Update + log server: the Windows widget, reached via `tailscale serve` on the PC,
     *  only while "Debug over Tailscale" is on (settings screen). Must be the MagicDNS
     *  NAME — tailscale serve routes by Host and 404s a bare IP. "" = off. */
    public static String baseUrl(Context c) {
        String host = Prefs.getDebugHost(c);
        return Prefs.getDebugOn(c) && !host.isEmpty() ? "http://" + host + ":8787/" : "";
    }

    public static boolean enabled(Context c) {
        return !baseUrl(c).isEmpty();
    }

    /** GitHub repository whose latest release carries qudelix-widget.apk. */
    public static final String GITHUB_REPO = "Eger37/qudelix-companion";

    /** A newer build somewhere: version + where to download it. */
    private static final class Offer {
        final long code; final String name; final String url;
        Offer(long code, String name, String url) { this.code = code; this.name = name; this.url = url; }
    }

    /**
     * Check, download and install if newer. Blocking — call off the main thread.
     * Sources: GitHub Releases (always) and the PC on the tailnet (debug mode); the
     * higher version wins, the PC on a tie (its build keeps the debug setting seeded).
     */
    public static synchronized String checkAndInstall(Context c) {
        long local = installedVersion(c);
        String localName = installedVersionName(c);
        Offer best = null;
        String errors = "";
        try {
            Offer gh = fromGitHub();
            if (gh != null) best = gh;
        } catch (Exception e) {
            errors += " github: " + e.getMessage();
        }
        String base = baseUrl(c);
        if (!base.isEmpty()) {
            try {
                JSONObject v = new JSONObject(new String(get(base + "version.json"), "UTF-8"));
                Offer pc = new Offer(v.getLong("versionCode"), v.optString("versionName", "?"),
                        base + v.optString("apk", "qudelix-widget.apk"));
                if (best == null || pc.code >= best.code) best = pc;
            } catch (Exception e) {
                errors += " pc: " + e.getMessage();
            }
        }
        if (best == null) {
            Dbg.log(c, "update: check failed:" + errors);
            return "update check failed (offline?)";
        }
        if (best.code <= local) {
            Dbg.log(c, "update: up to date (v" + localName + ")" + errors);
            return "up to date (v" + localName + ")";
        }
        try {
            Dbg.log(c, "update: v" + localName + " → v" + best.name + " from " + best.url);
            File apk = new File(c.getCacheDir(), "update.apk");
            try (OutputStream out = new FileOutputStream(apk)) {
                out.write(get(best.url));
            }
            install(c, apk);
            return "installing v" + best.name + "…";
        } catch (Exception e) {
            Dbg.log(c, "update: download/install failed: " + e);
            return "update failed: " + e.getMessage();
        }
    }

    /** Latest GitHub release, or null if it has no APK. Tag "vX.Y.Z" → versionCode. */
    private static Offer fromGitHub() throws Exception {
        JSONObject rel = new JSONObject(new String(
                get("https://api.github.com/repos/" + GITHUB_REPO + "/releases/latest"), "UTF-8"));
        String name = rel.getString("tag_name").replaceFirst("^v", "");
        org.json.JSONArray assets = rel.optJSONArray("assets");
        if (assets == null) return null;
        for (int i = 0; i < assets.length(); i++) {
            JSONObject a = assets.getJSONObject(i);
            if ("qudelix-widget.apk".equals(a.optString("name"))) {
                return new Offer(versionCode(name), name, a.getString("browser_download_url"));
            }
        }
        return null;
    }

    /** "1.5.2" → 1005002 — the same formula as build_apk.sh. */
    static long versionCode(String semver) {
        String[] p = semver.split("\\.");
        long code = 0;
        for (int i = 0; i < 3; i++) {
            long v = i < p.length ? Long.parseLong(p[i].replaceAll("[^0-9]", "")) : 0;
            code = code * 1000 + v;
        }
        return code;
    }

    /** Semantic version (shared with the Windows widget via ../VERSION), e.g. "1.0.3". */
    public static String installedVersionName(Context c) {
        try {
            return c.getPackageManager().getPackageInfo(c.getPackageName(), 0).versionName;
        } catch (Exception e) {
            return "?";
        }
    }

    public static long installedVersion(Context c) {
        try {
            return c.getPackageManager().getPackageInfo(c.getPackageName(), 0).getLongVersionCode();
        } catch (Exception e) {
            return 0;
        }
    }

    /** Debug-mode connection test: the PC's version.json, or why it failed. Blocking. */
    static String testPc(Context c) {
        String base = baseUrl(c);
        if (base.isEmpty()) return "debug is off or no PC name set";
        try {
            JSONObject v = new JSONObject(new String(get(base + "version.json"), "UTF-8"));
            return "✓ PC reached (it offers v" + v.optString("versionName", "?") + ")";
        } catch (Exception e) {
            return "✗ " + e.getMessage();
        }
    }

    private static byte[] get(String url) throws Exception {
        HttpURLConnection h = (HttpURLConnection) new URL(url).openConnection();
        h.setRequestProperty("User-Agent", "qudelix-widget");          // GitHub API requires one
        h.setRequestProperty("Accept", "application/vnd.github+json");
        h.setConnectTimeout(5000);
        h.setReadTimeout(20000);
        h.setUseCaches(false);
        try (InputStream in = h.getInputStream()) {
            if (h.getResponseCode() != 200) throw new Exception("HTTP " + h.getResponseCode());
            ByteArrayOutputStream b = new ByteArrayOutputStream();
            byte[] buf = new byte[16384];
            int r;
            while ((r = in.read(buf)) > 0) b.write(buf, 0, r);
            return b.toByteArray();
        } finally {
            h.disconnect();
        }
    }

    private static void install(Context c, File apk) throws Exception {
        PackageInstaller pi = c.getPackageManager().getPackageInstaller();
        PackageInstaller.SessionParams params =
                new PackageInstaller.SessionParams(PackageInstaller.SessionParams.MODE_FULL_INSTALL);
        params.setAppPackageName(c.getPackageName());
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
            params.setRequireUserAction(PackageInstaller.SessionParams.USER_ACTION_NOT_REQUIRED);
        }
        int id = pi.createSession(params);
        try (PackageInstaller.Session s = pi.openSession(id)) {
            try (OutputStream out = s.openWrite("base.apk", 0, apk.length());
                 InputStream in = new FileInputStream(apk)) {
                byte[] buf = new byte[65536];
                int r;
                while ((r = in.read(buf)) > 0) out.write(buf, 0, r);
                s.fsync(out);
            }
            Intent done = new Intent(c, InstallReceiver.class);
            int flags = PendingIntent.FLAG_UPDATE_CURRENT
                    | (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S ? PendingIntent.FLAG_MUTABLE : 0);
            s.commit(PendingIntent.getBroadcast(c, id, done, flags).getIntentSender());
        }
        Dbg.log(c, "update: install session " + id + " committed");
    }
}
