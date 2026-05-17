# wfb_rs

Rust-only plaintext framing implementation:

- synthetic IEEE802.11 header injection (stream-id embedded in addr2/addr3)
- HT radiotap TX header template
- pcap capture + radiotap parsing (metadata best-effort)
- plaintext payload framing (`WFB_PROTO_VERSION = 0`)

## Terminology

Two different "channel" concepts, do not confuse:

- **Wifi channel** — physical RF channel (e.g. `36`, `52`). Set on the NIC out-of-band with `iw dev … set channel`. wfb_rs does not touch it.
- **`stream_id`** — a 32-bit logical demux tag baked into the synthetic 802.11 addr2/addr3 MACs as `57:42:<stream_id big-endian>`. Lets multiple independent wfb streams share the same RF channel; RX drops frames whose embedded `stream_id` does not match.

Two peers must agree on **both** the wifi channel and the `stream_id` to exchange traffic.

## Runtime prerequisites

1. A `wlan*` interface capable of monitor mode (required for both TX injection and RX).
2. Privileges for raw sockets / packet capture: `CAP_NET_RAW` and `CAP_NET_ADMIN`. Either run with `sudo`, or grant caps to the built binary:

   ```bash
   sudo setcap cap_net_raw,cap_net_admin=eip ./target/debug/examples/simple_txrx
   ```

   (Caps are stripped on every rebuild — re-apply after `cargo build`.)
3. The capture interface must produce `DLT_IEEE802_11_RADIO` packets.
4. Plaintext framing only — FEC/encryption are intentionally deferred.

## System dependencies

```bash
sudo apt install libpcap-dev
```

## Putting an interface into monitor mode

```bash
iw dev              # list network interfaces
ethtool -i wlan1    # print attached driver, kernel version (should be rtl88xxau_wfb)

NIC=wlan1           # adjust

sudo nmcli dev set "$NIC" managed no
sudo ip link set "$NIC" down
sudo iw dev "$NIC" set type monitor
sudo ip link set "$NIC" up
sudo iw dev "$NIC" set channel 36 HT20 # = WiFi ch 36, High Throughput, 20MHz wide
sudo iw dev "$NIC" set power_save off

iw dev "$NIC" info        # verify type=monitor, channel set
```

Both peers must be on the same wifi channel.

## Build

```bash
cargo build                 # library only (rlib + cdylib + staticlib)
cargo build --examples      # also builds simple_txrx and bandwidth binaries
cargo build --release       # release artifacts
```

Example binaries land in `target/debug/examples/` (or `target/release/examples/`). Library artifacts land in `target/{debug,release}/`:

- `libwfb_rs.rlib`
- `libwfb_rs.a`
- `libwfb_rs.so`

If `CARGO_TARGET_DIR` is set, outputs go under `$CARGO_TARGET_DIR/...` instead.

## Examples

The two example invocation styles are equivalent — pick one:

- `cargo run --example <name> -- <args>` — builds and runs in one step. Awkward with `sudo` as it'll pollute `target/` ownership.
- `sudo ./target/debug/examples/<name> <args>` — run the prebuilt binary (after `cargo build --examples`).

### `simple_txrx` — interactive stdin chat

Both TX and RX on a single interface. Frames it injects itself are filtered out of its RX (`ignore_self_injected: true`), so to see anything received you need a **second peer** running the same example on the same wifi channel and `stream_id`.

On each peer:
```bash
sudo ./target/debug/examples/simple_txrx --iface "$NIC" --stream-id 1
```

Then type a line on peer A's stdin — it appears as `RX seq=… payload="…"` on peer B.

Flags:

- `-m data` (default) | `-m rts` — synthetic frame subtype
- `--print-rssi` — include radiotap RSSI in RX output
- `-c` accepts hex (`0x1234`) or decimal

### `bandwidth` — self-generating throughput test

Separate `--role tx` and `--role rx` processes on two peers. The TX side generates payload on its own (no stdin).

Receiver:

```bash
sudo ./target/debug/examples/bandwidth --role rx --iface "$NIC" --stream-id 1
```

Sender:

```bash
sudo ./target/debug/examples/bandwidth --role tx --iface "$NIC" --stream-id 1
```

Useful knobs:

- `--payload-size` (default `1200`)
- `--tx-interval-us` (default `0`, as fast as possible)
- `--report-ms` (default `1000`)

### `image_ex`, raw binary file transfer 

Sends and receives files or images directly over raw 802.11 frames by chunking payloads, acting as a rudimentary data link

Receiver:

```bash
sudo ./target/debug/examples/image_txrx --iface "$NIC" --stream-id 1
```

Sender:

```bash
sudo ./target/debug/examples/image_txrx --iface "$NIC" --stream-id 1 --file test_image.jpg
```

The receiver reconstructs the chunks and saves the completed file to disk upon an RX timeout. 

### `mesh_node_img`, tactical layer 2 routing

A custom flood-routing mesh implementation that bridges RF dead zones. Features anti-storm memory filters and hop-count tracking. It encapsulates payloads in a custom 5-byte routing header ([Target] [Source] [Pkt High] [Pkt Low] [Hops])

Demonstrates A -> B -> C relay setup where node B automatically bridges a physical gap between node A and node C.

Node C (Target Receiver):

```bash
sudo ./target/debug/examples/mesh_node --iface "$NIC" --node-id 3
```

Node B (The relay):

```bash
sudo ./target/debug/examples/mesh_node --iface "$NIC" --node-id 2
```

Node A (The Sender):
```bash
sudo ./target/debug/examples/mesh_node --iface "$NIC" --node-id 1 --dest-id 3 --file test_image.jpg
```

Node B will print "[*] RELAYING..." as it rebroadcasts the traffic. Node C will receive the chunks and tag the link status as [Relayed via 1 HOPS]

## Sniffing wfb_rs traffic with tcpdump

wfb_rs-injected frames carry a fixed addr2/addr3 of `57:42:<stream_id big-endian>`. For `--stream-id 1` that's `57:42:00:00:00:01`:

```bash
sudo tcpdump -i "$NIC" -y IEEE802_11_RADIO -nn -e -vvv 'wlan addr2 57:42:00:00:00:01'
```

To see all wfb_rs traffic regardless of `stream_id`, drop the filter and grep visually for addr2 starting `57:42:` — tcpdump's BPF for `wlan[N:M]` indexing on radiotap captures is not reliable across versions.

```bash
sudo tcpdump -y IEEE802_11_RADIO -nn -e -vvv 'wlan[10:2] = 0x5742'
```

## C ABI

Public C ABI declarations live in `include/wfb_rs.h`. Regenerate with `cbindgen`:

```bash
make gen-header
# install cbindgen if missing:
cargo install cbindgen
```

Build and link the C smoke binaries:

```bash
make c-smoke
```

This produces:

- `examples_c/smoke_shared` (links `libwfb_rs.so`)
- `examples_c/smoke_static` (links `libwfb_rs.a`)

Minimal manual link:

```bash
# shared
cc -O2 -Wall -Wextra -Iinclude -o smoke_shared examples_c/smoke.c \
   -L"${CARGO_TARGET_DIR:-target}/release" -lwfb_rs

# static
cc -O2 -Wall -Wextra -Iinclude -o smoke_static examples_c/smoke.c \
   "${CARGO_TARGET_DIR:-target}/release/libwfb_rs.a" -lpcap -ldl -lpthread -lm
```

### ABI stability

Minimal v1 ABI: opaque TX/RX handles plus lifecycle/send/recv calls. Intentionally narrow; future extensions should be additive.

## Python bindings (ctypes)

Standalone bindings under `python/`, loaded over `libwfb_rs.so` via `ctypes`.

```bash
cargo build --release
python -m venv .venv
source .venv/bin/activate
pip install -e python
```

If the loader cannot find the shared library, point it explicitly:

```bash
export WFB_RS_LIB_PATH=/absolute/path/to/libwfb_rs.so
```

Usage:

```python
from wfb_rs_py import Tx, Rx

with Tx(iface="wlan0", stream_id=1) as tx:
    tx.send(b"hello", seq=1)

with Rx(iface="wlan0", stream_id=1) as rx:
    maybe_frame = rx.recv_optional(timeout_ms=100)
    if maybe_frame is not None:
        payload, meta = maybe_frame
        print(payload, meta.seq)
```
