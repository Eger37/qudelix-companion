//! Background transport thread + CLI/diagnostic entry points.
//!
//! The real control path is a GAIA session over classic RFCOMM (`rfcomm.rs`):
//! connect via SDP → handshake → commands → release IDLE_RELEASE after the last
//! click. Connecting blocks, so it lives on this thread: the UI only drops
//! commands in a channel (never blocks) and polls `status` / `dev_state`.
//!
//! The USB-HID / BLE probes at the bottom (`--set-priority`, `--set-volume`,
//! `--raw`) are the early transports; kept for diagnostics (USB-HID still works
//! when the 5K is on a USB data cable).

use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use crate::device::Qudelix;
use crate::proto;
use crate::usb::UsbHid;

/// Release the 5K this soon after the last click: it serves ONE controller at a
/// time, so holding it would lock out the phone. Short bursts of clicks share
/// one session.
#[cfg(feature = "ble")]
const IDLE_RELEASE: std::time::Duration = std::time::Duration::from_secs(3);

/// A command the UI asks the device to perform.
enum Cmd {
    Priority(u8),
    Volume(i16),
    /// Connect, read volume / priority / playing source, release.
    Refresh,
    /// Change this PC's Bluetooth links to the 5K. On a drop, the PC's sound goes
    /// to the fallback output and the 5K's priority to the source that is left.
    PcAudio(PcLink),
}

/// What the PcAudio buttons ask for.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PcLink {
    /// Drop music and calls (A2DP + HFP); pairing and radio link stay.
    Unlink,
    /// Unlink + drop the radio link: the full Bluetooth disconnect.
    Disconnect,
    /// Bring music and calls back (after Unlink), and the PC's sound with them.
    Link,
    /// The same after Disconnect.
    Connect,
}

/// Handle the UI keeps; sending never blocks.
pub struct Worker {
    /// None only while dropping (closing the channel stops the thread).
    tx: Option<Sender<Cmd>>,
    status: Arc<Mutex<String>>,
    dev: Arc<Mutex<crate::state::DevState>>,
    /// What of the 5K is live on this PC (Windows' view), checked every
    /// IDLE_RELEASE: bit 0 known, bit 1 music, bit 2 calls, bit 3 radio link.
    /// Drives the Unlink/Link and Disconnect/Connect labels.
    pc_linked: Arc<std::sync::atomic::AtomicU8>,
    thread: Option<JoinHandle<()>>,
}

/// On exit, wait this long for the thread to close the session cleanly
/// (stop stream, half-close: ~1.5 s at most) before the process ends anyway.
const JOIN_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(3);

impl Drop for Worker {
    fn drop(&mut self) {
        drop(self.tx.take()); // the thread sees Disconnected and drops its session
        if let Some(t) = self.thread.take() {
            let start = std::time::Instant::now();
            while !t.is_finished() && start.elapsed() < JOIN_TIMEOUT {
                thread::sleep(std::time::Duration::from_millis(20));
            }
            if t.is_finished() {
                let _ = t.join();
            } else {
                crate::log::line("[worker] still busy at exit — not waiting");
            }
        }
    }
}

impl Worker {
    pub fn start() -> Self {
        let (tx, rx) = mpsc::channel::<Cmd>();
        let status = Arc::new(Mutex::new("Ready".to_string()));

        let dev = Arc::new(Mutex::new(crate::state::DevState::default()));
        let st = status.clone();
        let dv = dev.clone();
        #[cfg(feature = "ble")]
        let dev_read = dev.clone();
        let pc_linked = Arc::new(std::sync::atomic::AtomicU8::new(0));
        #[cfg(feature = "ble")]
        let linked = pc_linked.clone();
        let thread = thread::spawn(move || {
            let set_status = |s: &str| {
                if let Ok(mut g) = st.lock() {
                    *g = s.to_string();
                }
            };
            init_com(); // WinRT (address lookup) needs a COM apartment on this thread.

            // One persistent session, opened on the first click (like the official
            // app); `rfcomm::Session` drains incoming data so the 5K keeps it. The 5K
            // has ONE control channel for all hosts, so release it after IDLE_RELEASE
            // without clicks — then the phone app can connect.
            #[cfg(feature = "ble")]
            {
                use std::sync::mpsc::RecvTimeoutError;
                let mut session: Option<crate::rfcomm::Session> = None;
                let sink: crate::rfcomm::FrameSink = Arc::new(move |f: &[u8]| {
                    if let Ok(mut d) = dv.lock() {
                        d.apply_frame(f);
                    }
                });
                // The 5K as a Windows Bluetooth device (looked up once, retried if missing).
                let mut bt: Option<crate::btaudio::BtDevice> = None;
                let check_linked = |bt: &mut Option<crate::btaudio::BtDevice>| {
                    if bt.is_none() {
                        *bt = crate::ble::find_qudelix().ok().map(|(name, addr)| crate::btaudio::BtDevice { name, addr });
                    }
                    if let Some(d) = bt.as_ref() {
                        let (music, calls) = crate::btaudio::live(d);
                        let acl = crate::btaudio::acl_connected(d.addr);
                        let v = 1 | (music as u8) << 1 | (calls as u8) << 2 | (acl as u8) << 3;
                        linked.store(v, std::sync::atomic::Ordering::Relaxed);
                    }
                };
                check_linked(&mut bt);
                loop {
                    let first = match rx.recv_timeout(IDLE_RELEASE) {
                        Ok(c) => c,
                        Err(RecvTimeoutError::Timeout) => {
                            if session.take().is_some() {
                                crate::log::line("[worker] idle — released the 5K for other hosts");
                                set_status("Ready");
                            }
                            check_linked(&mut bt);
                            continue;
                        }
                        Err(RecvTimeoutError::Disconnected) => {
                            if session.take().is_some() {
                                crate::log::line("[worker] exit — released the 5K");
                            }
                            break;
                        }
                    };
                    // Rapid clicks queue up; only the latest of each kind matters.
                    // Sending stale steps just keeps the 5K busy (and it lags).
                    let mut pri = None;
                    let mut vol = None;
                    let mut refresh = false;
                    let mut pc_audio = None;
                    for c in std::iter::once(first).chain(rx.try_iter()) {
                        match c {
                            Cmd::Priority(_) => pri = Some(c),
                            Cmd::Volume(_) => vol = Some(c),
                            Cmd::Refresh => refresh = true,
                            Cmd::PcAudio(m) => pc_audio = Some(m),
                        }
                    }
                    if let Some(m) = pc_audio {
                        // Unknown while it runs: the UI keeps its optimistic label.
                        linked.store(0, std::sync::atomic::Ordering::Relaxed);
                        set_pc_audio(&mut session, &sink, &dev_read, m, &set_status);
                        check_linked(&mut bt);
                        continue;
                    }
                    let sent = pri.is_some() || vol.is_some();
                    for cmd in pri.into_iter().chain(vol) {
                        send_one(&mut session, &sink, &cmd, &set_status);
                    }
                    if refresh {
                        refresh_state(&mut session, &sink, &set_status);
                    } else if sent {
                        // Every click also reads the state back, so the widget
                        // shows what the 5K really did (the label stays).
                        read_back(&mut session);
                    }
                }
            }
            #[cfg(not(feature = "ble"))]
            while rx.recv().is_ok() {}
        });

        Worker { tx: Some(tx), status, dev, pc_linked, thread: Some(thread) }
    }

    fn send(&self, c: Cmd) {
        if let Some(tx) = &self.tx {
            let _ = tx.send(c);
        }
    }

    pub fn priority(&self, v: u8) {
        self.send(Cmd::Priority(v));
    }

    pub fn volume(&self, n: i16) {
        self.send(Cmd::Volume(n));
    }

    /// Show a note on the status line right away (from the UI thread).
    pub fn note(&self, s: &str) {
        if let Ok(mut g) = self.status.lock() {
            *g = s.to_string();
        }
    }

    /// Show a note on the status line (e.g. from the update checker thread).
    pub fn status_arc(&self) -> Arc<Mutex<String>> {
        self.status.clone()
    }

    pub fn refresh(&self) {
        self.send(Cmd::Refresh);
    }

    /// Unlink / disconnect / link this PC's Bluetooth links to the 5K.
    pub fn pc_audio(&self, m: PcLink) {
        self.send(Cmd::PcAudio(m));
    }

    /// What of the 5K is live on this PC: (music, calls, radio link); None = not known yet.
    pub fn pc_live(&self) -> Option<(bool, bool, bool)> {
        let v = self.pc_linked.load(std::sync::atomic::Ordering::Relaxed);
        (v & 1 != 0).then_some((v & 2 != 0, v & 4 != 0, v & 8 != 0))
    }

    /// Latest state read back from the 5K (empty fields = not known yet).
    pub fn dev_state(&self) -> crate::state::DevState {
        self.dev.lock().map(|g| g.clone()).unwrap_or_default()
    }

    pub fn status(&self) -> String {
        self.status.lock().map(|g| g.clone()).unwrap_or_default()
    }
}

/// Send one command on the persistent session, (re)opening it if needed.
#[cfg(feature = "ble")]
fn send_one(
    session: &mut Option<crate::rfcomm::Session>,
    sink: &crate::rfcomm::FrameSink,
    cmd: &Cmd,
    set_status: &dyn Fn(&str),
) {
    // Volume shows no status: the dB readout already shows the result.
    let quiet = matches!(cmd, Cmd::Volume(_));
    if !quiet {
        set_status("sending…");
    }
    let Some(s) = ensure_session(session, sink, set_status) else { return };
    match s.command(&command_frame(cmd)) {
        Ok(()) if quiet => {}
        Ok(()) => set_status(&command_label(cmd)),
        Err(e) => {
            crate::log::line(&format!("[worker] cmd failed: {e}"));
            *session = None;
            set_status("link lost — click again");
        }
    }
}

/// Reuse the open session or open a new one. None (status set) if unreachable.
#[cfg(feature = "ble")]
fn ensure_session<'a>(
    session: &'a mut Option<crate::rfcomm::Session>,
    sink: &crate::rfcomm::FrameSink,
    set_status: &dyn Fn(&str),
) -> Option<&'a crate::rfcomm::Session> {
    if session.as_ref().is_some_and(|s| !s.is_alive()) {
        *session = None;
    }
    if session.is_none() {
        match open_session(sink) {
            Ok(s) => *session = Some(s),
            Err(e) => {
                crate::log::line(&format!("[worker] open failed: {e}"));
                set_status(if e == BUSY {
                    "5K busy — close phone app, else power-cycle 5K"
                } else {
                    "5K not answering"
                });
                return None;
            }
        }
    }
    session.as_ref()
}

/// Refresh: (re)request the full state and give the replies a moment to land.
/// A fresh session already got ReqInitData in its handshake.
#[cfg(feature = "ble")]
fn refresh_state(
    session: &mut Option<crate::rfcomm::Session>,
    sink: &crate::rfcomm::FrameSink,
    set_status: &dyn Fn(&str),
) {
    set_status("reading…");
    let was_open = session.as_ref().is_some_and(|s| s.is_alive());
    let Some(s) = ensure_session(session, sink, set_status) else { return };
    if was_open {
        if let Err(e) = s.request_state() {
            crate::log::line(&format!("[worker] refresh failed: {e}"));
            *session = None;
            set_status("link lost — try again");
            return;
        }
    }
    std::thread::sleep(std::time::Duration::from_millis(1200));
    set_status("updated");
}

/// Change this PC's Bluetooth links to the 5K. Dropping: move the PC's sound to
/// the fallback output first (so nothing plays into a dead device), release the
/// control session, disconnect, give the 5K a moment, then read which A2DP slot
/// is left and point the priority there (USB if none) — so the other source
/// keeps the 5K even once this PC comes back. Link: reconnect, then send the
/// PC's sound back to the 5K once its output shows up.
#[cfg(feature = "ble")]
fn set_pc_audio(
    session: &mut Option<crate::rfcomm::Session>,
    sink: &crate::rfcomm::FrameSink,
    dev: &Arc<Mutex<crate::state::DevState>>,
    mode: PcLink,
    set_status: &dyn Fn(&str),
) {
    crate::log::line(&format!("[pc-audio] {mode:?}"));
    set_status(match mode {
        PcLink::Unlink => "unlinking PC…",
        PcLink::Link => "linking PC…",
        PcLink::Disconnect => "disconnecting PC…",
        PcLink::Connect => "connecting PC…",
    });
    *session = None;
    let bt = match crate::ble::find_qudelix() {
        Ok((name, addr)) => crate::btaudio::BtDevice { name, addr },
        Err(e) => {
            crate::log::line(&format!("[pc-audio] {e}"));
            set_status("5K not paired");
            return;
        }
    };
    if matches!(mode, PcLink::Link | PcLink::Connect) {
        set_status(&link_pc(&bt, mode == PcLink::Connect));
        return;
    }
    output_away_from(&bt);
    let full = mode == PcLink::Disconnect;
    // Point the 5K's priority away from this PC FIRST: asking the 5K afterwards
    // would open the control channel and bring the Bluetooth link back.
    let moved = point_priority_away(session, sink, dev, set_status);
    *session = None;
    if let Err(e) = crate::btaudio::set_connected(&bt, false) {
        crate::log::line(&format!("[pc-audio] KS disconnect: {e}"));
    }
    // The KS one-shot is ignored by some drivers (Intel Smart Sound offload says OK
    // and stays connected): check Windows' view, else switch the MUSIC service off.
    // Never the calls service — Windows may not bring the microphone back (see
    // btaudio::restore). Calls go with the radio link (Disconnect).
    let music_gone = |bt: &crate::btaudio::BtDevice| !crate::btaudio::live(bt).0;
    if !wait_for(|| music_gone(&bt), 3) {
        crate::log::line("[pc-audio] music still connected — switching the music service off");
        crate::btaudio::set_service(bt.addr, crate::btaudio::Service::Music, false);
        if !wait_for(|| music_gone(&bt), 8) {
            crate::log::line("[pc-audio] Windows kept the 5K's music");
            set_status(if full { "couldn't disconnect PC" } else { "couldn't unlink PC" });
            return;
        }
    }
    if full {
        // The radio link itself can't be dropped safely from here (the raw
        // disconnect IOCTL crashed the Intel audio DSP — see btaudio.rs): hand over
        // to Windows' own Disconnect button in Bluetooth settings.
        crate::win::open_url("ms-settings:bluetooth");
        set_status("music off · press Disconnect in Bluetooth settings");
        return;
    }
    let done = "PC unlinked";
    set_status(&match moved {
        Some(to) => format!("{done} · {to}"),
        None => done.to_string(),
    });
}

/// Ask the 5K which hosts it holds and set its priority to the source that stays
/// once this PC leaves (the other host's A2DP slot, else USB). Returns the
/// priority label if it changed.
#[cfg(feature = "ble")]
fn point_priority_away(
    session: &mut Option<crate::rfcomm::Session>,
    sink: &crate::rfcomm::FrameSink,
    dev: &Arc<Mutex<crate::state::DevState>>,
    set_status: &dyn Fn(&str),
) -> Option<String> {
    let me = crate::btaudio::local_address()?;
    if let Ok(mut d) = dev.lock() {
        d.hosts = None; // only trust a fresh reply
    }
    let s = ensure_session(session, sink, set_status)?;
    s.command(&proto::rfcomm::req_conn_status()).ok()?;
    let fresh = || dev.lock().map(|d| d.hosts.is_some()).unwrap_or(false);
    wait_ms(fresh, 1500);
    let (target, current) = dev.lock().map(|d| (d.source_without(me), d.priority)).unwrap_or((None, None));
    crate::log::line(&format!("[pc-audio] source without this PC {target:?}, priority {current:?}"));
    match target {
        Some(v) if Some(v) != current => {
            let cmd = Cmd::Priority(v);
            send_one(session, sink, &cmd, set_status);
            Some(command_label(&cmd))
        }
        _ => None,
    }
}

/// Poll `ok` every 50 ms for up to `ms` milliseconds.
#[cfg(feature = "ble")]
fn wait_ms(ok: impl Fn() -> bool, ms: u64) -> bool {
    let end = std::time::Instant::now() + std::time::Duration::from_millis(ms);
    while !ok() && std::time::Instant::now() < end {
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    ok()
}

/// If the PC's default output is the 5K, switch it to the fallback output
/// (picked in the speaker menu; else the first other active output).
#[cfg(feature = "ble")]
fn output_away_from(bt: &crate::btaudio::BtDevice) {
    let ours = crate::btaudio::playback_endpoints(bt);
    let Ok(outs) = crate::audio_out::list() else { return };
    if !outs.iter().any(|o| o.default && ours.contains(&o.id)) {
        return; // the PC isn't playing to the 5K — leave its output alone
    }
    let others: Vec<_> = outs.iter().filter(|o| !ours.contains(&o.id)).collect();
    let want = crate::config::load_fallback_output();
    let Some(to) = others.iter().find(|o| Some(&o.id) == want.as_ref()).or(others.first()) else {
        crate::log::line("[output] no other output to fall back to");
        return;
    };
    crate::log::line(&format!("[output] fallback → {}", to.name));
    if let Err(e) = crate::audio_out::set_default(&to.id) {
        crate::log::line(&format!("[output] fallback failed: {e}"));
    }
}

/// Link (music) / Connect (music + calls): bring the PC's audio back to the 5K and
/// make it the default output.
/// 1. switch the service(s) back on (Unlink/Disconnect may have switched them off)
///    and send the one-shot KS reconnect; 2. if the music output isn't back, cycle
///    the A2DP service (Windows reinstalls + connects it); 3. if still nothing,
///    open Windows' Bluetooth settings so the user can press Connect there.
/// Returns the status line.
#[cfg(feature = "ble")]
fn link_pc(bt: &crate::btaudio::BtDevice, connect: bool) -> String {
    // Both bring music AND calls (the 5K's microphone) back — Unlink and Disconnect
    // both drop both; only the wording differs.
    let label = if connect { "PC connected" } else { "PC linked" };
    let (music, calls) = crate::btaudio::restore(bt);
    crate::log::line(&format!("[pc-audio] restored: music={music} calls={calls}"));
    if !music {
        crate::log::line("[pc-audio] music not back — opening Bluetooth settings");
        crate::win::open_url("ms-settings:bluetooth");
        return "press Connect for the 5K in Bluetooth settings".to_string();
    }
    let label = if calls { label.to_string() } else { format!("{label} · mic missing") };
    output_back_to(bt, 3, &label).unwrap_or(label)
}

/// Poll `ok` once a second for up to `secs` seconds.
#[cfg(feature = "ble")]
fn wait_for(ok: impl Fn() -> bool, secs: u32) -> bool {
    for _ in 0..secs {
        if ok() {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(1000));
    }
    ok()
}

/// Wait up to `secs` for the 5K's music output to be live, then make it the
/// PC's default output. Returns the status line once linked, None if it never came.
#[cfg(feature = "ble")]
fn output_back_to(bt: &crate::btaudio::BtDevice, secs: u32, label: &str) -> Option<String> {
    for _ in 0..secs {
        if let Some(id) = crate::btaudio::playback_endpoints(bt).into_iter().next() {
            return Some(match crate::audio_out::set_default(&id) {
                Ok(()) => format!("{label} · sound → 5K"),
                Err(e) => {
                    crate::log::line(&format!("[output] back to 5K failed: {e}"));
                    label.to_string()
                }
            });
        }
        std::thread::sleep(std::time::Duration::from_millis(1000));
    }
    None
}

/// Quietly re-request the full state on the open session (after a command).
#[cfg(feature = "ble")]
fn read_back(session: &mut Option<crate::rfcomm::Session>) {
    if let Some(s) = session.as_ref().filter(|s| s.is_alive()) {
        if let Err(e) = s.request_state() {
            crate::log::line(&format!("[worker] read-back failed: {e}"));
            *session = None;
        }
    }
}

/// Find the 5K and open a session, retrying the connect (the 5K can briefly
/// refuse a fresh RFCOMM page → WSAEHOSTDOWN).
#[cfg(feature = "ble")]
fn open_session(sink: &crate::rfcomm::FrameSink) -> Result<crate::rfcomm::Session, String> {
    let addr = crate::ble::find_qudelix_address().map_err(|e| e.to_string())?;
    let _lock = RfcommLock::acquire();
    // Ask the 5K where GAIA is (SDP, Serial Port UUID) and connect there — exactly
    // like the official app; the channel hops anywhere (1, 2, 14…). NEVER probe
    // other channels: half-open connects to them while another controller holds
    // the session left a ghost session in the 5K that refuses everyone until it is
    // power-cycled (seen twice: from the phone in v1.0.7, from this PC's old
    // fallback scan on 2026-10-09 19:27).
    match crate::rfcomm::Session::open_sdp(addr, sink.clone()) {
        Ok(s) => Ok(s),
        Err(e) => {
            let e = e.to_string();
            // Accepted, then dropped at once: another controller holds the 5K.
            if e.contains("10053") || e.contains("closed") {
                Err(BUSY.to_string())
            } else {
                Err(e)
            }
        }
    }
}

/// Error text when another controller (phone) holds the 5K's control session.
#[cfg(feature = "ble")]
const BUSY: &str = "busy";

/// Diagnostics (`--cycle-test N`): N rounds of open a real GAIA session → close →
/// poll every 2 s (≤ 60 s) until a session opens again, logging channel + delay.
/// Shows how/when the 5K re-arms its GAIA server after a session ends.
#[cfg(feature = "ble")]
pub fn cycle_test(rounds: u32) {
    init_com();
    let Ok(addr) = crate::ble::find_qudelix_address() else { return };
    let sink: crate::rfcomm::FrameSink = Arc::new(|_: &[u8]| {});
    // SDP only — scanning channels is what left the 5K with a ghost session.
    let try_open = || -> Option<crate::rfcomm::Session> {
        crate::rfcomm::Session::open_sdp(addr, sink.clone())
            .map_err(|e| crate::log::line(&format!("[cycle] sdp open: {e}")))
            .ok()
    };
    for round in 1..=rounds {
        let t = std::time::Instant::now();
        let mut found = None;
        while t.elapsed() < std::time::Duration::from_secs(60) {
            if let Some(x) = try_open() {
                found = Some(x);
                break;
            }
            std::thread::sleep(std::time::Duration::from_secs(2));
        }
        match found {
            Some(s) => {
                crate::log::line(&format!("[cycle] round {round}: session after {} ms", t.elapsed().as_millis()));
                std::thread::sleep(std::time::Duration::from_millis(1500));
                drop(s);
                crate::log::line(&format!("[cycle] round {round}: closed"));
            }
            None => {
                crate::log::line(&format!("[cycle] round {round}: NO session within 60 s — stopping"));
                return;
            }
        }
    }
}

/// Diagnostics (`--collision-test N`): N rounds of "two controllers at once":
/// session A open → session B connects on top (like phone + PC colliding) →
/// close both → a fresh session C must open. Fails loudly if the 5K is left
/// in the ghost state (accept-then-drop for everyone).
#[cfg(feature = "ble")]
pub fn collision_test(rounds: u32) {
    use std::time::Duration;
    init_com();
    let Ok(addr) = crate::ble::find_qudelix_address() else { return };
    let sink: crate::rfcomm::FrameSink = Arc::new(|_: &[u8]| {});
    let open = |tag: &str| match crate::rfcomm::Session::open_sdp(addr, sink.clone()) {
        Ok(s) => {
            crate::log::line(&format!("[collide] {tag}: open"));
            Some(s)
        }
        Err(e) => {
            crate::log::line(&format!("[collide] {tag}: {e}"));
            None
        }
    };
    for round in 1..=rounds {
        let a = open("A");
        std::thread::sleep(Duration::from_millis(500));
        let b = open("B (on top of A)");
        std::thread::sleep(Duration::from_millis(1500));
        crate::log::line(&format!(
            "[collide] round {round}: A alive={} B alive={}",
            a.as_ref().is_some_and(|s| s.is_alive()),
            b.as_ref().is_some_and(|s| s.is_alive())
        ));
        drop(b);
        drop(a);
        std::thread::sleep(Duration::from_millis(1500));
        // Health check: a fresh controller must get in (allow a few tries).
        let ok = (0..5).any(|_| {
            let c = open("C (health)");
            std::thread::sleep(Duration::from_millis(400));
            c.is_some()
        });
        crate::log::line(&format!("[collide] round {round}: {}", if ok { "HEALTHY" } else { "GHOST — 5K refuses everyone" }));
        if !ok {
            return;
        }
    }
}


/// Build the GAIA command frame for a UI command.
#[cfg(feature = "ble")]
fn command_frame(cmd: &Cmd) -> Vec<u8> {
    match *cmd {
        Cmd::Priority(v) => proto::rfcomm::priority(v),
        Cmd::Volume(n) => proto::rfcomm::volume(n),
        Cmd::Refresh => unreachable!("Refresh is handled by refresh_state"),
        Cmd::PcAudio(_) => unreachable!("PcAudio is handled by set_pc_audio"),
    }
}

/// The status-line label to show after a command succeeds.
#[cfg(feature = "ble")]
fn command_label(cmd: &Cmd) -> String {
    match *cmd {
        Cmd::Priority(v) => format!(
            "→ {}",
            ["USB", "Bluetooth 1", "Bluetooth 2"].get(v as usize).copied().unwrap_or("?")
        ),
        Cmd::Volume(n) => format!("{:+.0} dB", proto::n_to_db(n)),
        Cmd::Refresh => "updated".to_string(),
        Cmd::PcAudio(m) => format!("{m:?}"),
    }
}

/// One-shot CLI probe: connect, send one priority command, log the outcome, exit.
/// Runs on the calling thread (no window). Used to validate the idea end-to-end.
pub fn probe_priority(v: u8) {
    init_com();
    crate::log::line(&format!("[probe] --set-priority {v}"));
    match connect_any() {
        Some((d, label)) => {
            crate::log::line(&format!("[probe] {label}"));
            match d.set_priority(v) {
                Ok(()) => crate::log::line("[probe] set_priority -> OK"),
                Err(e) => crate::log::line(&format!("[probe] set_priority -> ERR: {e}")),
            }
            // Give the async write a moment to flush before the process exits.
            std::thread::sleep(std::time::Duration::from_millis(400));
        }
        None => crate::log::line("[probe] not connected"),
    }
}

/// One-shot CLI probe: send one absolute volume (n = dB*60, signed).
pub fn probe_volume(n: i16) {
    init_com();
    crate::log::line(&format!("[probe] --set-volume n={n}"));
    match connect_any() {
        Some((d, label)) => {
            crate::log::line(&format!("[probe] {label}"));
            match d.set_volume(proto::VOLUME_SINK_MAIN, n) {
                Ok(()) => crate::log::line("[probe] set_volume -> OK"),
                Err(e) => crate::log::line(&format!("[probe] set_volume -> ERR: {e}")),
            }
            std::thread::sleep(std::time::Duration::from_millis(400));
        }
        None => crate::log::line("[probe] not connected"),
    }
}

/// One-shot CLI probe: write arbitrary hex bytes to the BLE GAIA characteristic.
/// Lets us try the exact frame captured from the phone (GAIA v1 SOF form).
#[cfg(feature = "ble")]
pub fn probe_raw(hex: &str) {
    init_com();
    let bytes: Vec<u8> = hex
        .split(|c: char| c == ' ' || c == ',')
        .filter(|s| !s.is_empty())
        .filter_map(|s| u8::from_str_radix(s.trim_start_matches("0x"), 16).ok())
        .collect();
    crate::log::line(&format!("[probe] --raw {}", crate::log::hex(&bytes)));
    match crate::ble::Ble::connect() {
        Ok(b) => {
            match b.write_bytes(&bytes) {
                Ok(()) => crate::log::line("[probe] raw write -> OK"),
                Err(e) => crate::log::line(&format!("[probe] raw write -> ERR: {e}")),
            }
            std::thread::sleep(std::time::Duration::from_millis(400));
        }
        Err(e) => crate::log::line(&format!("[probe] BLE connect failed: {e}")),
    }
}

/// One-shot CLI probe: send hex bytes to the 5K over classic RFCOMM (the real
/// control path when connected as audio).
#[cfg(feature = "ble")]
pub fn probe_rfcomm(hex: &str) {
    init_com();
    // Multiple frames separated by ';'; bytes within a frame by space/comma.
    let frames: Vec<Vec<u8>> = hex
        .split(';')
        .map(|frame| {
            frame
                .split(|c: char| c == ' ' || c == ',')
                .filter(|s| !s.is_empty())
                .filter_map(|s| u8::from_str_radix(s.trim_start_matches("0x"), 16).ok())
                .collect::<Vec<u8>>()
        })
        .filter(|f| !f.is_empty())
        .collect();
    crate::log::line(&format!("[probe] --rfcomm: {} frame(s)", frames.len()));
    match run_session(frames) {
        Ok(()) => crate::log::line("[probe] rfcomm -> OK"),
        Err(e) => crate::log::line(&format!("[probe] rfcomm -> ERR: {e}")),
    }
}

/// CLI one-shot (`--priority`, `--volume`, `--rfcomm` without a running widget): the
/// same SDP-found, registered session the widget uses, then the frames, then release
/// (Session's Drop stops the stream and closes cleanly). Never a fixed or guessed
/// channel — see `open_session`.
#[cfg(feature = "ble")]
fn run_session(cmd_frames: Vec<Vec<u8>>) -> Result<(), String> {
    init_com();
    let sink: crate::rfcomm::FrameSink = Arc::new(|_: &[u8]| {});
    let mut last = String::from("unknown");
    // Right after another session the 5K needs a few seconds to accept a new one.
    for attempt in 0..5 {
        match open_session(&sink) {
            Ok(session) => {
                for f in &cmd_frames {
                    session.command(f).map_err(|e| e.to_string())?;
                }
                std::thread::sleep(std::time::Duration::from_millis(300)); // let the 5K act before release
                return Ok(());
            }
            Err(e) if e == BUSY => return Err(e), // another controller holds the 5K
            Err(e) => {
                crate::log::line(&format!("[session] attempt {attempt} failed: {e}"));
                last = e;
                std::thread::sleep(std::time::Duration::from_millis(1000 + attempt as u64 * 500));
            }
        }
    }
    Err(last)
}

/// CONFIRMED (user-verified): change source priority. v = 0 USB / 1 A2DP#1 / 2 A2DP#2.
/// The phone's Qudelix app must be CLOSED (single GAIA channel).
#[cfg(feature = "ble")]
pub fn set_priority_via_rfcomm(v: u8) -> Result<(), String> {
    crate::log::line(&format!("[priority] SetAudioTopSource={v}"));
    let r = run_session(vec![proto::rfcomm::priority(v)]);
    crate::log::line(&format!("[priority] -> {r:?}"));
    r
}

/// Set the main volume via the same session (SetVolume=0x0200, [sink, nHi, nLo]).
/// n is signed dB*60.
#[cfg(feature = "ble")]
pub fn set_volume_via_rfcomm(n: i16) -> Result<(), String> {
    crate::log::line(&format!("[volume] SetVolume n={n}"));
    let r = run_session(vec![proto::rfcomm::volume(n)]);
    crate::log::line(&format!("[volume] -> {r:?}"));
    r
}

/// Cross-process mutex around the connect + handshake: two processes paging the
/// 5K at the same moment is what confuses it. (The session itself isn't covered:
/// the 5K refuses a second controller cleanly once one is registered.)
#[cfg(feature = "ble")]
struct RfcommLock {
    handle: Option<windows::Win32::Foundation::HANDLE>,
    /// Release only what we own: after a timeout we go ahead without the lock.
    owned: bool,
}

#[cfg(feature = "ble")]
impl RfcommLock {
    fn acquire() -> Self {
        use windows::core::w;
        use windows::Win32::Foundation::{WAIT_ABANDONED, WAIT_OBJECT_0};
        use windows::Win32::System::Threading::{CreateMutexW, WaitForSingleObject};
        unsafe {
            match CreateMutexW(None, false, w!("Local\\QudelixRfcommLock")) {
                Ok(h) => {
                    // 20 s cap: a wedged holder must not block us forever.
                    let r = WaitForSingleObject(h, 20_000);
                    // Abandoned = the holder died; the mutex is ours now.
                    let owned = r == WAIT_OBJECT_0 || r == WAIT_ABANDONED;
                    if !owned {
                        crate::log::line("[rfcomm] connect lock busy for 20 s — going ahead without it");
                    }
                    RfcommLock { handle: Some(h), owned }
                }
                Err(_) => RfcommLock { handle: None, owned: false },
            }
        }
    }
}

#[cfg(feature = "ble")]
impl Drop for RfcommLock {
    fn drop(&mut self) {
        use windows::Win32::Foundation::CloseHandle;
        use windows::Win32::System::Threading::ReleaseMutex;
        if let Some(h) = self.handle {
            unsafe {
                if self.owned {
                    let _ = ReleaseMutex(h);
                }
                let _ = CloseHandle(h);
            }
        }
    }
}

pub(crate) fn init_com() {
    #[cfg(feature = "ble")]
    unsafe {
        use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
}

fn connect_any() -> Option<(Box<dyn Qudelix>, String)> {
    match UsbHid::connect() {
        Ok(u) => {
            let label = format!("Connected  ·  USB (PID {:04X})", u.pid());
            return Some((Box::new(u), label));
        }
        Err(e) => crate::log::line(&format!("[connect] USB failed: {e}")),
    }
    #[cfg(feature = "ble")]
    match crate::ble::Ble::connect() {
        Ok(b) => return Some((Box::new(b), "Connected  ·  BLE".to_string())),
        Err(e) => crate::log::line(&format!("[connect] BLE failed: {e}")),
    }
    None
}
