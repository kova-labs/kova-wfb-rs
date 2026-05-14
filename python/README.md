# wfb-rs-py

Standalone Python bindings for `wfb_rs` using `ctypes` over the C ABI (`libwfb_rs.so`).

## Prerequisites

1. Build the Rust shared library:

```bash
cd ../
cargo build --release
```

2. Ensure the dynamic library is discoverable:
   - Set `WFB_RS_LIB_PATH` to the full `libwfb_rs.so` path, or
   - Keep the default build output in `../target/release/libwfb_rs.so`.

## Install

From this directory (`wfb_rs/python`):

```bash
python -m venv .venv
source .venv/bin/activate
pip install -e .
```

## Usage

```python
from wfb_rs_py import Tx, Rx

with Tx(iface="wlan0", channel_id=1) as tx:
    tx.send(b"hello", seq=1)

with Rx(iface="wlan0", channel_id=1) as rx:
    result = rx.recv_optional(timeout_ms=100)
    if result is not None:
        payload, meta = result
        print(payload, meta)
```

Runnable example script:

```bash
python examples/simple_txrx.py --role tx --iface $NIC --channel-id 1
python examples/simple_txrx.py --role rx --iface $NIC --channel-id 1
```

## Tests

```bash
pip install -e .[test]
pytest -q
```

Runtime tests auto-skip when `libwfb_rs.so` is not available.
