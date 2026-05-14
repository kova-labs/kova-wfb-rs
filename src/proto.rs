//! Plaintext WFB-rs framing codec + shared configuration/meta types.

use thiserror::Error;

pub const WFB_PROTO_VERSION: u8 = 0;

pub const WFB_FRAME_TYPE_DATA: u8 = 0x08;
pub const WFB_FRAME_TYPE_RTS: u8 = 0xb4;

const WFB_PLAINTEXT_HDR_LEN: usize = 1 + 4 + 2 + 1;

pub const IEE80211_HEADER_LEN: usize = 24;

const WIFI_MTU: usize = 4045;

pub fn compute_max_payload() -> usize {
    // WIFI_MTU intentionally excludes radiotap bytes from its MTU reasoning.
    WIFI_MTU - IEE80211_HEADER_LEN - WFB_PLAINTEXT_HDR_LEN
}

#[derive(Debug, Error)]
pub enum WfbError {
    #[error("invalid argument: {0}")]
    InvalidArgument(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("pcap error: {0}")]
    Pcap(String),
}

pub type Result<T> = std::result::Result<T, WfbError>;

#[derive(Debug, Clone)]
pub struct WfbTxConfig {
    pub iface: String,
    pub channel_id: u32,
    pub frame_type: u8, // WFB_FRAME_TYPE_DATA or _RTS
    pub mcs_index: u8,
    pub bandwidth: u8, // 20 or 40
}

#[derive(Debug, Clone)]
pub struct WfbRxConfig {
    pub iface: String,
    pub channel_id: u32,
    pub rcv_buf_size: Option<i32>,
    pub ignore_self_injected: bool,
    pub ring_size: usize,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct WfbRxMeta {
    pub seq: u32,
    pub flags: u8,
    pub freq: u16,
    pub mcs_index: u8,
    pub bandwidth: u8,

    pub antenna: [u8; 4], // 0xff for unused
    pub rssi: [i8; 4],    // IEEE80211_RADIOTAP_DBM_ANTSIGNAL
    pub noise: [i8; 4],   // IEEE80211_RADIOTAP_DBM_ANTNOISE

    pub antenna_count: u8,
    pub truncated: bool,
}

/// All multi-byte integers are in network byte order on the wire.
#[derive(Debug, Clone, Copy)]
pub struct WfbFrameHeader {
    pub version: u8,
    pub seq: u32,
    pub payload_len: u16,
    pub flags: u8,
}

impl WfbFrameHeader {
    pub fn encode(&self) -> [u8; WFB_PLAINTEXT_HDR_LEN] {
        let mut out = [0u8; WFB_PLAINTEXT_HDR_LEN];
        out[0] = self.version;
        out[1..5].copy_from_slice(&self.seq.to_be_bytes());
        out[5..7].copy_from_slice(&self.payload_len.to_be_bytes());
        out[7] = self.flags;
        out
    }

    pub fn decode(buf: &[u8]) -> Result<(Self, usize)> {
        if buf.len() < WFB_PLAINTEXT_HDR_LEN {
            return Err(WfbError::InvalidArgument(format!(
                "buffer too small for hdr: {}",
                buf.len()
            )));
        }

        let version = buf[0];
        let seq = u32::from_be_bytes([buf[1], buf[2], buf[3], buf[4]]);
        let payload_len = u16::from_be_bytes([buf[5], buf[6]]);
        let flags = buf[7];

        Ok((
            Self {
                version,
                seq,
                payload_len,
                flags,
            },
            WFB_PLAINTEXT_HDR_LEN,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_hdr_roundtrip() {
        let hdr = WfbFrameHeader {
            version: WFB_PROTO_VERSION,
            seq: 0x1234_5678,
            payload_len: 42,
            flags: 7,
        };

        let bytes = hdr.encode();
        let (decoded, consumed) = WfbFrameHeader::decode(&bytes).unwrap();

        assert_eq!(consumed, WFB_PLAINTEXT_HDR_LEN);
        assert_eq!(decoded.version, hdr.version);
        assert_eq!(decoded.seq, hdr.seq);
        assert_eq!(decoded.payload_len, hdr.payload_len);
        assert_eq!(decoded.flags, hdr.flags);
    }

    #[test]
    fn frame_hdr_decode_too_small() {
        assert!(WfbFrameHeader::decode(&[0u8; 3]).is_err());
    }
}
