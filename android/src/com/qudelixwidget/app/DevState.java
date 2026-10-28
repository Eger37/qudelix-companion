package com.qudelixwidget.app;

import java.io.ByteArrayOutputStream;
import java.util.ArrayList;
import java.util.List;

/**
 * Device state read back from the 5K — a port of windows/src/state.rs (which has
 * tests on real captured frames). Three DIFFERENT things, never conflate them:
 *   volumeN  — main output volume (dB*60, signed)
 *   priority — source-priority preference (0 USB / 1 A2DP#1 / 2 A2DP#2)
 *   playing  — what is actually playing right now
 * Layouts: captures/state-decoding-notes.md.
 */
public final class DevState {
    public Integer volumeN;
    public Integer priority;
    public String playing;
    /** Which A2DP slots have a host connected: {A2DP#1, A2DP#2}; null until conn status arrives. */
    public boolean[] a2dp;
    /** The two hosts of the conn status: Bluetooth address and profile bits
     *  (4 A2DP#1, 8 A2DP#2, 1/2 HFP#1/#2); null until conn status arrives. */
    public long[] hostAddr;
    public int[] hostBits;

    /** Where to point the priority once this host dropped its audio link: the A2DP
     *  slot still connected, else USB (0). Null if unknown or both are still up. */
    public Integer remainingSource() {
        if (a2dp == null) return null;
        if (a2dp[0] && a2dp[1]) return null;
        return a2dp[0] ? 1 : a2dp[1] ? 2 : 0;
    }

    /** Where to point the priority once host `me` is gone (like state.rs source_without):
     *  another host's A2DP slot, else USB. Null until conn status arrives. */
    public Integer sourceWithout(long me) {
        if (hostAddr == null) return null;
        int other = 0;
        for (int i = 0; i < 2; i++) if (hostAddr[i] != me) other |= hostBits[i];
        return (other & 0x4) != 0 ? 1 : (other & 0x8) != 0 ? 2 : 0;
    }

    /** This phone's address, told apart without reading it (Android hides it from apps):
     *  while the phone's music is on the 5K, the only host holding an A2DP slot is the
     *  phone. Null if it can't be told (both hosts on A2DP, or unknown). */
    public Long phoneAddr(boolean phoneMusicOn) {
        if (hostAddr == null || !phoneMusicOn) return null;
        boolean a = (hostBits[0] & 0xC) != 0, b = (hostBits[1] & 0xC) != 0;
        return a == b ? null : hostAddr[a ? 0 : 1];
    }

    /** The host whose A2DP slot is gone in `after` (this host's, after it dropped its
     *  music) — how the phone learns its own address. Null if not exactly one. */
    public Long droppedHost(DevState after) {
        if (hostAddr == null || after.hostAddr == null) return null;
        Long found = null;
        for (int i = 0; i < 2; i++) {
            if ((hostBits[i] & 0xC) == 0) continue;
            boolean stillThere = false;
            for (int j = 0; j < 2; j++) {
                if (after.hostAddr[j] == hostAddr[i] && (after.hostBits[j] & 0xC) != 0) stillThere = true;
            }
            if (stillThere) continue;
            if (found != null) return null;
            found = hostAddr[i];
        }
        return found;
    }

    /** A copy of the conn part (for comparing before / after a drop). */
    public DevState connCopy() {
        DevState c = new DevState();
        if (hostAddr != null) {
            c.hostAddr = hostAddr.clone();
            c.hostBits = hostBits.clone();
        }
        if (a2dp != null) c.a2dp = a2dp.clone();
        return c;
    }

    private static final int HEADER_LEN = 8; // FF 01 flags len vendor(2) id(2)

    /** Fold one complete frame into the state. Unknown frames are ignored. */
    public void apply(byte[] f) {
        if (f.length < HEADER_LEN + 1 || u(f[0]) != 0xFF || u(f[4]) != 0xF0 || u(f[5]) != 0x01) return;
        int id = (u(f[6]) << 8) | u(f[7]);
        int p = HEADER_LEN + 1; // payload after the status byte
        if (id == 0xA000 && f.length >= p + 2) {           // Notification: notiId, kind
            int kind = u(f[p + 1]);
            int o = p + 2;
            if ((kind & 0x01) != 0) {
                int n = statusBlock(f, o);
                if (n < 0) return;
                o += n;
            }
            if ((kind & 0x02) != 0) configBlock(f, o);
        } else if (id == 0x8101 && f.length > p + 7) {     // RspInitData: devId(2) result(1) fw(4)
            int n = statusBlock(f, p + 7);
            if (n >= 0) configBlock(f, p + 7 + n);
        } else if (id == 0x8111) {
            statusBlock(f, p);
        } else if (id == 0x8121) {
            configBlock(f, p);
        }
    }

    /** Status block: mask, then audio(0x01,4) power(0x02,8) conn(0x04,16) runtimeInfo(0x08,4)
     *  rms(0x10,16) vol(0x40,16+1). Returns bytes consumed, or -1 if unparseable. */
    private int statusBlock(byte[] f, int at) {
        if (at >= f.length) return -1;
        int m = u(f[at]);
        if ((m & 0xA0) != 0) return -1;
        int o = at + 1;
        int[][] parts = {{0x01, 4}, {0x02, 8}, {0x04, 16}, {0x08, 4}, {0x10, 16}, {0x40, 17}};
        for (int[] part : parts) {
            if ((m & part[0]) == 0) continue;
            if (o + part[1] > f.length) return -1;
            if (part[0] == 0x01) playing = playingFromAudio(f, o);
            if (part[0] == 0x04) {
                // 2 × 8 bytes, one per host: high nibble of byte 0 = connected (4 a2dpPri, 8 a2dpSec).
                int c = (u(f[o]) >> 4) | (u(f[o + 8]) >> 4);
                a2dp = new boolean[]{(c & 0x4) != 0, (c & 0x8) != 0};
                // Bytes 1..6: the host's address (NAP LE, UAP, LAP LE) — state.rs host_from_conn.
                hostAddr = new long[2];
                hostBits = new int[2];
                for (int h = 0; h < 2; h++) {
                    int b = o + 8 * h;
                    long nap = u(f[b + 1]) | (long) u(f[b + 2]) << 8;
                    long lap = u(f[b + 4]) | (long) u(f[b + 5]) << 8 | (long) u(f[b + 6]) << 16;
                    hostAddr[h] = nap << 32 | (long) u(f[b + 3]) << 24 | lap;
                    hostBits[h] = u(f[b]) >> 4;
                }
            }
            if (part[0] == 0x40) volumeN = (int) (short) (u(f[o]) | (u(f[o + 1]) << 8)); // int16 LE
            o += part[1];
        }
        return o - at;
    }

    /** Config block: mask, then sys(0x01,12) vol(0x02,16); later parts not needed. */
    private void configBlock(byte[] f, int at) {
        if (at >= f.length) return;
        int m = u(f[at]);
        int o = at + 1;
        if ((m & 0x01) != 0) {
            if (o + 12 > f.length) return;
            int v = (u(f[o + 1]) >> 2) & 0x07; // audio_in_top_priority
            if (v <= 2) priority = v;
            o += 12;
        }
        if ((m & 0x02) != 0 && o + 2 <= f.length) {
            volumeN = (int) (short) (u(f[o]) | (u(f[o + 1]) << 8)); // int16 LE
        }
    }

    /** Audio part: running = bit 1; active_in_source = bits 26..28; codec = bits 5..8. */
    private static String playingFromAudio(byte[] f, int o) {
        boolean running = ((u(f[o]) >> 1) & 1) == 1;
        int active = (u(f[o + 3]) >> 2) & 0x07;
        if (!running || active == 0) return "nothing";
        // User-facing names: the A2DP slots are "Bluetooth 1/2", HFP = a call on them.
        String[] srcs = {"none", "USB", "Bluetooth 1", "Bluetooth 2", "Bluetooth 1 call", "Bluetooth 2 call"};
        String src = active < srcs.length ? srcs[active] : "?";
        int codec = (u(f[o]) >> 5) | ((u(f[o + 1]) & 1) << 3);
        String[] codecs = {"", "SBC", "AAC", "aptX", "aptX HD", "aptX Adaptive", "LDAC"};
        return codec > 0 && codec < codecs.length ? src + " (" + codecs[codec] + ")" : src;
    }

    private static int u(byte x) { return x & 0xFF; }

    /** 0xAC3EB1AC09BF → "AC:3E:B1:AC:09:BF" (BluetoothDevice.getAddress() form). */
    public static String addrString(long a) {
        StringBuilder s = new StringBuilder();
        for (int i = 5; i >= 0; i--) {
            s.append(String.format("%02X", (a >> (8 * i)) & 0xFF));
            if (i > 0) s.append(':');
        }
        return s.toString();
    }

    /** "AC:3E:B1:AC:09:BF" → 0xAC3EB1AC09BF; -1 if malformed. */
    public static long parseAddr(String s) {
        if (s == null) return -1;
        String hex = s.replace(":", "");
        if (hex.length() != 12) return -1;
        try {
            return Long.parseLong(hex, 16);
        } catch (NumberFormatException e) {
            return -1;
        }
    }

    /** Splits the RFCOMM byte stream into complete GAIA v1 frames. */
    public static final class Splitter {
        private final ByteArrayOutputStream buf = new ByteArrayOutputStream();

        public List<byte[]> push(byte[] data, int len) {
            buf.write(data, 0, len);
            List<byte[]> out = new ArrayList<>();
            byte[] b = buf.toByteArray();
            int i = 0;
            while (true) {
                while (i < b.length && u(b[i]) != 0xFF) i++; // resync on SOF
                if (b.length - i < 4) break;
                int total = HEADER_LEN + u(b[i + 3]) + (u(b[i + 2]) & 0x01); // flags bit0 = checksum
                if (b.length - i < total) break;
                byte[] fr = new byte[total];
                System.arraycopy(b, i, fr, 0, total);
                out.add(fr);
                i += total;
            }
            buf.reset();
            buf.write(b, i, b.length - i);
            return out;
        }
    }
}
