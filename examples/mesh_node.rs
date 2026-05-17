use std::collections::HashSet;
use std::io::{self, BufRead};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use clap::Parser;
use wfb_rs::{WfbRx, WfbRxConfig, WfbTx, WfbTxConfig, WFB_FRAME_TYPE_DATA};

#[derive(Debug, Parser)]
#[command(name = "mesh_node", about = "Layer 2 Tactical Mesh Router")]
struct Args {
    #[arg(short = 'i', long = "iface", help = "Wi-Fi interface")]
    iface: String,

    #[arg(short = 'n', long = "node-id", help = "My Node ID (1, 2, or 3)")]
    node_id: u8,

    #[arg(short = 'd', long = "dest-id", help = "Target Node ID to send to (optional)")]
    dest_id: Option<u8>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let stream_id = 1; 

    let tx_cfg = WfbTxConfig {
        iface: args.iface.clone(),
        stream_id,
        frame_type: WFB_FRAME_TYPE_DATA,
        mcs_index: 1,
        bandwidth: 20,
    };
    
    let mut tx = WfbTx::open(&tx_cfg)?;
    let tx = Arc::new(Mutex::new(tx));

    let rx_cfg = WfbRxConfig {
        iface: args.iface.clone(),
        stream_id,
        rcv_buf_size: None,
        ignore_self_injected: true,
        ring_size: 16,
    };
    let rx = WfbRx::open(&rx_cfg)?;

    let seen_packets = Arc::new(Mutex::new(HashSet::new()));

    let my_id = args.node_id;
    let tx_clone = Arc::clone(&tx);
    let seen_clone = Arc::clone(&seen_packets);

    let rx_thread = thread::spawn(move || {
        let mut rx = rx;
        let mut buf = vec![0u8; 4096];

        loop {
            match rx.recv(&mut buf, Duration::from_millis(1000)) {
                Ok(Some((n, _meta))) => {
                    if n < 3 { continue; }                    
                    let dest_id = buf[0];
                    let src_id = buf[1];
                    let pkt_id = buf[2];

                    let mut seen = seen_clone.lock().unwrap();
                    if !seen.insert((src_id, pkt_id)) {
                        continue;
                    }

                    if dest_id == my_id {
                        let payload = &buf[3..n];
                        println!("\n[+] RECEIVED from Node {}: {}", src_id, String::from_utf8_lossy(payload));
                    } else {
                        println!("\n[*] RELAYING packet from Node {} aimed at Node {}...", src_id, dest_id);
                        let mut tx_lock = tx_clone.lock().unwrap();
                        let _ = tx_lock.send(&buf[..n], 0); 
                    }
                }
                Ok(None) => continue,
                Err(e) => {
                    eprintln!("RX error: {e}");
                    break;
                }
            }
        }
    });

    if let Some(target) = args.dest_id {
        println!("--- Node {} Online ---", my_id);
        println!("Type a message and hit Enter to send to Node {}.", target);
        
        let stdin = io::stdin();
        let mut pkt_id: u8 = 0;

        for line in stdin.lock().lines() {
            let line = line?;
            if line.is_empty() { continue; }

            let mut packet = vec![target, my_id, pkt_id];
            packet.extend_from_slice(line.as_bytes());

            let mut seen = seen_packets.lock().unwrap();
            seen.insert((my_id, pkt_id)); 
            drop(seen);

            let mut tx_lock = tx.lock().unwrap();
            tx_lock.send(&packet, 0)?;
            
            println!("[-] SENT to Node {}: {}", target, line);
            pkt_id = pkt_id.wrapping_add(1);
        }
    } else {
        println!("--- Node {} RELAY/LISTENING MODE ---", my_id);
        loop { thread::sleep(Duration::from_secs(60)); }
    }

    let _ = rx_thread.join();
    Ok(())
}