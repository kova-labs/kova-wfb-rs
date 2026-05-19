use std::fs::File;
use std::io::{self, Read, Write, BufRead}; 
use std::thread;
use std::time::Duration;

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
    name = "image_txrx",
    about = "Send and receive images/files over WFB-rs frames",
    version
)]
struct Args {
    #[arg(short = 'i', long = "iface", help = "Wi-Fi interface")]
    iface: String,

    #[arg(short = 'c', long = "stream-id", help = "Stream ID", value_parser = parse_u32)]
    stream_id: u32,

    #[arg(short = 'f', long = "file", help = "Path to image/file to send (optional)")]
    file: Option<String>,

    #[arg(short = 'm', long = "mode", value_enum, default_value_t = Mode::Data, help = "Frame type")]
    mode: Mode,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    if args.stream_id == 0 {
        return Err("stream-id must be non-zero".into());
    }

    let tx_cfg = WfbTxConfig {
        iface: args.iface.clone(),
        stream_id: args.stream_id,
        frame_type: args.mode.frame_type(),
        mcs_index: 1,
        bandwidth: 20, 
    };
    let mut tx = WfbTx::open(&tx_cfg)?;

    let rx_cfg = WfbRxConfig {
        iface: args.iface.clone(),
        stream_id: args.stream_id,
        rcv_buf_size: None,
        ignore_self_injected: true,
        ring_size: 16,
    };
    let rx = WfbRx::open(&rx_cfg)?;

    let rx_thread = thread::spawn(move || {
        let mut rx = rx;
        let mut buf = vec![0u8; 4096 + 1];
        let mut image_bytes = Vec::new();

        loop {
            match rx.recv(&mut buf, Duration::from_millis(2000)) { 
                Ok(Some((n, meta))) => {
                    image_bytes.extend_from_slice(&buf[..n]);
                    println!("Received seq={} len={}", meta.seq, n);
                }
                Ok(None) => {
                    if !image_bytes.is_empty() {
                        if let Ok(mut file) = File::create("received_file.jpg") { 
                            let _ = file.write_all(&image_bytes);
                            println!(
                                "File saved as received_file.jpg ({} bytes)",
                                image_bytes.len()
                            );
                        }
                        image_bytes.clear();
                    }
                }
                Err(e) => {
                    eprintln!("RX error: {e}");
                    break;
                }
            }
        }
    });

    if let Some(file_path) = args.file {
        let mut file = File::open(file_path)?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;

        const CHUNK_SIZE: usize = 1024;
        let mut seq: u32 = 1;
        for chunk in buf.chunks(CHUNK_SIZE) {
            tx.send(chunk, seq)?;
            println!("Sent seq={} len={}", seq, chunk.len());
            seq = seq.wrapping_add(1);
            
            thread::sleep(Duration::from_millis(5)); 
        }
        println!("File transmission complete!");
    } else {
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
    }

    let _ = rx_thread.join();
    Ok(())
}