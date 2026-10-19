# Qudelix 5K — control protocol (interoperability notes)

A personal companion widget for the owner's **own** Qudelix 5K. These are interoperability
notes: the runtime control commands the official Qudelix apps send, so our widget can send the
same ones. Learned from the official Qudelix software (Chrome app v4.0.0.0, Android app
`com.qudelix.qudelix`) and from observing the official app's own Bluetooth traffic to the owner's
device. The device firmware is signed/closed and is **never modified** — we only send the same
documented runtime commands the official app already sends over its normal control channel.

## Command model
Every command: 16-bit **opcode** (big-endian) + payload. The inner "application payload" is
`[opcodeHi, opcodeLo, ...data]`, transport-agnostic. The device speaks **Qualcomm GAIA**;
Qudelix opcodes ride as GAIA commands under vendor ID **0xF003**. The outer framing differs
per transport (see below); the opcode+data core is identical.

Opcodes used here:
- `SetAudioInTopPriority = 515` (0x0203) — source priority. data `[v]`, v: 0=USB DAC, 1=A2DP#1, 2=A2DP#2.
- `SetVolume = 512` (0x0200) — data `[target, nHi, nLo]`. target: **sink = 0x01** (main output).
  n = round(dB * 60) as **signed** 16-bit big-endian (two's-complement; attenuation = negative).
  scale = 60.0 (1/60-dB steps). dB min/max not static — read back from device.
- Read-back (corrected 2026-10-09 from the Chrome app parsers): `ReqDevStatus = 0x0110`, `ReqDevConfig = 0x0120`
  (each + 1 mask byte) → `RspDevStatus = 0x0111`, `RspDevConfig = 0x0121`; `0x2000` = Notification, `0x2002` = Warning
  (NOT Rsp*). Reported volume is **int16 LITTLE-endian** dB*60 (SetVolume is big-endian). Volume range -60…0 dB.
  Initial state pull = `ReqInitData = 0x0100`; live streaming = `SetRuntimeStatusPeriod = 4102 (0x1006)`.
- **Priority ≠ playing source.** Priority = sys-config `audio_in_top_priority`; what is playing now = audio-status
  `running` + `active_in_source` (0 none,1 USB,2 A2DP#1,3 A2DP#2,4 HFP#1,5 HFP#2 — a different enum).
  Full layouts, offsets and verified samples: `captures/state-decoding-notes.md`; decoder: `windows/src/state.rs`.
- **No "drop host X" command.** The Chrome app v4.0 opcode table has only `Disconnect = 263` (ends the
  control session) and `SysPairedDeviceReset = 4099` (forgets all pairings). To make the 5K move to its
  other source, the host drops its own A2DP link (Windows: KSPROPSETID_BtAudio one-shot disconnect on the
  endpoint's KS filter; Android: hidden `BluetoothA2dp.disconnect`, may be blocked), then reads the conn
  status (`ReqDevStatus` mask 0x04: `FF 01 00 01 F0 01 01 10 04`) and sets the priority to the slot left.
- `SaveAll (0x1007)`: **NOT needed** — the app never emits it; priority & volume apply live (5K persists in NVM). Do not send it.

## Transports

### RFCOMM (classic BT, GAIA v1 framing) — CONFIRMED ON HARDWARE, the path actually used
When the 5K is connected to the host as an **audio** device (A2DP), it **ignores BLE GATT
writes**; GAIA control lives on classic RFCOMM/SPP.

**Finding the channel — SDP only, never guess or scan.** The official app queries SDP for
**Serial Port, UUID 0x1101**; the 5K answers with a record named **"GAIA"** and the current RFCOMM
channel. The channel **hops after every session** (seen 1 → 14 → 2). Windows: Winsock `connect`
with `SOCKADDR_BTH{serviceClassId = SPP UUID, port = 0}` does the lookup; Android:
`createRfcommSocketToServiceRecord(SPP UUID)`. (WinRT `GetRfcommServicesAsync` lists only HFP/HSP
and does not show this record.) Connecting to a channel without a server gets DM → Windows
`connect()` fails in ~16 ms with WSA 10064.

> ⚠ **Never probe other channels.** Half-open connects to non-GAIA channels while another
> controller held the session left a **ghost session** in the 5K: every host then got "accepted,
> warning newConnection, dropped" until the 5K was power-cycled. Happened twice (phone v1.0.7
> channel scan; PC v1.1.3 fallback scan). Both widgets are SDP-only since v1.2.0.

**Frame:** `FF 01 00 <len> <vendorHi vendorLo> <cmdHi cmdLo> <data…>` — SOF `FF`, version `01`,
flags `00` (bit 0 = trailing checksum), `len` = data bytes after the command. Qudelix vendor is
**0xF001** here (not the BLE 0xF003). Device→host replies: id | 0x8000, then a status byte.

**Session** (one connection, in the official app's order — a lone command is ignored):
1. `FF 01 00 00 00 0A 03 00` — GetApiVersion
2. `FF 01 00 03 F0 01 01 00 00 00 04` — ReqInitData (reply `81 01` carries status + config blocks)
3. `FF 01 00 01 00 0A 40 01 12` — register-notification (vendor 000A, cmd 0x4001, event 0x12).
   ACK `FF 01 00 02 00 0A C0 01 00 12` **opens the session**.
4. `FF 01 00 03 F0 01 10 06 00 00 00` — SetRuntimeStatusPeriod = 0 (keep streaming off)
5. commands: priority `FF 01 00 01 F0 01 02 03 0v`, volume `FF 01 00 03 F0 01 02 00 01 nHi nLo`
6. stop stream again, close (graceful: half-close, wait for the 5K's EOF).

**Rules learned on hardware**
- **Keep reading.** Once registered, the 5K pushes notifications (~every 5 s plus on every
  change: volume, priority, playback). Unread data starves RFCOMM credits and the 5K drops the link.
- **Never enable status streaming** (`…10 06 18 03 E8`) unless you also stop it — a leftover
  stream makes later sessions start with `A0 02 00 03` and fail.
- **One controller at a time; the existing one wins.** A second connection while a session is
  open gets Warning `newConnection` and is dropped; after the first closes, the next connects
  normally (verified: `--collision-test 5`, all healthy). Treat "accepted then dropped" as *busy*.
- Release the session a few seconds after the last command, so other hosts (phone ↔ PC ↔
  official app) can take turns. Verified many back-to-back sessions.
- Priority is a *preference*, not a forced preempt: it decides which source wins when both stream.
- Warning codes (`F0 01 A0 02 00 <code>`, app enum `cv`): 1 volumeUpWhenNoAudio, 2 volumeTooLoud,
  **3 newConnection**, 4 eqOptimizing, 5 ampFault, 6 eqBandLimitForQxOver, 7 a2dpDisconn,
  8 a2dpReconn, 9 ldacAccelerationDisconnect.

### USB (WebHID / native HID) — CONFIRMED
- VID `0x0A12`; 5K PIDs: 0x4003–0x4007, 0x4112/3, 0x4125/6, 0x4135/6. HID usagePage 0xFF00.
- Out: `sendReport(reportId=8, data)` (fallback reportId 7). In: reportId 9 or 2.
- data = `[len, 0x80, opcodeHi, opcodeLo, ...data, 0x00…]` zero-padded to report size (read from descriptor, ~64).
  - len = (2 opcode + N data) + 1. 0x80 = host→device marker (USB-only; inbound frames omit it).
  - This USB frame does NOT carry the GAIA 0xF003 vendor prefix (that's BLE-only).
- Set priority to v:  reportId 8, `[0x04, 0x80, 0x02, 0x03, v, 0x00…]`.
- Set main volume: reportId 8, `[0x06, 0x80, 0x02, 0x00, 0x01, nHi, nLo, 0x00…]` (len 6 = 2 opcode+3 data+1).

### BLE (GATT / Qualcomm GAIA V3) — from the Android APK; IGNORED by the 5K while it is connected as audio
(kept for reference; use RFCOMM above)
- Scan: match advertised name prefix **`Qudelix-5K`**.
- **Service** `00001100-d102-11e1-9b23-00025b00a5a5`
- **Write char (host→device)** `00001101-d102-11e1-9b23-00025b00a5a5` (Write / WriteNoResponse) — GaiaCommand
- **Notify char (device→host)** `00001102-d102-11e1-9b23-00025b00a5a5` (Notify) — GaiaResponse
- (Data/DFU char `…1103` — not needed.) CCCD `00002902-0000-1000-8000-00805f9b34fb`.
- Handshake: enable notify (write CCCD `01 00` on `…1102`) → requestMtu (~512, confirm live) →
  send `ReqInitData` (0x0100) → then write commands to `…1101`.
- **BLE frame (one ATT write to …1101): `[0xF0, 0x03, opcodeHi, opcodeLo, ...data]`** — big-endian,
  no SOF, no length, no checksum, no 0x80.
  - Set priority: **`F0 03 02 03 vv`**.
  - Set main volume: **`F0 03 02 00 01 nHi nLo`**.
- NOTE: V3 assumed (current fw). If a live capture shows GAIA **V2**, wrap with an `0xFF` SOF +
  flags + length header around the vendor/command. Verify MTU and the volume field offset in
  RspDevConfig/RspDevStatus with one Bluetooth capture of the official app.

## Status
- [x] Priority + volume over an RFCOMM/GAIA session — confirmed on hardware (PC + phone).
- [x] Channel via SDP; multi-host turn-taking (PC, phone widget, official app) — confirmed.
- [x] Read-back of volume / priority / playing source — decoded and verified on captured frames.
- [x] Priority/volume USB-HID frames — exact (from the Chrome app).
- [x] BLE UUIDs + GAIA framing — documented (ignored by the 5K while it plays audio).
- [x] SaveAll — not needed (app never sends it).
- [ ] No known way to clear a ghost session without a power-cycle — so never create one.
