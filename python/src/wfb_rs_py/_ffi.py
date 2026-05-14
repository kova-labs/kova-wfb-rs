from __future__ import annotations

import ctypes
import ctypes.util
import os
from dataclasses import dataclass
from pathlib import Path
from typing import Optional

WFB_RS_OK = 0
WFB_RS_ERR_NULL_PTR = 1
WFB_RS_ERR_INVALID_ARGUMENT = 2
WFB_RS_ERR_IO = 3
WFB_RS_ERR_PCAP = 4
WFB_RS_ERR_TIMEOUT = 5
WFB_RS_ERR_INTERNAL = 255
WFB_RS_ABI_VERSION = 1


class WfbRsError(RuntimeError):
    def __init__(self, message: str, code: int):
        super().__init__(message)
        self.code = code


class WfbRsNullPointerError(WfbRsError):
    pass


class WfbRsInvalidArgumentError(WfbRsError):
    pass


class WfbRsIoError(WfbRsError):
    pass


class WfbRsPcapError(WfbRsError):
    pass


class WfbRsTimeoutError(WfbRsError, TimeoutError):
    pass


class WfbRsInternalError(WfbRsError):
    pass


def _exc_from_code(code: int) -> type[WfbRsError]:
    return {
        WFB_RS_ERR_NULL_PTR: WfbRsNullPointerError,
        WFB_RS_ERR_INVALID_ARGUMENT: WfbRsInvalidArgumentError,
        WFB_RS_ERR_IO: WfbRsIoError,
        WFB_RS_ERR_PCAP: WfbRsPcapError,
        WFB_RS_ERR_TIMEOUT: WfbRsTimeoutError,
        WFB_RS_ERR_INTERNAL: WfbRsInternalError,
    }.get(code, WfbRsError)


def raise_for_code(code: int, fn_name: str) -> None:
    if code == WFB_RS_OK:
        return
    exc = _exc_from_code(code)
    raise exc(f"{fn_name} failed with error code {code}", code)


class WfbTxConfig(ctypes.Structure):
    _fields_ = [
        ("iface", ctypes.c_char_p),
        ("channel_id", ctypes.c_uint32),
        ("frame_type", ctypes.c_uint8),
        ("mcs_index", ctypes.c_uint8),
        ("bandwidth", ctypes.c_uint8),
    ]


class WfbRxConfig(ctypes.Structure):
    _fields_ = [
        ("iface", ctypes.c_char_p),
        ("channel_id", ctypes.c_uint32),
        ("ignore_self_injected", ctypes.c_uint8),
        ("ring_size", ctypes.c_uint32),
    ]


class WfbRxMeta(ctypes.Structure):
    _fields_ = [
        ("seq", ctypes.c_uint32),
        ("flags", ctypes.c_uint8),
        ("freq", ctypes.c_uint16),
        ("mcs_index", ctypes.c_uint8),
        ("bandwidth", ctypes.c_uint8),
        ("antenna", ctypes.c_uint8 * 4),
        ("rssi", ctypes.c_int8 * 4),
        ("noise", ctypes.c_int8 * 4),
        ("antenna_count", ctypes.c_uint8),
        ("truncated", ctypes.c_uint8),
    ]


WfbTxHandle = ctypes.c_void_p
WfbRxHandle = ctypes.c_void_p


@dataclass
class WfbRsCapi:
    lib: ctypes.CDLL
    source: str
    abi_version: int


def _crate_root() -> Path:
    return Path(__file__).resolve().parents[3]


def _library_name() -> str:
    return "libwfb_rs.so"


def _candidate_paths() -> list[Path]:
    out: list[Path] = []
    target_env = os.getenv("CARGO_TARGET_DIR")
    if target_env:
        out.append(Path(target_env) / "release" / _library_name())

    root = _crate_root()
    out.append(root / "target" / "release" / _library_name())
    return out


def _load_from_cdll_path(path: str) -> ctypes.CDLL:
    try:
        return ctypes.CDLL(path)
    except OSError as exc:
        raise RuntimeError(f"failed to load shared library from '{path}': {exc}") from exc


def _load_library(lib_path: Optional[str] = None) -> tuple[ctypes.CDLL, str]:
    if lib_path:
        return _load_from_cdll_path(lib_path), lib_path

    env_path = os.getenv("WFB_RS_LIB_PATH")
    if env_path:
        return _load_from_cdll_path(env_path), env_path

    for candidate in _candidate_paths():
        if candidate.exists():
            return _load_from_cdll_path(str(candidate)), str(candidate)

    find_name = ctypes.util.find_library("wfb_rs")
    if find_name:
        return _load_from_cdll_path(find_name), find_name

    searched = [str(p) for p in _candidate_paths()]
    raise RuntimeError(
        "unable to locate libwfb_rs.so. Set WFB_RS_LIB_PATH or build with `cargo build --release`. "
        f"Searched: {searched}"
    )


def _wire_signatures(lib: ctypes.CDLL) -> None:
    lib.wfb_rs_abi_version.argtypes = []
    lib.wfb_rs_abi_version.restype = ctypes.c_uint32

    lib.wfb_rs_max_payload.argtypes = []
    lib.wfb_rs_max_payload.restype = ctypes.c_size_t

    lib.wfb_tx_open.argtypes = [ctypes.POINTER(WfbTxConfig), ctypes.POINTER(WfbTxHandle)]
    lib.wfb_tx_open.restype = ctypes.c_int32

    lib.wfb_tx_close.argtypes = [WfbTxHandle]
    lib.wfb_tx_close.restype = ctypes.c_int32

    lib.wfb_tx_send.argtypes = [
        WfbTxHandle,
        ctypes.POINTER(ctypes.c_uint8),
        ctypes.c_size_t,
        ctypes.c_uint32,
    ]
    lib.wfb_tx_send.restype = ctypes.c_int32

    lib.wfb_rx_open.argtypes = [ctypes.POINTER(WfbRxConfig), ctypes.POINTER(WfbRxHandle)]
    lib.wfb_rx_open.restype = ctypes.c_int32

    lib.wfb_rx_close.argtypes = [WfbRxHandle]
    lib.wfb_rx_close.restype = ctypes.c_int32

    lib.wfb_rx_recv.argtypes = [
        WfbRxHandle,
        ctypes.POINTER(ctypes.c_uint8),
        ctypes.c_size_t,
        ctypes.c_uint32,
        ctypes.POINTER(ctypes.c_size_t),
        ctypes.POINTER(WfbRxMeta),
    ]
    lib.wfb_rx_recv.restype = ctypes.c_int32


_GLOBAL_CAPI: Optional[WfbRsCapi] = None


def get_capi(lib_path: Optional[str] = None, *, force_reload: bool = False) -> WfbRsCapi:
    global _GLOBAL_CAPI

    if lib_path is None and _GLOBAL_CAPI is not None and not force_reload:
        return _GLOBAL_CAPI

    lib, source = _load_library(lib_path=lib_path)
    _wire_signatures(lib)

    abi = int(lib.wfb_rs_abi_version())
    if abi != WFB_RS_ABI_VERSION:
        raise RuntimeError(
            f"ABI mismatch: expected {WFB_RS_ABI_VERSION}, got {abi} from '{source}'"
        )

    capi = WfbRsCapi(lib=lib, source=source, abi_version=abi)
    if lib_path is None:
        _GLOBAL_CAPI = capi
    return capi
