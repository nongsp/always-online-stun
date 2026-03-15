use tokio::net::UdpSocket;
use tokio::time::{timeout, Duration};

use futures::stream::{self, StreamExt};

use rand::Rng;

use serde::Serialize;

use std::net::SocketAddr;

const CONCURRENCY: usize = 100;
const TIMEOUT: u64 = 3;

const STUN_SERVERS: &[&str] = &[
"stun.l.google.com:19302",
"stun1.l.google.com:19302",
"stun2.l.google.com:19302",
"stun3.l.google.com:19302",
"stun4.l.google.com:19302",
"stun.cloudflare.com:3478",
"stun.nextcloud.com:443",
"stun.sipgate.net:3478",
"stun.callwithus.com:3478",
"stun.counterpath.net:3478",
"stun.ekiga.net:3478",
"stun.voipbuster.com:3478",
"stun.voipstunt.com:3478",
"stun.voxgratia.org:3478",
"stun.services.mozilla.com:3478",
"stun.sipgate.net:10000",
"stun.syncthing.net:3478",
"stun.miwifi.com:3478",
"stun.qq.com:3478",
"stunserver.stunprotocol.org:3478",
];

#[derive(Serialize)]
struct StunResult {
    server: String,
    mapped: String,
}

#[tokio::main]
async fn main() {

    println!("STUN concurrent scanner starting...");
    println!("servers: {}", STUN_SERVERS.len());
    println!("concurrency: {}", CONCURRENCY);

    let results: Vec<_> = stream::iter(STUN_SERVERS)
        .map(|server| async move {

            match scan_server(server).await {

                Some(mapped) => {

                    println!("OK {} -> {}", server, mapped);

                    Some(StunResult {
                        server: server.to_string(),
                        mapped,
                    })
                }

                None => {

                    println!("FAIL {}", server);

                    None
                }
            }
        })
        .buffer_unordered(CONCURRENCY)
        .collect()
        .await;

    let valid: Vec<StunResult> =
        results.into_iter().flatten().collect();

    println!("valid servers: {}", valid.len());

    let txt = valid
        .iter()
        .map(|v| v.server.clone())
        .collect::<Vec<_>>()
        .join("\n");

    std::fs::write("stun_servers.txt", txt).unwrap();

    let json = serde_json::to_string_pretty(&valid).unwrap();

    std::fs::write("stun_servers.json", json).unwrap();

    println!("files generated:");
    println!("stun_servers.txt");
    println!("stun_servers.json");
}

async fn scan_server(server: &str) -> Option<String> {

    let addr: SocketAddr = server.parse().ok()?;

    let socket = UdpSocket::bind("0.0.0.0:0").await.ok()?;

    socket.connect(addr).await.ok()?;

    let req = build_stun_request();

    socket.send(&req).await.ok()?;

    let mut buf = [0u8; 1024];

    let res = timeout(
        Duration::from_secs(TIMEOUT),
        socket.recv(&mut buf)
    )
    .await;

    let size = match res {

        Ok(Ok(n)) => n,

        _ => return None,
    };

    parse_stun(&buf[..size])
}

fn build_stun_request() -> Vec<u8> {

    let mut msg = vec![0u8; 20];

    msg[0] = 0x00;
    msg[1] = 0x01;

    msg[4] = 0x21;
    msg[5] = 0x12;
    msg[6] = 0xA4;
    msg[7] = 0x42;

    let mut rng = rand::thread_rng();

    for i in 8..20 {
        msg[i] = rng.gen();
    }

    msg
}

fn parse_stun(data: &[u8]) -> Option<String> {

    if data.len() < 20 {
        return None;
    }

    let cookie = [0x21,0x12,0xA4,0x42];

    let mut i = 20;

    while i + 4 <= data.len() {

        let t = u16::from_be_bytes([data[i],data[i+1]]);
        let l = u16::from_be_bytes([data[i+2],data[i+3]]) as usize;

        if i + 4 + l > data.len() {
            break;
        }

        let v = &data[i+4..i+4+l];

        if t == 0x0020 && l >= 8 {

            let port =
                u16::from_be_bytes([v[2],v[3]]) ^ 0x2112;

            let ip = [
                v[4]^cookie[0],
                v[5]^cookie[1],
                v[6]^cookie[2],
                v[7]^cookie[3],
            ];

            return Some(format!(
                "{}.{}.{}.{}:{}",
                ip[0],ip[1],ip[2],ip[3],port
            ));
        }

        i += 4 + ((l + 3) & !3);
    }

    None
}
