//! BLE transport (Qualcomm GAIA V3 over WinRT GATT) — the `Ble` impl of
//! [`Qudelix`]. Compiled only with the `ble` cargo feature.
//!
//! BLE differs from USB only in the *outer* framing: one ATT write of
//! `[0xF0, 0x03, opcodeHi, opcodeLo, ...data]` (no len byte, no 0x80 marker, no
//! checksum) — see [`crate::proto::gaia_frame`]. The opcode+data core is shared.
//!
//! Flow: find a paired/known device whose name starts with `Qudelix-5K`, open
//! the GAIA service, subscribe to the notify characteristic, send `ReqInitData`,
//! then write commands to the write characteristic (WriteWithoutResponse).
//!
//! Write-only: the `--raw` / `--set-*` probes only. While the 5K is connected as
//! an audio device it ignores GATT; the widget uses RFCOMM (`rfcomm.rs`). This
//! module also finds the 5K's Bluetooth address and lists paired devices.

use windows::core::{Interface, GUID};
use windows::Devices::Bluetooth::GenericAttributeProfile::{
    GattCharacteristic, GattClientCharacteristicConfigurationDescriptorValue,
    GattCommunicationStatus, GattSession, GattWriteOption,
};
use windows::Devices::Bluetooth::{BluetoothCacheMode, BluetoothDevice, BluetoothLEDevice};
use windows::Devices::Enumeration::DeviceInformation;
use windows::Storage::Streams::DataWriter;

use crate::device::{QResult, Qudelix};
use crate::proto;

const SERVICE: GUID = GUID::from_u128(0x00001100_d102_11e1_9b23_00025b00a5a5);
const WRITE_CHAR: GUID = GUID::from_u128(0x00001101_d102_11e1_9b23_00025b00a5a5);
const NOTIFY_CHAR: GUID = GUID::from_u128(0x00001102_d102_11e1_9b23_00025b00a5a5);

/// An open BLE/GAIA connection to a Qudelix 5K.
pub struct Ble {
    write_char: GattCharacteristic,
    /// Held to keep the LE link up (MaintainConnection). Dropping it lets Windows
    /// tear the connection down.
    _session: GattSession,
}

impl Ble {
    /// Find the Qudelix-5K and open its GAIA service over BLE.
    ///
    /// The 5K is paired with Windows as a *classic* audio device, so it does NOT
    /// appear under the BLE device selector. But Windows can still reach its GATT
    /// by Bluetooth address: we take the address from the paired classic device
    /// and hand it to `BluetoothLEDevice::FromBluetoothAddressAsync`. (Verified:
    /// that returns the GAIA service `00001100-…` even while LE shows Disconnected.)
    pub fn connect() -> QResult<Self> {
        let addr = find_qudelix_address()?;
        crate::log::line(&format!("[ble] Qudelix-5K address = {addr:012X}; opening LE by address"));

        let dev = BluetoothLEDevice::FromBluetoothAddressAsync(addr)?.get()?;
        if dev.as_raw().is_null() {
            return Err("FromBluetoothAddressAsync returned null (no LE binding)".into());
        }

        // Force Windows to actually establish (and hold) the LE link. Without this
        // the device is paired only as classic audio, so GetGattServices returns a
        // cached service with ZERO characteristics — nothing to write to.
        let session = GattSession::FromDeviceIdAsync(&dev.BluetoothDeviceId()?)?.get()?;
        session.SetMaintainConnection(true)?;
        crate::log::line("[ble] GattSession MaintainConnection=true; waiting for LE link…");

        // Discovery can race the connection coming up: retry briefly until the
        // GAIA write characteristic appears.
        let mut write_char = None;
        let mut service = None;
        for attempt in 0..20 {
            let svc_result = dev
                .GetGattServicesForUuidWithCacheModeAsync(SERVICE, BluetoothCacheMode::Uncached)?
                .get()?;
            if svc_result.Status()? == GattCommunicationStatus::Success {
                let services = svc_result.Services()?;
                if services.Size()? > 0 {
                    let svc = services.GetAt(0)?;
                    let wc = svc
                        .GetCharacteristicsForUuidWithCacheModeAsync(
                            WRITE_CHAR,
                            BluetoothCacheMode::Uncached,
                        )?
                        .get()?;
                    let chars = wc.Characteristics()?;
                    if chars.Size()? > 0 {
                        crate::log::line(&format!("[ble] write char found on attempt {attempt}"));
                        write_char = Some(chars.GetAt(0)?);
                        service = Some(svc);
                        break;
                    }
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(150));
        }

        let write_char = write_char
            .ok_or("GAIA write characteristic not found (LE link not established)")?;
        let service = service.expect("service set alongside write_char");

        // Subscribe to notifications (best-effort; write-only still works).
        if let Ok(nc_result) = service
            .GetCharacteristicsForUuidWithCacheModeAsync(NOTIFY_CHAR, BluetoothCacheMode::Uncached)
            .and_then(|op| op.get())
        {
            if let Ok(notify_chars) = nc_result.Characteristics() {
                if notify_chars.Size().unwrap_or(0) > 0 {
                    if let Ok(nc) = notify_chars.GetAt(0) {
                        let _ = nc
                            .WriteClientCharacteristicConfigurationDescriptorAsync(
                                GattClientCharacteristicConfigurationDescriptorValue::Notify,
                            )
                            .and_then(|op| op.get());
                    }
                }
            }
        }

        crate::log::line("[ble] GAIA ready; sending ReqInitData");
        let ble = Ble { write_char, _session: session };
        let _ = ble.write_raw(&proto::gaia_init_frame());
        Ok(ble)
    }

    /// Write arbitrary bytes to the GAIA write characteristic (for probing).
    pub fn write_bytes(&self, bytes: &[u8]) -> QResult {
        self.write_raw(bytes)
    }

    /// Write one GAIA frame to the write characteristic (WriteWithoutResponse).
    fn write_raw(&self, bytes: &[u8]) -> QResult {
        let writer = DataWriter::new()?;
        writer.WriteBytes(bytes)?;
        let buffer = writer.DetachBuffer()?;
        // Test A: WriteWithResponse — some GAIA devices honor commands only when
        // acked. This also tells us if the write is really accepted (the device
        // returns a GATT status rather than fire-and-forget).
        let status = self
            .write_char
            .WriteValueWithOptionAsync(&buffer, GattWriteOption::WriteWithResponse)?
            .get()?;
        crate::log::line(&format!("[ble] write {} -> {status:?}", crate::log::hex(bytes)));
        if status == GattCommunicationStatus::Success {
            Ok(())
        } else {
            Err(format!("BLE write failed: {status:?}").into())
        }
    }
}

/// Get the 5K's Bluetooth address from the paired *classic* device list (it is
/// paired as audio, so this is where Windows knows it — not the LE selector).
pub fn find_qudelix_address() -> QResult<u64> {
    find_qudelix().map(|(_, addr)| addr)
}

/// The 5K's (Bluetooth name, address): Windows names its audio outputs after the
/// name, e.g. "Headphones (Qudelix-5K)".
pub fn find_qudelix() -> QResult<(String, u64)> {
    // A device picked in the settings menu wins; otherwise the first paired
    // device whose name starts with "Qudelix-5K".
    let chosen = crate::config::load_device();
    let devices = paired_devices()?;
    crate::log::line(&format!("[ble] {} paired classic device(s)", devices.len()));
    for (name, _) in &devices {
        crate::log::line(&format!("[ble]  classic name={name:?}"));
    }
    let hit = match &chosen {
        Some(want) => devices.iter().find(|(name, _)| name == want),
        None => devices.iter().find(|(name, _)| name.starts_with(proto::ble::NAME_PREFIX)),
    };
    match (hit, chosen) {
        (Some((name, addr)), _) => Ok((name.clone(), *addr)),
        (None, Some(want)) => Err(format!("{want:?} not found among paired Bluetooth devices").into()),
        (None, None) => Err("Qudelix-5K not found among paired Bluetooth devices".into()),
    }
}

/// All paired classic Bluetooth devices: (name, address).
pub fn paired_devices() -> QResult<Vec<(String, u64)>> {
    let selector = BluetoothDevice::GetDeviceSelector()?;
    let devices = DeviceInformation::FindAllAsyncAqsFilter(&selector)?.get()?;
    let mut out = Vec::new();
    for i in 0..devices.Size()? {
        let info = devices.GetAt(i)?;
        let name = info.Name()?.to_string();
        if let Ok(bd) = BluetoothDevice::FromIdAsync(&info.Id()?)?.get() {
            out.push((name, bd.BluetoothAddress()?));
        }
    }
    Ok(out)
}

impl Qudelix for Ble {
    fn set_priority(&self, v: u8) -> QResult {
        self.write_raw(&proto::gaia_priority_frame(v))
    }

    fn set_volume(&self, target: u8, n: i16) -> QResult {
        self.write_raw(&proto::gaia_volume_frame(target, n))
    }
}
