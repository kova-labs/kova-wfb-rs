use std::collections::HashSet;
use std::fs::File;
use std::io::{self, Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use clap::Parser;
use wfb_rs::{WfbRx, WfbRxConfig, WfbTx, WfbTxConfig, WFB_FRAME_TYPE_DATA};

#[derive(Debug, Parser)]
#[command(name = "mesh_node", about = "Layer 2 Tactical Mesh - Image Relay w/ Hop Tracking")]
struct Args {
    #[arg(short = 'i', long = "iface")]
    iface: String,

    #[arg(short = 'n', long = "node-id")]
    node_id: u8,

    #[arg(short = 'd', long = "dest-id")]
    dest_id: Option<u8>,

    #[arg(short = 'f', long = "file")]
    file: Option<String>,
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

    let seen_packets: Arc<Mutex<HashSet<(u8, u16)>>> = Arc::new(Mutex::new(HashSet::new()));

    let my_id = args.node_id;
    let tx_clone = Arc::clone(&tx);
    let seen_clone = Arc::clone(&seen_packets);
    let rx_thread = thread::spawn(move || {
        let mut rx = rx;
        let mut buf = vec![0u8; 4096];
        let mut image_bytes = Vec::new();

        loop {
            match rx.recv(&mut buf, Duration::from_millis(2000)) {
                Ok(Some((n, _meta))) => {
                    if n < 5 { continue; }
                    
                    let dest_id = buf[0];
                    let src_id = buf[1];
                    let pkt_id = u16::from_be_bytes([buf[2], buf[3]]);
                    let hop_count = buf[4]; 
                    let mut seen = seen_clone.lock().unwrap();
                    if !seen.insert((src_id, pkt_id)) {
                        continue; 
                    }
                    drop(seen);

                    if dest_id == my_id {
                        let payload = &buf[5..n];
                        image_bytes.extend_from_slice(payload);
                    
                        let link_status = if hop_count == 0 {
                            "[DIRECT LINK]"
                        } else {
                            Box::leak(format!("[RELAYED via {} HOPS]", hop_count).into_boxed_str())
                        };

                        print!("\r{} Downloading... Pkt: {} | Size: {} bytes", link_status, pkt_id, image_bytes.len());
                        io::stdout().flush().unwrap();
                        
                    } else {
                        buf[4] = buf[4].saturating_add(1);
                        println!("[*] RELAYING Pkt {} from Node {} -> Node {} (Hop Count now: {})", pkt_id, src_id, dest_id, buf[4]);
                        
                        let mut tx_lock = tx_clone.lock().unwrap();
                        let _ = tx_lock.send(&buf[..n], 0); 
                    }
                }
                Ok(None) => {
                    if !image_bytes.is_empty() {
                        let filename = format!("intel_node_{}.jpg", my_id);
                        if let Ok(mut file) = File::create(&filename) {
                            let _ = file.write_all(&image_bytes);
                            println!("\n\n[SUCCESS] Image fully reconstructed and saved as: {}", filename);
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

    if let Some(target) = args.dest_id {
        println!("--- Node {} C2 Link Active ---", my_id);

        if let Some(file_path) = args.file {
            println!("[-] Initiating Mesh Transfer to Node {}", target);
            
            let mut file = File::open(file_path)?;
            let mut buf = Vec::new();
            file.read_to_end(&mut buf)?;

            let mut pkt_id: u16 = 0;
            const CHUNK_SIZE: usize = 1024;
            let total_chunks = (buf.len() as f32 / CHUNK_SIZE as f32).ceil() as u16;

            for chunk in buf.chunks(CHUNK_SIZE) {
                // 5-BYTE HEADER: [Target] [Source] [Pkt High] [Pkt Low] [Hops]
                let mut packet = vec![target, my_id, (pkt_id >> 8) as u8, (pkt_id & 0xFF) as u8, 0];
                packet.extend_from_slice(chunk);
                
                seen_packets.lock().unwrap().insert((my_id, pkt_id));
                tx.lock().unwrap().send(&packet, 0).unwrap();
                
                print!("\r[-] Transmitting Chunk {}/{}", pkt_id + 1, total_chunks);
                io::stdout().flush().unwrap();
                
                pkt_id = pkt_id.wrapping_add(1);
                thread::sleep(Duration::from_millis(5)); 
            }
            println!("\n[-] Transmission complete!");
        }
        loop { thread::sleep(Duration::from_secs(60)); }
    } else {
        println!("--- Node {} RELAY/LISTENING MODE ---", my_id);
        loop { thread::sleep(Duration::from_secs(60)); }
    }

    let _ = rx_thread.join();
    Ok(())
}