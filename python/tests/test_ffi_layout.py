import ctypes

import pytest

from wfb_rs_py import WFB_RS_ABI_VERSION, get_capi
from wfb_rs_py._ffi import WfbRxConfig, WfbRxMeta, WfbTxConfig


def test_tx_config_layout():
    assert WfbTxConfig.iface.offset == 0
    assert WfbTxConfig.channel_id.offset >= ctypes.sizeof(ctypes.c_void_p)
    assert WfbTxConfig.frame_type.offset > WfbTxConfig.channel_id.offset


def test_rx_config_layout():
    assert WfbRxConfig.iface.offset == 0
    assert WfbRxConfig.channel_id.offset >= ctypes.sizeof(ctypes.c_void_p)
    assert WfbRxConfig.ring_size.offset > WfbRxConfig.ignore_self_injected.offset


def test_rx_meta_layout():
    assert WfbRxMeta.seq.offset == 0
    assert WfbRxMeta.flags.offset == 4
    assert WfbRxMeta.freq.offset == 6
    assert ctypes.sizeof(WfbRxMeta) == 24


def test_optional_runtime_abi_smoke():
    try:
        capi = get_capi(force_reload=True)
    except RuntimeError as exc:
        pytest.skip(f"shared library not available for runtime smoke: {exc}")

    assert capi.abi_version == WFB_RS_ABI_VERSION
    assert int(capi.lib.wfb_rs_max_payload()) > 0
