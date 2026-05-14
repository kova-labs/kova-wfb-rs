from __future__ import annotations

import ctypes
from dataclasses import dataclass
from typing import Optional

from ._ffi import (
    WFB_RS_ERR_TIMEOUT,
    WfbRsTimeoutError,
    WfbRxConfig,
    WfbRxHandle,
    WfbRxMeta,
    WfbTxConfig,
    WfbTxHandle,
    get_capi,
    raise_for_code,
)


@dataclass(frozen=True)
class RxMeta:
    seq: int
    flags: int
    freq: int
    mcs_index: int
    bandwidth: int
    antenna: tuple[int, int, int, int]
    rssi: tuple[int, int, int, int]
    noise: tuple[int, int, int, int]
    antenna_count: int
    truncated: bool

    @classmethod
    def from_ctype(cls, meta: WfbRxMeta) -> "RxMeta":
        return cls(
            seq=int(meta.seq),
            flags=int(meta.flags),
            freq=int(meta.freq),
            mcs_index=int(meta.mcs_index),
            bandwidth=int(meta.bandwidth),
            antenna=tuple(int(x) for x in meta.antenna),
            rssi=tuple(int(x) for x in meta.rssi),
            noise=tuple(int(x) for x in meta.noise),
            antenna_count=int(meta.antenna_count),
            truncated=bool(meta.truncated),
        )


class Tx:
    def __init__(
        self,
        iface: str,
        channel_id: int,
        *,
        frame_type: int = 0x08,
        mcs_index: int = 1,
        bandwidth: int = 20,
        lib_path: Optional[str] = None,
    ):
        if not iface:
            raise ValueError("iface must not be empty")

        self._capi = get_capi(lib_path=lib_path)
        self._handle = WfbTxHandle()
        self._closed = False

        cfg = WfbTxConfig(
            iface=iface.encode("utf-8"),
            channel_id=channel_id,
            frame_type=frame_type,
            mcs_index=mcs_index,
            bandwidth=bandwidth,
        )

        code = int(self._capi.lib.wfb_tx_open(ctypes.byref(cfg), ctypes.byref(self._handle)))
        raise_for_code(code, "wfb_tx_open")

    def send(self, payload: bytes, seq: int) -> None:
        if self._closed:
            raise RuntimeError("Tx handle is closed")
        if not isinstance(payload, (bytes, bytearray, memoryview)):
            raise TypeError("payload must be bytes-like")

        data = bytes(payload)
        max_payload = int(self._capi.lib.wfb_rs_max_payload())
        if len(data) > max_payload:
            raise ValueError(f"payload too large: {len(data)} > {max_payload}")

        if data:
            payload_buf = (ctypes.c_uint8 * len(data)).from_buffer_copy(data)
            payload_ptr = ctypes.cast(payload_buf, ctypes.POINTER(ctypes.c_uint8))
        else:
            payload_ptr = None

        code = int(
            self._capi.lib.wfb_tx_send(
                self._handle,
                payload_ptr,
                len(data),
                seq,
            )
        )
        raise_for_code(code, "wfb_tx_send")

    def close(self) -> None:
        if self._closed:
            return
        if self._handle and self._handle.value:
            code = int(self._capi.lib.wfb_tx_close(self._handle))
            self._handle = WfbTxHandle()
            self._closed = True
            raise_for_code(code, "wfb_tx_close")
        self._closed = True

    def __enter__(self) -> "Tx":
        return self

    def __exit__(self, exc_type, exc, tb) -> None:
        self.close()

    def __del__(self) -> None:
        try:
            self.close()
        except Exception:
            pass


class Rx:
    def __init__(
        self,
        iface: str,
        channel_id: int,
        *,
        ignore_self_injected: bool = True,
        ring_size: int = 16,
        lib_path: Optional[str] = None,
    ):
        if not iface:
            raise ValueError("iface must not be empty")

        self._capi = get_capi(lib_path=lib_path)
        self._handle = WfbRxHandle()
        self._closed = False

        cfg = WfbRxConfig(
            iface=iface.encode("utf-8"),
            channel_id=channel_id,
            ignore_self_injected=1 if ignore_self_injected else 0,
            ring_size=ring_size,
        )

        code = int(self._capi.lib.wfb_rx_open(ctypes.byref(cfg), ctypes.byref(self._handle)))
        raise_for_code(code, "wfb_rx_open")

    def recv(self, timeout_ms: int, *, buf_size: Optional[int] = None) -> tuple[bytes, RxMeta]:
        if self._closed:
            raise RuntimeError("Rx handle is closed")
        if timeout_ms < 0:
            raise ValueError("timeout_ms must be >= 0")

        if buf_size is None:
            buf_size = int(self._capi.lib.wfb_rs_max_payload())
        if buf_size <= 0:
            raise ValueError("buf_size must be > 0")

        out_buf = (ctypes.c_uint8 * buf_size)()
        out_len = ctypes.c_size_t(0)
        out_meta = WfbRxMeta()

        code = int(
            self._capi.lib.wfb_rx_recv(
                self._handle,
                out_buf,
                buf_size,
                timeout_ms,
                ctypes.byref(out_len),
                ctypes.byref(out_meta),
            )
        )
        if code == WFB_RS_ERR_TIMEOUT:
            raise WfbRsTimeoutError("wfb_rx_recv timed out", code)
        raise_for_code(code, "wfb_rx_recv")

        payload = bytes(out_buf[: out_len.value])
        return payload, RxMeta.from_ctype(out_meta)

    def recv_optional(
        self, timeout_ms: int, *, buf_size: Optional[int] = None
    ) -> Optional[tuple[bytes, RxMeta]]:
        try:
            return self.recv(timeout_ms=timeout_ms, buf_size=buf_size)
        except WfbRsTimeoutError:
            return None

    def close(self) -> None:
        if self._closed:
            return
        if self._handle and self._handle.value:
            code = int(self._capi.lib.wfb_rx_close(self._handle))
            self._handle = WfbRxHandle()
            self._closed = True
            raise_for_code(code, "wfb_rx_close")
        self._closed = True

    def __enter__(self) -> "Rx":
        return self

    def __exit__(self, exc_type, exc, tb) -> None:
        self.close()

    def __del__(self) -> None:
        try:
            self.close()
        except Exception:
            pass
