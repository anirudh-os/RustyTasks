use std::str;
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};

use crate::peer::{Peer, PeerId, SharedPeers};

#[derive(Debug, Serialize, Deserialize)]
struct Hello {
    peer_id: String,
    public_key: String, // base64 encoded 32-byte key
}

pub async fn connect_to_peer(
    target_ip: String,
    local_peer_id: PeerId,
    public_key: [u8; 32],
) -> Result<(), Box<dyn std::error::Error>> {
    let port = 58008;
    let address = format!("{}:{}", target_ip, port);
    let socket_addr: SocketAddr = address.parse()?;

    println!("Attempting to connect to {}", socket_addr);

    match TcpStream::connect(socket_addr).await {
        Ok(mut stream) => {
            println!("Connected to {}", socket_addr);

            let hello = Hello {
                peer_id: local_peer_id.id,
                public_key: general_purpose::STANDARD.encode(public_key),
            };

            let handshake_json = serde_json::to_string(&hello)?;

            stream.write_all(handshake_json.as_bytes()).await?;
            // println!("Sent handshake to {}", socket_addr);

            Ok(())
        }
        Err(e) => {
            eprintln!("Failed to connect to {}: {}", socket_addr, e);
            Err(Box::new(e))
        }
    }
}


pub async fn connections(shared_peers: SharedPeers) -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("0.0.0.0:58008").await?;

    loop {
        let (socket, address) = listener.accept().await?;
        let shared_peers = shared_peers.clone();

        tokio::spawn(async move {
            handle_connection(socket, address, shared_peers).await;
        });
    }
}

async fn handle_connection(mut socket: TcpStream, address: SocketAddr, shared_peers: SharedPeers) {
    let mut buffer = vec![0u8; 1024];

    match socket.read(&mut buffer).await {
        Ok(n) if n == 0 => {
            println!("Connection from {} closed immediately", address);
            return;
        }
        Ok(n) => {
            let msg = match str::from_utf8(&buffer[..n]) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Invalid UTF-8 from {}: {}", address, e);
                    return;
                }
            };

            let handshake: Hello = match serde_json::from_str(msg) {
                Ok(h) => h,
                Err(e) => {
                    eprintln!("Failed to parse handshake JSON from {}: {}", address, e);
                    return;
                }
            };

            let public_key_bytes = match general_purpose::STANDARD.decode(&handshake.public_key) {
                Ok(bytes) if bytes.len() == 32 => {
                    let mut key = [0u8; 32];
                    key.copy_from_slice(&bytes);
                    key
                }
                Ok(_) => {
                    eprintln!("Invalid public key length from {}", address);
                    return;
                }
                Err(e) => {
                    eprintln!("Failed to decode public key from {}: {}", address, e);
                    return;
                }
            };

            let peer_id = PeerId {
                id: handshake.peer_id.clone(),
            };

            let peer = Peer {
                peer_id: peer_id.clone(),
                address,
                public_key: public_key_bytes,
            };

            {
                let mut peers = shared_peers.lock().unwrap();
                peers.insert(peer_id.clone(), peer);
            }

            // println!("Registered peer '{}' from {}", handshake.peer_id, address);
        }
        Err(e) => {
            eprintln!("Failed to read from socket {}: {}", address, e);
        }
    }
}