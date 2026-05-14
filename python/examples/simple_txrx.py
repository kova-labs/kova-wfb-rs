#!/usr/bin/env python3
from __future__ import annotations

import argparse
import sys
from typing import Optional

from wfb_rs_py import Rx, Tx


def run_tx(iface: str, channel_id: int, interval_ms: int) -> int:
    seq = 1
    with Tx(iface=iface, channel_id=channel_id) as tx:
        print("TX mode: enter lines to send, Ctrl-D to exit")
        for line in sys.stdin:
            payload = line.rstrip("\n").encode("utf-8")
            tx.send(payload, seq=seq)
            print(f"sent seq={seq} bytes={len(payload)}")
            seq += 1
            if interval_ms > 0:
                import time

                time.sleep(interval_ms / 1000.0)
    return 0


def run_rx(iface: str, channel_id: int, timeout_ms: int) -> int:
    with Rx(iface=iface, channel_id=channel_id) as rx:
        print("RX mode: waiting for frames, Ctrl-C to exit")
        while True:
            result: Optional[tuple[bytes, object]] = rx.recv_optional(timeout_ms=timeout_ms)
            if result is None:
                continue
            payload, meta = result
            try:
                body = payload.decode("utf-8")
            except UnicodeDecodeError:
                body = payload.hex()
            print(
                f"seq={meta.seq} len={len(payload)} mcs={meta.mcs_index} bw={meta.bandwidth} payload={body}"
            )


def main() -> int:
    parser = argparse.ArgumentParser(description="Simple wfb_rs_py TX/RX example")
    parser.add_argument("--role", choices=["tx", "rx"], required=True)
    parser.add_argument("--iface", required=True, help="monitor-mode interface")
    parser.add_argument("--channel-id", type=int, required=True, help="channel id (u32)")
    parser.add_argument(
        "--timeout-ms",
        type=int,
        default=100,
        help="RX poll timeout in milliseconds",
    )
    parser.add_argument(
        "--tx-interval-ms",
        type=int,
        default=0,
        help="optional delay between transmitted lines",
    )
    args = parser.parse_args()

    if args.role == "tx":
        return run_tx(args.iface, args.channel_id, args.tx_interval_ms)
    return run_rx(args.iface, args.channel_id, args.timeout_ms)


if __name__ == "__main__":
    raise SystemExit(main())
