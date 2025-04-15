use std::str;
use std::net::SocketAddr;
use std::sync::{Arc};
use automerge::Change;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;

use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};
use crate::crdt::CrdtToDoList;
use crate::peer::{Peer, PeerId, SharedPeers};
use crate::sync::SyncState;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum Message {
    Hello {
        peer_id: String,
        public_key: String,
    },
    Changes(Vec<Vec<u8>>),
    // Add more message types here, like Ping, RequestState, etc.
}

pub async fn connect_to_peer(target_ip: String, local_peer_id: PeerId, public_key: [u8; 32]) -> Result<(), Box<dyn std::error::Error>> {
    let port = 58008;
    let address = format!("{}:{}", target_ip, port);
    let socket_addr: SocketAddr = address.parse()?;

    println!("Attempting to connect to {}", socket_addr);

    match TcpStream::connect(socket_addr).await {
        Ok(mut stream) => {
            println!("Connected to {}", socket_addr);

            let hello = Message::Hello {
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


pub async fn connections(shared_peers: SharedPeers, crdt:Arc<Mutex<CrdtToDoList>>, sync_state: Arc<Mutex<SyncState>>) -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("0.0.0.0:58008").await?;

    loop {
        let (socket, address) = listener.accept().await?;
        let shared_peers = shared_peers.clone();
        let crdt = crdt.clone();
        let sync_state = sync_state.clone();

        tokio::spawn(async move {
            handle_connection(socket, address, shared_peers, crdt, sync_state).await;
        });
    }
}

async fn handle_connection(
    mut socket: TcpStream,
    address: SocketAddr,
    shared_peers: SharedPeers,
    crdt: Arc<Mutex<CrdtToDoList>>,
    sync_state: Arc<Mutex<SyncState>>,
) {
    let mut buffer = vec![0u8; 4096];

    match socket.read(&mut buffer).await {
        Ok(n) if n == 0 => {
            println!("Connection from {} closed immediately", address);
            return;
        }
        Ok(n) => {
            let msg_str = match str::from_utf8(&buffer[..n]) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Invalid UTF-8 from {}: {}", address, e);
                    return;
                }
            };

            let msg: Message = match serde_json::from_str(msg_str) {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("Failed to parse message from {}: {}", address, e);
                    return;
                }
            };

            match msg {
                Message::Hello { peer_id, public_key } => {
                    let public_key_bytes = match general_purpose::STANDARD.decode(&public_key) {
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

                    let peer_id = PeerId { id: peer_id.clone() };
                    let peer = Peer {
                        peer_id: peer_id.clone(),
                        address,
                        public_key: public_key_bytes,
                    };

                    {
                        let peers = shared_peers.lock();
                        peers.await.insert(peer_id.clone(), peer);
                    }

                    println!("Registered peer '{}' from {}", peer_id.id, address);
                }

                Message::Changes(raw_changes) => {
                    let mut crdt = crdt.lock().await;
                    let mut sync_state = sync_state.lock().await;
                    crdt.apply_changes_from_bytes(raw_changes, &mut sync_state).await;
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to read from socket {}: {}", address, e);
        }
    }
}


pub async fn send_changes_to_peer(peer: &Peer, changes: &[Change]) -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = TcpStream::connect(peer.address).await?;

    let raw_changes: Vec<Vec<u8>> = changes
        .iter()
        .map(|c| c.raw_bytes().to_vec())
        .collect();

    let serialized = serde_json::to_vec(&raw_changes)?;
    stream.write_all(&serialized).await?;

    Ok(())
}