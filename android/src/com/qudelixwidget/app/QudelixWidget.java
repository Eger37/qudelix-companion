package com.qudelixwidget.app;

import android.app.PendingIntent;
import android.appwidget.AppWidgetManager;
import android.appwidget.AppWidgetProvider;
import android.content.ComponentName;
import android.content.Context;
import android.content.Intent;
import android.os.Build;
import android.widget.RemoteViews;

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
        // Button taps go to the non-exported TapReceiver, never here: this provider
        // is exported, so any app could send it a broadcast.
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
        if (ids == null || ids.length == 0) return;
        // The views don't depend on the widget id: build once for all instances.
        mgr.updateAppWidget(ids, buildViews(context));
    }

    private static RemoteViews buildViews(Context context) {
        RemoteViews v = new RemoteViews(context.getPackageName(), R.layout.widget);
        // Two-line title like the PC widget: "QDLX ⚙" / "5K v…" (both shrink to fit).
        v.setTextViewText(R.id.subtitle, "5K  v" + Updater.installedVersionName(context));
        Skin k = Skin.of(context);
        k.bg(v, R.id.root, k.bg, k.bgTint);
        for (int id : new int[]{R.id.title, R.id.subtitle, R.id.vol, R.id.btn_vol_dn, R.id.btn_vol_up,
                R.id.btn_refresh, R.id.btn_disconnect}) {
            k.text(v, context, id, k.text, k.textInt);
        }
        k.text(v, context, R.id.playing, k.muted, k.mutedInt);
        k.tint(v, context, R.id.btn_settings, k.muted, k.mutedInt);
        k.tint(v, context, R.id.btn_output, k.text, k.textInt);
        // ⚙ → the app's settings (the same screen as the widget's reconfigure).
        v.setOnClickPendingIntent(R.id.btn_settings, PendingIntent.getActivity(context, 800,
                new Intent(context, MainActivity.class).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK),
                PendingIntent.FLAG_UPDATE_CURRENT | PendingIntent.FLAG_IMMUTABLE));
        for (int id : new int[]{R.id.btn_vol_dn, R.id.btn_vol_up, R.id.btn_output, R.id.btn_refresh, R.id.btn_disconnect}) {
            k.bg(v, id, k.btn, k.btnTint);
        }
        // Unlink turns accent (and reads "Link") while this phone is unlinked.
        boolean audioOff = Prefs.getPhoneAudioOff(context);
        if (audioOff) {
            k.accentBg(v, R.id.btn_phone_audio, R.drawable.w_btn_tint, R.drawable.w_btn_on);
            v.setTextColor(R.id.btn_phone_audio, k.onAccent);
        } else {
            k.bg(v, R.id.btn_phone_audio, k.btn, k.btnTint);
            k.text(v, context, R.id.btn_phone_audio, k.text, k.textInt);
        }
        v.setTextViewText(R.id.btn_phone_audio, audioOff ? "Link" : "Unlink");
        // 🔊 → Android's own output picker (apps can't switch the output themselves).
        v.setOnClickPendingIntent(R.id.btn_output, PendingIntent.getActivity(context, 700,
                new Intent(context, OutputActivity.class).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK),
                PendingIntent.FLAG_UPDATE_CURRENT | PendingIntent.FLAG_IMMUTABLE));

        v.setOnClickPendingIntent(R.id.btn_usb, priorityPI(context, 0));
        v.setOnClickPendingIntent(R.id.btn_bt1, priorityPI(context, 1));
        v.setOnClickPendingIntent(R.id.btn_bt2, priorityPI(context, 2));
        v.setOnClickPendingIntent(R.id.btn_vol_dn, volumePI(context, -1));
        v.setOnClickPendingIntent(R.id.btn_vol_up, volumePI(context, +1));
        v.setOnClickPendingIntent(R.id.btn_refresh, refreshPI(context));
        // Unlink = this phone drops its music + calls links to the 5K (the 5K moves on to the
        // other source); Link brings it back.
        Intent pa = new Intent(context, TapReceiver.class)
                .setAction(RfcommService.ACTION_PHONE_AUDIO)
                .putExtra(RfcommService.EXTRA_VALUE, audioOff ? RfcommService.LINK : RfcommService.UNLINK);
        v.setOnClickPendingIntent(R.id.btn_phone_audio, PendingIntent.getBroadcast(context, 600, pa, piFlags()));
        // Disconnect = drop every Bluetooth link to the 5K (audio + calls); then the
        // button reads "Connect" (accent) and brings everything back (LINK does both).
        boolean gone = Prefs.getPhoneDisconnected(context);
        v.setTextViewText(R.id.btn_disconnect, gone ? "Connect" : "Disconnect");
        if (gone) {
            k.accentBg(v, R.id.btn_disconnect, R.drawable.w_btn_tint, R.drawable.w_btn_on);
            v.setTextColor(R.id.btn_disconnect, k.onAccent);
        }
        Intent dc = new Intent(context, TapReceiver.class)
                .setAction(RfcommService.ACTION_PHONE_AUDIO)
                .putExtra(RfcommService.EXTRA_VALUE, gone ? RfcommService.LINK : RfcommService.DISCONNECT);
        v.setOnClickPendingIntent(R.id.btn_disconnect, PendingIntent.getBroadcast(context, 601, dc, piFlags()));
        // A tap anywhere that isn't a button (background, title, status, "Playing")
        // refreshes too: any touch shows the current state.
        for (int id : new int[]{R.id.root, R.id.title, R.id.playing}) {
            v.setOnClickPendingIntent(id, refreshPI(context));
        }
        // Tap the dB readout → volume popup (slider + hold-to-repeat; a widget
        // itself can't get press-and-hold, the launcher takes long-press).
        v.setOnClickPendingIntent(R.id.vol, PendingIntent.getActivity(context, 400,
                new Intent(context, VolumeActivity.class).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK),
                PendingIntent.FLAG_UPDATE_CURRENT | PendingIntent.FLAG_IMMUTABLE));

        int pri = Prefs.getPriority(context);
        int[] priIds = {R.id.btn_usb, R.id.btn_bt1, R.id.btn_bt2};
        // Text of each priority button (Bluetooth ones: the number), and their icons.
        int[] priText = {R.id.btn_usb, R.id.lbl_bt1, R.id.lbl_bt2};
        int[] priIcon = {0, R.id.ic_bt1, R.id.ic_bt2};
        for (int i = 0; i < 3; i++) {
            boolean on = pri == i;
            if (on) {
                k.accentBg(v, priIds[i], R.drawable.w_btn_tint, R.drawable.w_btn_on);
                v.setTextColor(priText[i], k.onAccent);
                if (priIcon[i] != 0) k.tint(v, context, priIcon[i], 0, k.onAccent);
            } else {
                k.bg(v, priIds[i], k.btn, k.btnTint);
                k.text(v, context, priText[i], k.text, k.textInt);
                if (priIcon[i] != 0) k.tint(v, context, priIcon[i], k.text, k.textInt);
            }
        }

        v.setTextViewText(R.id.vol, Prefs.hasVolume(context)
                ? Proto.db(Prefs.getVolumeN(context)) : "— dB");
        // Volume bar: 12 segments of 5 dB; filled up to the current level, tap = jump there.
        int volN = Prefs.getVolumeN(context);
        boolean known = Prefs.hasVolume(context);
        for (int i = 0; i < SEG_IDS.length; i++) {
            int segN = segmentN(i);
            if (known && volN >= segN - 150) k.accentBg(v, SEG_IDS[i], R.drawable.w_seg_tint, R.drawable.w_seg_on);
            else k.bg(v, SEG_IDS[i], k.segOff, k.segTint);
            Intent seg = new Intent(context, TapReceiver.class)
                    .setAction(RfcommService.ACTION_VOLUME_SET)
                    .putExtra(RfcommService.EXTRA_VALUE, segN);
            v.setOnClickPendingIntent(SEG_IDS[i], PendingIntent.getBroadcast(context, 500 + i, seg, piFlags()));
        }
        // Bottom line: the status while something happens (RfcommService clears it
        // when it lets the 5K go), otherwise what is playing — same as the PC widget.
        String status = Prefs.getStatus(context);
        String playing = Prefs.getPlaying(context);
        v.setTextViewText(R.id.playing, !status.isEmpty() ? status
                : "Playing: " + (playing != null ? playing : "?"));
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
        int bg, btn, segOff, text, muted, panel;       // drawable / colour resources
        Integer bgTint, btnTint, segTint;              // custom theme only
        Integer textInt, mutedInt, panelInt;           // custom theme only
        int accent, onAccent;

        static Skin of(Context c) {
            Skin k = new Skin();
            k.accent = WTheme.accent(c);
            k.onAccent = WTheme.onAccent(c);
            switch (WTheme.widgetVariant(c)) {
                case 'l': k.res(R.drawable.w_bg_l, R.drawable.w_btn_l, R.drawable.w_seg_off_l, R.color.text_l, R.color.muted_l, R.color.panel_l); break;
                case 'd': k.res(R.drawable.w_bg_d, R.drawable.w_btn_d, R.drawable.w_seg_off_d, R.color.text_d, R.color.muted_d, R.color.panel_d); break;
                case 'c':
                    k.res(R.drawable.w_bg_tint, R.drawable.w_btn_tint, R.drawable.w_seg_tint, 0, 0, 0);
                    k.bgTint = WTheme.bg(c);
                    k.btnTint = WTheme.surface(c);
                    k.segTint = WTheme.border(c);
                    k.textInt = WTheme.text(c);
                    k.mutedInt = WTheme.muted(c);
                    k.panelInt = WTheme.surface(c);
                    break;
                default: k.res(R.drawable.w_bg_a, R.drawable.w_btn_a, R.drawable.w_seg_off_a, R.color.text_a, R.color.muted_a, R.color.panel_a);
            }
            return k;
        }

        private void res(int bg, int btn, int segOff, int text, int muted, int panel) {
            this.bg = bg; this.btn = btn; this.segOff = segOff; this.text = text; this.muted = muted; this.panel = panel;
        }

        /** Image colour (white vector shapes, icons): explicit int for custom / accent;
         *  otherwise a resource the launcher resolves, so the system theme switches live. */
        void tint(RemoteViews v, Context c, int id, int colorRes, Integer colorInt) {
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
                if (colorInt != null) {
                    v.setColorStateList(id, "setImageTintList", android.content.res.ColorStateList.valueOf(colorInt));
                } else {
                    v.setColorStateList(id, "setImageTintList", colorRes);
                }
            } else {
                v.setInt(id, "setColorFilter", colorInt != null ? colorInt : c.getColor(colorRes));
            }
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
        Intent i = new Intent(context, TapReceiver.class)
                .setAction(RfcommService.ACTION_PRIORITY)
                .putExtra(RfcommService.EXTRA_VALUE, value);
        return PendingIntent.getBroadcast(context, 100 + value, i, piFlags());
    }

    private static PendingIntent refreshPI(Context context) {
        Intent i = new Intent(context, TapReceiver.class).setAction(RfcommService.ACTION_REFRESH);
        return PendingIntent.getBroadcast(context, 300, i, piFlags());
    }

    private static PendingIntent volumePI(Context context, int step) {
        Intent i = new Intent(context, TapReceiver.class)
                .setAction(RfcommService.ACTION_VOLUME)
                .putExtra(RfcommService.EXTRA_VALUE, step);
        return PendingIntent.getBroadcast(context, 200 + (step > 0 ? 1 : 0), i, piFlags());
    }
}
