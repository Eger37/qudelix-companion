package com.qudelixwidget.app;

/**
 * Plain-Java check of the 5K state decoder (DevState) on real captured frames —
 * the same samples as windows/src/state.rs. Run: ./test.sh (no Android needed).
 */
public class DevStateTest {
    static byte[] h(String s) {
        String[] p = s.trim().split(" ");
        byte[] b = new byte[p.length];
        for (int i = 0; i < p.length; i++) b[i] = (byte) Integer.parseInt(p[i], 16);
        return b;
    }

    static int failures;

    static void check(boolean ok, String what) {
        System.out.println((ok ? "ok    " : "FAIL  ") + what);
        if (!ok) failures++;
    }

    public static void main(String[] a) {
        DevState s = new DevState();
        s.apply(h("FF 01 00 10 F0 01 A0 00 00 0F 02 01 B0 25 FF 40 0F 3C 00 00 00 00 00 00"));
        check(s.priority == 1, "priority echo → A2DP 1");
        s.apply(h("FF 01 00 10 F0 01 A0 00 00 0F 02 01 B0 29 FF 40 0F 3C 00 00 00 00 00 00"));
        check(s.priority == 2, "priority echo → A2DP 2");

        s = new DevState();
        s.apply(h("FF 01 00 19 F0 01 A0 00 00 02 03 01 42 56 14 0C 02 06 FA 68 01 00 00 00 00 00 00 F8 F8 00 00 00 00"));
        check("A2DP #2 (AAC)".equals(s.playing), "playing source + codec: " + s.playing);
        check(s.volumeN == -1530, "volume -25.5 dB (int16 LE): " + s.volumeN);
        s.apply(h("FF 01 00 15 F0 01 A0 00 00 06 01 40 C4 FF 68 01 00 00 00 00 00 00 F8 F8 00 00 C4 FF 00"));
        check(s.volumeN == -60, "volume notification -1 dB");
        s.apply(h("FF 01 00 19 F0 01 A0 00 00 02 03 01 00 56 00 08 02 C4 FF 68 01 00 00 00 00 00 00 F8 F8 00 00 C4 FF"));
        check("nothing".equals(s.playing), "not running → nothing playing");

        DevState.Splitter sp = new DevState.Splitter();
        byte[] x = h("FF 01 00 01 F0 01 02 03 02 FF 01 00 02 00 0A C0");
        check(sp.push(x, x.length).size() == 1, "splitter: one complete + one partial frame");
        byte[] y = h("01 00 12");
        check(sp.push(y, y.length).size() == 1, "splitter: partial frame completed");

        // Hearing safety: one command raises the volume by at most 10 dB.
        check(Proto.safeVolume(-3000, 0) == -2400, "safeVolume: -50 → 0 dB capped at -40 dB");
        check(Proto.safeVolume(-600, -570) == -570, "safeVolume: small step up passes");
        check(Proto.safeVolume(-600, -3600) == -3600, "safeVolume: down is never limited");
        check(Proto.safeVolume(-300, 600) == 0, "safeVolume: never above 0 dB");
        check(Proto.safeVolume(-3600, -9000) == -3600, "safeVolume: never below -60 dB");

        if (failures > 0) {
            System.out.println(failures + " FAILED");
            System.exit(1);
        }
        System.out.println("all passed");
    }
}
