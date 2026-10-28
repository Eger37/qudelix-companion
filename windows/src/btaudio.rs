//! Drop / restore this PC's own Bluetooth audio links to the 5K — the same thing
//! "Disconnect" / "Connect" next to a Bluetooth output in Windows Sound settings
//! does: a one-shot KSPROPSETID_BtAudio property on the endpoint's KS filter.
//! No admin rights; the pairing stays. The 5K itself has no "drop host X"
//! command (checked the whole opcode table of the official app), so this is the
//! only way to make it fall back to its other source.
//!
//! Finding the 5K's endpoints: Windows can have TWO sets for one Bluetooth device —
//! the classic Bluetooth audio driver (KS filter id `…bthenum#{0000110b-…}…&<addr>_…`)
//! and, on PCs with Intel Smart Sound Bluetooth offload, the offload driver
//! (`…intelaudio#…\intcbttoporender_…`, no address in it). Only one set is live at a
//! time, so match an endpoint by the address in its filter id OR by its name,
//! which Windows always builds from the device name: "Headphones (Qudelix-5K)".
//! Music vs calls by form factor: Headphones = A2DP, Headset = HFP.
//! Only Disconnect drops links here (disconnect_device, Settings' own way); the
//! widget's Unlink just moves the PC's output and microphone (worker::Roles).

use windows::core::{Interface, PCWSTR};
use windows::Win32::Devices::FunctionDiscovery::PKEY_Device_FriendlyName;
use windows::Win32::Media::Audio::{
    eAll, eRender, IDeviceTopology, IMMDevice, IMMDeviceEnumerator, IPart, MMDeviceEnumerator, DEVICE_STATE,
    DEVICE_STATEMASK_ALL, DEVICE_STATE_ACTIVE, PKEY_AudioEndpoint_FormFactor,
};
use windows::Win32::Media::KernelStreaming::{
    IKsControl, KSIDENTIFIER, KSIDENTIFIER_0, KSIDENTIFIER_0_0, KSPROPERTY_ONESHOT_DISCONNECT,
    KSPROPERTY_ONESHOT_RECONNECT, KSPROPERTY_TYPE_GET, KSPROPSETID_BtAudio,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CLSCTX_ALL, COINIT_APARTMENTTHREADED, STGM_READ,
};

/// The paired Bluetooth device: its name (as in "Headphones (<name>)") and address.
#[derive(Clone, Debug)]
pub struct BtDevice {
    pub name: String,
    pub addr: u64,
}

/// One audio endpoint of the device.
struct Ep {
    id: String,
    name: String,
    /// DEVICE_STATE_* (1 active, 4 unplugged, …).
    state: u32,
    render: bool,
    /// A2DP ("Headphones"); otherwise HFP ("Headset").
    music: bool,
    /// Candidate KS filter ids, best first (topology object, then connector target).
    filters: Vec<String>,
}

/// Form factors (EndpointFormFactor): Headphones = 3, Headset = 5.
const FORM_HEADSET: u32 = 5;

/// Disconnect (`connect = false`) or reconnect this PC's music and calls links to
/// `dev`. Returns how many KS filters took the request.
pub fn set_connected(dev: &BtDevice, connect: bool) -> Result<usize, String> {
    let prop = if connect { KSPROPERTY_ONESHOT_RECONNECT } else { KSPROPERTY_ONESHOT_DISCONNECT };
    unsafe {
        let en = enumerator()?;
        // All states: once disconnected, the endpoints are "unplugged", not gone.
        let eps: Vec<Ep> = endpoints(&en, dev, DEVICE_STATE(DEVICE_STATEMASK_ALL))?;
        let mut tried: Vec<String> = Vec::new();
        let mut done = 0;
        // Live endpoints first: their filters are the ones that can be opened.
        for e in eps.iter().filter(|e| e.state == 1).chain(eps.iter().filter(|e| e.state != 1)) {
            for fid in &e.filters {
                if tried.contains(fid) {
                    continue;
                }
                tried.push(fid.clone());
                match ks_oneshot(&en, fid, prop.0 as u32) {
                    Ok(()) => {
                        crate::log::line(&format!("[btaudio] {:?} (state {}) ← {fid}", e.name, e.state));
                        done += 1;
                    }
                    Err(err) => crate::log::line(&format!("[btaudio] {:?} {fid} → {err}", e.name)),
                }
            }
        }
        if done == 0 {
            Err(format!("no usable Bluetooth audio endpoint of {:?} on this PC", dev.name))
        } else {
            Ok(done)
        }
    }
}

/// What of `dev` is live on this PC right now: (music = an A2DP output,
/// calls = a hands-free endpoint).
pub fn live(dev: &BtDevice) -> (bool, bool) {
    unsafe {
        let Ok(en) = enumerator() else { return (false, false) };
        let eps = endpoints(&en, dev, DEVICE_STATE_ACTIVE).unwrap_or_default();
        (eps.iter().any(|e| e.render && e.music), eps.iter().any(|e| !e.music))
    }
}

/// Whether this PC's music (A2DP) output of `dev` is live — i.e. "linked".
pub fn is_linked(dev: &BtDevice) -> bool {
    live(dev).0
}

/// Active playback (`capture` = false) or microphone endpoints of `dev` (ids as
/// the sound-device lists have them), the music (A2DP) one first.
pub fn audio_endpoints(dev: &BtDevice, capture: bool) -> Vec<String> {
    unsafe {
        let Ok(en) = enumerator() else { return Vec::new() };
        let mut eps: Vec<Ep> = endpoints(&en, dev, DEVICE_STATE_ACTIVE)
            .unwrap_or_default()
            .into_iter()
            .filter(|e| e.render != capture)
            .collect();
        eps.sort_by_key(|e| !e.music);
        eps.into_iter().map(|e| e.id).collect()
    }
}

/// Every endpoint (render + capture) in `state` that belongs to `dev`.
unsafe fn endpoints(en: &IMMDeviceEnumerator, dev: &BtDevice, state: DEVICE_STATE) -> Result<Vec<Ep>, String> {
    let needle = format!("{:012x}", dev.addr);
    let suffix = format!("({})", dev.name);
    let all = en.EnumAudioEndpoints(eAll, state).map_err(|e| format!("EnumAudioEndpoints: {e}"))?;
    let render_ids: Vec<String> = match en.EnumAudioEndpoints(eRender, state) {
        Ok(r) => (0..r.GetCount().unwrap_or(0)).filter_map(|i| r.Item(i).ok().and_then(|d| pwstr(d.GetId()))).collect(),
        Err(_) => Vec::new(),
    };
    let mut out = Vec::new();
    for i in 0..all.GetCount().unwrap_or(0) {
        let Ok(ep) = all.Item(i) else { continue };
        let Some(id) = pwstr(ep.GetId()) else { continue };
        let filters = filter_ids(&ep);
        let props = ep.OpenPropertyStore(STGM_READ).ok();
        let name = props
            .as_ref()
            .and_then(|p| p.GetValue(&PKEY_Device_FriendlyName).ok())
            .map(|v| v.to_string())
            .unwrap_or_default();
        let by_addr = filters.iter().any(|f| f.to_lowercase().contains(&needle));
        if !by_addr && !name.ends_with(&suffix) {
            continue;
        }
        let form = props
            .as_ref()
            .and_then(|p| p.GetValue(&PKEY_AudioEndpoint_FormFactor).ok())
            .and_then(|v| u32::try_from(&v).ok())
            .unwrap_or(0);
        let hfp_filter = filters.iter().any(|f| f.to_lowercase().contains("0000111e"));
        out.push(Ep {
            render: render_ids.contains(&id),
            music: form != FORM_HEADSET && !hfp_filter,
            state: ep.GetState().map(|s| s.0).unwrap_or(0),
            id,
            name,
            filters,
        });
    }
    Ok(out)
}

/// Send one KSPROPSETID_BtAudio one-shot property to the KS filter `fid`.
unsafe fn ks_oneshot(en: &IMMDeviceEnumerator, fid: &str, id: u32) -> windows::core::Result<()> {
    let wide: Vec<u16> = fid.encode_utf16().chain(std::iter::once(0)).collect();
    let ks: IKsControl = en.GetDevice(PCWSTR(wide.as_ptr()))?.Activate(CLSCTX_ALL, None)?;
    let p = KSIDENTIFIER {
        Anonymous: KSIDENTIFIER_0 { Anonymous: KSIDENTIFIER_0_0 { Set: KSPROPSETID_BtAudio, Id: id, Flags: KSPROPERTY_TYPE_GET } },
    };
    let mut returned = 0u32;
    ks.KsProperty(&p, std::mem::size_of::<KSIDENTIFIER>() as u32, std::ptr::null_mut(), 0, &mut returned)
}

/// Candidate ids of the KS filter behind an endpoint: the topology object of the
/// part its connector is wired to (works while live), then the connector target
/// id itself (all that's left once the endpoint is unplugged).
unsafe fn filter_ids(ep: &IMMDevice) -> Vec<String> {
    let mut out = Vec::new();
    let Ok(topo) = ep.Activate::<IDeviceTopology>(CLSCTX_ALL, None) else { return out };
    let Ok(conn) = topo.GetConnector(0) else { return out };
    if let Some(id) = conn
        .GetConnectedTo()
        .ok()
        .and_then(|c| c.cast::<IPart>().ok())
        .and_then(|p| p.GetTopologyObject().ok())
        .and_then(|t| pwstr(t.GetDeviceId()))
    {
        out.push(id);
    }
    if let Some(id) = pwstr(conn.GetDeviceIdConnectedTo()) {
        if !out.contains(&id) {
            out.push(id);
        }
    }
    out
}

/// Bluetooth services of the device, as BluetoothSetServiceState switches them.
#[derive(Clone, Copy, Debug)]
pub enum Service {
    /// A2DP audio sink (0x110B): music.
    Music,
    /// Hands-free (0x111E): calls.
    Calls,
}

/// Switch one Bluetooth service of the paired device `addr` off or on — the
/// checkboxes under Devices and Printers → device → Services; no admin needed.
/// Off: Windows drops that profile's link and its audio endpoints (the way to
/// unlink when the one-shot KS disconnect is ignored, as by the Intel offload
/// driver). On: Windows reinstalls the endpoints and connects them.
/// Returns the Win32 error code (0 = ok; 87 when it's already in that state).
pub fn set_service(addr: u64, service: Service, on: bool) -> u32 {
    use windows::Win32::Devices::Bluetooth::{
        BluetoothGetDeviceInfo, BluetoothSetServiceState, BLUETOOTH_ADDRESS, BLUETOOTH_ADDRESS_0,
        BLUETOOTH_DEVICE_INFO, BLUETOOTH_SERVICE_DISABLE, BLUETOOTH_SERVICE_ENABLE,
    };
    use windows::Win32::Foundation::CloseHandle;
    let uuid = match service {
        Service::Music => windows::core::GUID::from_u128(0x0000110b_0000_1000_8000_00805f9b34fb),
        Service::Calls => windows::core::GUID::from_u128(0x0000111e_0000_1000_8000_00805f9b34fb),
    };
    unsafe {
        let radio = first_radio();
        let mut info = BLUETOOTH_DEVICE_INFO {
            dwSize: std::mem::size_of::<BLUETOOTH_DEVICE_INFO>() as u32,
            Address: BLUETOOTH_ADDRESS { Anonymous: BLUETOOTH_ADDRESS_0 { ullLong: addr } },
            ..Default::default()
        };
        let mut r = BluetoothGetDeviceInfo(radio, &mut info);
        if r == 0 {
            let flag = if on { BLUETOOTH_SERVICE_ENABLE } else { BLUETOOTH_SERVICE_DISABLE };
            r = BluetoothSetServiceState(radio, &info, &uuid, flag);
        }
        crate::log::line(&format!("[btaudio] {service:?} service {} → {r}", if on { "on" } else { "off" }));
        if !radio.0.is_null() {
            let _ = CloseHandle(radio);
        }
        r
    }
}

/// Switch a service off and on: Windows reinstalls its audio endpoints and
/// connects them — reconnects even when the endpoint is "unplugged" and its KS
/// filter can't be opened. Returns the Win32 error code of the "on" (0 = ok).
pub fn cycle_service(addr: u64, service: Service) -> u32 {
    set_service(addr, service, false);
    set_service(addr, service, true)
}

/// Bring music back on this PC (and calls, if Windows brings them along): service
/// on, the one-shot KS reconnect, then cycle the music service if it's still down.
/// NEVER switches the calls (hands-free) service off: on the owner's PC (Intel
/// offload) Windows then didn't recreate the Headset endpoint — the 5K's
/// microphone stayed gone (2026-10-18). Only switches it back on, in case an
/// older version left it off. Waits up to ~30 s. Returns what is live: (music, calls).
pub fn restore(dev: &BtDevice) -> (bool, bool) {
    set_service(dev.addr, Service::Music, true); // 87 = it was on already — fine
    set_service(dev.addr, Service::Calls, true);
    let wait = |secs: u32| {
        for _ in 0..secs {
            if live(dev) == (true, true) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(1000));
        }
        live(dev)
    };
    if live(dev) != (true, true) {
        if let Err(e) = set_connected(dev, true) {
            crate::log::line(&format!("[btaudio] KS reconnect: {e}"));
        }
    }
    let (music, calls) = wait(10);
    if music {
        return (music, calls);
    }
    crate::log::line("[btaudio] music still down after reconnect — cycling the music service");
    cycle_service(dev.addr, Service::Music);
    wait(20)
}

/// Whether Windows has a radio link (ACL) to `addr` at all — any profile, or just
/// the widget's control channel.
pub fn acl_connected(addr: u64) -> bool {
    use windows::Win32::Devices::Bluetooth::{BluetoothGetDeviceInfo, BLUETOOTH_ADDRESS, BLUETOOTH_ADDRESS_0, BLUETOOTH_DEVICE_INFO};
    use windows::Win32::Foundation::CloseHandle;
    unsafe {
        let radio = first_radio();
        let mut info = BLUETOOTH_DEVICE_INFO {
            dwSize: std::mem::size_of::<BLUETOOTH_DEVICE_INFO>() as u32,
            Address: BLUETOOTH_ADDRESS { Anonymous: BLUETOOTH_ADDRESS_0 { ullLong: addr } },
            ..Default::default()
        };
        let ok = BluetoothGetDeviceInfo(radio, &mut info) == 0 && info.fConnected.as_bool();
        if !radio.0.is_null() {
            let _ = CloseHandle(radio);
        }
        ok
    }
}

/// Drop the radio link (ACL) to `dev` exactly the way Windows Settings'
/// "Disconnect" does (reversed from SettingsHandlers_Devices.dll): first the KS
/// one-shot disconnect on every audio endpoint, then BluetoothDisconnectDevice
/// (BluetoothApis.dll, loaded at run time like Settings does; not in the SDK
/// headers). No service toggles. 1.6.21 sent the raw IOCTL after switching the
/// services off, and the Intel audio DSP ("Smart Sound Technology OED") failed
/// 5 s later; Settings' own order survived on the owner's laptop (2026-10-28).
/// Returns the API's result (0 = sent).
pub fn disconnect_device(dev: &BtDevice) -> Result<(), String> {
    use windows::core::{s, w};
    use windows::Win32::Devices::Bluetooth::{BLUETOOTH_ADDRESS, BLUETOOTH_ADDRESS_0};
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
    type Disconnect = unsafe extern "system" fn(HANDLE, *const BLUETOOTH_ADDRESS) -> u32;

    if let Err(e) = set_connected(dev, false) {
        crate::log::line(&format!("[btaudio] KS disconnect: {e}"));
    }
    unsafe {
        let lib = LoadLibraryW(w!("BluetoothApis.dll")).map_err(|e| format!("BluetoothApis.dll: {e}"))?;
        let f = GetProcAddress(lib, s!("BluetoothDisconnectDevice")).ok_or("no BluetoothDisconnectDevice")?;
        let f: Disconnect = std::mem::transmute(f);
        let addr = BLUETOOTH_ADDRESS { Anonymous: BLUETOOTH_ADDRESS_0 { ullLong: dev.addr } };
        // NULL radio = every radio, as Settings calls it.
        match f(HANDLE::default(), &addr) {
            0 => Ok(()),
            r => Err(format!("BluetoothDisconnectDevice: {r}")),
        }
    }
}

/// Handle of the first Bluetooth radio (null if none — the APIs then pick one).
unsafe fn first_radio() -> windows::Win32::Foundation::HANDLE {
    use windows::Win32::Devices::Bluetooth::{BluetoothFindFirstRadio, BluetoothFindRadioClose, BLUETOOTH_FIND_RADIO_PARAMS};
    let params = BLUETOOTH_FIND_RADIO_PARAMS { dwSize: std::mem::size_of::<BLUETOOTH_FIND_RADIO_PARAMS>() as u32 };
    let mut radio = windows::Win32::Foundation::HANDLE::default();
    if let Ok(find) = BluetoothFindFirstRadio(&params, &mut radio) {
        let _ = BluetoothFindRadioClose(find);
    }
    radio
}

/// This PC's own Bluetooth address (first radio).
pub fn local_address() -> Option<u64> {
    use windows::Win32::Devices::Bluetooth::{BluetoothGetRadioInfo, BLUETOOTH_RADIO_INFO};
    use windows::Win32::Foundation::CloseHandle;
    unsafe {
        let radio = first_radio();
        if radio.0.is_null() {
            return None;
        }
        let mut info = BLUETOOTH_RADIO_INFO { dwSize: std::mem::size_of::<BLUETOOTH_RADIO_INFO>() as u32, ..Default::default() };
        let r = BluetoothGetRadioInfo(radio, &mut info);
        let _ = CloseHandle(radio);
        (r == 0).then(|| info.address.Anonymous.ullLong)
    }
}

/// Diagnostics (`--bt-endpoints`, read-only — never sends a connect/disconnect):
/// the device's endpoints with state, kind, filters and whether each filter opens
/// as an IKsControl. Lines go to the log.
pub fn diagnose(dev: &BtDevice) {
    crate::log::line(&format!(
        "[bt-diag] {:?} {:012x}, this PC {:012x}, linked={}",
        dev.name,
        dev.addr,
        local_address().unwrap_or(0),
        is_linked(dev)
    ));
    unsafe {
        let Ok(en) = enumerator() else { return };
        for e in endpoints(&en, dev, DEVICE_STATE(DEVICE_STATEMASK_ALL)).unwrap_or_default() {
            crate::log::line(&format!(
                "[bt-diag] {:?} state={} render={} music={} id={}",
                e.name, e.state, e.render, e.music, e.id
            ));
            for fid in &e.filters {
                let wide: Vec<u16> = fid.encode_utf16().chain(std::iter::once(0)).collect();
                let ks = en.GetDevice(PCWSTR(wide.as_ptr())).and_then(|d| d.Activate::<IKsControl>(CLSCTX_ALL, None));
                crate::log::line(&format!("[bt-diag]   {fid} → IKsControl {}", if ks.is_ok() { "ok" } else { "no" }));
            }
        }
    }
}

unsafe fn enumerator() -> Result<IMMDeviceEnumerator, String> {
    let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED); // already initialised (any mode) is fine
    CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).map_err(|e| format!("MMDeviceEnumerator: {e}"))
}

/// Take a COM-allocated string, freeing it.
unsafe fn pwstr(p: windows::core::Result<windows::core::PWSTR>) -> Option<String> {
    let p = p.ok()?;
    let s = p.to_string().ok();
    CoTaskMemFree(Some(p.0 as *const _));
    s
}
