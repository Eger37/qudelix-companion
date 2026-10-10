//! Qudelix 5K command framing — a faithful mirror of `PROTOCOL.md`.
//!
//! This module is transport-agnostic and has **no platform dependencies**, so it
//! compiles and unit-tests on the host (Linux/WSL) as well as on the Windows
//! target. Keep every opcode / byte-layout fact here so new commands (volume,
//! BLE) drop in by adding a constant and a frame builder — nothing else changes.
//!
//! ## Command model (from PROTOCOL.md)
//! Every command is a 16-bit **opcode** (big-endian) followed by a payload. The
//! transport-agnostic "application payload" is `[opcodeHi, opcodeLo, ...payload]`.
//!
//! ## USB-HID framing (CONFIRMED)
//! Output: `sendReport(reportId = 8, data)` where
//! `data = [len, 0x80, opcodeHi, opcodeLo, ...payload, 0x00…]` zero-padded to the
//! device's output-report length (~64). `len = 1 (marker) + app_payload.len()`.

/// 16-bit opcodes (big-endian on the wire). Qudelix opcodes ride as GAIA
/// commands under vendor id 0xF003 (BLE); the opcode+data core is identical on
/// every transport.
#[allow(dead_code)] // some opcodes are reference-only / used by the `ble` feature
pub mod opcode {
    /// `SetVolume` — data `[target, nHi, nLo]`, n = round(dB * 60) as signed i16 BE.
    pub const SET_VOLUME: u16 = 512; // 0x0200
    /// `SetAudioInTopPriority` — source priority. data `[v]`, 0=USB,1=A2DP#1,2=A2DP#2.
    pub const SET_AUDIO_IN_TOP_PRIORITY: u16 = 515; // 0x0203
    /// `ReqInitData` — pull initial device state (used in the BLE handshake).
    pub const REQ_INIT_DATA: u16 = 256; // 0x0100
    /// `SetRuntimeStatusPeriod` — enable live status streaming (read-back seam).
    pub const SET_RUNTIME_STATUS_PERIOD: u16 = 4102; // 0x1006
    /// `SaveAll` — NOT needed; the app never sends it. Kept for reference only.
    pub const SAVE_ALL: u16 = 4103; // 0x1007
}

/// `SetVolume` target byte for the main output (sink).
pub const VOLUME_SINK_MAIN: u8 = 0x01;
/// Volume scale: raw `n` is dB in 1/60-dB steps (`n = round(dB * 60)`).
pub const VOLUME_SCALE: f32 = 60.0;

/// USB-HID output report id used by the 5K (PROTOCOL.md: fallback 7 on reject).
pub const HID_REPORT_ID_OUT: u8 = 8;
/// Host→device marker byte that precedes the opcode in a USB frame (USB-only).
pub const HOST_MARKER: u8 = 0x80;
/// Output report length (bytes, **excluding** the leading report-id byte) used
/// when the device caps cannot be read. The 5K uses 64.
pub const DEFAULT_REPORT_LEN: usize = 64;

/// Qualcomm GAIA vendor header for Qudelix (BLE outer framing): `0xF0 0x03`.
#[allow(dead_code)]
pub const GAIA_HEADER: [u8; 2] = [0xF0, 0x03];

/// BLE GATT identifiers (Qualcomm GAIA V3). See PROTOCOL.md.
#[allow(dead_code)] // consumed by the `ble` feature / kept as protocol reference
pub mod ble {
    /// GAIA service.
    pub const SERVICE: &str = "00001100-d102-11e1-9b23-00025b00a5a5";
    /// Host→device write characteristic (GaiaCommand).
    pub const WRITE_CHAR: &str = "00001101-d102-11e1-9b23-00025b00a5a5";
    /// Device→host notify characteristic (GaiaResponse).
    pub const NOTIFY_CHAR: &str = "00001102-d102-11e1-9b23-00025b00a5a5";
    /// Advertised name prefix to scan for.
    pub const NAME_PREFIX: &str = "Qudelix-5K";
}

/// Opcode as big-endian bytes, e.g. `515 -> [0x02, 0x03]`.
pub fn opcode_bytes(op: u16) -> [u8; 2] {
    op.to_be_bytes()
}

/// Application payload: `[opcodeHi, opcodeLo, ...payload]` (transport-agnostic).
pub fn app_payload(op: u16, payload: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(2 + payload.len());
    v.extend_from_slice(&opcode_bytes(op));
    v.extend_from_slice(payload);
    v
}

/// Build the USB-HID report **data** (the bytes that follow the report-id byte),
/// zero-padded to `report_len`.
///
/// Layout: `[len, 0x80, opcodeHi, opcodeLo, ...payload, 0x00…]`, where
/// `len = 1 (the 0x80 marker) + app_payload.len()`.
pub fn hid_frame(op: u16, payload: &[u8], report_len: usize) -> Vec<u8> {
    let app = app_payload(op, payload);
    // 1 len byte + 1 marker byte + application payload must fit the report.
    debug_assert!(
        report_len >= 2 + app.len(),
        "report_len {report_len} too small for a {}-byte app payload",
        app.len()
    );
    let mut frame = vec![0u8; report_len];
    frame[0] = (1 + app.len()) as u8; // len counts the marker + opcode + payload
    frame[1] = HOST_MARKER;
    frame[2..2 + app.len()].copy_from_slice(&app);
    frame
}

/// `SetAudioInTopPriority` report data for priority value `v` (0/1/2).
pub fn set_priority_frame(v: u8, report_len: usize) -> Vec<u8> {
    hid_frame(opcode::SET_AUDIO_IN_TOP_PRIORITY, &[v], report_len)
}

/// `SetVolume` report data. `n` is the signed 1/60-dB value (two's-complement,
/// big-endian on the wire); `target` is e.g. [`VOLUME_SINK_MAIN`].
pub fn set_volume_frame(target: u8, n: i16, report_len: usize) -> Vec<u8> {
    let n = n.to_be_bytes(); // i16 -> two's-complement big-endian
    hid_frame(opcode::SET_VOLUME, &[target, n[0], n[1]], report_len)
}

/// Convert dB to the raw signed `n` value (`n = round(dB * 60)`), saturating to
/// the i16 range.
#[allow(dead_code)]
pub fn db_to_n(db: f32) -> i16 {
    (db * VOLUME_SCALE).round().clamp(i16::MIN as f32, i16::MAX as f32) as i16
}

/// Convert a raw signed `n` value back to dB.
pub fn n_to_db(n: i16) -> f32 {
    n as f32 / VOLUME_SCALE
}

/// Prepend the report id to a frame, producing the exact buffer handed to the
/// HID write (`[reportId, ...frame]`).
pub fn write_buffer(report_id: u8, frame: &[u8]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(1 + frame.len());
    buf.push(report_id);
    buf.extend_from_slice(frame);
    buf
}

// ── BLE / GAIA framing ─────────────────────────────────────────────────────────

/// Build a BLE GAIA frame (one ATT write): `[0xF0, 0x03, opcodeHi, opcodeLo,
/// ...data]`. No SOF, no length, no checksum, no 0x80 marker — that is the key
/// difference from the USB frame.
#[allow(dead_code)]
pub fn gaia_frame(op: u16, data: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(4 + data.len());
    v.extend_from_slice(&GAIA_HEADER);
    v.extend_from_slice(&opcode_bytes(op));
    v.extend_from_slice(data);
    v
}

/// BLE `ReqInitData` handshake frame: `F0 03 01 00`.
#[allow(dead_code)]
pub fn gaia_init_frame() -> Vec<u8> {
    gaia_frame(opcode::REQ_INIT_DATA, &[])
}

/// BLE set-priority frame: `F0 03 02 03 vv`.
#[allow(dead_code)]
pub fn gaia_priority_frame(v: u8) -> Vec<u8> {
    gaia_frame(opcode::SET_AUDIO_IN_TOP_PRIORITY, &[v])
}

/// BLE set-volume frame: `F0 03 02 00 01 nHi nLo`.
#[allow(dead_code)]
pub fn gaia_volume_frame(target: u8, n: i16) -> Vec<u8> {
    let n = n.to_be_bytes();
    gaia_frame(opcode::SET_VOLUME, &[target, n[0], n[1]])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opcode_math_priority() {
        // 515 -> [0x02, 0x03]
        assert_eq!(opcode_bytes(opcode::SET_AUDIO_IN_TOP_PRIORITY), [0x02, 0x03]);
    }

    #[test]
    fn opcode_math_volume() {
        // 512 -> [0x02, 0x00]
        assert_eq!(opcode_bytes(opcode::SET_VOLUME), [0x02, 0x00]);
    }

    #[test]
    fn app_payload_priority() {
        // opcode 515 + payload [1] -> [0x02, 0x03, 0x01]
        assert_eq!(
            app_payload(opcode::SET_AUDIO_IN_TOP_PRIORITY, &[1]),
            vec![0x02, 0x03, 0x01]
        );
    }

    #[test]
    fn set_priority_1_bytes() {
        // The headline assertion from the brief:
        // set_priority(1) -> [04 80 02 03 01 00 …] zero-padded to the report size.
        let frame = set_priority_frame(1, 64);
        assert_eq!(frame.len(), 64);
        assert_eq!(&frame[..6], &[0x04, 0x80, 0x02, 0x03, 0x01, 0x00]);
        assert!(frame[6..].iter().all(|&b| b == 0), "tail must be zero padding");
    }

    #[test]
    fn set_priority_all_values() {
        for v in 0u8..=2 {
            let frame = set_priority_frame(v, 64);
            assert_eq!(&frame[..5], &[0x04, 0x80, 0x02, 0x03, v]);
        }
    }

    #[test]
    fn len_byte_counts_marker_plus_app() {
        // priority: marker(1) + opcode(2) + payload(1) = 4
        assert_eq!(set_priority_frame(0, 64)[0], 0x04);
        // volume:   marker(1) + opcode(2) + payload(3) = 6
        assert_eq!(set_volume_frame(0, 0, 64)[0], 0x06);
    }

    #[test]
    fn volume_framing_zero_db() {
        // [len, 0x80, 0x02, 0x00, 0x01, nHi, nLo, 0x00…]; 0 dB -> n = 0.
        let frame = set_volume_frame(VOLUME_SINK_MAIN, db_to_n(0.0), 64);
        assert_eq!(&frame[..7], &[0x06, 0x80, 0x02, 0x00, 0x01, 0x00, 0x00]);
    }

    #[test]
    fn volume_negative_db_is_twos_complement_be() {
        // -1 dB -> n = -60 -> i16 two's-complement = 0xFFC4 -> BE bytes [0xFF, 0xC4].
        assert_eq!(db_to_n(-1.0), -60);
        let frame = set_volume_frame(VOLUME_SINK_MAIN, db_to_n(-1.0), 64);
        assert_eq!(&frame[..7], &[0x06, 0x80, 0x02, 0x00, 0x01, 0xFF, 0xC4]);
        // round-trips back to dB.
        let n = i16::from_be_bytes([frame[5], frame[6]]);
        assert_eq!(n, -60);
        assert!((n_to_db(n) - (-1.0)).abs() < 1e-6);
    }

    #[test]
    fn gaia_frames_have_no_marker_or_len() {
        // Priority: F0 03 02 03 vv
        assert_eq!(gaia_priority_frame(1), vec![0xF0, 0x03, 0x02, 0x03, 0x01]);
        // Volume: F0 03 02 00 01 nHi nLo
        assert_eq!(
            gaia_volume_frame(VOLUME_SINK_MAIN, db_to_n(-1.0)),
            vec![0xF0, 0x03, 0x02, 0x00, 0x01, 0xFF, 0xC4]
        );
        // Handshake: F0 03 01 00
        assert_eq!(gaia_init_frame(), vec![0xF0, 0x03, 0x01, 0x00]);
    }

    #[test]
    fn write_buffer_prepends_report_id() {
        let frame = set_priority_frame(2, 64);
        let buf = write_buffer(HID_REPORT_ID_OUT, &frame);
        assert_eq!(buf.len(), 65);
        assert_eq!(buf[0], 0x08); // report id
        assert_eq!(&buf[1..7], &[0x04, 0x80, 0x02, 0x03, 0x02, 0x00]);
    }

    #[test]
    fn report_len_is_respected() {
        assert_eq!(set_priority_frame(0, 32).len(), 32);
        assert_eq!(set_priority_frame(0, DEFAULT_REPORT_LEN).len(), 64);
    }

}
