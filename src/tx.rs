use crate::proto::{
    WFB_FRAME_TYPE_DATA, WFB_FRAME_TYPE_RTS, WFB_PROTO_VERSION, WfbError, WfbFrameHeader,
    WfbTxConfig, compute_max_payload,
};

use libc::{c_char, c_int};

use std::io;
use std::mem::{size_of, zeroed};

/// HT-only radiotap header template (13 bytes), patched per configuration.
///
/// This matches `wfb_simple/vendor/wifibroadcast.hpp` `radiotap_header_ht`.
const RADIOTAP_HEADER_HT_LEN: usize = 13;
const RADIOTAP_HT_TEMPLATE: [u8; RADIOTAP_HEADER_HT_LEN] = [
    0x00, 0x00, // radiotap version + pad
    0x0d, 0x00, // radiotap header length (LE)
    0x00, 0x80, 0x08, 0x00, // present flags: RADIOTAP_TX_FLAGS + RADIOTAP_MCS
    0x08, 0x00, // RADIOTAP_F_TX_NOACK
    0x37, // MCS_KNOWN (see wifibroadcast.hpp) 0b00110111
    0x00, // patched: MCS_FLAGS_OFF
    0x00, // patched: MCS_IDX_OFF
];

// wifibroadcast.hpp
const MCS_FLAGS_OFF: usize = 11;
const MCS_IDX_OFF: usize = 12;

// Bandwidth -> radiotap MCS_BW value.
const MCS_BW_20: u8 = 0;
const MCS_BW_40: u8 = 1;

/// Synthetic IEEE802.11 header template (24 bytes), patched for channel_id + seq.
///
/// This matches `wfb_simple/vendor/wifibroadcast.hpp` `ieee80211_header`.
const IEEE80211_HEADER_LEN: usize = 24;
const IEEE80211_HEADER_TEMPLATE: [u8; IEEE80211_HEADER_LEN] = [
    0x08, 0x01, 0x00, 0x00, // frame control + duration
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, // receiver is broadcast
    0x57, 0x42, 0xaa, 0xbb, 0xcc, 0xdd, // dst MAC (channel_id replaces last 4 bytes)
    0x57, 0x42, 0xaa, 0xbb, 0xcc, 0xdd, // src MAC (channel_id replaces last 4 bytes)
    0x00, 0x00, // (seq_num << 4) + fragment_num (patched per send)
];

// wifibroadcast.hpp last-four-byte offsets.
const SRC_MAC_THIRD_BYTE: usize = 12;
const DST_MAC_THIRD_BYTE: usize = 18;
const FRAME_SEQ_LB: usize = 22;
const FRAME_SEQ_HB: usize = 23;

pub struct WfbTx {
    fd: c_int,

    channel_id: u32,
    frame_type: u8,

    ieee80211_seq: u16,
    ieee_hdr: [u8; IEEE80211_HEADER_LEN],

    radiotap_ht: [u8; RADIOTAP_HEADER_HT_LEN],
}

impl WfbTx {
    fn open_pfpacket_tx_socket(iface: &str) -> Result<c_int, WfbError> {
        let fd = unsafe { libc::socket(libc::PF_PACKET, libc::SOCK_RAW, 0) };
        if fd < 0 {
            return Err(io::Error::last_os_error().into());
        }

        // Best-effort bypass qdisc like the C++ implementation (reduces latency if supported).
        let optval: c_int = 1;
        unsafe {
            let _ = libc::setsockopt(
                fd,
                libc::SOL_PACKET,
                libc::PACKET_QDISC_BYPASS,
                &optval as *const _ as *const libc::c_void,
                size_of::<c_int>() as _,
            );
        }

        // Lookup ifindex via SIOCGIFINDEX.
        let mut ifr: libc::ifreq = unsafe { zeroed() };
        let mut name = [0u8; libc::IFNAMSIZ as usize];
        let bytes = iface.as_bytes();
        if bytes.is_empty() || bytes.len() >= name.len() {
            unsafe { libc::close(fd) };
            return Err(WfbError::InvalidArgument(format!(
                "iface must fit in IFNAMSIZ ({}): {:?}",
                libc::IFNAMSIZ,
                iface
            )));
        }
        name[..bytes.len()].copy_from_slice(bytes);
        for (i, b) in name.iter().enumerate() {
            ifr.ifr_name[i] = *b as c_char;
        }

        if unsafe { libc::ioctl(fd, libc::SIOCGIFINDEX, &mut ifr) } < 0 {
            let e = io::Error::last_os_error();
            unsafe { libc::close(fd) };
            return Err(e.into());
        }
        let ifindex = unsafe { ifr.ifr_ifru.ifru_ifindex };

        let mut sll: libc::sockaddr_ll = unsafe { zeroed() };
        sll.sll_family = libc::AF_PACKET as u16;
        sll.sll_ifindex = ifindex;
        sll.sll_protocol = 0;

        let rc = unsafe {
            libc::bind(
                fd,
                &sll as *const _ as *const libc::sockaddr,
                size_of::<libc::sockaddr_ll>() as _,
            )
        };
        if rc < 0 {
            let e = io::Error::last_os_error();
            unsafe { libc::close(fd) };
            return Err(e.into());
        }

        Ok(fd)
    }

    fn build_radiotap_ht(&mut self, mcs_index: u8, bandwidth: u8) -> Result<(), WfbError> {
        if mcs_index > 0x0f {
            return Err(WfbError::InvalidArgument(format!(
                "mcs_index must be <= 0x0f, got {}",
                mcs_index
            )));
        }
        let bw = match bandwidth {
            20 => MCS_BW_20,
            40 => MCS_BW_40,
            _ => {
                return Err(WfbError::InvalidArgument(format!(
                    "bandwidth must be 20 or 40, got {}",
                    bandwidth
                )));
            }
        };

        self.radiotap_ht.copy_from_slice(&RADIOTAP_HT_TEMPLATE);

        // Patch MCS flags + MCS index.
        self.radiotap_ht[MCS_FLAGS_OFF] = bw;
        self.radiotap_ht[MCS_IDX_OFF] = mcs_index;
        Ok(())
    }

    pub fn open(cfg: &WfbTxConfig) -> Result<Self, WfbError> {
        if cfg.iface.is_empty() {
            return Err(WfbError::InvalidArgument("iface is empty".into()));
        }
        if cfg.channel_id == 0 {
            return Err(WfbError::InvalidArgument(
                "channel_id=0 is disallowed in v0".into(),
            ));
        }
        if cfg.frame_type != WFB_FRAME_TYPE_DATA && cfg.frame_type != WFB_FRAME_TYPE_RTS {
            return Err(WfbError::InvalidArgument(
                "frame_type must be DATA or RTS".into(),
            ));
        }
        let mut mcs_index = cfg.mcs_index;
        if mcs_index == 0 {
            // Keep default-ish behavior from the C++ code.
            mcs_index = 1;
        }
        let bandwidth = if cfg.bandwidth == 0 {
            20
        } else {
            cfg.bandwidth
        };

        let fd = Self::open_pfpacket_tx_socket(&cfg.iface)?;

        let mut tx = Self {
            fd,
            channel_id: cfg.channel_id,
            frame_type: cfg.frame_type,
            ieee80211_seq: 0,
            ieee_hdr: IEEE80211_HEADER_TEMPLATE,
            radiotap_ht: RADIOTAP_HT_TEMPLATE,
        };

        // Patch frame control (matches intended behavior from wifibroadcast.hpp constants).
        // wfb_simple.cpp currently validates cfg->frame_type but does not patch it; we apply it here
        // so the Rust API is usable for RTS as well.
        tx.ieee_hdr[0] = tx.frame_type;

        // Patch channel_id into synthetic MAC addresses.
        let channel_id_be = tx.channel_id.to_be_bytes();
        tx.ieee_hdr[SRC_MAC_THIRD_BYTE..SRC_MAC_THIRD_BYTE + 4].copy_from_slice(&channel_id_be);
        tx.ieee_hdr[DST_MAC_THIRD_BYTE..DST_MAC_THIRD_BYTE + 4].copy_from_slice(&channel_id_be);

        tx.build_radiotap_ht(mcs_index, bandwidth)?;

        Ok(tx)
    }

    pub fn close(self) {
        unsafe {
            libc::close(self.fd);
        }
    }

    pub fn send(&mut self, payload: &[u8], seq: u32) -> Result<(), WfbError> {
        if payload.is_empty() {
            // WFB-rs still allows len=0, so keep it as-is.
        }
        let max_payload = compute_max_payload();
        if payload.len() > max_payload {
            return Err(WfbError::InvalidArgument(format!(
                "payload too large: {} > {}",
                payload.len(),
                max_payload
            )));
        }

        let hdr = WfbFrameHeader {
            version: WFB_PROTO_VERSION,
            seq,
            payload_len: payload.len() as u16,
            flags: 0,
        };
        let hdr_bytes = hdr.encode();

        // Update 802.11 sequence number in the synthetic header.
        let seq16 = self.ieee80211_seq;
        self.ieee_hdr[FRAME_SEQ_LB] = (seq16 & 0xff) as u8;
        self.ieee_hdr[FRAME_SEQ_HB] = ((seq16 >> 8) & 0xff) as u8;
        self.ieee80211_seq = self.ieee80211_seq.wrapping_add(16);

        let mut iovecs: [libc::iovec; 4] = unsafe { zeroed() };
        iovecs[0] = libc::iovec {
            iov_base: self.radiotap_ht.as_ptr() as *mut _,
            iov_len: RADIOTAP_HEADER_HT_LEN,
        };
        iovecs[1] = libc::iovec {
            iov_base: self.ieee_hdr.as_ptr() as *mut _,
            iov_len: IEEE80211_HEADER_LEN,
        };
        iovecs[2] = libc::iovec {
            iov_base: hdr_bytes.as_ptr() as *mut _,
            iov_len: hdr_bytes.len(),
        };
        iovecs[3] = libc::iovec {
            iov_base: payload.as_ptr() as *mut _,
            iov_len: payload.len(),
        };

        let mut msghdr: libc::msghdr = unsafe { zeroed() };
        msghdr.msg_iov = iovecs.as_mut_ptr();
        msghdr.msg_iovlen = iovecs.len();

        let rc = unsafe { libc::sendmsg(self.fd, &msghdr, 0) };
        if rc < 0 {
            return Err(io::Error::last_os_error().into());
        }
        Ok(())
    }
}

impl Drop for WfbTx {
    fn drop(&mut self) {
        unsafe {
            if self.fd >= 0 {
                libc::close(self.fd);
            }
        }
    }
}
