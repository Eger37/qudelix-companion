package com.qudelixwidget.app;

import android.annotation.SuppressLint;
import android.app.Notification;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.app.Service;
import android.bluetooth.BluetoothAdapter;
import android.bluetooth.BluetoothDevice;
import android.bluetooth.BluetoothManager;
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
import java.util.Set;
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
        if (ACTION_PRIORITY.equals(action)) {
            Prefs.setPriority(this, value); // optimistic; the 5K's echo confirms it
            QudelixWidget.update(this);
        }
        final long seq = ACTION_VOLUME_SET.equals(action) ? volumeSeq.incrementAndGet() : 0;
        io.execute(() -> {
            if (seq != 0 && seq != volumeSeq.get()) return; // superseded by a newer value
            // Executor tasks swallow exceptions — surface every failure instead
            // of leaving the widget stuck on "sending…".
            try {
                handle(action, value);
            } catch (Throwable t) {
                Dbg.log(this, "handle failed: " + android.util.Log.getStackTraceString(t));
                QudelixWidget.pushStatus(this, "error: " + t.getClass().getSimpleName());
                closeSession();
                scheduleRelease();
            }
        });
        return START_NOT_STICKY;
    }

    // ── command handling (io thread) ───────────────────────────────────────────

    private void handle(String action, int value) {
        Dbg.log(this, "handle " + action + " value=" + value);
        cancelRelease();
        QudelixWidget.pushStatus(this, ACTION_REFRESH.equals(action) ? "reading…" : "sending…");
        boolean fresh = session == null || !session.alive;
        if (!ensureSession()) {
            scheduleRelease();
            return;
        }
        try {
            if (ACTION_PRIORITY.equals(action)) {
                session.write(Proto.setPriority(value));
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
                    QudelixWidget.pushStatus(this, capped ? "max +" + Proto.MAX_UP_N / 60 + " dB per tap" : "✓");
                }
            } else if (ACTION_REFRESH.equals(action)) {
                if (!fresh) session.write(Proto.REQ_INIT_DATA); // a new session already asked
                sleep(1200);
                QudelixWidget.pushStatus(this, "updated");
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
        BluetoothDevice dev = findTarget();
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
        release = io.schedule(() -> {
            if (session != null) Dbg.log(this, "idle — released the 5K");
            closeSession();
            stopForeground(STOP_FOREGROUND_REMOVE);
            stopSelf(lastStartId);
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
        NotificationManager nm = (NotificationManager) getSystemService(NOTIFICATION_SERVICE);
        if (nm != null && nm.getNotificationChannel(CHANNEL_ID) == null) {
            NotificationChannel ch = new NotificationChannel(CHANNEL_ID, "Qudelix", NotificationManager.IMPORTANCE_LOW);
            ch.setShowBadge(false);
            nm.createNotificationChannel(ch);
        }
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

    private BluetoothAdapter adapter() {
        BluetoothManager bm = (BluetoothManager) getSystemService(Context.BLUETOOTH_SERVICE);
        return bm == null ? null : bm.getAdapter();
    }

    /** The device picked in settings, else the first paired "Qudelix-5K…". */
    private BluetoothDevice findTarget() {
        BluetoothAdapter a = adapter();
        if (a == null || !a.isEnabled()) return null;
        Set<BluetoothDevice> bonded = a.getBondedDevices();
        if (bonded == null) return null;
        String want = Prefs.getDevice(this);
        for (BluetoothDevice d : bonded) {
            if (!want.isEmpty()) {
                if (want.equals(d.getAddress())) return d;
                continue;
            }
            String name = d.getName();
            if (name != null && name.startsWith(Proto.NAME_PREFIX)) return d;
        }
        return null;
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
