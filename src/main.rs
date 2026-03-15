use tokio::net::UdpSocket;
use std::net::SocketAddr;
use std::time::Duration;

const STUN_SERVERS: &[&str] = &[
    "stun.l.google.com:19302",
    "stun1.l.google.com:19302",
    "stun2.l.google.com:19302",
    "stun3.l.google.com:19302",
    "stun4.l.google.com:19302",
    "stun.cloudflare.com:3478",
];

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    println!("Starting STUN availability scan...");

    let mut valid_servers = Vec::new();

    for server in STUN_SERVERS {

        let addr: SocketAddr = match server.parse() {
            Ok(v) => v,
            Err(_) => {
                println!("Invalid address {}", server);
                continue;
            }
        };

        match test_stun(addr).await {

            Ok(mapped_ip) => {
                println!("OK {} -> {}", server, mapped_ip);
                valid_servers.push(server.to_string());
            }

            Err(e) => {
                println!("FAIL {} ({})", server, e);
            }

        }
    }

    println!("Valid STUN servers: {}", valid_servers.len());

    let output = valid_servers.join("\n");

    std::fs::write("stun_servers.txt", output)?;

    println!("Saved stun_servers.txt");

    Ok(())
}

async fn test_stun(server: SocketAddr) -> Result<String, Box<dyn std::error::Error>> {

    let socket = UdpSocket::bind("0.0.0.0:0").await?;

    socket.connect(server).await?;

    let binding_request = build_stun_request();

    socket.send(&binding_request).await?;

    let mut buf = [0u8; 1024];

    let timeout = tokio::time::timeout(
        Duration::from_secs(3),
        socket.recv(&mut buf)
    ).await;

    match timeout {

        Ok(Ok(size)) => {

            let mapped = parse_xor_mapped_address(&buf[..size])?;

            Ok(mapped)
        }

        _ => Err("timeout".into())

    }

}

fn build_stun_request() -> Vec<u8> {

    let mut msg = vec![0u8; 20];

    msg[0] = 0x00;
    msg[1] = 0x01;

    msg[4] = 0x21;
    msg[5] = 0x12;
    msg[6] = 0xA4;
    msg[7] = 0x42;

    for i in 8..20 {
        msg[i] = rand::random();
    }

    msg
}

fn parse_xor_mapped_address(data: &[u8]) -> Result<String, Box<dyn std::error::Error>> {

    if data.len() < 20 {
        return Err("invalid stun response".into());
    }

    let mut i = 20;

    while i + 4 < data.len() {

        let attr_type = u16::from_be_bytes([data[i], data[i + 1]]);
        let attr_len = u16::from_be_bytes([data[i + 2], data[i + 3]]) as usize;

        if attr_type == 0x0020 {

            let port = u16::from_be_bytes([data[i + 6], data[i + 7]]) ^ 0x2112;

            let ip = [
                data[i + 8] ^ 0x21,
                data[i + 9] ^ 0x12,
                data[i + 10] ^ 0xA4,
                data[i + 11] ^ 0x42,
            ];

            return Ok(format!(
                "{}.{}.{}.{}:{}",
                ip[0], ip[1], ip[2], ip[3], port
            ));
        }

        i += 4 + attr_len;
    }

    Err("no mapped address".into())
}
