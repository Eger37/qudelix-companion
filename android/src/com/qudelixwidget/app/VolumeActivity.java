package com.qudelixwidget.app;

import android.app.Activity;
import android.content.Intent;
import android.content.SharedPreferences;
import android.graphics.drawable.GradientDrawable;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.view.Gravity;
import android.view.MotionEvent;
import android.view.View;
import android.widget.Button;
import android.widget.LinearLayout;
import android.widget.SeekBar;
import android.widget.TextView;

import java.util.Locale;

/**
 * Small volume popup over the home screen (opened by tapping the dB readout on
 * the widget). A home-screen widget can't receive press-and-hold — the launcher
 * owns long-press — so hold-to-repeat and a slider live here instead.
 * Follows the system light/dark theme.
 */
public class VolumeActivity extends Activity {

    private static final long REPEAT_DELAY_MS = 380;
    private static final long REPEAT_EVERY_MS = 90;
    /** Slider resolution: 0.5 dB (the official app's step), range -60..0 dB. */
    private static final int HALF_DB_N = 30;

    private final Handler ui = new Handler(Looper.getMainLooper());
    private TextView level;
    private SeekBar bar;
    private int volumeN;
    private boolean userDragging;
    private boolean repeating;

    private final SharedPreferences.OnSharedPreferenceChangeListener prefsListener = (p, key) -> {
        // The 5K's read-back (or another controller) changed the volume.
        if ("volN".equals(key) && !userDragging && !repeating) {
            volumeN = Prefs.getVolumeN(this);
            showVolume();
        }
    };

    @Override
    protected void onCreate(Bundle b) {
        super.onCreate(b);
        if (!RfcommService.hasBtPermission(this)) {
            // Without it the service can't start (crash on Android 14+): the
            // settings screen asks for the permission.
            startActivity(new Intent(this, MainActivity.class).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK));
            finish();
            return;
        }
        int bg = WTheme.bg(this);
        int fg = WTheme.text(this);
        int muted = WTheme.muted(this);
        int btnBg = WTheme.surface(this);

        LinearLayout root = new LinearLayout(this);
        root.setOrientation(LinearLayout.VERTICAL);
        root.setGravity(Gravity.CENTER_HORIZONTAL);
        int pad = dp(20);
        root.setPadding(pad, pad, pad, pad);
        GradientDrawable card = new GradientDrawable();
        card.setColor(bg);
        card.setCornerRadius(dp(20));
        root.setBackground(card);

        TextView title = new TextView(this);
        title.setText("Qudelix 5K · volume");
        title.setTextColor(muted);
        title.setTextSize(13);
        root.addView(title);

        LinearLayout row = new LinearLayout(this);
        row.setGravity(Gravity.CENTER_VERTICAL);
        row.setPadding(0, dp(12), 0, dp(8));
        Button minus = roundButton("−", fg, btnBg);
        Button plus = roundButton("+", fg, btnBg);
        level = new TextView(this);
        level.setTextColor(fg);
        level.setTextSize(28);
        level.setGravity(Gravity.CENTER);
        row.addView(minus, new LinearLayout.LayoutParams(dp(56), dp(56)));
        row.addView(level, new LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f));
        row.addView(plus, new LinearLayout.LayoutParams(dp(56), dp(56)));
        root.addView(row, new LinearLayout.LayoutParams(dp(300), LinearLayout.LayoutParams.WRAP_CONTENT));

        bar = new SeekBar(this);
        bar.setMax((-Proto.VOL_MIN_N) / HALF_DB_N); // 120 steps of 0.5 dB
        bar.setOnSeekBarChangeListener(new SeekBar.OnSeekBarChangeListener() {
            @Override public void onProgressChanged(SeekBar s, int p, boolean fromUser) {
                if (!fromUser) return;
                volumeN = Proto.VOL_MIN_N + p * HALF_DB_N;
                showVolume();
                send(); // the service coalesces bursts onto one session
            }
            @Override public void onStartTrackingTouch(SeekBar s) { userDragging = true; }
            @Override public void onStopTrackingTouch(SeekBar s) { userDragging = false; resync(); }
        });
        root.addView(bar, new LinearLayout.LayoutParams(dp(300), LinearLayout.LayoutParams.WRAP_CONTENT));

        TextView hint = new TextView(this);
        hint.setText("Hold − / + to change continuously");
        hint.setTextColor(muted);
        hint.setTextSize(11);
        hint.setPadding(0, dp(8), 0, 0);
        root.addView(hint);

        tint(bar);
        hold(minus, -Prefs.getStepN(this));
        hold(plus, Prefs.getStepN(this));

        setContentView(root);
        getWindow().setBackgroundDrawableResource(android.R.color.transparent);
        setFinishOnTouchOutside(true);

        volumeN = Prefs.getVolumeN(this);
        showVolume();
        // Pull the real current volume from the 5K.
        startService(RfcommService.ACTION_REFRESH, 0);
    }

    @Override
    protected void onResume() {
        super.onResume();
        getSharedPreferences("qudelix", MODE_PRIVATE).registerOnSharedPreferenceChangeListener(prefsListener);
    }

    @Override
    protected void onPause() {
        getSharedPreferences("qudelix", MODE_PRIVATE).unregisterOnSharedPreferenceChangeListener(prefsListener);
        stopRepeat();
        super.onPause();
    }

    // ── hold-to-repeat ───────────────────────────────────────────────────────────

    private int repeatDelta;
    private final Runnable repeatTick = new Runnable() {
        @Override public void run() {
            step(repeatDelta);
            ui.postDelayed(this, REPEAT_EVERY_MS);
        }
    };

    private void hold(View v, int deltaN) {
        v.setOnTouchListener((view, e) -> {
            switch (e.getActionMasked()) {
                case MotionEvent.ACTION_DOWN:
                    view.setPressed(true);
                    repeating = true;
                    repeatDelta = deltaN;
                    step(deltaN);
                    ui.postDelayed(repeatTick, REPEAT_DELAY_MS);
                    return true;
                case MotionEvent.ACTION_UP:
                case MotionEvent.ACTION_CANCEL:
                    view.setPressed(false);
                    stopRepeat();
                    return true;
            }
            return true;
        });
    }

    private void stopRepeat() {
        repeating = false;
        ui.removeCallbacks(repeatTick);
        resync();
    }

    /**
     * Show what the service really set. It caps each increase (Proto.MAX_UP_N),
     * so after a hold or a fast drag up the 5K may be lower than shown. The
     * service writes the volume after the burst, so read it a little later.
     */
    private void resync() {
        ui.postDelayed(() -> {
            if (userDragging || repeating) return;
            volumeN = Prefs.getVolumeN(this);
            showVolume();
        }, 700);
    }

    private void step(int deltaN) {
        volumeN = Proto.clampVol(volumeN + deltaN);
        showVolume();
        send();
    }

    // ── plumbing ─────────────────────────────────────────────────────────────────

    private void showVolume() {
        level.setText(String.format(Locale.US, "%+.1f dB", volumeN / 60.0));
        bar.setProgress((volumeN - Proto.VOL_MIN_N) / HALF_DB_N);
    }

    private void send() {
        startService(RfcommService.ACTION_VOLUME_SET, volumeN);
    }

    private void startService(String action, int value) {
        Intent i = new Intent(this, RfcommService.class).setAction(action).putExtra(RfcommService.EXTRA_VALUE, value);
        try {
            startForegroundService(i);
        } catch (Exception e) {
            Dbg.log(this, "volume popup: service start failed: " + e);
        }
    }

    private void tint(SeekBar s) {
        android.content.res.ColorStateList red = android.content.res.ColorStateList.valueOf(WTheme.accent(this));
        s.setProgressTintList(red);
        s.setThumbTintList(red);
    }

    private Button roundButton(String label, int fg, int bg) {
        Button b = new Button(this);
        b.setText(label);
        b.setTextSize(22);
        b.setTextColor(fg);
        b.setPadding(0, 0, 0, 0);
        GradientDrawable d = new GradientDrawable();
        d.setShape(GradientDrawable.OVAL);
        d.setColor(bg);
        b.setBackground(d);
        return b;
    }

    private int dp(int v) {
        return Math.round(v * getResources().getDisplayMetrics().density);
    }
}
