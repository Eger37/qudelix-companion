package com.qudelixwidget.app;

import android.app.Activity;
import android.content.pm.PackageManager;
import android.graphics.Color;
import android.os.Build;
import android.os.Bundle;
import android.view.Gravity;
import android.view.View;
import android.widget.Button;
import android.widget.LinearLayout;
import android.widget.TextView;

import java.util.ArrayList;
import java.util.List;

/**
 * Setup + settings: Bluetooth permission, theme (red & grey / red & black /
 * system), volume step, self-update, debug log. Also the widget's "Customize"
 * screen (long-press the widget → reconfigure).
 */
public class MainActivity extends Activity {

    @Override
    protected void onCreate(Bundle b) {
        super.onCreate(b);
        Dbg.log(this, "main screen opened");
        // Opened as the widget's configure/reconfigure screen: confirm right away
        // so the launcher keeps the widget, whatever the user does here.
        int widgetId = getIntent().getIntExtra(android.appwidget.AppWidgetManager.EXTRA_APPWIDGET_ID,
                android.appwidget.AppWidgetManager.INVALID_APPWIDGET_ID);
        if (widgetId != android.appwidget.AppWidgetManager.INVALID_APPWIDGET_ID) {
            setResult(RESULT_OK, new android.content.Intent()
                    .putExtra(android.appwidget.AppWidgetManager.EXTRA_APPWIDGET_ID, widgetId));
        }
        build();
    }

    /** (Re)build the screen — called again when the theme changes. */
    private void build() {
        int fg = WTheme.text(this), muted = WTheme.muted(this);

        LinearLayout root = new LinearLayout(this);
        root.setOrientation(LinearLayout.VERTICAL);
        root.setBackgroundColor(WTheme.bg(this));
        root.setPadding(48, 64, 48, 48);
        root.setGravity(Gravity.CENTER_HORIZONTAL);

        TextView title = new TextView(this);
        title.setText("Qudelix Widget");
        title.setTextColor(WTheme.accent(this));
        title.setTextSize(22);
        root.addView(title);

        TextView body = new TextView(this);
        body.setTextColor(muted);
        body.setTextSize(15);
        body.setPadding(0, 32, 0, 32);
        body.setText(
            "1. Grant Bluetooth below.\n" +
            "2. Pair the Qudelix-5K in Android Bluetooth settings (as usual).\n" +
            "3. Long-press your home screen → Widgets → Qudelix Widget → place it.\n\n" +
            "USB / A2DP 1 / A2DP 2 — source priority; − / + — volume; tap the bar to jump; " +
            "tap the dB value for a slider; ↻ — read state from the 5K.\n" +
            "The 5K has one control channel: the widget releases it 3 s after the last tap, " +
            "so the Qudelix app and the PC widget can use it too (just not at the same moment).");
        root.addView(body);

        TextView status = new TextView(this);
        status.setTextSize(15);
        status.setPadding(0, 8, 0, 24);
        root.addView(status);

        Button grant = new Button(this);
        grant.setText("Grant Bluetooth permission");
        grant.setOnClickListener(v -> requestBt());
        root.addView(grant);

        // ── Settings ──
        root.addView(header("Theme", fg));
        String[][] themes = {{WTheme.SYSTEM, "As on the phone"}, {WTheme.LIGHT, "Light — red & grey"},
                {WTheme.DARK, "Dark — red & black"}, {WTheme.CUSTOM, "Custom — your background + accent"}};
        android.widget.RadioGroup tg = new android.widget.RadioGroup(this);
        for (String[] t : themes) {
            android.widget.RadioButton rb = radio(t[1], fg);
            tg.addView(rb);
            rb.setChecked(t[0].equals(Prefs.getTheme(this)));
            rb.setOnClickListener(v -> {
                Prefs.setTheme(this, t[0]);
                QudelixWidget.update(this);
                build(); // recolour this screen
            });
        }
        root.addView(tg);

        if (WTheme.CUSTOM.equals(Prefs.getTheme(this))) {
        root.addView(small("Custom: background", muted));
        root.addView(swatches(new int[]{0xFF1B1F3B, 0xFF0B0B0C, 0xFF1F2937, 0xFF3B0A0A, 0xFF0F2E1F, 0xFF2E1065,
                0xFF0C4A6E, 0xFFFFFFFF, 0xFFF4F1EA, 0xFFFDE2E4, 0xFFE0F2FE, 0xFFECFCCB},
                CUSTOM_BG_PICK, Prefs.getCustomBg(this)));
        root.addView(small("Custom: accent", muted));
        root.addView(swatches(new int[]{WTheme.RED, 0xFFF97316, 0xFFF59E0B, 0xFF10B981, 0xFF06B6D4, 0xFF2F6FEB,
                0xFF8B5CF6, 0xFFEC4899, 0xFFFFFFFF, 0xFF111111},
                ACCENT_PICK, Prefs.getAccent(this)));
        }

        // ── Logo (launcher icon) background ──
        root.addView(header("Logo background", fg));
        root.addView(swatches(Logo.COLORS, LOGO_PICK, Prefs.getLogo(this)));

        // ── Device to control ──
        root.addView(header("Device", fg));
        android.widget.RadioGroup dg = new android.widget.RadioGroup(this);
        android.widget.RadioButton auto = radio("Auto — first “Qudelix-5K…”", fg);
        dg.addView(auto);
        auto.setChecked(Prefs.getDevice(this).isEmpty());
        auto.setOnClickListener(v -> Prefs.setDevice(this, ""));
        for (String[] d : pairedDevices()) {
            android.widget.RadioButton rb = radio(d[0], fg);
            dg.addView(rb);
            rb.setChecked(d[1].equals(Prefs.getDevice(this)));
            rb.setOnClickListener(v -> Prefs.setDevice(this, d[1]));
        }
        root.addView(dg);

        root.addView(header("Volume step (− / +)", fg));
        int[][] steps = {{30}, {60}, {120}};
        android.widget.RadioGroup sg = new android.widget.RadioGroup(this);
        sg.setOrientation(LinearLayout.HORIZONTAL);
        for (int[] st : steps) {
            android.widget.RadioButton rb = radio(String.format(java.util.Locale.US, "%.1f dB  ", st[0] / 60.0), fg);
            sg.addView(rb);
            rb.setChecked(st[0] == Prefs.getStepN(this));
            rb.setOnClickListener(v -> Prefs.setStepN(this, st[0]));
        }
        root.addView(sg);

        // ── Self-update (over Tailscale) ──
        TextView upd = new TextView(this);
        upd.setTextColor(muted);
        upd.setTextSize(14);
        upd.setPadding(0, 32, 0, 8);
        upd.setText("Version " + Updater.installedVersionName(this) + " · updates: GitHub "
                + Updater.GITHUB_REPO + (Updater.enabled(this) ? " + this PC (Tailscale)" : ""));
        root.addView(upd);

        Button unknown = new Button(this);
        unknown.setText("Allow installing updates");
        unknown.setOnClickListener(v -> startActivity(new android.content.Intent(
                android.provider.Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES,
                android.net.Uri.parse("package:" + getPackageName()))));
        root.addView(unknown);

        Button check = new Button(this);
        check.setText("Check for update now");
        check.setOnClickListener(v -> {
            upd.setText("checking…");
            new Thread(() -> {
                String r = Updater.checkAndInstall(this);
                runOnUiThread(() -> upd.setText("Version " + Updater.installedVersionName(this) + " · " + r));
            }).start();
        });
        root.addView(check);

        // ── Debug over Tailscale (for developers) ──
        root.addView(header("Debug over Tailscale", fg));
        android.widget.CheckBox dbg = new android.widget.CheckBox(this);
        dbg.setText("Send the log to my PC and take test builds from it");
        dbg.setTextColor(fg);
        dbg.setButtonTintList(android.content.res.ColorStateList.valueOf(WTheme.accent(this)));
        dbg.setChecked(Prefs.getDebugOn(this));
        root.addView(dbg);
        android.widget.EditText host = new android.widget.EditText(this);
        host.setSingleLine(true);
        host.setHint("PC name, e.g. my-pc.tailnet-name.ts.net");
        host.setText(Prefs.getDebugHost(this));
        host.setTextColor(fg);
        host.setHintTextColor(muted);
        host.setInputType(android.text.InputType.TYPE_CLASS_TEXT | android.text.InputType.TYPE_TEXT_VARIATION_URI);
        root.addView(host);
        android.widget.EditText key = new android.widget.EditText(this);
        key.setSingleLine(true);
        key.setHint("PC key, e.g. abcd-efgh-jkmn-pqrs");
        key.setText(Prefs.getDebugKey(this));
        key.setTextColor(fg);
        key.setHintTextColor(muted);
        key.setInputType(android.text.InputType.TYPE_CLASS_TEXT | android.text.InputType.TYPE_TEXT_VARIATION_VISIBLE_PASSWORD);
        root.addView(key);
        TextView dbgResult = small("", muted);
        Button test = new Button(this);
        test.setText("Save & test connection");
        test.setOnClickListener(v -> {
            Prefs.setDebugHost(this, host.getText().toString());
            Prefs.setDebugKey(this, key.getText().toString());
            Prefs.setDebugOn(this, dbg.isChecked());
            dbgResult.setText("testing…");
            new Thread(() -> {
                String r = Updater.testPc(this);
                runOnUiThread(() -> dbgResult.setText(r));
            }).start();
        });
        dbg.setOnCheckedChangeListener((v, on) -> {
            Prefs.setDebugOn(this, on);
            Dbg.log(this, "debug over Tailscale " + (on ? "on" : "off"));
        });
        root.addView(test);
        root.addView(dbgResult);
        root.addView(small(
            "What it does: while on, every log line and crash of this app goes to the Qudelix widget on "
            + "your Windows PC (%APPDATA%\\qudelix-widget\\phone.log), and the app also installs test "
            + "builds the PC offers (the higher version of GitHub and PC wins). Off = GitHub updates only, "
            + "nothing leaves the phone.\n\n"
            + "Needs: Tailscale on this phone and on the PC, signed in to the same tailnet; the PC widget "
            + "with ⚙ → \"Phone debug over Tailscale\" on (it shows the name and the key to enter here). "
            + "Use the PC's MagicDNS name, not its 100.x address.\n\n"
            + "How it works: the PC widget serves http://<PC name>:8787 inside the tailnet only (tailscale "
            + "serve → 127.0.0.1); nothing is open to the internet. It answers only requests that carry "
            + "its key, so other devices in the tailnet can't use it. The log has app version, phone model, "
            + "commands and errors — no accounts, no Bluetooth addresses.\n\n"
            + "AI assistants: an assistant on the PC can read phone.log at "
            + "/mnt/c/Users/<you>/AppData/Roaming/qudelix-widget/phone.log to debug the phone without a cable. "
            + "The setting is kept across updates.", muted));

        Runnable refresh = () -> {
            unknown.setVisibility(getPackageManager().canRequestPackageInstalls() ? View.GONE : View.VISIBLE);
            if (hasBt()) {
                status.setText("✓ Bluetooth permission granted");
                status.setTextColor(Color.parseColor("#4CAF50"));
                grant.setEnabled(false);
            } else {
                status.setText("✗ Bluetooth permission needed");
                status.setTextColor(Color.parseColor("#FF7043"));
                grant.setEnabled(true);
            }
        };
        grant.post(refresh);
        this.refresh = refresh;

        // Surface the last crash (if any) so it can be read without adb.
        String crash = App.readCrash(this);
        if (crash != null) {
            TextView ct = new TextView(this);
            ct.setText("Last crash:\n" + crash);
            ct.setTextColor(Color.parseColor("#FF8A80"));
            ct.setTextSize(11);
            ct.setPadding(0, 32, 0, 0);
            ct.setTextIsSelectable(true);
            root.addView(ct);
        }

        // Debug log of the last sessions (no adb needed).
        String log = Dbg.read(this);
        if (log != null) {
            TextView lt = new TextView(this);
            lt.setText("Log:\n" + log);
            lt.setTextColor(muted);
            lt.setTextSize(10);
            lt.setPadding(0, 32, 0, 0);
            lt.setTextIsSelectable(true);
            root.addView(lt);
        }

        android.widget.ScrollView page = new android.widget.ScrollView(this);
        page.setBackgroundColor(WTheme.bg(this));
        page.addView(root);
        setContentView(page);
        if (!hasBt() && !askedBt) {
            askedBt = true;
            requestBt();
        }
    }

    private boolean askedBt;

    /** Paired devices as {name, address}; empty without the Bluetooth permission. */
    @android.annotation.SuppressLint("MissingPermission")
    private List<String[]> pairedDevices() {
        List<String[]> out = new ArrayList<>();
        if (!hasBt()) return out;
        android.bluetooth.BluetoothManager bm =
                (android.bluetooth.BluetoothManager) getSystemService(BLUETOOTH_SERVICE);
        if (bm == null || bm.getAdapter() == null) return out;
        try {
            for (android.bluetooth.BluetoothDevice d : bm.getAdapter().getBondedDevices()) {
                String name = d.getName();
                out.add(new String[]{name != null ? name : d.getAddress(), d.getAddress()});
            }
        } catch (SecurityException ignored) {}
        return out;
    }
    private static final int CUSTOM_BG_PICK = 1, ACCENT_PICK = 2, LOGO_PICK = 3;

    private TextView small(String text, int color) {
        TextView t = new TextView(this);
        t.setText(text);
        t.setTextColor(color);
        t.setTextSize(13);
        t.setPadding(0, 16, 0, 6);
        return t;
    }

    /** A row of tappable colour dots; the current one gets a ring. */
    private android.widget.HorizontalScrollView swatches(int[] colors, int kind, int current) {
        LinearLayout row = new LinearLayout(this);
        int size = Math.round(34 * getResources().getDisplayMetrics().density);
        int gap = Math.round(8 * getResources().getDisplayMetrics().density);
        for (int c : colors) {
            View dot = new View(this);
            android.graphics.drawable.GradientDrawable d = new android.graphics.drawable.GradientDrawable();
            d.setShape(android.graphics.drawable.GradientDrawable.OVAL);
            d.setColor(c);
            int ring = (c & 0xFFFFFF) == (current & 0xFFFFFF) ? WTheme.text(this) : WTheme.border(this);
            d.setStroke(Math.round(((c & 0xFFFFFF) == (current & 0xFFFFFF) ? 3 : 1) * getResources().getDisplayMetrics().density), ring);
            dot.setBackground(d);
            dot.setOnClickListener(v -> {
                if (kind == LOGO_PICK) {
                    Logo.apply(this, c);
                    build();
                    return;
                }
                if (kind == CUSTOM_BG_PICK) {
                    Prefs.setCustomBg(this, c);
                    Prefs.setTheme(this, WTheme.CUSTOM);
                } else {
                    Prefs.setAccent(this, c);
                    Prefs.setTheme(this, WTheme.CUSTOM);
                }
                QudelixWidget.update(this);
                build();
            });
            LinearLayout.LayoutParams lp = new LinearLayout.LayoutParams(size, size);
            lp.setMarginEnd(gap);
            row.addView(dot, lp);
        }
        android.widget.HorizontalScrollView sv = new android.widget.HorizontalScrollView(this);
        sv.addView(row);
        return sv;
    }

    private TextView header(String text, int color) {
        TextView h = new TextView(this);
        h.setText(text);
        h.setTextColor(color);
        h.setTextSize(16);
        h.setTypeface(null, android.graphics.Typeface.BOLD);
        h.setPadding(0, 32, 0, 4);
        return h;
    }

    private android.widget.RadioButton radio(String text, int color) {
        android.widget.RadioButton rb = new android.widget.RadioButton(this);
        rb.setText(text);
        rb.setTextColor(color);
        rb.setButtonTintList(android.content.res.ColorStateList.valueOf(WTheme.accent(this)));
        return rb;
    }

    private Runnable refresh;

    private boolean hasBt() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
            return checkSelfPermission(android.Manifest.permission.BLUETOOTH_CONNECT)
                    == PackageManager.PERMISSION_GRANTED;
        }
        return true;
    }

    private void requestBt() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
            List<String> perms = new ArrayList<>();
            perms.add(android.Manifest.permission.BLUETOOTH_CONNECT);
            if (Build.VERSION.SDK_INT >= 33) perms.add(android.Manifest.permission.POST_NOTIFICATIONS);
            requestPermissions(perms.toArray(new String[0]), 1);
        }
    }

    @Override
    protected void onResume() {
        super.onResume();
        if (refresh != null) refresh.run();
        // Opening the app reads the 5K's current state for the widget too.
        if (hasBt()) {
            try {
                startForegroundService(new android.content.Intent(this, RfcommService.class).setAction(RfcommService.ACTION_REFRESH));
            } catch (Exception e) {
                Dbg.log(this, "refresh on open failed: " + e);
            }
        }
    }

    @Override
    public void onRequestPermissionsResult(int rc, String[] p, int[] r) {
        super.onRequestPermissionsResult(rc, p, r);
        if (refresh != null) refresh.run();
    }
}
