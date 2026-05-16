use std::io::{self, BufRead};
use std::thread;
use std::time::Duration;

use clap::{Parser, ValueEnum};
use wfb_rs::{WFB_FRAME_TYPE_DATA, WFB_FRAME_TYPE_RTS, WfbRxConfig, WfbTx, WfbTxConfig};

fn parse_u32(s: &str) -> Result<u32, String> {
    let s = s.trim();
    if s.starts_with("0x") || s.starts_with("0X") {
        u32::from_str_radix(&s[2..], 16).map_err(|e| e.to_string())
    } else {
        s.parse::<u32>().map_err(|e| e.to_string())
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Mode {
    Data,
    Rts,
}

impl Mode {
    fn frame_type(self) -> u8 {
        match self {
            Mode::Data => WFB_FRAME_TYPE_DATA,
            Mode::Rts => WFB_FRAME_TYPE_RTS,
        }
    }
}

#[derive(Debug, Parser)]
#[command(
    name = "simple_txrx",
    about = "Send plaintext WFB-rs frames from stdin and print received frames",
    version
)]
struct Args {
    #[arg(
        short = 'i',
        long = "iface",
        help = "Monitor-mode capture/injection interface"
    )]
    iface: String,

    #[arg(
        short = 'c',
        long = "stream-id",
        help = "WFB stream_id to embed in synthetic headers (hex supported, e.g. 0x1234)",
        value_parser = parse_u32
    )]
    stream_id: u32,

    #[arg(
        short = 'm',
        long = "mode",
        value_enum,
        default_value_t = Mode::Data,
        help = "On-air synthetic frame type"
    )]
    mode: Mode,

    #[arg(long = "print-rssi", help = "Print antenna slot 0 RSSI")]
    print_rssi: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    if args.stream_id == 0 {
        return Err("stream-id must be non-zero (strict WFB-rs behavior)".into());
    }

    let tx_cfg = WfbTxConfig {
        iface: args.iface.clone(),
        stream_id: args.stream_id,
        frame_type: args.mode.frame_type(),
        mcs_index: 1,
        bandwidth: 20,
    };

    let rx_cfg = WfbRxConfig {
        iface: args.iface,
        stream_id: args.stream_id,
        rcv_buf_size: None,
        ignore_self_injected: true,
        ring_size: 16,
    };

    let mut tx = WfbTx::open(&tx_cfg)?;
    let rx = wfb_rs::WfbRx::open(&rx_cfg)?;

    let rx_thread = thread::spawn(move || {
        let mut rx = rx;
        let mut buf = vec![0u8; 4096 + 1];
        loop {
            match rx.recv(&mut buf, Duration::from_millis(1000)) {
                Ok(Some((n, meta))) => {
                    let payload = &buf[..n];
                    if args.print_rssi {
                        println!(
                            "RX seq={} len={} bw={} mcs={} freq={} rssi0={} truncated={} payload=\"{}\"",
                            meta.seq,
                            n,
                            meta.bandwidth,
                            meta.mcs_index,
                            meta.freq,
                            meta.rssi[0],
                            meta.truncated as u8,
                            String::from_utf8_lossy(payload)
                        );
                    } else {
                        println!(
                            "RX seq={} len={} truncated={} payload=\"{}\"",
                            meta.seq,
                            n,
                            meta.truncated as u8,
                            String::from_utf8_lossy(payload)
                        );
                    }
                    println!();
                }
                Ok(None) => continue, // timeout
                Err(e) => {
                    eprintln!("RX error: {e}");
                    break;
                }
            }
        }
    });

    let stdin = io::stdin();
    let mut seq: u32 = 1;
    for line in stdin.lock().lines() {
        let line = line?;
        let payload = line.as_bytes();
        if payload.is_empty() {
            continue;
        }
        tx.send(payload, seq)?;
        seq = seq.wrapping_add(1);
    }

    let _ = rx_thread.join();
    Ok(())
}
