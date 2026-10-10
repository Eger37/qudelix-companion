//! USB-HID transport for the Qudelix 5K (the `UsbHid` impl of [`Qudelix`]).
//!
//! Enumeration and the output-report write go through the `hidapi` crate (which
//! cross-compiles cleanly for `x86_64-pc-windows-gnu`, bundling the Windows
//! hid.dll backend). The one thing hidapi does not expose — the device's
//! *output report byte length* — we read straight from the HID caps via the
//! Windows `HidP_GetCaps` API so we pad to the real report size instead of
//! blindly assuming 64 (we fall back to 64 only if the query fails).

use std::ffi::CStr;

use hidapi::{HidApi, HidDevice};

use crate::device::{QResult, Qudelix};
use crate::proto;

/// Qudelix vendor id.
const QUDELIX_VID: u16 = 0x0A12;
/// Known Qudelix **5K** product ids (PROTOCOL.md). Other PIDs (e.g. T71) are
/// deliberately excluded so we never send 5K frames to a different device.
const QUDELIX_5K_PIDS: &[u16] = &[
    0x4003, 0x4004, 0x4005, 0x4006, 0x4007, 0x4112, 0x4113, 0x4125, 0x4126, 0x4135, 0x4136,
];

/// An open USB-HID connection to a Qudelix 5K.
pub struct UsbHid {
    dev: HidDevice,
    /// Output report length in bytes, **excluding** the report-id byte — the
    /// size we zero-pad every frame to.
    report_len: usize,
    /// PID we matched, for diagnostics / the UI status line.
    pid: u16,
}

impl UsbHid {
    /// Find the first connected Qudelix 5K and open it, or return why not.
    pub fn connect() -> QResult<Self> {
        let api = HidApi::new()?;

        // The 5K exposes several HID collections; only the vendor collection
        // (usage page 0xFF00) accepts the command report. Prefer it, but log every
        // candidate so we can see what Windows actually enumerated.
        let candidates: Vec<_> = api
            .device_list()
            .filter(|d| d.vendor_id() == QUDELIX_VID && QUDELIX_5K_PIDS.contains(&d.product_id()))
            .collect();

        crate::log::line(&format!("[usb] {} candidate interface(s)", candidates.len()));
        for d in &candidates {
            crate::log::line(&format!(
                "[usb]  pid={:04X} usage_page={:04X} usage={:04X} iface={} path={:?}",
                d.product_id(), d.usage_page(), d.usage(), d.interface_number(), d.path()
            ));
        }

        let info = candidates
            .iter()
            .find(|d| d.usage_page() == 0xFF00)
            .or_else(|| candidates.first())
            .ok_or("Qudelix 5K not found (is it connected over USB?)")?;

        let pid = info.product_id();
        let path = info.path().to_owned();

        let report_len = query_output_report_len(&path).unwrap_or(proto::DEFAULT_REPORT_LEN);
        crate::log::line(&format!(
            "[usb] opening pid={:04X} usage_page={:04X} report_len={}",
            pid, info.usage_page(), report_len
        ));

        let dev = api.open_path(&path)?;
        crate::log::line("[usb] open OK");
        Ok(Self { dev, report_len, pid })
    }

    /// The matched product id (for the status line).
    pub fn pid(&self) -> u16 {
        self.pid
    }

    /// The negotiated output-report length we pad frames to.
    #[allow(dead_code)]
    pub fn report_len(&self) -> usize {
        self.report_len
    }

    /// Send a fully framed report (report-id already prepended).
    fn send_frame(&self, frame: &[u8]) -> QResult {
        let buf = proto::write_buffer(proto::HID_REPORT_ID_OUT, frame);
        crate::log::line(&format!("[usb] write {} bytes: {}", buf.len(), crate::log::hex(&buf)));
        match self.dev.write(&buf) {
            Ok(n) => { crate::log::line(&format!("[usb] write OK ({n} bytes)")); Ok(()) }
            Err(e) => { crate::log::line(&format!("[usb] write ERR: {e}")); Err(e.into()) }
        }
    }
}

impl Qudelix for UsbHid {
    fn set_priority(&self, v: u8) -> QResult {
        self.send_frame(&proto::set_priority_frame(v, self.report_len))
    }

    fn set_volume(&self, target: u8, n: i16) -> QResult {
        self.send_frame(&proto::set_volume_frame(target, n, self.report_len))
    }
}

/// Query the HID output-report byte length for a device path via the Windows HID
/// API. `HidP_GetCaps().OutputReportByteLength` includes the leading report-id
/// byte, so we subtract 1 to get the frame (data) length. Returns `None` on any
/// failure, letting the caller fall back to the default.
fn query_output_report_len(path: &CStr) -> Option<usize> {
    use std::iter::once;

    use windows::core::PCWSTR;
    use windows::Win32::Devices::HumanInterfaceDevice::{
        HidD_FreePreparsedData, HidD_GetPreparsedData, HidP_GetCaps, HIDP_CAPS, HIDP_STATUS_SUCCESS,
    };
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::Storage::FileSystem::{
        CreateFileW, FILE_FLAGS_AND_ATTRIBUTES, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
    };

    // The Windows HID device path is ASCII; widen it for CreateFileW.
    let wide: Vec<u16> = path.to_bytes().iter().map(|&b| b as u16).chain(once(0)).collect();

    unsafe {
        // Desired access 0 = query only (don't contend with hidapi's handle).
        let handle = CreateFileW(
            PCWSTR(wide.as_ptr()),
            0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_FLAGS_AND_ATTRIBUTES(0),
            None,
        )
        .ok()?;

        let mut preparsed = Default::default();
        let got = HidD_GetPreparsedData(handle, &mut preparsed).as_bool();

        let out_len = if got {
            let mut caps = HIDP_CAPS::default();
            let status = HidP_GetCaps(preparsed, &mut caps);
            let _ = HidD_FreePreparsedData(preparsed);
            if status == HIDP_STATUS_SUCCESS {
                Some(caps.OutputReportByteLength as usize)
            } else {
                None
            }
        } else {
            None
        };

        let _ = CloseHandle(handle);

        // Strip the report-id byte; reject a nonsensical zero length.
        out_len.map(|n| n.saturating_sub(1)).filter(|&n| n > 0)
    }
}
