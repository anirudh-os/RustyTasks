use std::net::SocketAddr;
use std::str;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;

use crate::crdt::CrdtToDoList;
use crate::peer::{Peer, PeerId, SharedPeers};
use crate::sync::SyncState;
use base64::{engine::general_purpose, Engine as _};
use ed25519_dalek::{Signer, SigningKey, Verifier};
use rand::random;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", content = "data")]
pub enum Message {
    Hello {
        peer_id: String,
        public_key: String,
        challenge: String,
        signature: String,
    },
    Changes(Vec<Vec<u8>>), // Check if it is Vec<u8>
}

pub async fn connect_to_peer(target_ip: String, local_peer_id: PeerId, public_key: [u8; 32], private_key: [u8; 32]) -> Result<(), Box<dyn std::error::Error>> {
    let port = 58008;
    let address = format!("{}:{}", target_ip, port);
    let socket_addr: SocketAddr = address.parse()?;

    println!("Attempting to connect to {}", socket_addr);

    match TcpStream::connect(socket_addr).await {
        Ok(mut stream) => {
            println!("Connected to {}", socket_addr);

            let challenge: [u8; 32] = random();
            let signing_key = SigningKey::from_bytes(&private_key);
            let signature = signing_key.sign(&challenge);

            let hello = Message::Hello {
                peer_id: local_peer_id.id,
                public_key: general_purpose::STANDARD.encode(public_key),
                challenge: general_purpose::STANDARD.encode(challenge),
                signature: general_purpose::STANDARD.encode(signature.to_bytes()),
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
    use tokio::sync::mpsc;
    use crate::peer::PeerId;

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
                Message::Hello { peer_id, public_key, challenge, signature } => {
                    // Decode public key
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

                    let verifying_key = match ed25519_dalek::VerifyingKey::from_bytes(&public_key_bytes) {
                        Ok(k) => k,
                        Err(e) => {
                            eprintln!("Invalid verifying key from {}: {}", address, e);
                            return;
                        }
                    };

                    let challenge_bytes = match general_purpose::STANDARD.decode(&challenge) {
                        Ok(bytes) => bytes,
                        Err(e) => {
                            eprintln!("Failed to decode challenge from {}: {}", address, e);
                            return;
                        }
                    };

                    let signature_bytes = match general_purpose::STANDARD.decode(&signature) {
                        Ok(bytes) => bytes,
                        Err(e) => {
                            eprintln!("Failed to decode signature from {}: {}", address, e);
                            return;
                        }
                    };

                    let signature = match signature_bytes.as_slice().try_into() {
                        Ok(bytes) => ed25519_dalek::Signature::from_bytes(&bytes),
                        Err(_) => {
                            eprintln!("Signature must be 64 bytes, got {}", signature_bytes.len());
                            return;
                        }
                    };

                    if let Err(e) = verifying_key.verify(&challenge_bytes, &signature) {
                        eprintln!("Signature verification failed from {}: {}", address, e);
                        return;
                    }


                    let (_, mut writer) = socket.into_split();

                    let (tx, mut rx) = mpsc::channel::<Message>(100);

                    tokio::spawn(async move {
                        while let Some(message) = rx.recv().await {
                            match serde_json::to_vec(&message) {
                                Ok(serialized) => {
                                    if let Err(e) = writer.write_all(&serialized).await {
                                        eprintln!("Failed to send message to {}: {}", address, e);
                                        break;
                                    }
                                }
                                Err(e) => {
                                    eprintln!("Serialization error for {}: {}", address, e);
                                }
                            }
                        }
                    });

                    let peer_id = PeerId { id: peer_id.clone() };
                    let peer = Peer {
                        peer_id: peer_id.clone(),
                        address,
                        public_key: public_key_bytes,
                        sender: Some(tx),
                    };

                    {
                        let mut peers = shared_peers.lock().await;
                        peers.insert(peer_id.clone(), peer);
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