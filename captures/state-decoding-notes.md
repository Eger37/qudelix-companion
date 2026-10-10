# Qudelix 5K: decoding device state (volume, priority, playing source)

This comes from the official Qudelix Chrome app v4.0.0.0 (CWS id `phaodiidhofhdmfkjiacigibgikhfafn`,
bundle `assets/App-1X1jlnX8.js`). I checked it against `official-app-btsnoop_hci.log` and the
Windows widget log (`qudelix.log`, 2026-10-09 09:03 and 09:10 sessions).

## Opcode corrections (PROTOCOL.md is wrong here)
| name | opcode | wire id (device→host) |
|---|---|---|
| ReqInitData / RspInitData | 0x0100 / 0x0101 | `81 01` |
| ReqDevStatus / RspDevStatus | 0x0110 / 0x0111 | `81 11` |
| ReqDevConfig / RspDevConfig | 0x0120 / 0x0121 | `81 21` |
| ReqEqPreset / RspEqPreset | 0x0123 / 0x0128 | `81 28` |
| Notification | 0x2000 | `A0 00` |
| Warning | 0x2002 | `A0 02` (this is the "wedged" reply `A0 02 00 03`) |

0x2000 and 0x2002 are **Notification** and **Warning**, not RspDevConfig/RspDevStatus.

## RFCOMM framing of device→host messages
`FF 01 00 <len> F0 01 <idHi|0x80> <idLo> <gaiaStatus=00> <payload…>`
- The device sets bit 0x8000 on the opcode, and a GAIA status byte (00) comes before the Qudelix payload.
  The JS (USB path) sees neither: strip both, then `opcode = id & 0x7FFF`.
- `len` counts every byte after `F0 01 id id`, including the status byte.
- Byte offsets below are counted from the `FF` (index 0). The **payload** starts at index 9.

## Bitfield convention (load-bearing)
The status and config structs are read with `G.dataFromArray`, a **LSB-first bit reader**. A 16-bit
field is therefore **little-endian**, and sub-byte fields are packed from bit 0 upward. The
`SetVolume` *command* uses big-endian, but the *reported* `sink_dB60` uses little-endian.

## Payload grammar (JS: class `xue`: `recv`, `rspInitData`, `notification`, `notiDef`; 5K `_E.devStatus` / `_E.devConfig`)
- **devStatus block** = `smask` byte, then the parts in this order, each present only if its bit is set:
  `0x01 audio(4)`, `0x02 power(8)`, `0x04 conn(16 = 2×8)`, `0x08 runtimeInfo(4)`, `0x10 runtimeRms(16)`,
  `0x40 vol(16)+usbMute(1)`. Bit 0x20 (runtimeEq) is NOT parsed by the app, so never request it.
- **devConfig block** = `cmask` byte, then `0x01 sys(12)`, `0x02 vol(16)`, `0x04 playtime`, `0x08 dac`,
  `0x10 mic`, `0x20 batt`, `0x40 sys2`, `0x80 eq`.
- **RspInitData** payload: `devId(2,BE) result(1, 3=success) fwVer(4) devStatus-block devConfig-block warranty(4) usbMute(1)`.
- **RspDevStatus** payload = one devStatus block. **RspDevConfig** payload = one devConfig block.
- **Notification** payload: `notiId(1)`. If notiId < 0x80: `kind(1)` (bit0 = devStatus block follows,
  bit1 = devConfig block follows, status first). The app ignores notiId, so decode by the masks.
  Observed notiIds: 02 audio+vol, 03 runtime (only when streaming is enabled), 04 power (every ~5 s),
  05 conn, 06 vol-status, 07 playtime, 0D batt cfg, 0F sys cfg.

## Struct layouts used
- **audio (xv, 4 bytes, LSB-first bits)**: mute:1, running:1, active_call:1, hfp_codec:2,
  a2dp_codec:4, in_source:3, in_bits:2, out_unbalanced:1, out_power:1, sample_rate_index:4,
  a2dp_index:1, avrcp_index:1, active_usb_speaker:1, active_usb_mic:1, hfp_index:1,
  usb_enumerated:1, active_in_source:3, reserved:3.
  - `running = (a[0]>>1)&1`, `a2dp_codec = (a[0]>>5)|((a[1]&1)<<3)` (0 none, 1 SBC, 2 AAC, 3 aptX, 4 aptX HD, 5 aptX Ad, 6 LDAC),
    `in_source = (a[1]>>1)&7`, `sample_rate_index = a[2]&0xF` (8k,16k,32k,44.1k,48k,88.2k,96k),
    `usb_enumerated = (a[3]>>1)&1`, `active_in_source = (a[3]>>2)&7`.
  - Source enum `hv`: **0 none, 1 USB, 2 A2DP#1, 3 A2DP#2, 4 HFP#1, 5 HFP#2**.
    This is a different numbering from the priority enum.
- **vol (vy, 16 bytes)**: 8 × int16 **LE**, in dB×60: sink, sink_limit, call, sink_left, sink_right, tone, source, sys_last.
  `dB = int16le / 60`. The app's sink range is -60…0 dB in 0.5 dB steps (`sinkVolume=new R_(-60,0,.5)`).
- **sys (py, 12 bytes)**: byte0 = flags. byte1: reconnect_mode bits0-1, **audio_in_top_priority bits2-4**,
  auto_power_mode bits5-7. Bytes 6..11 hold the device BD_ADDR (nap LE16, uap, lap LE24).
  - Priority enum `sy`: **0 USB, 1 A2DP#1, 2 A2DP#2**.
- **conn (Dv = 2 × Tv, 8 bytes each)**: profiles:4, connected:4 (bitmask: 1 hfpPri, 2 hfpSec, 4 a2dpPri, 8 a2dpSec),
  nap:16, uap:8, lap:24, isActive:1 (byte 7 bit 0).

## 1. Volume
- **Push:** Notification kind=0x03, with cmask 0x02. On the wire this is `FF 01 00 19 F0 01 A0 00 00 02 03 01 <aud×4> 02 <sinkLo> <sinkHi> …`.
  sink = int16le at **[17..18]**. Notification 06 (`… A0 00 00 06 01 40 <sinkLo> <sinkHi> …`) puts sink at **[12..13]**.
- **Initial value:** RspInitData. With masks 03/03 as observed, sink is at **[42..43]**. Parse the blocks generically, because the masks decide the offsets.
- **On demand:** ReqDevConfig vol `FF 01 00 01 F0 01 01 20 02` → `FF 01 00 12 F0 01 81 21 00 02 <vol×16>`, sink at [10..11].
  Alternatively ReqDevStatus vol `FF 01 00 01 F0 01 01 10 40` → `FF 01 00 13 F0 01 81 11 00 40 <vol×16> <usbMute>`, sink at [10..11].
- **Verified in captures:** `06 FA` → 0xFA06 = -1530 → **-25.5 dB** (snoop). `C4 FF` → -60 → **-1.0 dB** and
  `5E FC` → -930 → **-15.5 dB** (widget log). In the widget log, sink changes as A2DP#1 and A2DP#2 take turns, so the volume appears to be stored per source.
  sink_limit `68 01` = +6.0 dB.

## 2. Source priority (SetAudioTopSource)
- **Push:** after each SetAudioTopSource the device sends Notification 0F with sys config:
  `FF 01 00 10 F0 01 A0 00 00 0F 02 01 <sys×12>`. Priority = **(frame[13] >> 2) & 7**.
- In RspInitData (masks 03/03) the sys block is at [30..41], so priority = (frame[31]>>2)&7.
- **On demand:** ReqDevConfig sys `FF 01 00 01 F0 01 01 20 01` → `FF 01 00 0E F0 01 81 21 00 01 <sys×12>`, priority = (frame[11]>>2)&7.
- **Verified in captures:** byte 0x25 → 1 (A2DP#1), 0x29 → 2 (A2DP#2). This matched each `02 03 0v` sent, in both captures.
  The remaining sys bytes carry the 5K's own Bluetooth address (byte-swapped); it matched the address in the HCI log.

## 3. Currently playing source (separate from priority)
- Source: audio-status struct, field `active_in_source` (bits 26-28) together with `running` (bit 1).
  The app's source list marks a row "playing" when `connected && xv.running>=1 && sourceToDevice(active_in_source)==row`.
  Its "Input source" title uses `in_source` (bits 9-11), shown only if running.
- **Push:** Notification 02 (`… A0 00 00 02 03 01 a0 a1 a2 a3 …`), with the audio bytes at **[12..15]**:
  `running=(f[12]>>1)&1`, `in_source=(f[13]>>1)&7`, `active_in_source=(f[15]>>2)&7`, codec as above.
- RspInitData (smask 03): audio at [17..20].
- **On demand:** ReqDevStatus audio `FF 01 00 01 F0 01 01 10 01` → `FF 01 00 06 F0 01 81 11 00 01 <aud×4>`.
- Display rule: if running==0, show "idle". Otherwise show `hv[active_in_source]` (1 USB, 2 A2DP#1, 3 A2DP#2, 4/5 call).
- **Verified:** `42 56 14 0C` → running=1, AAC, in_source=3, 48 kHz, active_in_source=3 (A2DP#2). This was taken while priority was A2DP#1, so the two fields really do differ.
  Widget log 09:03:10: after priority was set to A2DP#1, the device first reported running=0, in_source=A2DP#1, active_in_source=A2DP#2, and then
  `running=1 in/active=A2DP#1`. During a switch, `in_source` moves first and `active_in_source` follows once audio is actually flowing.
- Extra signal: in conn status (Notification 05 / RspDevStatus 0x04), `isActive` per BT device.

## Notes
- Notifications arrive without enabling streaming. Priority, volume and audio changes are pushed by themselves. `SetRuntimeStatusPeriod`
  only adds notiId 03 (runtimeInfo/RMS), so keep it at 0.
- **Verified** = decoded from a real frame in the captures. The on-demand ReqDevStatus/ReqDevConfig frames with
  masks 0x01/0x02/0x40 are **inferred**: the official app was seen sending `01 20 3c`, `01 20 c0` and `01 10 04` and got
  `81 21`/`81 11` back, but those exact masks have not been sent yet.
