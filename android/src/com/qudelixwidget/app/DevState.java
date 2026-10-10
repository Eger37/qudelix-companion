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
        String[] srcs = {"none", "USB", "A2DP #1", "A2DP #2", "HFP #1", "HFP #2"};
        String src = active < srcs.length ? srcs[active] : "?";
        int codec = (u(f[o]) >> 5) | ((u(f[o + 1]) & 1) << 3);
        String[] codecs = {"", "SBC", "AAC", "aptX", "aptX HD", "aptX Adaptive", "LDAC"};
        return codec > 0 && codec < codecs.length ? src + " (" + codecs[codec] + ")" : src;
    }

    private static int u(byte x) { return x & 0xFF; }

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
