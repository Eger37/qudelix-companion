//! Classic Bluetooth **RFCOMM** (SPP) transport — the channel the official app
//! uses for GAIA when the 5K is connected as an audio device (BLE GATT is
//! ignored in that mode), so this is the real control path.
//!
//! Raw Winsock `AF_BTH` socket. The GAIA channel is found via SDP (Serial Port
//! UUID, record "GAIA") — it hops after every session, so never hard-code or
//! probe channel numbers (see PROTOCOL.md). Frames are GAIA v1, built by
//! [`crate::proto::rfcomm`].

use windows::Win32::Devices::Bluetooth::{BTHPROTO_RFCOMM, SOCKADDR_BTH};
use windows::Win32::Networking::WinSock::{
    closesocket, connect, recv, send, setsockopt, shutdown, socket, SD_BOTH, SD_SEND, WSACleanup, WSAGetLastError,
    WSAStartup, SEND_RECV_FLAGS, SOCKADDR, SOCKET_ERROR, SOCK_STREAM, SOL_SOCKET, SO_RCVTIMEO, WSADATA,
};

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use windows::Win32::Networking::WinSock::SOCKET;

use crate::device::QResult;
use crate::proto::rfcomm as frame;

const AF_BTH: u16 = 32;

/// Serial Port Profile UUID (0x1101) — the 5K advertises its GAIA endpoint under it.
const SPP_UUID: windows::core::GUID = windows::core::GUID::from_u128(0x00001101_0000_1000_8000_00805f9b34fb);

/// A **persistent** GAIA session held open like the official app's, with a
/// reader thread that keeps draining what the 5K sends.
///
/// The draining is the point: once registered, the 5K pushes a notification
/// every ~5 s. Unread data starves RFCOMM credit flow and the 5K drops the
/// link within seconds — then refuses reconnects for minutes (WSA 10064).
/// The official app (btsnoop in `captures/`) reads continuously and holds one
/// session for minutes without trouble.
/// Called with every complete GAIA frame the 5K sends.
pub type FrameSink = Arc<dyn Fn(&[u8]) + Send + Sync>;

pub struct Session {
    sock: SOCKET,
    alive: Arc<AtomicBool>,
    reader: Option<JoinHandle<()>>,
}

impl Session {
    /// Like the official app: let Windows look up the 5K's Serial Port service
    /// (SDP, UUID 0x1101 — the 5K's record is named "GAIA") and connect to
    /// whatever channel it currently has (seen 1, 2, 14…). Then run the official app's
    /// handshake: API version → init data → register (waits for the ACK) → stop stream.
    /// The only way in: the channel hops after every session, and guessing or scanning
    /// channels left the 5K with a ghost session (see `worker::open_session`).
    pub fn open_sdp(bt_addr: u64, on_frame: FrameSink) -> QResult<Self> {
        unsafe {
            let mut wsa = WSADATA::default();
            if WSAStartup(0x0202, &mut wsa) != 0 {
                return Err("WSAStartup failed".into());
            }
            let sock = match socket(AF_BTH as i32, SOCK_STREAM, BTHPROTO_RFCOMM as i32) {
                Ok(s) => s,
                Err(e) => {
                    WSACleanup();
                    return Err(format!("socket() failed: {e}").into());
                }
            };
            // port 0 + service UUID: Windows runs the SDP lookup and picks the channel.
            let sa = SOCKADDR_BTH {
                addressFamily: AF_BTH,
                btAddr: bt_addr,
                serviceClassId: SPP_UUID,
                port: 0,
            };
            if connect(
                sock,
                &sa as *const _ as *const SOCKADDR,
                std::mem::size_of::<SOCKADDR_BTH>() as i32,
            ) == SOCKET_ERROR
            {
                let e = WSAGetLastError();
                let _ = closesocket(sock);
                WSACleanup();
                return Err(format!("RFCOMM connect() failed: WSA {}", e.0).into());
            }
            // Short recv timeout so the reader notices shutdown promptly.
            let tmo: u32 = 1000;
            let _ = setsockopt(sock, SOL_SOCKET as i32, SO_RCVTIMEO as i32, Some(&tmo.to_ne_bytes()));

            let alive = Arc::new(AtomicBool::new(true));
            let registered = Arc::new(AtomicBool::new(false));
            let reader = {
                let (alive, registered) = (alive.clone(), registered.clone());
                std::thread::spawn(move || read_loop(sock, &alive, &registered, &*on_frame))
            };
            let s = Session { sock, alive, reader: Some(reader) };
            crate::log::line("[rfcomm] connected SDP/GAIA, handshake…");

            for f in [frame::api_version(), frame::req_init_data(), frame::register()] {
                s.write(&f)?;
                std::thread::sleep(Duration::from_millis(150));
            }
            let t = Instant::now();
            while !registered.load(Ordering::SeqCst) && t.elapsed() < Duration::from_millis(2000) {
                if !s.is_alive() {
                    return Err("link closed during handshake".into());
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            if !registered.load(Ordering::SeqCst) {
                return Err("no register ACK from 5K".into());
            }
            s.write(&frame::stop_stream())?;
            crate::log::line("[rfcomm] session open (registered)");
            Ok(s)
        }
    }

    pub fn is_alive(&self) -> bool {
        self.alive.load(Ordering::SeqCst)
    }

    fn write(&self, b: &[u8]) -> QResult {
        unsafe {
            let n = send(self.sock, b, SEND_RECV_FLAGS(0));
            if n as usize == b.len() {
                Ok(())
            } else {
                self.alive.store(false, Ordering::SeqCst);
                Err(format!("send() returned {n} (WSA {})", WSAGetLastError().0).into())
            }
        }
    }

    /// Ask the 5K to resend its full state (the same ReqInitData as the handshake).
    pub fn request_state(&self) -> QResult {
        self.write(&frame::req_init_data())
    }

    /// Send one GAIA command frame on the open session.
    pub fn command(&self, frame: &[u8]) -> QResult {
        if !self.is_alive() {
            return Err("session closed".into());
        }
        crate::log::line(&format!("[rfcomm] TX {}", crate::log::hex(frame)));
        self.write(frame)
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        unsafe {
            if self.is_alive() {
                let _ = send(self.sock, &frame::stop_stream(), SEND_RECV_FLAGS(0)); // leave device idle
            }
            // Graceful close: half-close, let the 5K finish (its recv sees EOF),
            // then release — rather than an abrupt close.
            let _ = shutdown(self.sock, SD_SEND);
            let t = Instant::now();
            while self.is_alive() && t.elapsed() < Duration::from_millis(1500) {
                std::thread::sleep(Duration::from_millis(50));
            }
            // Stop the reader before closing the handle: a closed SOCKET value can
            // be reused at once, and the reader must never recv() on someone else's.
            // shutdown wakes its recv(); the 1 s receive timeout bounds the rest.
            self.alive.store(false, Ordering::SeqCst);
            let _ = shutdown(self.sock, SD_BOTH);
            if let Some(r) = self.reader.take() {
                let _ = r.join();
            }
            let _ = closesocket(self.sock);
            WSACleanup();
        }
    }
}

/// Drain everything the 5K sends until the link closes or the session drops.
fn read_loop(sock: SOCKET, alive: &AtomicBool, registered: &AtomicBool, on_frame: &dyn Fn(&[u8])) {
    const WSAETIMEDOUT: i32 = 10060;
    let mut buf = [0u8; 1024];
    let mut splitter = crate::state::FrameSplitter::default();
    while alive.load(Ordering::SeqCst) {
        let r = unsafe { recv(sock, &mut buf, SEND_RECV_FLAGS(0)) };
        if r > 0 {
            for f in splitter.push(&buf[..r as usize]) {
                if f.get(4..8) == Some(&frame::REGISTER_ACK[..]) {
                    registered.store(true, Ordering::SeqCst);
                }
                // Warning (0xA002) code 3 = `newConnection` (official app enum): the 5K
                // thinks another controller holds its single GAIA session and drops us.
                if f.get(4..10) == Some(&[0xF0, 0x01, 0xA0, 0x02, 0x00, 0x03]) {
                    crate::log::line("[rfcomm] 5K says: control session held elsewhere (warning newConnection)");
                }
                // Skip the periodic ~5 s notification (F0 01 A0 00 00 04 …) to keep the log readable.
                if f.get(4..10) != Some(&[0xF0, 0x01, 0xA0, 0x00, 0x00, 0x04]) {
                    crate::log::line(&format!("[rfcomm] RX {}", crate::log::hex(&f)));
                }
                on_frame(&f);
            }
        } else if r == 0 {
            crate::log::line("[rfcomm] 5K closed the link");
            break;
        } else {
            let e = unsafe { WSAGetLastError().0 };
            if e == WSAETIMEDOUT {
                continue;
            }
            if alive.load(Ordering::SeqCst) {
                crate::log::line(&format!("[rfcomm] recv error WSA {e} — link lost"));
            }
            break;
        }
    }
    alive.store(false, Ordering::SeqCst);
}
