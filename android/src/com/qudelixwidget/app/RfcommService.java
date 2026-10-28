package com.qudelixwidget.app;

import android.annotation.SuppressLint;
import android.app.Notification;
import android.app.NotificationManager;
import android.app.Service;
import android.bluetooth.BluetoothDevice;
import android.bluetooth.BluetoothProfile;
import android.bluetooth.BluetoothSocket;
import android.content.Context;
import android.content.Intent;
import android.content.pm.PackageManager;
import android.content.pm.ServiceInfo;
import android.os.Build;
import android.os.IBinder;

import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.util.UUID;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.Executors;
import java.util.concurrent.ScheduledExecutorService;
import java.util.concurrent.ScheduledFuture;
import java.util.concurrent.TimeUnit;

/**
 * Talks to the 5K over classic RFCOMM — the same hardware-verified method as the
 * Windows widget (windows/src/worker.rs + rfcomm.rs):
 *  - one GAIA session per burst of taps, released RELEASE_MS after the last tap
 *    (the 5K has ONE control channel for all hosts — holding it locks out the PC);
 *  - connect exactly like the official Qudelix app: SDP lookup of the Serial Port
 *    service (UUID 0x1101 — the 5K's record is named "GAIA"), one connect. NO
 *    channel scanning: poking other channels (HFP/HSP) and aborting half-open
 *    connects knocked out the 5K's GAIA server for every host (v1.0.7);
 *  - a reader thread drains everything the 5K sends (unread data makes it drop the link).
 * Foreground service because widget taps start it from the background.
 */
@SuppressLint("MissingPermission") // checked in hasBtPermission(Context)
public class RfcommService extends Service {

    public static final String ACTION_PRIORITY = "com.qudelixwidget.app.PRIORITY";
    public static final String ACTION_VOLUME   = "com.qudelixwidget.app.VOLUME";
    public static final String ACTION_REFRESH  = "com.qudelixwidget.app.REFRESH";
    /** Absolute volume (value = n, dB*60) — from the volume popup's slider / hold-repeat. */
    public static final String ACTION_VOLUME_SET = "com.qudelixwidget.app.VOLUME_SET";
    /** This phone's link to the 5K: value UNLINK (music + calls off), LINK / CONNECT (back on), DISCONNECT (everything off). */
    public static final String ACTION_PHONE_AUDIO = "com.qudelixwidget.app.PHONE_AUDIO";
    public static final String EXTRA_VALUE     = "value"; // priority 0/1/2, or volume step -1/+1

    private static final String CHANNEL_ID = "qudelix";
    private static final int NOTIF_ID = 1;
    private static final long RELEASE_MS = 3000;
    /** Safety net only: a connect() still pending after this is aborted. */
    private static final long CONNECT_TIMEOUT_MS = 10000;
    /** Serial Port Profile — what the official app looks up (btsnoop: SDP for 0x1101 → record "GAIA"). */
    private static final UUID SPP_UUID = UUID.fromString("00001101-0000-1000-8000-00805f9b34fb");

    /** All Bluetooth I/O happens on this one thread, in order. */
    private final ScheduledExecutorService io = Executors.newSingleThreadScheduledExecutor();
    /** Connect watchdogs (separate from io, which is blocked inside connect()). */
    private final ScheduledExecutorService watchdogs = Executors.newSingleThreadScheduledExecutor();
    private ScheduledFuture<?> release;
    private Session session;
    private final DevState state = new DevState();
    /** Latest start id: release only stops the service if no newer tap arrived. */
    private volatile int lastStartId;
    /** Absolute-volume requests arrive in bursts (slider, hold-repeat): only the newest is sent. */
    private final java.util.concurrent.atomic.AtomicLong volumeSeq = new java.util.concurrent.atomic.AtomicLong();

    @Override
    public int onStartCommand(Intent intent, int flags, int startId) {
        lastStartId = startId;
        // Callers check the permission first: on Android 14+ a connectedDevice
        // foreground service can't start without BLUETOOTH_CONNECT.
        if (!hasBtPermission(this) || !goForeground()) {
            QudelixWidget.pushStatus(this, "open app → grant Bluetooth");
            stopSelf(startId);
            return START_NOT_STICKY;
        }
        if (intent == null || intent.getAction() == null) {
            scheduleRelease();
            return START_NOT_STICKY;
        }
        final String action = intent.getAction();
        final int value = intent.getIntExtra(EXTRA_VALUE, 0);
        if (ACTION_PHONE_AUDIO.equals(action)) {
            io.execute(() -> {
                try {
                    setPhoneAudio(value);
                } catch (Throwable t) {
                    Dbg.log(this, "phone audio failed: " + android.util.Log.getStackTraceString(t));
                    QudelixWidget.pushStatus(this, "error: " + t.getClass().getSimpleName());
                    closeSession();
                    scheduleRelease();
                }
            });
            return START_NOT_STICKY;
        }
        final int shownBefore = Prefs.getPriority(this);
        if (ACTION_PRIORITY.equals(action)) {
            Prefs.setPriority(this, value); // optimistic; the 5K's echo confirms it
            QudelixWidget.update(this);
        }
        final long seq = ACTION_VOLUME_SET.equals(action) ? volumeSeq.incrementAndGet() : 0;
        io.execute(() -> {
            if (seq != 0 && seq != volumeSeq.get()) return; // superseded by a newer value
            // Executor tasks swallow exceptions — surface every failure instead
            // of leaving the widget stuck on "sending…".
            boolean sent = false;
            try {
                sent = handle(action, value);
            } catch (Throwable t) {
                Dbg.log(this, "handle failed: " + android.util.Log.getStackTraceString(t));
                QudelixWidget.pushStatus(this, "error: " + t.getClass().getSimpleName());
                closeSession();
                scheduleRelease();
            }
            if (!sent && ACTION_PRIORITY.equals(action) && Prefs.getPriority(this) == value) {
                // Not delivered: show what was selected before, not the failed choice.
                Prefs.setPriority(this, shownBefore);
                QudelixWidget.update(this);
            }
        });
        return START_NOT_STICKY;
    }

    // ── command handling (io thread) ───────────────────────────────────────────

    /** Runs one command; true if it reached the 5K. */
    private boolean handle(String action, int value) {
        Dbg.log(this, "handle " + action + " value=" + value);
        cancelRelease();
        // Volume taps show no status: the dB readout already shows the result.
        boolean volume = ACTION_VOLUME.equals(action) || ACTION_VOLUME_SET.equals(action);
        if (!volume) QudelixWidget.pushStatus(this, ACTION_REFRESH.equals(action) ? "updating…" : "sending…");
        // Refresh also re-reads this phone's links (Unlink/Link, Disconnect/Connect) —
        // before the session, whose own radio link would count as "connected".
        if (ACTION_REFRESH.equals(action) && (session == null || !session.alive)) PhoneLink.refresh(this);
        boolean fresh = session == null || !session.alive;
        if (!ensureSession()) {
            scheduleRelease();
            return false;
        }
        boolean sent = false;
        try {
            if (ACTION_PRIORITY.equals(action)) {
                session.write(Proto.setPriority(value));
                sent = true;
                QudelixWidget.pushStatus(this, "✓");
            } else if (ACTION_VOLUME.equals(action) || ACTION_VOLUME_SET.equals(action)) {
                // Always move from the 5K's real volume (it arrives with the init
                // data), never from the cached one: a stale cache could make a
                // "+" tap a big jump up.
                Integer cur = realVolume();
                if (cur == null) {
                    QudelixWidget.pushStatus(this, "volume unknown — tap again");
                } else {
                    int want = ACTION_VOLUME.equals(action) ? cur + value * Prefs.getStepN(this) : value;
                    int n = Proto.safeVolume(cur, want);
                    boolean capped = n != Proto.clampVol(want);
                    if (capped) Dbg.log(this, "volume capped: " + cur + " → " + n + " (asked " + want + ")");
                    synchronized (state) { state.volumeN = n; }
                    Prefs.setVolumeN(this, n);
                    session.write(Proto.setVolume(n));
                    if (capped) QudelixWidget.pushStatus(this, "max +" + Proto.MAX_UP_N / 60 + " dB per tap");
                    else QudelixWidget.update(this);
                }
            } else if (ACTION_REFRESH.equals(action)) {
                if (!fresh) session.write(Proto.REQ_INIT_DATA); // a new session already asked
                sleep(1200);
                QudelixWidget.pushStatus(this, ""); // no "updated": straight to what's playing
            }
            if (!ACTION_REFRESH.equals(action)) {
                // Every tap also reads the state back, so the widget shows what
                // the 5K really did (the reader thread applies the reply).
                session.write(Proto.REQ_INIT_DATA);
            }
        } catch (IOException e) {
            Dbg.log(this, "write failed: " + e);
            closeSession();
            QudelixWidget.pushStatus(this, "link lost — tap again");
        }
        scheduleRelease();
        return sent;
    }

    // ── phone audio link (io thread) ─────────────────────────────────────────

    /**
     * UNLINK drops this phone's music and calls links (A2DP + HFP) to the 5K, DISCONNECT
     * the whole Bluetooth connection; LINK / CONNECT bring music and calls back. The 5K
     * has no "drop host X" command, so the phone does it itself — via hidden Bluetooth
     * APIs (PhoneLink). The 5K's priority moves to the source that stays BEFORE the drop
     * (asking the 5K afterwards would bring the link back), like the PC. Every result is
     * checked against what Android reports before the status claims it.
     */
    private void setPhoneAudio(int mode) {
        Dbg.log(this, "phone audio " + NAMES[mode]);
        cancelRelease();
        BluetoothDevice dev = PhoneLink.target(this);
        if (dev == null) {
            closeSession();
            QudelixWidget.pushStatus(this, "5K not paired / BT off");
            scheduleRelease();
            return;
        }
        QudelixWidget.pushStatus(this, NAMES[mode] + "ing…");
        if (mode == LINK || mode == CONNECT) {
            closeSession();
            String done = link(dev, mode);
            readBack();
            QudelixWidget.pushStatus(this, done);
            scheduleRelease();
            return;
        }
        boolean[] live = PhoneLink.read(this, dev);
        if (!live[0] && !live[1] && (mode == UNLINK || !live[2])) {
            PhoneLink.show(this, live);
            if (live[2]) readBack();
            QudelixWidget.pushStatus(this, mode == UNLINK ? "phone already unlinked" : "phone already disconnected");
            scheduleRelease();
            return;
        }
        DevState before = new DevState();
        String moved = pointPriorityAway(live[0], before);
        closeSession();
        String fail = drop(dev, mode);
        // Unlink: music and calls gone. Disconnect: the radio link too (if Android lets it).
        boolean full = mode == DISCONNECT && fail == null;
        boolean[] now = PhoneLink.waitFor(this, dev, l -> !l[0] && !l[1] && (!full || !l[2]), full ? 8000 : 5000);
        PhoneLink.show(this, now);
        Dbg.log(this, NAMES[mode] + ": music=" + now[0] + " calls=" + now[1] + " radio=" + now[2]);
        String status;
        if (now[0]) {
            status = "couldn't " + NAMES[mode] + " phone" + (fail != null ? " · " + fail : "");
        } else {
            if (moved == null && before.hostAddr != null) moved = pointPriorityAfter(before);
            String done = now[1] ? "music off · calls still on"
                    : mode == UNLINK ? "phone unlinked"
                    : now[2] ? "music + calls off · link stays" : "phone disconnected";
            status = moved != null && !moved.isEmpty() ? done + " · → " + moved : done;
        }
        // Not after a full disconnect: a session would bring the radio link back.
        if (!full || now[2]) readBack();
        QudelixWidget.pushStatus(this, status);
        scheduleRelease();
    }

    /** After a link action: read the 5K's state again ("updating…"), so the widget
     *  shows what the 5K does now. The caller then shows the action's result. */
    private void readBack() {
        QudelixWidget.pushStatus(this, "updating…");
        boolean fresh = session == null || !session.alive;
        if (!ensureSession()) return;
        try {
            if (!fresh) session.write(Proto.REQ_INIT_DATA); // a new session already asked
            sleep(1200);
        } catch (IOException e) {
            Dbg.log(this, "read-back failed: " + e);
            closeSession();
        }
    }

    /** Modes of ACTION_PHONE_AUDIO (EXTRA_VALUE). */
    static final int UNLINK = 0, LINK = 1, DISCONNECT = 2, CONNECT = 3;
    private static final String[] NAMES = {"unlink", "link", "disconnect", "connect"};
    private static final String[] SOURCES = {"USB", "Bluetooth 1", "Bluetooth 2"};

    /** Drop music + calls (Unlink), or everything (Disconnect: the full disconnect if the
     *  CDM association allows it, else per profile). Null = Android took it, else why not. */
    private String drop(BluetoothDevice dev, int mode) {
        if (mode == DISCONNECT) {
            String all = PhoneLink.allProfiles(this, dev, false);
            if (all == null) return null;
            Dbg.log(this, "full disconnect: " + all + " — per profile instead");
            String audio = PhoneLink.profileCall(this, dev, BluetoothProfile.A2DP, "disconnect");
            String calls = PhoneLink.profileCall(this, dev, BluetoothProfile.HEADSET, "disconnect");
            Dbg.log(this, "disconnect: audio " + (audio == null ? "ok" : audio) + ", calls " + (calls == null ? "ok" : calls));
            return all; // the radio link may stay: say so
        }
        String audio = PhoneLink.profileCall(this, dev, BluetoothProfile.A2DP, "disconnect");
        String calls = PhoneLink.profileCall(this, dev, BluetoothProfile.HEADSET, "disconnect");
        Dbg.log(this, "unlink: audio " + (audio == null ? "ok" : audio) + ", calls " + (calls == null ? "ok" : calls));
        return audio != null ? audio : calls;
    }

    /** Link / Connect: every profile at once (CDM association), else music and calls one
     *  by one; then wait until Android shows them. Returns the status line. */
    private String link(BluetoothDevice dev, int mode) {
        String done = mode == CONNECT ? "phone connected" : "phone linked";
        String fail = PhoneLink.allProfiles(this, dev, true);
        if (fail != null) {
            fail = PhoneLink.profileCall(this, dev, BluetoothProfile.A2DP, "connect");
            String calls = PhoneLink.profileCall(this, dev, BluetoothProfile.HEADSET, "connect");
            if (calls != null) Dbg.log(this, "calls: " + calls);
        }
        boolean[] now = PhoneLink.waitFor(this, dev, l -> l[0] && l[1], 10000);
        PhoneLink.show(this, now);
        Dbg.log(this, NAMES[mode] + ": music=" + now[0] + " calls=" + now[1]);
        if (!now[0]) return "couldn't " + NAMES[mode] + " phone" + (fail != null ? " · " + fail : "");
        return now[1] ? done : done + " · calls missing";
    }

    /**
     * Before a drop: ask the 5K which hosts hold its A2DP slots and point the priority at
     * the source that stays once this phone leaves. Needs this phone's own address (learned,
     * see DevState.phoneAddr); if it can't be told yet, leaves `before` filled for
     * pointPriorityAfter. Returns the new priority's name, "" if it stays, null if unknown.
     */
    private String pointPriorityAway(boolean musicOn, DevState before) {
        if (!ensureSession()) return "";
        try {
            synchronized (state) { state.hostAddr = null; } // only trust a fresh reply
            session.write(Proto.REQ_CONN_STATUS);
            waitFor(() -> { synchronized (state) { return state.hostAddr != null; } }, 1500);
            Integer target, cur;
            synchronized (state) {
                if (state.hostAddr == null) return "";
                long self = Prefs.getSelfAddr(this);
                if (self < 0) {
                    Long p = state.phoneAddr(musicOn);
                    if (p != null) {
                        self = p;
                        Prefs.setSelfAddr(this, p);
                        Dbg.log(this, "this phone on the 5K = " + DevState.addrString(p));
                    }
                }
                // Phone not on A2DP: every A2DP host listed is another one (-1 matches none).
                target = self >= 0 ? state.sourceWithout(self) : !musicOn ? state.sourceWithout(-1) : null;
                cur = state.priority;
                if (target == null) {
                    DevState c = state.connCopy();
                    before.hostAddr = c.hostAddr;
                    before.hostBits = c.hostBits;
                    Dbg.log(this, "can't tell this phone's slot yet — priority after the drop");
                    return null;
                }
            }
            Dbg.log(this, "source without this phone " + target + ", priority " + cur);
            if (target.equals(cur)) return "";
            session.write(Proto.setPriority(target));
            Prefs.setPriority(this, target);
            QudelixWidget.update(this);
            return SOURCES[target];
        } catch (IOException e) {
            Dbg.log(this, "write failed: " + e);
            closeSession();
            return "";
        }
    }

    /** Fallback (once, until this phone's address is learned): after the drop, read the
     *  slot that's left — which also tells which host was this phone. This brings the radio
     *  link back for a moment; Android lets it go once the session is released. */
    private String pointPriorityAfter(DevState before) {
        sleep(2500); // the 5K needs a moment to notice the link is gone
        synchronized (state) { state.hostAddr = null; state.a2dp = null; }
        if (!ensureSession()) return "";
        try {
            session.write(Proto.REQ_CONN_STATUS);
            waitFor(() -> { synchronized (state) { return state.hostAddr != null; } }, 1500);
            Integer target, cur;
            synchronized (state) {
                Long me = before.droppedHost(state);
                if (me != null) {
                    Prefs.setSelfAddr(this, me);
                    Dbg.log(this, "this phone on the 5K = " + DevState.addrString(me));
                }
                target = state.remainingSource();
                cur = state.priority;
            }
            Dbg.log(this, "after the drop: remaining source " + target + ", priority " + cur);
            if (target == null || target.equals(cur)) return "";
            session.write(Proto.setPriority(target));
            Prefs.setPriority(this, target);
            session.write(Proto.REQ_INIT_DATA);
            return SOURCES[target];
        } catch (IOException e) {
            Dbg.log(this, "write failed: " + e);
            closeSession();
            return "";
        }
    }

    /** The volume the 5K reported in this session (waits briefly for the init data), or null. */
    private Integer realVolume() {
        waitFor(() -> { synchronized (state) { return state.volumeN != null; } }, 1500);
        synchronized (state) { return state.volumeN; }
    }

    /** Reuse the open session or open a new one (SDP lookup, one connect). */
    private boolean ensureSession() {
        if (session != null && session.alive) return true;
        closeSession();
        BluetoothDevice dev = PhoneLink.target(this);
        if (dev == null) {
            QudelixWidget.pushStatus(this, "5K not paired / BT off");
            return false;
        }
        // (No cancelDiscovery(): it needs BLUETOOTH_SCAN on Android 12+ and threw
        // SecurityException here, silently killing the task. We never discover.)
        try {
            Session s = tryOpen(dev.createRfcommSocketToServiceRecord(SPP_UUID), "SPP/GAIA");
            if (s != null) { session = s; return true; }
        } catch (IOException e) {
            Dbg.log(this, "SPP socket: " + e);
        }
        QudelixWidget.pushStatus(this, "5K busy — close Qudelix app / PC");
        return false;
    }

    /** Connect + the official app's handshake; null if this isn't a live GAIA endpoint. */
    private Session tryOpen(BluetoothSocket sock, String label) {
        long t0 = System.currentTimeMillis();
        ScheduledFuture<?> watchdog = watchdogs.schedule(() -> {
            Dbg.log(this, label + ": connect timeout — aborting");
            closeQuietly(sock);
        }, CONNECT_TIMEOUT_MS, TimeUnit.MILLISECONDS);
        try {
            sock.connect();
        } catch (IOException e) {
            Dbg.log(this, label + ": connect failed in " + (System.currentTimeMillis() - t0) + " ms: " + e.getMessage());
            closeQuietly(sock);
            return null;
        } finally {
            watchdog.cancel(false);
        }
        Dbg.log(this, label + ": connected in " + (System.currentTimeMillis() - t0) + " ms");
        Session s = new Session(sock);
        try {
            s.start();
            s.write(Proto.API_VERSION);
            sleep(150);
            s.write(Proto.REQ_INIT_DATA);
            sleep(150);
            s.write(Proto.REGISTER);
            if (!s.registered.await(2000, TimeUnit.MILLISECONDS)) throw new IOException("no register ACK");
            s.write(Proto.STOP_STREAM);
            Dbg.log(this, label + ": session open (registered)");
            return s;
        } catch (Exception e) {
            Dbg.log(this, label + ": handshake failed: " + e.getMessage());
            s.close();
            return null;
        }
    }

    private void closeSession() {
        if (session != null) {
            session.close();
            session = null;
        }
        // Another controller may change the volume while we're away: the next
        // session must read it again before any volume step.
        synchronized (state) { state.volumeN = null; }
    }

    // ── release after idle ─────────────────────────────────────────────────────

    private void cancelRelease() {
        if (release != null) release.cancel(false);
    }

    private void scheduleRelease() {
        cancelRelease();
        final int id = lastStartId;
        release = io.schedule(() -> {
            // A tap that arrived meanwhile is queued behind us and schedules its own
            // release: stopping now would drop it (and the service with it).
            if (id != lastStartId) return;
            if (session != null) Dbg.log(this, "idle — released the 5K");
            closeSession();
            QudelixWidget.pushStatus(this, ""); // bottom line back to "Playing"
            stopForeground(STOP_FOREGROUND_REMOVE);
            stopSelf(id);
        }, RELEASE_MS, TimeUnit.MILLISECONDS);
    }

    // ── one GAIA session ───────────────────────────────────────────────────────

    private final class Session {
        final BluetoothSocket sock;
        final CountDownLatch registered = new CountDownLatch(1);
        volatile boolean alive = true;
        private OutputStream out;

        Session(BluetoothSocket sock) { this.sock = sock; }

        void start() throws IOException {
            out = sock.getOutputStream();
            final InputStream in = sock.getInputStream();
            Thread reader = new Thread(() -> readLoop(in), "qudelix-rx");
            reader.setDaemon(true);
            reader.start();
        }

        /** Drain everything the 5K sends; feed frames into the shared state. */
        private void readLoop(InputStream in) {
            DevState.Splitter splitter = new DevState.Splitter();
            byte[] buf = new byte[1024];
            try {
                int r;
                while (alive && (r = in.read(buf)) > 0) {
                    for (byte[] f : splitter.push(buf, r)) {
                        if (Proto.isRegisterAck(f)) registered.countDown();
                        onFrame(f);
                    }
                }
            } catch (IOException ignored) {
                // socket closed
            }
            alive = false;
        }

        synchronized void write(byte[] f) throws IOException {
            if (!alive) throw new IOException("session closed");
            out.write(f);
            out.flush();
        }

        void close() {
            if (alive) {
                try { write(Proto.STOP_STREAM); } catch (IOException ignored) {} // leave device idle
            }
            alive = false;
            closeQuietly(sock);
        }
    }

    /** Called on the reader thread for every complete frame. */
    private void onFrame(byte[] f) {
        Integer pri, vol;
        String playing;
        synchronized (state) {
            state.apply(f);
            pri = state.priority;
            vol = state.volumeN;
            playing = state.playing;
        }
        boolean changed = false;
        if (pri != null && pri != Prefs.getPriority(this)) { Prefs.setPriority(this, pri); changed = true; }
        if (vol != null && vol != Prefs.getVolumeN(this)) { Prefs.setVolumeN(this, vol); changed = true; }
        if (playing != null && !playing.equals(Prefs.getPlaying(this))) { Prefs.setPlaying(this, playing); changed = true; }
        if (changed) QudelixWidget.update(this);
    }

    // ── plumbing ───────────────────────────────────────────────────────────────

    /** Must happen promptly after startForegroundService(); false if the system refused. */
    private boolean goForeground() {
        App.ensureChannel((NotificationManager) getSystemService(NOTIFICATION_SERVICE),
                CHANNEL_ID, "Qudelix", NotificationManager.IMPORTANCE_LOW, false);
        Notification n = new Notification.Builder(this, CHANNEL_ID)
                .setContentTitle("Qudelix 5K")
                .setContentText("Sending…")
                .setSmallIcon(R.drawable.ic_widget)
                .setOngoing(true)
                .build();
        try {
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
                startForeground(NOTIF_ID, n, ServiceInfo.FOREGROUND_SERVICE_TYPE_CONNECTED_DEVICE);
            } else {
                startForeground(NOTIF_ID, n);
            }
            Dbg.log(this, "foreground ok");
            return true;
        } catch (Exception e) {
            Dbg.log(this, "startForeground failed: " + e);
            return false;
        }
    }

    /** Check before starting this service: without it the foreground start fails on Android 14+. */
    static boolean hasBtPermission(Context c) {
        return Build.VERSION.SDK_INT < Build.VERSION_CODES.S
                || c.checkSelfPermission(android.Manifest.permission.BLUETOOTH_CONNECT) == PackageManager.PERMISSION_GRANTED;
    }

    /** Send this service one command; false (logged) if Android refused the start. Check hasBtPermission first. */
    static boolean start(Context c, String action, int value) {
        try {
            c.startForegroundService(new Intent(c, RfcommService.class).setAction(action).putExtra(EXTRA_VALUE, value));
            return true;
        } catch (Exception e) {
            Dbg.log(c, "service start (" + action + ") failed: " + e);
            return false;
        }
    }

    private interface Cond { boolean ok(); }

    private static void waitFor(Cond c, long ms) {
        long end = System.currentTimeMillis() + ms;
        while (!c.ok() && System.currentTimeMillis() < end) sleep(50);
    }

    private static void sleep(long ms) {
        try { Thread.sleep(ms); } catch (InterruptedException ignored) {}
    }

    private static void closeQuietly(BluetoothSocket s) {
        try { s.close(); } catch (IOException ignored) {}
    }

    @Override public IBinder onBind(Intent i) { return null; }

    @Override
    public void onDestroy() {
        closeSession();
        io.shutdownNow();
        watchdogs.shutdownNow();
        super.onDestroy();
    }
}
