# Widget design (reference, v1.6.14)

The owner's agreed design for both widgets. Change it only on the owner's request;
reference screenshots: `docs/pc-widget.png` (Windows) and `docs/phone-widget.png` (Android).

## One design on both platforms

The PC widget (`windows/src/ui.rs`) and the phone widget (`android/res/layout/widget.xml`)
have the **same layout, labels, icons and font roles**. The only difference: the PC
has a ✕ (close) next to ⚙; the phone has none.

```
QDLX ⚙ ✕        [ Output ][  ↻   ]
5K  v1.6.14     [ Unlink ][Disconnect]
[ USB ][ ᛒ 1 ][ ᛒ 2 ]
[  −  ]  -29.0 dB  [  +  ]
▬▬▬▬▬▬▭▭▭▭▭▭            (12 segments, 5 dB each)
Playing: Bluetooth 1 (AAC)
```

1. **Title** in two lines: `QDLX` (bold) with ⚙ (and ✕ on PC) beside it, then `5K  v<version>`.
   The version line is the same size as the bottom line.
2. **Top right: 4 normal buttons, 2×2** — Output (speaker icon), Refresh (↻), Unlink/Link,
   Disconnect. Not triangles (tried in 1.6.10 and rejected).
   * Output icon = Segoe MDL2 "Speakers" (U+E7F5); the phone draws the same shape
     (`ic_speaker.xml`). Gear = MDL2 "Settings" outline (`ic_gear.xml` on the phone).
   * Unlink turns accent and reads **Link** while this host is unlinked or disconnected.
3. **Priority row**: `USB`, then the **Bluetooth icon + number** for the two Bluetooth
   slots (never the word "A2DP"). The selected one is accent-filled.
4. **Volume row**: −, the dB value, +, all in the **same font as the priority buttons**.
5. **Volume bar**: 12 tappable segments (−55 … 0 dB in 5-dB steps), lit up to the level.
6. **One bottom line**: the status while an action runs (and a few seconds after),
   otherwise `Playing: <source> (<codec>)`. There's no separate status line. Volume taps
   show no status, because the dB value already shows the result. Sources are named
   "Bluetooth 1/2" here, and "USB".

## Size and responsiveness (phone)

* At the **minimum height (150 dp)** every function above must be visible and usable.
  `tools/check_widget_layout.py` sums the layout's minimum heights against
  `widget_info.xml` `minResizeHeight`.
* Bigger widget: the priority and volume rows stretch, and their text grows with the
  button (`autoSize`, always one line). Narrow widget: text shrinks and never wraps
  (e.g. `-21.0 dB` stays on one line).
* Only RemoteViews-allowed classes in the layout. A plain `<View>` makes the launcher
  show "Can't load widget". This is checked by `tools/check_widget_layout.py`.

## Behaviour tied to the buttons

* **Unlink** on the phone: it drops its music **and calls** links (A2DP + HFP) to the 5K;
  the pairing stays. **Unlink on the PC** leaves Bluetooth alone (dropping the 5K's links
  isn't safe with the Intel Bluetooth offload): it moves Windows' default output and
  microphone, both the plain and the communications defaults, off the 5K to the devices
  they had before (for the output, the Output menu's "On Unlink / Disconnect, switch to"
  pick comes first).
* **Disconnect**: Unlink plus the radio link itself, a real Bluetooth disconnect. The PC
  does it exactly like the "Disconnect" button in Windows Settings (endpoints first,
  then the device; no service toggles) and opens Bluetooth settings only if that fails.
* In both cases the 5K's priority moves to the source that stays (the other host's slot,
  else USB). On the PC this happens *before* any drop, because asking the 5K afterwards
  would bring the link back.
* The labels follow the real state: Unlink ↔ **Link** (phone: music down; PC: the 5K isn't
  Windows' default output) and Disconnect ↔ **Connect** (nothing of the 5K left). Link /
  Connect restore music and calls if they are down; on the PC, the output and microphone
  go back to the 5K. The status claims success only after it is verified.
* On the phone, the first Link / Connect / Disconnect shows Android's own "allow Qudelix
  Widget to manage Qudelix-5K?" dialog once (Companion Device Manager): without it Android
  blocks the full connect / disconnect and connecting calls. Declined: per profile instead.
* **Output**: on the PC, a menu of playback devices (pick = Windows default). On the
  phone, it opens Android's own output picker.
* Refresh is tap-only. There's no automatic refresh on screen-on or visibility, which the
  owner decided after a test.

## Tests

* `android/test.sh`: the protocol decoder plus the widget layout checks.
* `cargo test --target x86_64-pc-windows-gnu --features ble ui::` (from WSL): PC layout.
  It checks that every element is inside the gadget, that no elements overlap, and that a
  click on any element's centre hits it at 100/150/200 % scale. Both run in `tools/release.sh`.
