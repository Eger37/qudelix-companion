package com.qudelixwidget.app;

import android.annotation.SuppressLint;
import android.bluetooth.BluetoothAdapter;
import android.bluetooth.BluetoothDevice;
import android.bluetooth.BluetoothManager;
import android.bluetooth.BluetoothProfile;
import android.companion.AssociationInfo;
import android.companion.CompanionDeviceManager;
import android.content.Context;
import android.content.pm.PackageManager;
import android.net.MacAddress;
import android.os.Build;

import java.util.Set;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;

/**
 * This phone's own Bluetooth links to the 5K: what is live (music = A2DP, calls = HFP,
 * radio = any link at all), and the hidden Android calls that drop / bring them back.
 * The widget's Unlink/Link and Disconnect/Connect labels follow what is read here,
 * never what was last pressed.
 *
 * The full connect/disconnect (BluetoothDevice.connect()/disconnect()) needs a
 * Companion Device Manager association with the 5K; without it Android throws
 * "does not have a CDM association" (seen on Android 17, 2026-10-18). LinkActivity
 * asks for it once. Blocking calls: never on the main thread.
 */
@SuppressLint("MissingPermission") // callers check RfcommService.hasBtPermission
final class PhoneLink {
    private PhoneLink() {}

    static BluetoothAdapter adapter(Context c) {
        BluetoothManager bm = (BluetoothManager) c.getSystemService(Context.BLUETOOTH_SERVICE);
        return bm == null ? null : bm.getAdapter();
    }

    /** The device picked in settings, else the first paired "Qudelix-5K…"; null if none / BT off. */
    static BluetoothDevice target(Context c) {
        BluetoothAdapter a = adapter(c);
        if (a == null || !a.isEnabled()) return null;
        Set<BluetoothDevice> bonded = a.getBondedDevices();
        if (bonded == null) return null;
        String want = Prefs.getDevice(c);
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

    // ── what is live ───────────────────────────────────────────────────────────

    /** What of `dev` is live on this phone: {music, calls, radio}. */
    static boolean[] read(Context c, BluetoothDevice dev) {
        boolean music = state(c, dev, BluetoothProfile.A2DP) == BluetoothProfile.STATE_CONNECTED;
        boolean calls = state(c, dev, BluetoothProfile.HEADSET) == BluetoothProfile.STATE_CONNECTED;
        boolean radio = music || calls;
        try {
            // Hidden BluetoothDevice.isConnected(): any link, the widget's own control channel too.
            radio |= Boolean.TRUE.equals(dev.getClass().getMethod("isConnected").invoke(dev));
        } catch (Exception e) {
            Dbg.log(c, "isConnected() unavailable: " + e);
        }
        return new boolean[]{music, calls, radio};
    }

    /** Show `live` on the widget: Unlink ↔ Link (music off), Disconnect ↔ Connect (nothing left). */
    static void show(Context c, boolean[] live) {
        boolean off = !live[0], gone = !live[0] && !live[1] && !live[2];
        if (off == Prefs.getPhoneAudioOff(c) && gone == Prefs.getPhoneDisconnected(c)) return;
        Dbg.log(c, "phone link: music=" + live[0] + " calls=" + live[1] + " radio=" + live[2]);
        Prefs.setPhoneAudioOff(c, off);
        Prefs.setPhoneDisconnected(c, gone);
        QudelixWidget.update(c);
    }

    /** Read and show; false if the 5K isn't paired or Bluetooth is off. */
    static boolean refresh(Context c) {
        BluetoothDevice dev = target(c);
        if (dev == null) return false;
        show(c, read(c, dev));
        return true;
    }

    /** Poll until `want` holds on {music, calls, radio}; returns the last reading. */
    static boolean[] waitFor(Context c, BluetoothDevice dev, java.util.function.Predicate<boolean[]> want, long ms) {
        long end = System.currentTimeMillis() + ms;
        boolean[] live = read(c, dev);
        while (!want.test(live) && System.currentTimeMillis() < end) {
            try { Thread.sleep(300); } catch (InterruptedException e) { break; }
            live = read(c, dev);
        }
        return live;
    }

    /** A profile's connection state for `dev`; -1 if Android wouldn't say. */
    private static int state(Context c, BluetoothDevice dev, int profile) {
        Object r = withProxy(c, profile, p -> p.getConnectionState(dev));
        return r instanceof Integer ? (Integer) r : -1;
    }

    // ── changing it ────────────────────────────────────────────────────────────

    /** Hidden connect/disconnect(device) of a profile proxy (A2DP / HEADSET) by
     *  reflection; null = done, else why not. */
    static String profileCall(Context c, BluetoothDevice dev, int profile, String method) {
        String what = profile == BluetoothProfile.A2DP ? "audio" : "calls";
        Object r = withProxy(c, profile, p -> {
            try {
                int st = p.getConnectionState(dev);
                Dbg.log(c, what + " state before " + method + ": " + st);
                if (method.equals("disconnect") && st != BluetoothProfile.STATE_CONNECTED) return "phone " + what + " not on the 5K";
                if (method.equals("connect") && st == BluetoothProfile.STATE_CONNECTED) return null; // already there
                Object ok = p.getClass().getMethod(method, BluetoothDevice.class).invoke(p, dev);
                return Boolean.TRUE.equals(ok) ? null : "Android refused " + what + " " + method;
            } catch (java.lang.reflect.InvocationTargetException e) {
                return "Android blocks " + what + " " + method + ": " + e.getCause().getClass().getSimpleName();
            } catch (Exception e) {
                return "Android blocks " + what + " " + method + ": " + e.getClass().getSimpleName();
            }
        });
        return r == NO_PROXY ? "no " + what + " service" : (String) r;
    }

    /** Hidden BluetoothDevice.connect() / disconnect(): every enabled profile at once (and
     *  on disconnect the radio link too). Needs the CDM association. Null = done. */
    static String allProfiles(Context c, BluetoothDevice dev, boolean connect) {
        String m = connect ? "connect" : "disconnect";
        try {
            Object r = dev.getClass().getMethod(m).invoke(dev);
            Dbg.log(c, "BluetoothDevice." + m + "() → " + r);
            // BluetoothStatusCodes.SUCCESS = 0 (older builds return a boolean)
            return Integer.valueOf(0).equals(r) || Boolean.TRUE.equals(r) ? null : "Android refused " + m + " (" + r + ")";
        } catch (Exception e) {
            Throwable t = e instanceof java.lang.reflect.InvocationTargetException ? e.getCause() : e;
            Dbg.log(c, "BluetoothDevice." + m + "() blocked: " + t);
            return "Android blocks " + m + ": " + t.getClass().getSimpleName();
        }
    }

    // ── Companion Device Manager ───────────────────────────────────────────────

    static boolean cdmAvailable(Context c) {
        return c.getPackageManager().hasSystemFeature(PackageManager.FEATURE_COMPANION_DEVICE_SETUP);
    }

    /** Whether this app holds a CDM association with `dev` (lets it connect/disconnect it). */
    static boolean associated(Context c, BluetoothDevice dev) {
        if (!cdmAvailable(c)) return false;
        CompanionDeviceManager cdm = c.getSystemService(CompanionDeviceManager.class);
        if (cdm == null) return false;
        try {
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
                for (AssociationInfo a : cdm.getMyAssociations()) {
                    MacAddress mac = a.getDeviceMacAddress();
                    if (mac != null && mac.toString().equalsIgnoreCase(dev.getAddress())) return true;
                }
                return false;
            }
            for (String a : cdm.getAssociations()) {
                if (a.equalsIgnoreCase(dev.getAddress())) return true;
            }
        } catch (Exception e) {
            Dbg.log(c, "CDM associations: " + e);
        }
        return false;
    }

    /** For the widget: whether a tap on Link / Connect / Disconnect should first ask for
     *  the association (LinkActivity). False when that can't help or can't be told. */
    static boolean needsAssociation(Context c) {
        if (!RfcommService.hasBtPermission(c) || !cdmAvailable(c)) return false;
        try {
            BluetoothDevice dev = target(c);
            return dev != null && !associated(c, dev);
        } catch (Exception e) {
            return false;
        }
    }

    // ── plumbing ───────────────────────────────────────────────────────────────

    private interface ProxyCall { Object run(BluetoothProfile p); }

    private static final Object NO_PROXY = new Object();

    /** Run `call` on a profile proxy (waits up to 2 s for it); NO_PROXY if none. */
    private static Object withProxy(Context c, int profile, ProxyCall call) {
        BluetoothAdapter a = adapter(c);
        if (a == null || !a.isEnabled()) return NO_PROXY;
        final BluetoothProfile[] proxy = new BluetoothProfile[1];
        final CountDownLatch got = new CountDownLatch(1);
        boolean asked = a.getProfileProxy(c.getApplicationContext(), new BluetoothProfile.ServiceListener() {
            @Override public void onServiceConnected(int p, BluetoothProfile bp) { proxy[0] = bp; got.countDown(); }
            @Override public void onServiceDisconnected(int p) {}
        }, profile);
        try {
            if (!asked || !got.await(2000, TimeUnit.MILLISECONDS)) return NO_PROXY;
        } catch (InterruptedException e) {
            return NO_PROXY;
        }
        try {
            return call.run(proxy[0]);
        } finally {
            a.closeProfileProxy(profile, proxy[0]);
        }
    }
}
