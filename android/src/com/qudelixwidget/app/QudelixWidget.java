package com.qudelixwidget.app;

import android.app.PendingIntent;
import android.appwidget.AppWidgetManager;
import android.appwidget.AppWidgetProvider;
import android.content.ComponentName;
import android.content.Context;
import android.content.Intent;
import android.os.Build;
import android.widget.RemoteViews;

import java.util.Locale;

/** Home-screen widget: priority buttons, volume -/+, refresh, and what is playing. Clicks drive {@link RfcommService}. */
public class QudelixWidget extends AppWidgetProvider {

    @Override
    public void onUpdate(Context context, AppWidgetManager mgr, int[] ids) {
        update(context);
    }

    @Override
    public void onReceive(Context context, Intent intent) {
        String action = intent.getAction();
        if (Intent.ACTION_MY_PACKAGE_REPLACED.equals(action)) {
            Dbg.log(context, "package replaced → v" + Updater.installedVersionName(context));
            update(context);
            return;
        }
        if (RfcommService.ACTION_PRIORITY.equals(action) || RfcommService.ACTION_VOLUME.equals(action)
                || RfcommService.ACTION_REFRESH.equals(action)
                || RfcommService.ACTION_VOLUME_SET.equals(action)) {
            Dbg.log(context, "widget tap " + action + " value=" + intent.getIntExtra(RfcommService.EXTRA_VALUE, 0));
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S
                    && context.checkSelfPermission(android.Manifest.permission.BLUETOOTH_CONNECT)
                    != android.content.pm.PackageManager.PERMISSION_GRANTED) {
                Dbg.log(context, "no BLUETOOTH_CONNECT permission — not starting service");
                pushStatus(context, "open app → grant Bluetooth");
                return;
            }
            Intent svc = new Intent(context, RfcommService.class)
                    .setAction(action)
                    .putExtra(RfcommService.EXTRA_VALUE, intent.getIntExtra(RfcommService.EXTRA_VALUE, 0));
            try {
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                    context.startForegroundService(svc);
                } else {
                    context.startService(svc);
                }
            } catch (Exception e) {
                Dbg.log(context, "startForegroundService failed: " + e);
                pushStatus(context, "start failed");
            }
            return;
        }
        super.onReceive(context, intent);
    }

    /** Save a status line and refresh the widget. Safe to call from any thread. */
    public static void pushStatus(Context context, String status) {
        Prefs.setStatus(context, status);
        update(context);
    }

    public static void update(Context context) {
        AppWidgetManager mgr = AppWidgetManager.getInstance(context);
        ComponentName cn = new ComponentName(context, QudelixWidget.class);
        int[] ids = mgr.getAppWidgetIds(cn);
        if (ids == null) return;
        for (int id : ids) {
            mgr.updateAppWidget(id, buildViews(context));
        }
    }

    private static RemoteViews buildViews(Context context) {
        RemoteViews v = new RemoteViews(context.getPackageName(), R.layout.widget);
        v.setTextViewText(R.id.title, "Qudelix 5K · v" + Updater.installedVersionName(context));
        Skin k = Skin.of(context);
        k.bg(v, R.id.root, k.bg, k.bgTint);
        for (int id : new int[]{R.id.title, R.id.vol, R.id.btn_vol_dn, R.id.btn_vol_up, R.id.btn_refresh}) {
            k.text(v, context, id, k.text, k.textInt);
        }
        for (int id : new int[]{R.id.status, R.id.playing}) k.text(v, context, id, k.muted, k.mutedInt);
        for (int id : new int[]{R.id.btn_vol_dn, R.id.btn_vol_up, R.id.btn_refresh}) k.bg(v, id, k.btn, k.btnTint);

        v.setOnClickPendingIntent(R.id.btn_usb, priorityPI(context, 0));
        v.setOnClickPendingIntent(R.id.btn_bt1, priorityPI(context, 1));
        v.setOnClickPendingIntent(R.id.btn_bt2, priorityPI(context, 2));
        v.setOnClickPendingIntent(R.id.btn_vol_dn, volumePI(context, -1));
        v.setOnClickPendingIntent(R.id.btn_vol_up, volumePI(context, +1));
        v.setOnClickPendingIntent(R.id.btn_refresh, refreshPI(context));
        // Tap the dB readout → volume popup (slider + hold-to-repeat; a widget
        // itself can't get press-and-hold, the launcher takes long-press).
        v.setOnClickPendingIntent(R.id.vol, PendingIntent.getActivity(context, 400,
                new Intent(context, VolumeActivity.class).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK),
                PendingIntent.FLAG_UPDATE_CURRENT | PendingIntent.FLAG_IMMUTABLE));

        int pri = Prefs.getPriority(context);
        int[] priIds = {R.id.btn_usb, R.id.btn_bt1, R.id.btn_bt2};
        for (int i = 0; i < 3; i++) {
            boolean on = pri == i;
            if (on) {
                k.accentBg(v, priIds[i], R.drawable.w_btn_tint, R.drawable.w_btn_on);
                v.setTextColor(priIds[i], k.onAccent);
            } else {
                k.bg(v, priIds[i], k.btn, k.btnTint);
                k.text(v, context, priIds[i], k.text, k.textInt);
            }
        }

        v.setTextViewText(R.id.vol, Prefs.hasVolume(context)
                ? String.format(Locale.US, "%+.1f dB", Prefs.getVolumeN(context) / 60.0) : "— dB");
        // Volume bar: 12 segments of 5 dB; filled up to the current level, tap = jump there.
        int volN = Prefs.getVolumeN(context);
        boolean known = Prefs.hasVolume(context);
        for (int i = 0; i < SEG_IDS.length; i++) {
            int segN = segmentN(i);
            if (known && volN >= segN - 150) k.accentBg(v, SEG_IDS[i], R.drawable.w_seg_tint, R.drawable.w_seg_on);
            else k.bg(v, SEG_IDS[i], k.segOff, k.segTint);
            Intent seg = new Intent(context, QudelixWidget.class)
                    .setAction(RfcommService.ACTION_VOLUME_SET)
                    .putExtra(RfcommService.EXTRA_VALUE, segN);
            v.setOnClickPendingIntent(SEG_IDS[i], PendingIntent.getBroadcast(context, 500 + i, seg, piFlags()));
        }
        String playing = Prefs.getPlaying(context);
        v.setTextViewText(R.id.playing, "Playing: " + (playing != null ? playing : "?"));
        v.setTextViewText(R.id.status, Prefs.getStatus(context));
        return v;
    }

    private static final int[] SEG_IDS = {
            R.id.seg0, R.id.seg1, R.id.seg2, R.id.seg3, R.id.seg4, R.id.seg5,
            R.id.seg6, R.id.seg7, R.id.seg8, R.id.seg9, R.id.seg10, R.id.seg11};

    /** Volume (n = dB*60) that segment i stands for: -55, -50, … 0 dB. */
    private static int segmentN(int i) {
        return Proto.VOL_MIN_N + (i + 1) * 5 * 60;
    }

    /**
     * Colours for the current theme. 'a' (system: values/values-night, picked by the
     * launcher live), 'l', 'd' use resource drawables; 'c' (custom) tints white base
     * drawables at runtime. Accent fills are always tinted so a custom accent works
     * in every theme. Tints are always (re)set because the launcher may reuse views.
     */
    private static final class Skin {
        int bg, btn, segOff, text, muted;              // drawable / colour resources
        Integer bgTint, btnTint, segTint;              // custom theme only
        Integer textInt, mutedInt;                     // custom theme only
        int accent, onAccent;

        static Skin of(Context c) {
            Skin k = new Skin();
            k.accent = WTheme.accent(c);
            k.onAccent = WTheme.onAccent(c);
            switch (WTheme.widgetVariant(c)) {
                case 'l': k.res(R.drawable.w_bg_l, R.drawable.w_btn_l, R.drawable.w_seg_off_l, R.color.text_l, R.color.muted_l); break;
                case 'd': k.res(R.drawable.w_bg_d, R.drawable.w_btn_d, R.drawable.w_seg_off_d, R.color.text_d, R.color.muted_d); break;
                case 'c':
                    k.res(R.drawable.w_bg_tint, R.drawable.w_btn_tint, R.drawable.w_seg_tint, 0, 0);
                    k.bgTint = WTheme.bg(c);
                    k.btnTint = WTheme.surface(c);
                    k.segTint = WTheme.border(c);
                    k.textInt = WTheme.text(c);
                    k.mutedInt = WTheme.muted(c);
                    break;
                default: k.res(R.drawable.w_bg_a, R.drawable.w_btn_a, R.drawable.w_seg_off_a, R.color.text_a, R.color.muted_a);
            }
            return k;
        }

        private void res(int bg, int btn, int segOff, int text, int muted) {
            this.bg = bg; this.btn = btn; this.segOff = segOff; this.text = text; this.muted = muted;
        }

        void bg(RemoteViews v, int id, int drawable, Integer tint) {
            if (tint != null && Build.VERSION.SDK_INT < Build.VERSION_CODES.S) {
                v.setInt(id, "setBackgroundColor", tint); // no tinting before 12: square fill
                return;
            }
            v.setInt(id, "setBackgroundResource", drawable);
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
                v.setColorStateList(id, "setBackgroundTintList",
                        tint == null ? null : android.content.res.ColorStateList.valueOf(tint));
            }
        }

        /** Accent fill: tinted white base on 12+, the red resource before that. */
        void accentBg(RemoteViews v, int id, int tintable, int redFallback) {
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) bg(v, id, tintable, accent);
            else v.setInt(id, "setBackgroundResource", redFallback);
        }

        /** Text colour: explicit for custom; otherwise a resource the launcher resolves
         *  (so the system theme switches live on 12+). */
        void text(RemoteViews v, Context c, int id, int colorRes, Integer colorInt) {
            if (colorInt != null) v.setTextColor(id, colorInt);
            else if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) v.setColor(id, "setTextColor", colorRes);
            else v.setTextColor(id, c.getColor(colorRes));
        }
    }

    private static int piFlags() {
        int f = PendingIntent.FLAG_UPDATE_CURRENT;
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) f |= PendingIntent.FLAG_IMMUTABLE;
        return f;
    }

    private static PendingIntent priorityPI(Context context, int value) {
        Intent i = new Intent(context, QudelixWidget.class)
                .setAction(RfcommService.ACTION_PRIORITY)
                .putExtra(RfcommService.EXTRA_VALUE, value);
        return PendingIntent.getBroadcast(context, 100 + value, i, piFlags());
    }

    private static PendingIntent refreshPI(Context context) {
        Intent i = new Intent(context, QudelixWidget.class).setAction(RfcommService.ACTION_REFRESH);
        return PendingIntent.getBroadcast(context, 300, i, piFlags());
    }

    private static PendingIntent volumePI(Context context, int step) {
        Intent i = new Intent(context, QudelixWidget.class)
                .setAction(RfcommService.ACTION_VOLUME)
                .putExtra(RfcommService.EXTRA_VALUE, step);
        return PendingIntent.getBroadcast(context, 200 + (step > 0 ? 1 : 0), i, piFlags());
    }
}
