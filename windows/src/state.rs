//! Device state read back from the 5K (what the Refresh button shows).
//!
//! Platform-independent, host-testable. Fed with complete GAIA v1 frames
//! (`FF 01 <flags> <len> <vendor:2> <id:2> <payload:len>`) as they arrive on
//! the RFCOMM link: ReqInitData replies and the 5K's unsolicited notifications.
//!
//! Three DIFFERENT things — never conflate them:
//! * `volume_n`  — main output volume (dB*60, signed)
//! * `priority`  — source-priority *preference* (SetAudioTopSource: 0 USB / 1 A2DP#1 / 2 A2DP#2)
//! * `playing`   — the source that is actually playing right now

/// Length of the GAIA v1 header before the payload: FF 01 flags len vendor(2) id(2).
const HEADER_LEN: usize = 8;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DevState {
    pub volume_n: Option<i16>,
    pub priority: Option<u8>,
    pub playing: Option<String>,
}

impl DevState {
    /// Fold one complete frame into the state. Unknown frames are ignored.
    ///
    /// Device→host layout: `FF 01 00 <len> F0 01 <id|0x8000> <status> <payload>`,
    /// payload at index 9. Decoding from the official Chrome app's parsers
    /// (`captures/state-decoding-notes.md`), checked against captured frames.
    pub fn apply_frame(&mut self, f: &[u8]) {
        if f.len() < HEADER_LEN + 1 || f[0] != 0xFF || f[4..6] != [0xF0, 0x01] {
            return;
        }
        let p = &f[HEADER_LEN + 1..];
        match u16::from_be_bytes([f[6], f[7]]) {
            // Notification: notiId, kind (bit0 status block, bit1 config block).
            0xA000 if p.len() >= 2 => {
                let mut o = 2;
                if p[1] & 0x01 != 0 {
                    match self.status_block(&p[o..]) {
                        Some(n) => o += n,
                        None => return,
                    }
                }
                if p[1] & 0x02 != 0 {
                    self.config_block(&p[o..]);
                }
            }
            // RspInitData: devId(2) result(1) fw(4), status block, config block.
            0x8101 if p.len() > 7 => {
                if let Some(n) = self.status_block(&p[7..]) {
                    self.config_block(&p[7 + n..]);
                }
            }
            0x8111 => {
                self.status_block(p);
            }
            0x8121 => self.config_block(p),
            _ => {}
        }
    }

    /// Status block: mask, then audio(0x01,4) power(0x02,8) conn(0x04,16)
    /// runtimeInfo(0x08,4) rms(0x10,16) vol(0x40,16+1). Returns bytes consumed,
    /// or None if truncated / an unknown part (0x20) makes the rest unparseable.
    fn status_block(&mut self, b: &[u8]) -> Option<usize> {
        let m = *b.first()?;
        if m & 0xA0 != 0 {
            return None;
        }
        let mut o = 1;
        for (bit, len) in [(0x01, 4), (0x02, 8), (0x04, 16), (0x08, 4), (0x10, 16), (0x40, 17)] {
            if m & bit == 0 {
                continue;
            }
            let part = b.get(o..o + len)?;
            match bit {
                0x01 => self.playing = Some(playing_from_audio(part)),
                0x40 => self.volume_n = Some(i16::from_le_bytes([part[0], part[1]])),
                _ => {}
            }
            o += len;
        }
        Some(o)
    }

    /// Config block: mask, then sys(0x01,12) vol(0x02,16), then parts we don't
    /// need (and whose sizes we don't track) — so stop after vol.
    fn config_block(&mut self, b: &[u8]) {
        let Some(&m) = b.first() else { return };
        let mut o = 1;
        if m & 0x01 != 0 {
            let Some(sys) = b.get(o..o + 12) else { return };
            // audio_in_top_priority = bits 2..4 of the 2nd sys byte (0 USB, 1 A2DP#1, 2 A2DP#2).
            let v = (sys[1] >> 2) & 0x07;
            if v <= 2 {
                self.priority = Some(v);
            }
            o += 12;
        }
        if m & 0x02 != 0 {
            if let Some(vol) = b.get(o..o + 2) {
                // Sink volume: int16 LITTLE-endian dB*60 (unlike SetVolume, which is BE).
                self.volume_n = Some(i16::from_le_bytes([vol[0], vol[1]]));
            }
        }
    }
}

/// Audio part (4 bytes): what is actually playing — NOT the priority setting.
/// running = bit 1; active_in_source = bits 26..28 (0 none, 1 USB, 2 A2DP#1,
/// 3 A2DP#2, 4 HFP#1, 5 HFP#2); codec = bits 5..8.
fn playing_from_audio(a: &[u8]) -> String {
    let running = (a[0] >> 1) & 1 == 1;
    let active = (a[3] >> 2) & 0x07;
    if !running || active == 0 {
        return "nothing".to_string();
    }
    let src = ["none", "USB", "A2DP #1", "A2DP #2", "HFP #1", "HFP #2"]
        .get(active as usize)
        .copied()
        .unwrap_or("?");
    let codec = (a[0] >> 5) | ((a[1] & 1) << 3);
    match ["", "SBC", "AAC", "aptX", "aptX HD", "aptX Adaptive", "LDAC"].get(codec as usize) {
        Some(c) if !c.is_empty() => format!("{src} ({c})"),
        _ => src.to_string(),
    }
}

/// Splits the RFCOMM byte stream into complete GAIA v1 frames.
#[derive(Default)]
pub struct FrameSplitter {
    buf: Vec<u8>,
}

impl FrameSplitter {
    /// Append received bytes; return every complete frame now available.
    pub fn push(&mut self, data: &[u8]) -> Vec<Vec<u8>> {
        self.buf.extend_from_slice(data);
        let mut out = Vec::new();
        loop {
            // Resync on the SOF byte.
            match self.buf.iter().position(|&b| b == 0xFF) {
                Some(0) => {}
                Some(i) => {
                    self.buf.drain(..i);
                }
                None => {
                    self.buf.clear();
                    break;
                }
            }
            if self.buf.len() < 4 {
                break;
            }
            // flags bit 0 = trailing checksum byte present.
            let total = HEADER_LEN + self.buf[3] as usize + (self.buf[2] & 0x01) as usize;
            if self.buf.len() < total {
                break;
            }
            out.push(self.buf.drain(..total).collect());
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(s: &str) -> Vec<u8> {
        s.split_whitespace().map(|b| u8::from_str_radix(b, 16).unwrap()).collect()
    }

    #[test]
    fn splits_concatenated_and_partial_frames() {
        let a = hex("FF 01 00 01 F0 01 02 03 02");
        let b = hex("FF 01 00 02 00 0A C0 01 00 12");
        let mut s = FrameSplitter::default();
        let mut joined = a.clone();
        joined.extend_from_slice(&b[..5]);
        assert_eq!(s.push(&joined), vec![a]);
        assert_eq!(s.push(&b[5..]), vec![b]);
    }

    #[test]
    fn priority_echo_from_captured_frames() {
        let mut st = DevState::default();
        st.apply_frame(&hex("FF 01 00 10 F0 01 A0 00 00 0F 02 01 B0 25 FF 40 0F 3C 00 00 00 00 00 00"));
        assert_eq!(st.priority, Some(1));
        st.apply_frame(&hex("FF 01 00 10 F0 01 A0 00 00 0F 02 01 B0 29 FF 40 0F 3C 00 00 00 00 00 00"));
        assert_eq!(st.priority, Some(2));
    }

    #[test]
    fn notification_02_audio_and_volume() {
        let mut st = DevState::default();
        // Official-app capture: running, AAC, active = A2DP#2; sink 06 FA LE = -1530 = -25.5 dB.
        st.apply_frame(&hex("FF 01 00 19 F0 01 A0 00 00 02 03 01 42 56 14 0C 02 06 FA 68 01 00 00 00 00 00 00 F8 F8 00 00 00 00"));
        assert_eq!(st.playing.as_deref(), Some("A2DP #2 (AAC)"));
        assert_eq!(st.volume_n, Some(-1530));
    }

    #[test]
    fn notification_06_volume() {
        let mut st = DevState::default();
        st.apply_frame(&hex("FF 01 00 15 F0 01 A0 00 00 06 01 40 C4 FF 68 01 00 00 00 00 00 00 F8 F8 00 00 C4 FF 00"));
        assert_eq!(st.volume_n, Some(-60)); // -1.0 dB
    }

    #[test]
    fn not_running_means_nothing_playing() {
        let mut st = DevState::default();
        st.apply_frame(&hex("FF 01 00 19 F0 01 A0 00 00 02 03 01 00 56 00 08 02 C4 FF 68 01 00 00 00 00 00 00 F8 F8 00 00 C4 FF"));
        assert_eq!(st.playing.as_deref(), Some("nothing"));
        assert_eq!(st.priority, None); // the playing source never touches priority
    }
}
