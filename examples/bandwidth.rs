use std::thread;
use std::time::{Duration, Instant};

use clap::{Parser, ValueEnum};
use wfb_rs::{WFB_FRAME_TYPE_DATA, WFB_FRAME_TYPE_RTS, WfbRx, WfbRxConfig, WfbTx, WfbTxConfig};

fn parse_u32(s: &str) -> Result<u32, String> {
    let s = s.trim();
    if s.starts_with("0x") || s.starts_with("0X") {
        u32::from_str_radix(&s[2..], 16).map_err(|e| e.to_string())
    } else {
        s.parse::<u32>().map_err(|e| e.to_string())
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum FrameMode {
    Data,
    Rts,
}

impl FrameMode {
    fn frame_type(self) -> u8 {
        match self {
            FrameMode::Data => WFB_FRAME_TYPE_DATA,
            FrameMode::Rts => WFB_FRAME_TYPE_RTS,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Role {
    Tx,
    Rx,
}

#[derive(Debug, Parser)]
#[command(
    name = "bandwidth",
    about = "Measure WFB link throughput with TX or RX role",
    version
)]
struct Args {
    #[arg(long = "role", value_enum)]
    role: Role,

    #[arg(short = 'i', long = "iface")]
    iface: String,

    #[arg(short = 'c', long = "channel-id", value_parser = parse_u32)]
    channel_id: u32,

    #[arg(long = "frame-mode", value_enum, default_value_t = FrameMode::Data)]
    frame_mode: FrameMode,

    #[arg(long = "payload-size", default_value_t = 1200)]
    payload_size: usize,

    #[arg(long = "tx-interval-us", default_value_t = 0)]
    tx_interval_us: u64,

    #[arg(long = "report-ms", default_value_t = 1000)]
    report_ms: u64,
}

fn report_line(prefix: &str, bytes: u64, packets: u64, dt: Duration) {
    let secs = dt.as_secs_f64();
    if secs <= 0.0 {
        return;
    }
    let mbps = (bytes as f64 * 8.0) / secs / 1_000_000.0;
    let pps = packets as f64 / secs;
    println!(
        "{prefix} {:.3} Mbit/s ({:.1} pkt/s, {} bytes)",
        mbps, pps, bytes
    );
}

fn run_tx(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    let cfg = WfbTxConfig {
        iface: args.iface.clone(),
        channel_id: args.channel_id,
        frame_type: args.frame_mode.frame_type(),
        mcs_index: 7,
        bandwidth: 20,
    };
    let mut tx = WfbTx::open(&cfg)?;

    let payload_size = args.payload_size.max(1);
    let payload = vec![0x42u8; payload_size];
    let report_every = Duration::from_millis(args.report_ms.max(100));
    let pacing = (args.tx_interval_us > 0).then(|| Duration::from_micros(args.tx_interval_us));

    let mut seq: u32 = 1;
    let mut last = Instant::now();
    let mut bytes_interval: u64 = 0;
    let mut packets_interval: u64 = 0;
    let mut bytes_total: u64 = 0;
    let mut packets_total: u64 = 0;

    loop {
        tx.send(&payload, seq)?;
        seq = seq.wrapping_add(1);

        bytes_interval += payload.len() as u64;
        packets_interval += 1;
        bytes_total += payload.len() as u64;
        packets_total += 1;

        if last.elapsed() >= report_every {
            let dt = last.elapsed();
            report_line("TX", bytes_interval, packets_interval, dt);
            println!("TX total packets={} bytes={}", packets_total, bytes_total);
            last = Instant::now();
            bytes_interval = 0;
            packets_interval = 0;
        }

        if let Some(d) = pacing {
            thread::sleep(d);
        }
    }
}

fn run_rx(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    let cfg = WfbRxConfig {
        iface: args.iface.clone(),
        channel_id: args.channel_id,
        rcv_buf_size: None,
        ignore_self_injected: true,
        ring_size: 256,
    };
    let mut rx = WfbRx::open(&cfg)?;

    let mut buf = vec![0u8; args.payload_size.max(4096)];
    let report_every = Duration::from_millis(args.report_ms.max(100));

    let mut last = Instant::now();
    let mut bytes_interval: u64 = 0;
    let mut packets_interval: u64 = 0;
    let mut bytes_total: u64 = 0;
    let mut packets_total: u64 = 0;

    loop {
        match rx.recv(&mut buf, Duration::from_millis(200))? {
            Some((n, _meta)) => {
                bytes_interval += n as u64;
                packets_interval += 1;
                bytes_total += n as u64;
                packets_total += 1;
            }
            None => {}
        }

        if last.elapsed() >= report_every {
            let dt = last.elapsed();
            report_line("RX", bytes_interval, packets_interval, dt);
            println!("RX total packets={} bytes={}", packets_total, bytes_total);
            last = Instant::now();
            bytes_interval = 0;
            packets_interval = 0;
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    if args.channel_id == 0 {
        return Err("channel-id must be non-zero".into());
    }

    match args.role {
        Role::Tx => run_tx(&args),
        Role::Rx => run_rx(&args),
    }
}
