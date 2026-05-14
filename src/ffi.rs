use crate::proto::{WfbError, WfbRxConfig, WfbRxMeta, WfbTxConfig, compute_max_payload};
use crate::{WfbRx, WfbTx};

use libc::c_char;
use std::ffi::CStr;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::slice;
use std::time::Duration;

pub const WFB_RS_OK: i32 = 0;
pub const WFB_RS_ERR_NULL_PTR: i32 = 1;
pub const WFB_RS_ERR_INVALID_ARGUMENT: i32 = 2;
pub const WFB_RS_ERR_IO: i32 = 3;
pub const WFB_RS_ERR_PCAP: i32 = 4;
pub const WFB_RS_ERR_TIMEOUT: i32 = 5;
pub const WFB_RS_ERR_INTERNAL: i32 = 255;

pub const WFB_RS_ABI_VERSION: u32 = 1;

#[repr(C)]
#[allow(non_camel_case_types)]
pub struct wfb_tx_handle {
    _private: [u8; 0],
}

#[repr(C)]
#[allow(non_camel_case_types)]
pub struct wfb_rx_handle {
    _private: [u8; 0],
}

#[repr(C)]
#[allow(non_camel_case_types)]
pub struct wfb_tx_config {
    pub iface: *const c_char,
    pub channel_id: u32,
    pub frame_type: u8,
    pub mcs_index: u8,
    pub bandwidth: u8,
}

#[repr(C)]
#[allow(non_camel_case_types)]
pub struct wfb_rx_config {
    pub iface: *const c_char,
    pub channel_id: u32,
    pub ignore_self_injected: u8,
    pub ring_size: u32,
}

#[repr(C)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Default)]
pub struct wfb_rx_meta {
    pub seq: u32,
    pub flags: u8,
    pub freq: u16,
    pub mcs_index: u8,
    pub bandwidth: u8,
    pub antenna: [u8; 4],
    pub rssi: [i8; 4],
    pub noise: [i8; 4],
    pub antenna_count: u8,
    pub truncated: u8,
}

fn map_err(err: &WfbError) -> i32 {
    match err {
        WfbError::InvalidArgument(_) => WFB_RS_ERR_INVALID_ARGUMENT,
        WfbError::Io(_) => WFB_RS_ERR_IO,
        WfbError::Pcap(_) => WFB_RS_ERR_PCAP,
    }
}

fn run_ffi<F>(f: F) -> i32
where
    F: FnOnce() -> i32,
{
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(code) => code,
        Err(_) => WFB_RS_ERR_INTERNAL,
    }
}

fn c_iface(ptr: *const c_char) -> Result<String, i32> {
    if ptr.is_null() {
        return Err(WFB_RS_ERR_NULL_PTR);
    }
    let iface = unsafe { CStr::from_ptr(ptr) };
    let iface = iface.to_str().map_err(|_| WFB_RS_ERR_INVALID_ARGUMENT)?;
    if iface.is_empty() {
        return Err(WFB_RS_ERR_INVALID_ARGUMENT);
    }
    Ok(iface.to_owned())
}

fn to_c_meta(meta: WfbRxMeta) -> wfb_rx_meta {
    wfb_rx_meta {
        seq: meta.seq,
        flags: meta.flags,
        freq: meta.freq,
        mcs_index: meta.mcs_index,
        bandwidth: meta.bandwidth,
        antenna: meta.antenna,
        rssi: meta.rssi,
        noise: meta.noise,
        antenna_count: meta.antenna_count,
        truncated: u8::from(meta.truncated),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn wfb_rs_abi_version() -> u32 {
    WFB_RS_ABI_VERSION
}

#[unsafe(no_mangle)]
pub extern "C" fn wfb_rs_max_payload() -> usize {
    compute_max_payload()
}

#[unsafe(no_mangle)]
pub extern "C" fn wfb_tx_open(cfg: *const wfb_tx_config, out_handle: *mut *mut wfb_tx_handle) -> i32 {
    run_ffi(|| {
        if cfg.is_null() || out_handle.is_null() {
            return WFB_RS_ERR_NULL_PTR;
        }
        let cfg = unsafe { &*cfg };
        let iface = match c_iface(cfg.iface) {
            Ok(v) => v,
            Err(code) => return code,
        };
        let rust_cfg = WfbTxConfig {
            iface,
            channel_id: cfg.channel_id,
            frame_type: cfg.frame_type,
            mcs_index: cfg.mcs_index,
            bandwidth: cfg.bandwidth,
        };

        match WfbTx::open(&rust_cfg) {
            Ok(tx) => {
                let raw = Box::into_raw(Box::new(tx)).cast::<wfb_tx_handle>();
                unsafe {
                    *out_handle = raw;
                }
                WFB_RS_OK
            }
            Err(e) => map_err(&e),
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn wfb_tx_close(handle: *mut wfb_tx_handle) -> i32 {
    run_ffi(|| {
        if handle.is_null() {
            return WFB_RS_ERR_NULL_PTR;
        }
        unsafe {
            drop(Box::from_raw(handle.cast::<WfbTx>()));
        }
        WFB_RS_OK
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn wfb_tx_send(
    handle: *mut wfb_tx_handle,
    payload: *const u8,
    payload_len: usize,
    seq: u32,
) -> i32 {
    run_ffi(|| {
        if handle.is_null() {
            return WFB_RS_ERR_NULL_PTR;
        }
        if payload_len > 0 && payload.is_null() {
            return WFB_RS_ERR_NULL_PTR;
        }

        let tx = unsafe { &mut *handle.cast::<WfbTx>() };
        let payload = if payload_len == 0 {
            &[]
        } else {
            unsafe { slice::from_raw_parts(payload, payload_len) }
        };

        match tx.send(payload, seq) {
            Ok(_) => WFB_RS_OK,
            Err(e) => map_err(&e),
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn wfb_rx_open(cfg: *const wfb_rx_config, out_handle: *mut *mut wfb_rx_handle) -> i32 {
    run_ffi(|| {
        if cfg.is_null() || out_handle.is_null() {
            return WFB_RS_ERR_NULL_PTR;
        }
        let cfg = unsafe { &*cfg };
        let iface = match c_iface(cfg.iface) {
            Ok(v) => v,
            Err(code) => return code,
        };
        let rust_cfg = WfbRxConfig {
            iface,
            channel_id: cfg.channel_id,
            rcv_buf_size: None,
            ignore_self_injected: cfg.ignore_self_injected != 0,
            ring_size: cfg.ring_size as usize,
        };
        match WfbRx::open(&rust_cfg) {
            Ok(rx) => {
                let raw = Box::into_raw(Box::new(rx)).cast::<wfb_rx_handle>();
                unsafe {
                    *out_handle = raw;
                }
                WFB_RS_OK
            }
            Err(e) => map_err(&e),
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn wfb_rx_close(handle: *mut wfb_rx_handle) -> i32 {
    run_ffi(|| {
        if handle.is_null() {
            return WFB_RS_ERR_NULL_PTR;
        }
        unsafe {
            drop(Box::from_raw(handle.cast::<WfbRx>()));
        }
        WFB_RS_OK
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn wfb_rx_recv(
    handle: *mut wfb_rx_handle,
    out_buf: *mut u8,
    out_buf_len: usize,
    timeout_ms: u32,
    out_len: *mut usize,
    out_meta: *mut wfb_rx_meta,
) -> i32 {
    run_ffi(|| {
        if handle.is_null() || out_buf.is_null() || out_len.is_null() {
            return WFB_RS_ERR_NULL_PTR;
        }
        if out_buf_len == 0 {
            return WFB_RS_ERR_INVALID_ARGUMENT;
        }
        let rx = unsafe { &mut *handle.cast::<WfbRx>() };
        let out_buf = unsafe { slice::from_raw_parts_mut(out_buf, out_buf_len) };

        match rx.recv(out_buf, Duration::from_millis(timeout_ms as u64)) {
            Ok(Some((n, meta))) => {
                unsafe {
                    *out_len = n;
                    if !out_meta.is_null() {
                        *out_meta = to_c_meta(meta);
                    }
                }
                WFB_RS_OK
            }
            Ok(None) => {
                unsafe {
                    *out_len = 0;
                    if !out_meta.is_null() {
                        *out_meta = wfb_rx_meta::default();
                    }
                }
                WFB_RS_ERR_TIMEOUT
            }
            Err(e) => map_err(&e),
        }
    })
}
