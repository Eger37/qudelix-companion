//! The PC's sound output: list the active playback devices and make one the
//! default — what picking an output in the Windows volume flyout does.
//!
//! Setting the default goes through IPolicyConfig: undocumented, but stable since
//! Windows 7 and what every output-switcher tool uses (there is no public API).

use std::ffi::c_void;

use windows::core::{Interface, GUID, HRESULT, PCWSTR};
use windows::Win32::Devices::FunctionDiscovery::PKEY_Device_FriendlyName;
use windows::Win32::Media::Audio::{
    eCapture, eCommunications, eConsole, eMultimedia, eRender, EDataFlow, ERole, IMMDevice, IMMDeviceEnumerator,
    MMDeviceEnumerator, DEVICE_STATE_ACTIVE,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CLSCTX_ALL, COINIT_APARTMENTTHREADED, STGM_READ,
};

/// One playback device: endpoint id, friendly name, whether it is the default now.
pub struct Output {
    pub id: String,
    pub name: String,
    pub default: bool,
}

/// Active playback devices, in Windows' order.
pub fn list() -> Result<Vec<Output>, String> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED); // already initialised is fine
        let en = enumerator()?;
        let default = en
            .GetDefaultAudioEndpoint(eRender, eMultimedia)
            .ok()
            .and_then(|d| device_id(&d));
        let all = en
            .EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE)
            .map_err(|e| format!("EnumAudioEndpoints: {e}"))?;
        let mut out = Vec::new();
        for i in 0..all.GetCount().unwrap_or(0) {
            let Ok(dev) = all.Item(i) else { continue };
            let Some(id) = device_id(&dev) else { continue };
            let name = dev
                .OpenPropertyStore(STGM_READ)
                .and_then(|p| p.GetValue(&PKEY_Device_FriendlyName))
                .map(|v| v.to_string())
                .unwrap_or_else(|_| id.clone());
            out.push(Output { default: default.as_deref() == Some(id.as_str()), id, name });
        }
        Ok(out)
    }
}

/// One of the defaults Windows keeps: the output or the microphone, either the
/// plain default (console + multimedia) or the communications one.
#[derive(Clone, Copy, Debug)]
pub struct Role {
    pub capture: bool,
    pub comms: bool,
}

/// All four, in a fixed order (callers index their own arrays by it).
pub const ROLES: [Role; 4] = [
    Role { capture: false, comms: false },
    Role { capture: false, comms: true },
    Role { capture: true, comms: false },
    Role { capture: true, comms: true },
];

impl Role {
    fn flow(self) -> EDataFlow {
        if self.capture { eCapture } else { eRender }
    }
    fn roles(self) -> &'static [ERole] {
        if self.comms { &[eCommunications] } else { &[eConsole, eMultimedia] }
    }
}

/// The endpoint id holding `r` now; None = no device.
pub fn default_of(r: Role) -> Option<String> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let role = if r.comms { eCommunications } else { eConsole };
        enumerator().ok()?.GetDefaultAudioEndpoint(r.flow(), role).ok().and_then(|d| device_id(&d))
    }
}

/// Ids of the active outputs (`capture` = false) or microphones, in Windows' order.
pub fn active_ids(capture: bool) -> Vec<String> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let flow = if capture { eCapture } else { eRender };
        let Ok(all) = enumerator().and_then(|e| e.EnumAudioEndpoints(flow, DEVICE_STATE_ACTIVE).map_err(|e| e.to_string()))
        else {
            return Vec::new();
        };
        (0..all.GetCount().unwrap_or(0)).filter_map(|i| all.Item(i).ok().and_then(|d| device_id(&d))).collect()
    }
}

/// Make `id` (an output or a microphone) the device for `r`.
pub fn set_role(id: &str, r: Role) -> Result<(), String> {
    set_roles(id, r.roles())
}

/// Make `id` the default playback device for every role (console, multimedia, calls).
pub fn set_default(id: &str) -> Result<(), String> {
    set_roles(id, &[eConsole, eMultimedia, eCommunications])
}

fn set_roles(id: &str, roles: &[ERole]) -> Result<(), String> {
    // CPolicyConfigClient / IPolicyConfig (Windows 7+).
    const CLSID_POLICY_CONFIG: GUID = GUID::from_u128(0x870af99c_171d_4f9e_af0d_e63df40c2bc9);
    const IID_POLICY_CONFIG: GUID = GUID::from_u128(0xf8679f50_850a_41cf_9c72_430f290290c8);
    /// Vtable slot of SetDefaultEndpoint: IUnknown (3) + 10 methods before it.
    const SET_DEFAULT_ENDPOINT: usize = 13;
    type SetDefault = unsafe extern "system" fn(*mut c_void, PCWSTR, ERole) -> HRESULT;

    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let unk: windows::core::IUnknown = CoCreateInstance(&CLSID_POLICY_CONFIG, None, CLSCTX_ALL)
            .map_err(|e| format!("PolicyConfig: {e}"))?;
        let mut pc: *mut c_void = std::ptr::null_mut();
        unk.query(&IID_POLICY_CONFIG, &mut pc).ok().map_err(|e| format!("IPolicyConfig: {e}"))?;
        let vtbl = *(pc as *const *const usize);
        let set: SetDefault = std::mem::transmute(*vtbl.add(SET_DEFAULT_ENDPOINT));
        let wide: Vec<u16> = id.encode_utf16().chain(std::iter::once(0)).collect();
        let mut r = Ok(());
        for &role in roles {
            if let Err(e) = set(pc, PCWSTR(wide.as_ptr()), role).ok() {
                r = Err(format!("SetDefaultEndpoint: {e}"));
            }
        }
        // Release the reference query() added.
        let release: unsafe extern "system" fn(*mut c_void) -> u32 = std::mem::transmute(*vtbl.add(2));
        release(pc);
        r
    }
}

unsafe fn enumerator() -> Result<IMMDeviceEnumerator, String> {
    CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).map_err(|e| format!("MMDeviceEnumerator: {e}"))
}

unsafe fn device_id(d: &IMMDevice) -> Option<String> {
    let p = d.GetId().ok()?;
    let s = p.to_string().ok();
    CoTaskMemFree(Some(p.0 as *const _));
    s
}
