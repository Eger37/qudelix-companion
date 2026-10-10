package com.qudelixwidget.app;

/**
 * Qudelix 5K control protocol over classic Bluetooth RFCOMM (GAIA v1 framing).
 * Same frames the official app sends; see PROTOCOL.md (source of truth) and
 * windows/src/rfcomm.rs (the hardware-verified Windows implementation).
 *
 * Frame: FF 01 00 len  vendorHi vendorLo  cmdHi cmdLo  data[len]   (Qudelix vendor 0xF001)
 */
public final class Proto {
    private Proto() {}

    public static final String NAME_PREFIX = "Qudelix-5K";

    // Session handshake, in the official app's order (btsnoop in captures/).
    public static final byte[] API_VERSION   = b(0xFF, 0x01, 0x00, 0x00, 0x00, 0x0A, 0x03, 0x00);
    public static final byte[] REQ_INIT_DATA = b(0xFF, 0x01, 0x00, 0x03, 0xF0, 0x01, 0x01, 0x00, 0x00, 0x00, 0x04);
    public static final byte[] REGISTER      = b(0xFF, 0x01, 0x00, 0x01, 0x00, 0x0A, 0x40, 0x01, 0x12);
    /** SetRuntimeStatusPeriod = 0. NEVER send the "enable" variant (18 03 E8): it wedges the 5K. */
    public static final byte[] STOP_STREAM   = b(0xFF, 0x01, 0x00, 0x03, 0xF0, 0x01, 0x10, 0x06, 0x00, 0x00, 0x00);

    // Volume: n = dB * 60, signed. Official app range -60..0 dB (step: Prefs.getStepN).
    public static final int VOL_MIN_N = -3600;
    public static final int VOL_MAX_N = 0;

    /** SetAudioTopSource (0x0203): v = 0 USB, 1 A2DP#1, 2 A2DP#2. */
    public static byte[] setPriority(int v) {
        return b(0xFF, 0x01, 0x00, 0x01, 0xF0, 0x01, 0x02, 0x03, v & 0x03);
    }

    /** SetVolume (0x0200), main sink, n big-endian (the 5K REPORTS volume little-endian). */
    public static byte[] setVolume(int n) {
        return b(0xFF, 0x01, 0x00, 0x03, 0xF0, 0x01, 0x02, 0x00, 0x01, (n >> 8) & 0xFF, n & 0xFF);
    }

    public static int clampVol(int n) {
        return Math.max(VOL_MIN_N, Math.min(VOL_MAX_N, n));
    }

    /** Register ACK: vendor 000A, cmd 0xC001 at bytes 4..7. */
    public static boolean isRegisterAck(byte[] f) {
        return f.length >= 8 && u(f[4]) == 0x00 && u(f[5]) == 0x0A && u(f[6]) == 0xC0 && u(f[7]) == 0x01;
    }

    static int u(byte x) { return x & 0xFF; }

    private static byte[] b(int... v) {
        byte[] r = new byte[v.length];
        for (int i = 0; i < v.length; i++) r[i] = (byte) v[i];
        return r;
    }
}
