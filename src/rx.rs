use crate::proto::{
    IEE80211_HEADER_LEN, WFB_PROTO_VERSION, WfbError, WfbFrameHeader, WfbRxConfig, WfbRxMeta,
    compute_max_payload,
};

use libc::{c_int, poll, pollfd};
use std::io;
use std::mem::zeroed;
use std::os::unix::io::AsRawFd;
use std::time::{Duration, Instant};

const MAX_PCAP_PACKET_SIZE: i32 = 65535;

// Radiotap field indices.
const RTAP_TSFT: u32 = 0;
const RTAP_FLAGS: u32 = 1;
const RTAP_RATE: u32 = 2;
const RTAP_CHANNEL: u32 = 3;
const RTAP_FHSS: u32 = 4;
const RTAP_DBM_ANTSIGNAL: u32 = 5;
const RTAP_DBM_ANTNOISE: u32 = 6;
const RTAP_LOCK_QUALITY: u32 = 7;
const RTAP_TX_ATTENUATION: u32 = 8;
const RTAP_DB_TX_ATTENUATION: u32 = 9;
const RTAP_DBM_TX_POWER: u32 = 10;
const RTAP_ANTENNA: u32 = 11;
const RTAP_DB_ANTSIGNAL: u32 = 12;
const RTAP_DB_ANTNOISE: u32 = 13;
const RTAP_RX_FLAGS: u32 = 14;
const RTAP_TX_FLAGS: u32 = 15;
const RTAP_RTS_RETRIES: u32 = 16;
const RTAP_DATA_RETRIES: u32 = 17;
const RTAP_MCS: u32 = 19;
const RTAP_AMPDU_STATUS: u32 = 20;
const RTAP_VHT: u32 = 21;
const RTAP_TIMESTAMP: u32 = 22;

// Radiotap FLAGS bits.
const RTAP_F_FCS: u8 = 0x10;
const RTAP_F_BADFCS: u8 = 0x40;

// Radiotap MCS have bits.
const RTAP_MCS_HAVE_MCS: u8 = 0x02;
const RTAP_MCS_HAVE_BW: u8 = 0x01;

fn le_u16(x: &[u8]) -> u16 {
    u16::from_le_bytes([x[0], x[1]])
}

fn le_u32(x: &[u8]) -> u32 {
    u32::from_le_bytes([x[0], x[1], x[2], x[3]])
}

fn align_up(v: usize, align: usize) -> usize {
    if align <= 1 {
        return v;
    }
    (v + (align - 1)) & !(align - 1)
}

#[derive(Debug, Clone, Copy)]
struct RadiotapMeta {
    rt_flags: u8,
    self_injected: bool,
    freq: u16,
    antenna: [u8; 4],
    rssi: [i8; 4],
    noise: [i8; 4],
    ant_count: u8,
    mcs_index: u8,
    bandwidth: u8,
    radiotap_len: usize,
}

fn radiotap_meta_from_packet(pkt: &[u8]) -> Option<RadiotapMeta> {
    // Minimal radiotap header: version(1) + pad(1) + len(2) + present(4) ...
    if pkt.len() < 8 {
        return None;
    }
    let version = pkt[0];
    if version != 0 {
        // Radiotap v0 only.
        return None;
    }
    let radiotap_len = le_u16(&pkt[2..4]) as usize;
    if radiotap_len > pkt.len() {
        return None;
    }

    // Parse present words and extended present words.
    let mut present_words: Vec<u32> = Vec::new();
    let mut p_off = 4usize;
    loop {
        if p_off + 4 > pkt.len() {
            return None;
        }
        let w = le_u32(&pkt[p_off..p_off + 4]);
        present_words.push(w & 0x7fff_ffff);
        let has_ext = (w & 0x8000_0000) != 0;
        p_off += 4;
        if !has_ext {
            break;
        }
    }

    let present_words_count = present_words.len();
    let args_offset = 8 + 4 * (present_words_count.saturating_sub(1));
    if args_offset > pkt.len() {
        return None;
    }

    // Mapping from radiotap field index -> (align,size) from vendor radiotap.c.
    // Only include fields that might appear in v0 sender/receiver paths.
    fn field_layout(field: u32) -> Option<(usize, usize)> {
        match field {
            RTAP_TSFT => Some((8, 8)),
            RTAP_FLAGS => Some((1, 1)),
            RTAP_RATE => Some((1, 1)),
            RTAP_CHANNEL => Some((2, 4)),
            RTAP_FHSS => Some((2, 2)),
            RTAP_DBM_ANTSIGNAL => Some((1, 1)),
            RTAP_DBM_ANTNOISE => Some((1, 1)),
            RTAP_LOCK_QUALITY => Some((2, 2)),
            RTAP_TX_ATTENUATION => Some((2, 2)),
            RTAP_DB_TX_ATTENUATION => Some((2, 2)),
            RTAP_DBM_TX_POWER => Some((1, 1)),
            RTAP_ANTENNA => Some((1, 1)),
            RTAP_DB_ANTSIGNAL => Some((1, 1)),
            RTAP_DB_ANTNOISE => Some((1, 1)),
            RTAP_RX_FLAGS => Some((2, 2)),
            RTAP_TX_FLAGS => Some((2, 2)),
            RTAP_RTS_RETRIES => Some((1, 1)),
            RTAP_DATA_RETRIES => Some((1, 1)),
            RTAP_MCS => Some((1, 3)),
            RTAP_AMPDU_STATUS => Some((4, 8)),
            RTAP_VHT => Some((2, 12)),
            RTAP_TIMESTAMP => Some((8, 12)),
            _ => None,
        }
    }

    let mut antenna = [0xffu8; 4];
    let mut rssi = [i8::MIN; 4];
    let mut noise = [i8::MAX; 4];
    let mut ant_idx = 0usize;

    let mut rt_flags: u8 = 0;
    let mut self_injected = false;
    let mut freq: u16 = 0;
    let mut mcs_index: u8 = 0;
    let mut bandwidth: u8 = 20;

    // Radiotap iterator logic: walk present fields in increasing field index,
    // advancing `cur_off` over aligned present args.
    let mut cur_off = args_offset;
    let max_field_bit = present_words_count * 32;
    for field_index in 0..max_field_bit {
        let word_idx = field_index / 32;
        let bit_idx = field_index % 32;
        let present = (present_words[word_idx] & (1u32 << bit_idx)) != 0;
        if !present {
            continue;
        }

        if ant_idx >= 4 {
            break;
        }

        let field = field_index as u32;
        let (align, size) = match field_layout(field) {
            Some(v) => v,
            None => break,
        };

        cur_off = align_up(cur_off, align);
        if cur_off + size > pkt.len() {
            return None;
        }

        let field_bytes = &pkt[cur_off..cur_off + size];

        match field {
            RTAP_ANTENNA => {
                if ant_idx < 4 {
                    antenna[ant_idx] = field_bytes[0];
                    ant_idx += 1;
                }
            }
            RTAP_DBM_ANTSIGNAL => {
                if ant_idx < 4 {
                    rssi[ant_idx] = field_bytes[0] as i8;
                }
            }
            RTAP_DBM_ANTNOISE => {
                if ant_idx < 4 {
                    noise[ant_idx] = field_bytes[0] as i8;
                }
            }
            RTAP_CHANNEL => {
                // Two __le16 fields: MHz + flags bitmap.
                freq = (le_u32(field_bytes) & 0xffff) as u16;
            }
            RTAP_FLAGS => {
                rt_flags = field_bytes[0];
            }
            RTAP_TX_FLAGS => {
                // v0 receiver treats TX_FLAGS presence as self-injected.
                self_injected = true;
            }
            RTAP_MCS => {
                let mcs_have = field_bytes[0];
                if (mcs_have & RTAP_MCS_HAVE_MCS) != 0 {
                    mcs_index = field_bytes[2] & 0x7f;
                }
                if (mcs_have & RTAP_MCS_HAVE_BW) != 0 && (field_bytes[1] & 0x01) != 0 {
                    bandwidth = 40;
                }
            }
            _ => {}
        }

        cur_off += size;
    }

    Some(RadiotapMeta {
        rt_flags,
        self_injected,
        freq,
        antenna,
        rssi,
        noise,
        ant_count: ant_idx as u8,
        mcs_index,
        bandwidth,
        radiotap_len,
    })
}

pub struct WfbRx {
    _iface: String,
    ignore_self_injected: bool,

    max_payload: usize,

    // Bounded ring buffer (drop-new-frames when full, FIFO pop on recv).
    ring_size: usize,
    ring_count: usize,
    ring_head: usize,
    ring_tail: usize,
    ring_payload_store: Vec<u8>,  // ring_size * max_payload
    ring_payload_len: Vec<usize>, // ring_size
    ring_meta: Vec<WfbRxMeta>,    // ring_size

    fd: c_int,
    pcap: pcap::Capture<pcap::Active>,
}

impl WfbRx {
    pub fn open(cfg: &WfbRxConfig) -> Result<Self, WfbError> {
        if cfg.iface.is_empty() {
            return Err(WfbError::InvalidArgument("iface is empty".into()));
        }
        if cfg.channel_id == 0 {
            return Err(WfbError::InvalidArgument(
                "channel_id=0 is disallowed in WFB-rs".into(),
            ));
        }

        let ring_size = if cfg.ring_size == 0 {
            16
        } else {
            cfg.ring_size
        };
        let ring_size = ring_size.clamp(2, 128);

        let max_payload = compute_max_payload();
        if max_payload < 1 {
            return Err(WfbError::InvalidArgument("max payload too small".into()));
        }

        // Open pcap capture.
        let cap = pcap::Capture::from_device(cfg.iface.as_str())
            .map_err(|e| WfbError::Pcap(e.to_string()))?
            .promisc(true)
            .immediate_mode(true)
            .snaplen(MAX_PCAP_PACKET_SIZE)
            .timeout(0)
            .open()
            .map_err(|e| WfbError::Pcap(e.to_string()))?
            .setnonblock()
            .map_err(|e| WfbError::Pcap(e.to_string()))?;

        // Install the same channel_id filter used by the C++ receiver.
        let filter_exp = format!(
            "ether[0x0a:2]==0x5742 && ether[0x0c:4] == 0x{:08x}",
            cfg.channel_id
        );

        let mut cap = cap;
        cap.filter(&filter_exp, true)
            .map_err(|e| WfbError::Pcap(e.to_string()))?;

        let fd = cap.as_raw_fd() as c_int;

        Ok(Self {
            _iface: cfg.iface.clone(),
            ignore_self_injected: cfg.ignore_self_injected,
            max_payload,
            ring_size,
            ring_count: 0,
            ring_head: 0,
            ring_tail: 0,
            ring_payload_store: vec![0u8; ring_size * max_payload],
            ring_payload_len: vec![0usize; ring_size],
            ring_meta: vec![WfbRxMeta::default(); ring_size],
            fd,
            pcap: cap,
        })
    }

    fn enqueue_decoded(&mut self, payload: &[u8], meta: &WfbRxMeta) {
        if self.ring_count >= self.ring_size {
            // Drop new frames when full (matches C++ behavior).
            return;
        }
        let idx = self.ring_tail;
        self.ring_payload_len[idx] = payload.len();
        self.ring_meta[idx] = *meta;
        let dst_off = idx * self.max_payload;
        self.ring_payload_store[dst_off..dst_off + payload.len()].copy_from_slice(payload);
        self.ring_tail = (self.ring_tail + 1) % self.ring_size;
        self.ring_count += 1;
    }

    fn pop_one(&mut self, buf: &mut [u8], out_meta: Option<&mut WfbRxMeta>) -> usize {
        let idx = self.ring_head;
        let payload_len = self.ring_payload_len[idx];
        let to_copy = payload_len.min(buf.len());
        let src_off = idx * self.max_payload;
        buf[..to_copy].copy_from_slice(&self.ring_payload_store[src_off..src_off + to_copy]);

        if let Some(m) = out_meta {
            let mut meta = self.ring_meta[idx];
            meta.truncated = to_copy != payload_len;
            *m = meta;
        }

        self.ring_head = (self.ring_head + 1) % self.ring_size;
        self.ring_count -= 1;
        to_copy
    }

    fn drain_pcap_nonblock(&mut self) -> Result<(), WfbError> {
        loop {
            let pkt = self
                .pcap
                .next_packet()
                .map_err(|e| WfbError::Pcap(e.to_string()));

            match pkt {
                Ok(packet) => {
                    let caplen = packet.header.caplen as usize;
                    let data = packet.data[..caplen].to_vec();
                    drop(packet);
                    let _ = self.parse_and_enqueue_one_packet(&data);
                }
                Err(_) => {
                    // In nonblock mode, errors when nothing is available are expected.
                    // We interpret all pcap errors as "stop draining" here.
                    break;
                }
            };
        }
        Ok(())
    }

    fn parse_and_enqueue_one_packet(&mut self, pkt: &[u8]) -> Result<(), WfbError> {
        let radiotap = match radiotap_meta_from_packet(pkt) {
            Some(v) => v,
            None => return Ok(()),
        };

        if radiotap.self_injected && self.ignore_self_injected {
            return Ok(());
        }
        let mut effective_len = pkt.len();
        if (radiotap.rt_flags & RTAP_F_FCS) != 0 {
            if effective_len >= 4 {
                effective_len -= 4;
            }
        }
        if (radiotap.rt_flags & RTAP_F_BADFCS) != 0 {
            return Ok(());
        }

        if effective_len <= radiotap.radiotap_len + IEE80211_HEADER_LEN {
            return Ok(());
        }

        let after_rt_len = effective_len - radiotap.radiotap_len;
        let after_ieee_len = after_rt_len - IEE80211_HEADER_LEN;
        if after_ieee_len < 8 {
            return Ok(());
        }
        let ieee_start = radiotap.radiotap_len;
        let sh_start = ieee_start + IEE80211_HEADER_LEN;
        let plaintext_hdr_len = 8usize;

        if sh_start + plaintext_hdr_len > effective_len {
            return Ok(());
        }

        let (sh, _) = match WfbFrameHeader::decode(&pkt[sh_start..sh_start + plaintext_hdr_len]) {
            Ok(v) => v,
            Err(_) => return Ok(()),
        };
        if sh.version != WFB_PROTO_VERSION {
            return Ok(());
        }

        let payload_len = sh.payload_len as usize;
        let payload_start = sh_start + plaintext_hdr_len;
        if payload_start > effective_len {
            return Ok(());
        }
        let payload_avail = effective_len - payload_start;
        if payload_len > payload_avail {
            return Ok(());
        }
        if payload_len > self.max_payload {
            return Ok(());
        }

        let payload = &pkt[payload_start..payload_start + payload_len];

        let mut meta = WfbRxMeta::default();
        meta.seq = sh.seq;
        meta.flags = sh.flags;
        meta.freq = radiotap.freq;
        meta.mcs_index = radiotap.mcs_index;
        meta.bandwidth = radiotap.bandwidth;
        meta.antenna = radiotap.antenna;
        meta.rssi = radiotap.rssi;
        meta.noise = radiotap.noise;
        meta.antenna_count = radiotap.ant_count;
        meta.truncated = false;

        self.enqueue_decoded(payload, &meta);
        Ok(())
    }

    pub fn recv(
        &mut self,
        buf: &mut [u8],
        timeout: Duration,
    ) -> Result<Option<(usize, WfbRxMeta)>, WfbError> {
        if buf.is_empty() {
            return Err(WfbError::InvalidArgument(
                "recv buffer must be non-empty".into(),
            ));
        }

        if self.ring_count > 0 {
            let mut meta = WfbRxMeta::default();
            let n = self.pop_one(buf, Some(&mut meta));
            return Ok(Some((n, meta)));
        }

        let start = Instant::now();
        loop {
            if self.ring_count > 0 {
                let mut meta = WfbRxMeta::default();
                let n = self.pop_one(buf, Some(&mut meta));
                return Ok(Some((n, meta)));
            }

            let elapsed = start.elapsed();
            if elapsed >= timeout {
                return Ok(None);
            }
            let remaining_ms = (timeout - elapsed).as_millis();
            let remaining_ms = remaining_ms.min(i32::MAX as u128) as i32;

            let mut pfd: pollfd = unsafe { zeroed() };
            pfd.fd = self.fd;
            pfd.events = libc::POLLIN as i16;

            let rc = unsafe { poll(&mut pfd as *mut pollfd, 1, remaining_ms) };
            if rc < 0 {
                let e = io::Error::last_os_error();
                if e.kind() == io::ErrorKind::Interrupted {
                    continue;
                }
                return Err(e.into());
            }
            if rc == 0 {
                return Ok(None);
            }

            // Drain pcap and retry.
            let _ = self.drain_pcap_nonblock();
        }
    }
}

impl Drop for WfbRx {
    fn drop(&mut self) {
        // pcap::Capture will close itself.
        unsafe {
            let _ = libc::close(self.fd);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RADIOTAP_HT_TEMPLATE: [u8; 13] = [
        0x00, 0x00, // radiotap version + pad
        0x0d, 0x00, // radiotap header length (LE) = 13
        0x00, 0x80, 0x08, 0x00, // present flags: TX_FLAGS + MCS
        0x08, 0x00, // RADIOTAP_F_TX_NOACK (u16)
        0x37, // MCS_KNOWN
        0x00, // patched: MCS_FLAGS_OFF
        0x00, // patched: MCS_IDX_OFF
    ];

    #[test]
    fn radiotap_meta_parses_mcs_and_self_injected_40() {
        let mut pkt = Vec::from(RADIOTAP_HT_TEMPLATE);
        // bandwidth = 40 => MCS_FLAGS_OFF byte LSB set
        pkt[11] = 1;
        pkt[12] = 3;

        let meta = radiotap_meta_from_packet(&pkt).unwrap();
        assert!(meta.self_injected);
        assert_eq!(meta.mcs_index, 3);
        assert_eq!(meta.bandwidth, 40);
        assert_eq!(meta.ant_count, 0);
    }

    #[test]
    fn radiotap_meta_parses_mcs_and_self_injected_20() {
        let mut pkt = Vec::from(RADIOTAP_HT_TEMPLATE);
        // bandwidth = 20 => MCS_FLAGS_OFF byte LSB clear
        pkt[11] = 0;
        pkt[12] = 7;

        let meta = radiotap_meta_from_packet(&pkt).unwrap();
        assert!(meta.self_injected);
        assert_eq!(meta.mcs_index, 7);
        assert_eq!(meta.bandwidth, 20);
        assert_eq!(meta.ant_count, 0);
    }
}
