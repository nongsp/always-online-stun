use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant};
use std::io;

mod servers;
mod stun;
mod outputs;

use servers::{TransportProtocol, STUN_SERVERS};
use stun::{query_stun_server_udp};

#[tokio::main]
async fn main() -> io::Result<()> {

    println!("Starting STUN scan...");

    // reference STUN server (Google)
    let reference_server = "stun.l.google.com:19302";

    let reference_socket: SocketAddr = match reference_server.parse() {
        Ok(v) => v,
        Err(_) => {
            println!("Failed to parse reference STUN server");
            return Ok(());
        }
    };

    let local_socket = match UdpSocket::bind("0.0.0.0:0") {
        Ok(s) => s,
        Err(e) => {
            println!("Failed to bind UDP socket: {:?}", e);
            return Ok(());
        }
    };

    local_socket.set_nonblocking(true)?;

    let deadline = Instant::now() + Duration::from_secs(5);

    println!("Getting reference mapping...");

    let reference_mapping = match query_stun_server_udp(&local_socket, reference_socket, deadline).await {
        Ok(v) => v,
        Err(e) => {
            println!("Reference STUN failed: {:?}", e);
            println!("Skipping reference mapping and continuing...");
            None
        }
    };

    if let Some(ref_map) = &reference_mapping {
        println!("Reference mapped address: {:?}", ref_map.mapped_addr);
    }

    println!("Testing {} STUN servers...", STUN_SERVERS.len());

    let mut valid_servers = Vec::new();

    for server in STUN_SERVERS {

        if server.protocol != TransportProtocol::UDP {
            continue;
        }

        let socket_addr: SocketAddr = match format!("{}:{}", server.hostname, server.port).parse() {
            Ok(v) => v,
            Err(_) => {
                println!("Invalid address: {}:{}", server.hostname, server.port);
                continue;
            }
        };

        let local_socket = match UdpSocket::bind("0.0.0.0:0") {
            Ok(s) => s,
            Err(_) => continue,
        };

        local_socket.set_nonblocking(true)?;

        let deadline = Instant::now() + Duration::from_secs(3);

        match query_stun_server_udp(&local_socket, socket_addr, deadline).await {

            Ok(res) => {

                if let Some(mapped) = res.mapped_addr {

                    println!(
                        "OK {}:{} → {}",
                        server.hostname,
                        server.port,
                        mapped
                    );

                    valid_servers.push(format!("{}:{}", server.hostname, server.port));
                }

            }

            Err(_) => {
                println!("FAIL {}:{}", server.hostname, server.port);
            }

        }

    }

    println!("Valid STUN servers: {}", valid_servers.len());

    // write results
    let output = valid_servers.join("\n");

    std::fs::write("stun_servers.txt", output)?;

    println!("Saved results to stun_servers.txt");

    Ok(())
}
