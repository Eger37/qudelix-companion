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
        check("Bluetooth 2 (AAC)".equals(s.playing), "playing source + codec: " + s.playing);
        check(s.volumeN == -1530, "volume -25.5 dB (int16 LE): " + s.volumeN);
        s.apply(h("FF 01 00 15 F0 01 A0 00 00 06 01 40 C4 FF 68 01 00 00 00 00 00 00 F8 F8 00 00 C4 FF 00"));
        check(s.volumeN == -60, "volume notification -1 dB");
        s.apply(h("FF 01 00 19 F0 01 A0 00 00 02 03 01 00 56 00 08 02 C4 FF 68 01 00 00 00 00 00 00 F8 F8 00 00 C4 FF"));
        check("nothing".equals(s.playing), "not running → nothing playing");

        s = new DevState();
        check(s.remainingSource() == null, "conn unknown → no target");
        s.apply(h("FF 01 00 12 F0 01 81 11 00 04 5F 11 22 33 44 55 66 01 AF 77 88 99 AA BB CC 00"));
        check(s.remainingSource() == null, "both A2DP up → no target");
        s.apply(h("FF 01 00 12 F0 01 81 11 00 04 00 00 00 00 00 00 00 00 AF 77 88 99 AA BB CC 01"));
        check(s.remainingSource() == 2, "only A2DP 2 left → priority 2");
        s.apply(h("FF 01 00 12 F0 01 81 11 00 04 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00"));
        check(s.remainingSource() == 0, "no Bluetooth host left → USB");

        // Captured 2026-10-18 (same as state.rs): phone AC:3E:B1:AC:09:BF on A2DP#1+HFP#1,
        // PC 88:F4:DA:75:49:27 listed with no profiles.
        s = new DevState();
        check(s.sourceWithout(1) == null && s.phoneAddr(true) == null, "conn unknown → no host info");
        s.apply(h("FF 01 00 12 F0 01 81 11 00 04 57 3E AC B1 BF 09 AC 00 07 F4 88 DA 27 49 75 00"));
        long phone = 0xAC3EB1AC09BFL, pc = 0x88F4DA754927L;
        check(s.hostAddr[0] == phone && s.hostBits[0] == 0x5, "host 1 = phone on A2DP#1+HFP#1");
        check(s.hostAddr[1] == pc && s.hostBits[1] == 0x0, "host 2 = PC, no profiles");
        check(s.sourceWithout(pc) == 1, "without the PC → the phone's slot");
        check(s.sourceWithout(phone) == 0, "without the phone → USB");
        check(s.phoneAddr(true) == phone, "phone told apart: the only host on A2DP");
        check(s.phoneAddr(false) == null, "phone music off → not the A2DP host");
        DevState before = s.connCopy();
        s.apply(h("FF 01 00 12 F0 01 81 11 00 04 57 3E AC B1 BF 09 AC 00 AF F4 88 DA 27 49 75 00"));
        check(s.phoneAddr(true) == null, "both hosts on A2DP → can't tell");
        DevState both = s.connCopy();
        s.apply(h("FF 01 00 12 F0 01 81 11 00 04 07 3E AC B1 BF 09 AC 00 AF F4 88 DA 27 49 75 00"));
        check(both.droppedHost(s) == phone, "after a drop: the host that left A2DP is the phone");
        check(before.droppedHost(both) == null, "nobody left → no host learned");
        check(DevState.addrString(phone).equals("AC:3E:B1:AC:09:BF"), "address → text");
        check(DevState.parseAddr("ac:3e:b1:ac:09:bf") == phone, "text → address");

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
