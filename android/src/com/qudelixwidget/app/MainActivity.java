package com.qudelixwidget.app;

import android.annotation.SuppressLint;
import android.app.Activity;
import android.appwidget.AppWidgetManager;
import android.bluetooth.BluetoothDevice;
import android.bluetooth.BluetoothManager;
import android.content.Context;
import android.content.Intent;
import android.content.res.ColorStateList;
import android.graphics.Color;
import android.graphics.Typeface;
import android.graphics.drawable.GradientDrawable;
import android.net.Uri;
import android.os.Build;
import android.os.Bundle;
import android.provider.Settings;
import android.text.InputType;
import android.view.Gravity;
import android.view.View;
import android.widget.Button;
import android.widget.CheckBox;
import android.widget.EditText;
import android.widget.HorizontalScrollView;
import android.widget.LinearLayout;
import android.widget.RadioButton;
import android.widget.RadioGroup;
import android.widget.ScrollView;
import android.widget.TextView;

import java.lang.ref.WeakReference;
import java.util.ArrayList;
import java.util.List;
import java.util.Locale;
import java.util.function.Function;

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
        int widgetId = getIntent().getIntExtra(AppWidgetManager.EXTRA_APPWIDGET_ID,
                AppWidgetManager.INVALID_APPWIDGET_ID);
        if (widgetId != AppWidgetManager.INVALID_APPWIDGET_ID) {
            setResult(RESULT_OK, new Intent().putExtra(AppWidgetManager.EXTRA_APPWIDGET_ID, widgetId));
        }
        build();
    }

    /** (Re)build the screen — called again when the theme changes. */
    private void build() {
        int fg = WTheme.text(this), muted = WTheme.muted(this);

        LinearLayout root = new LinearLayout(this);
        root.setOrientation(LinearLayout.VERTICAL);
        root.setBackgroundColor(WTheme.bg(this));
        root.setPadding(dp(16), dp(20), dp(16), dp(16));
        root.setGravity(Gravity.CENTER_HORIZONTAL);

        TextView title = new TextView(this);
        title.setText("Qudelix Widget");
        title.setTextColor(WTheme.accent(this));
        title.setTextSize(22);
        root.addView(title);

        TextView body = new TextView(this);
        body.setTextColor(muted);
        body.setTextSize(15);
        body.setPadding(0, dp(12), 0, dp(12));
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
        status.setPadding(0, dp(4), 0, dp(8));
        root.addView(status);

        Button grant = new Button(this);
        grant.setText("Grant Bluetooth permission");
        grant.setOnClickListener(v -> requestBt());
        root.addView(grant);

        // ── Settings ──
        root.addView(header("Theme", fg));
        String[][] themes = {{WTheme.SYSTEM, "As on the phone"}, {WTheme.LIGHT, "Light — red & grey"},
                {WTheme.DARK, "Dark — red & black"}, {WTheme.CUSTOM, "Custom — your background + accent"}};
        RadioGroup tg = new RadioGroup(this);
        for (String[] t : themes) {
            RadioButton rb = radio(t[1], fg);
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
        RadioGroup dg = new RadioGroup(this);
        RadioButton auto = radio("Auto — first “Qudelix-5K…”", fg);
        dg.addView(auto);
        auto.setChecked(Prefs.getDevice(this).isEmpty());
        auto.setOnClickListener(v -> Prefs.setDevice(this, ""));
        for (String[] d : pairedDevices()) {
            RadioButton rb = radio(d[0], fg);
            dg.addView(rb);
            rb.setChecked(d[1].equals(Prefs.getDevice(this)));
            rb.setOnClickListener(v -> Prefs.setDevice(this, d[1]));
        }
        root.addView(dg);

        root.addView(header("Volume step (− / +)", fg));
        int[] steps = {30, 60, 120};
        RadioGroup sg = new RadioGroup(this);
        sg.setOrientation(LinearLayout.HORIZONTAL);
        for (int st : steps) {
            RadioButton rb = radio(String.format(Locale.US, "%.1f dB  ", st / 60.0), fg);
            sg.addView(rb);
            rb.setChecked(st == Prefs.getStepN(this));
            rb.setOnClickListener(v -> Prefs.setStepN(this, st));
        }
        root.addView(sg);

        // ── Self-update (over Tailscale) ──
        TextView upd = new TextView(this);
        upd.setTextColor(muted);
        upd.setTextSize(14);
        upd.setPadding(0, dp(12), 0, dp(4));
        upd.setText("Version " + Updater.installedVersionName(this) + " · updates: GitHub "
                + Updater.GITHUB_REPO + (Updater.enabled(this) ? " + this PC (Tailscale)" : ""));
        root.addView(upd);

        if (Updater.waiting(this)) {
            // InstallReceiver couldn't post its notification (notifications off).
            root.addView(small("An update is downloaded and waits for your OK — tap \"Check for update now\".",
                    WTheme.accent(this)));
        }

        Button unknown = new Button(this);
        unknown.setText("Allow installing updates");
        unknown.setOnClickListener(v -> startActivity(new Intent(
                Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES,
                Uri.parse("package:" + getPackageName()))));
        root.addView(unknown);

        Button check = new Button(this);
        check.setText("Check for update now");
        check.setOnClickListener(v -> {
            upd.setText("checking…");
            runAndShow(upd, app -> {
                String r = Updater.checkAndInstall(app, true);
                return "Version " + Updater.installedVersionName(app) + " · " + r;
            });
        });
        root.addView(check);

        // ── Debug over Tailscale (for developers) ──
        root.addView(header("Debug over Tailscale", fg));
        CheckBox dbg = new CheckBox(this);
        dbg.setText("Send the log to my PC and take test builds from it");
        dbg.setTextColor(fg);
        dbg.setButtonTintList(ColorStateList.valueOf(WTheme.accent(this)));
        dbg.setChecked(Prefs.getDebugOn(this));
        root.addView(dbg);
        EditText host = new EditText(this);
        host.setSingleLine(true);
        host.setHint("PC name, e.g. my-pc.tailnet-name.ts.net");
        host.setText(Prefs.getDebugHost(this));
        host.setTextColor(fg);
        host.setHintTextColor(muted);
        host.setInputType(InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_VARIATION_URI);
        root.addView(host);
        EditText key = new EditText(this);
        key.setSingleLine(true);
        key.setHint("PC key, e.g. abcd-efgh-jkmn-pqrs");
        key.setText(Prefs.getDebugKey(this));
        key.setTextColor(fg);
        key.setHintTextColor(muted);
        key.setInputType(InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_VARIATION_VISIBLE_PASSWORD);
        root.addView(key);
        TextView dbgResult = small("", muted);
        Button test = new Button(this);
        test.setText("Save & test connection");
        test.setOnClickListener(v -> {
            Prefs.setDebugHost(this, host.getText().toString());
            Prefs.setDebugKey(this, key.getText().toString());
            Prefs.setDebugOn(this, dbg.isChecked());
            dbgResult.setText("testing…");
            runAndShow(dbgResult, Updater::testPc);
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
            + "AI assistants: an assistant on the PC (e.g. Claude Code in WSL) can read phone.log at "
            + "/mnt/c/Users/<you>/AppData/Roaming/qudelix-widget/phone.log to debug the phone without a cable. "
            + "The setting is kept across updates.", muted));

        Runnable refresh = () -> {
            unknown.setVisibility(getPackageManager().canRequestPackageInstalls() ? View.GONE : View.VISIBLE);
            if (RfcommService.hasBtPermission(this)) {
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
            ct.setPadding(0, dp(12), 0, 0);
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
            lt.setPadding(0, dp(12), 0, 0);
            lt.setTextIsSelectable(true);
            root.addView(lt);
        }

        ScrollView page = new ScrollView(this);
        page.setBackgroundColor(WTheme.bg(this));
        page.addView(root);
        setContentView(page);
        if (!RfcommService.hasBtPermission(this) && !askedBt) {
            askedBt = true;
            requestBt();
        }
    }

    private boolean askedBt;

    /**
     * Run blocking work (given the app context) off the main thread and show its
     * result in {@code out} — only if this screen is still open. Holds no strong
     * reference to the activity meanwhile, so `work` must not capture it either.
     */
    private void runAndShow(TextView out, Function<Context, String> work) {
        final Context app = getApplicationContext();
        final WeakReference<MainActivity> self = new WeakReference<>(this);
        final WeakReference<TextView> view = new WeakReference<>(out);
        new Thread(() -> {
            String r = work.apply(app);
            MainActivity a = self.get();
            TextView t = view.get();
            if (a == null || t == null) return;
            a.runOnUiThread(() -> {
                if (!a.isFinishing() && !a.isDestroyed()) t.setText(r);
            });
        }).start();
    }

    /** Paired devices as {name, address}; empty without the Bluetooth permission. */
    @SuppressLint("MissingPermission")
    private List<String[]> pairedDevices() {
        List<String[]> out = new ArrayList<>();
        if (!RfcommService.hasBtPermission(this)) return out;
        BluetoothManager bm = (BluetoothManager) getSystemService(BLUETOOTH_SERVICE);
        if (bm == null || bm.getAdapter() == null) return out;
        try {
            for (BluetoothDevice d : bm.getAdapter().getBondedDevices()) {
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
        t.setPadding(0, dp(6), 0, dp(2));
        return t;
    }

    /** A row of tappable colour dots; the current one gets a ring. */
    private HorizontalScrollView swatches(int[] colors, int kind, int current) {
        LinearLayout row = new LinearLayout(this);
        int size = dp(34);
        int gap = dp(8);
        for (int c : colors) {
            View dot = new View(this);
            GradientDrawable d = new GradientDrawable();
            d.setShape(GradientDrawable.OVAL);
            d.setColor(c);
            boolean picked = (c & 0xFFFFFF) == (current & 0xFFFFFF);
            d.setStroke(dp(picked ? 3 : 1), picked ? WTheme.text(this) : WTheme.border(this));
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
        HorizontalScrollView sv = new HorizontalScrollView(this);
        sv.addView(row);
        return sv;
    }

    private TextView header(String text, int color) {
        TextView h = new TextView(this);
        h.setText(text);
        h.setTextColor(color);
        h.setTextSize(16);
        h.setTypeface(null, Typeface.BOLD);
        h.setPadding(0, dp(12), 0, dp(2));
        return h;
    }

    private RadioButton radio(String text, int color) {
        RadioButton rb = new RadioButton(this);
        rb.setText(text);
        rb.setTextColor(color);
        rb.setButtonTintList(ColorStateList.valueOf(WTheme.accent(this)));
        return rb;
    }

    private int dp(int v) {
        return Math.round(v * getResources().getDisplayMetrics().density);
    }

    private Runnable refresh;

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
        if (RfcommService.hasBtPermission(this)) {
            RfcommService.start(this, RfcommService.ACTION_REFRESH, 0);
        }
    }

    @Override
    public void onRequestPermissionsResult(int rc, String[] p, int[] r) {
        super.onRequestPermissionsResult(rc, p, r);
        if (refresh != null) refresh.run();
    }
}
