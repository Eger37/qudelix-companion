//! Transport-agnostic control surface for the Qudelix 5K.
//!
//! Used by the one-shot CLI probes (`--set-priority`, `--set-volume`): USB-HID
//! ([`crate::usb::UsbHid`]) and BLE GATT (`ble::Ble`). The widget itself talks
//! over the RFCOMM session (`rfcomm.rs`). Keep command *meaning* here and byte
//! *framing* in [`crate::proto`].

/// Boxed error so the trait stays independent of any one transport's error type.
pub type QResult<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

/// A connected Qudelix 5K that can receive runtime control commands.
pub trait Qudelix {
    /// Set the audio **source priority**: `v` = 0 USB DAC, 1 A2DP#1, 2 A2DP#2
    /// (opcode `SetAudioTopSource` = 515). CONFIRMED over USB-HID.
    fn set_priority(&self, v: u8) -> QResult;

    /// Set **volume** (opcode `SetVolume` = 512, data `[target, nHi, nLo]`).
    ///
    /// `target` is the sink byte (e.g. `VOLUME_SINK_MAIN = 0x01`) and `n` is the
    /// signed 1/60-dB value (`n = round(dB * 60)`), sent big-endian two's-complement.
    fn set_volume(&self, target: u8, n: i16) -> QResult;
}
