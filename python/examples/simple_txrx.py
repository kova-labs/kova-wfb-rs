#!/usr/bin/env python3
from __future__ import annotations

import argparse
import sys
import threading

from wfb_rs_py import Rx, Tx


def run_simple_txrx(
    iface: str, stream_id: int, timeout_ms: int, interval_ms: int, print_rssi: bool
) -> int:
    stop_event = threading.Event()
    rx_error: list[Exception] = []

    with Tx(iface=iface, stream_id=stream_id) as tx, Rx(
        iface=iface, stream_id=stream_id
    ) as rx:
        print("Simple TX/RX mode: type lines to send, Ctrl-D or Ctrl-C to exit")

        def rx_loop() -> None:
            while not stop_event.is_set():
                result = rx.recv_optional(timeout_ms=timeout_ms)
                if result is None:
                    continue
                payload, meta = result
                text = payload.decode("utf-8", errors="replace")
                if print_rssi:
                    print(
                        f'RX seq={meta.seq} len={len(payload)} bw={meta.bandwidth} '
                        f'mcs={meta.mcs_index} rssi0={meta.rssi[0]} truncated={int(meta.truncated)} '
                        f'payload="{text}"'
                    )
                else:
                    print(
                        f'RX seq={meta.seq} len={len(payload)} truncated={int(meta.truncated)} '
                        f'payload="{text}"'
                    )

        rx_thread = threading.Thread(target=rx_loop, name="wfb-rx", daemon=True)
        rx_thread.start()

        seq = 1
        try:
            for line in sys.stdin:
                payload = line.rstrip("\n").encode("utf-8")
                if not payload:
                    continue
                tx.send(payload, seq=seq)
                seq += 1

                if interval_ms > 0:
                    import time

                    time.sleep(interval_ms / 1000.0)
        except KeyboardInterrupt:
            pass
        except Exception as exc:
            rx_error.append(exc)
        finally:
            stop_event.set()
            rx_thread.join(timeout=(timeout_ms / 1000.0) + 0.2)

    if rx_error:
        raise rx_error[0]
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description="Simple wfb_rs_py TX/RX example")
    parser.add_argument("--iface", required=True, help="monitor-mode interface")
    parser.add_argument("--stream-id", type=int, required=True, help="stream id (u32)")
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
    parser.add_argument(
        "--print-rssi",
        action="store_true",
        help="print antenna slot 0 RSSI in RX lines",
    )
    args = parser.parse_args()

    return run_simple_txrx(
        args.iface,
        args.stream_id,
        args.timeout_ms,
        args.tx_interval_ms,
        args.print_rssi,
    )


if __name__ == "__main__":
    raise SystemExit(main())
