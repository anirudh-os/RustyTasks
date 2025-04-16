use std::net::SocketAddr;
use std::str;
use std::sync::Arc;
use std::time::Duration;
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
    Changes(Vec<Vec<u8>>),
    Ping,
    Pong,
}

async fn verify_handshake(
    msg: &Message,
    socket_addr: SocketAddr,
    tx: &tokio::sync::mpsc::Sender<Message>,
    shared_peers: &SharedPeers,
) -> Option<()> {
    if let Message::Hello {
        peer_id,
        public_key,
        challenge,
        signature,
    } = msg
    {
        let public_key_bytes = match general_purpose::STANDARD.decode(public_key) {
            Ok(bytes) if bytes.len() == 32 => {
                let mut key = [0u8; 32];
                key.copy_from_slice(&bytes);
                key
            }
            Ok(_) => {
                eprintln!("Invalid public key length from {}", socket_addr);
                return None;
            }
            Err(e) => {
                eprintln!("Failed to decode public_key from {}: {}", socket_addr, e);
                return None;
            }
        };

        let verifying_key = match ed25519_dalek::VerifyingKey::from_bytes(&public_key_bytes) {
            Ok(k) => k,
            Err(e) => {
                eprintln!("Invalid verifying key from {}: {}", socket_addr, e);
                return None;
            }
        };

        let challenge_bytes = match general_purpose::STANDARD.decode(challenge) {
            Ok(bytes) => bytes,
            Err(e) => {
                eprintln!("Failed to decode challenge from {}: {}", socket_addr, e);
                return None;
            }
        };

        let signature_bytes = match general_purpose::STANDARD.decode(signature) {
            Ok(bytes) => bytes,
            Err(e) => {
                eprintln!("Failed to decode signature from {}: {}", socket_addr, e);
                return None;
            }
        };

        let signature = match signature_bytes.as_slice().try_into() {
            Ok(bytes) => ed25519_dalek::Signature::from_bytes(bytes),
            Err(_) => {
                eprintln!("Signature must be 64 bytes, got {}", signature_bytes.len());
                return None;
            }
        };

        if let Err(e) = verifying_key.verify(&challenge_bytes, &signature) {
            eprintln!("Signature verification failed from {}: {}", socket_addr, e);
            return None;
        }

        let peer_id = PeerId { id: peer_id.clone() };
        let peer = Peer {
            peer_id: peer_id.clone(),
            address: socket_addr,
            public_key: public_key_bytes,
            sender: Some(tx.clone()),
        };

        {
            let mut peers = shared_peers.lock().await;
            peers.insert(peer_id.clone(), peer);
        }

        println!("Registered peer '{}' from {}", peer_id.id, socket_addr);
        Some(())
    } else {
        None
    }
}

pub async fn connect_to_peer(
    target_ip: String,
    local_peer_id: PeerId,
    public_key: [u8; 32],
    private_key: [u8; 32],
    shared_peers: SharedPeers,
    crdt: Arc<Mutex<CrdtToDoList>>,
    sync_state: Arc<Mutex<SyncState>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let port = 58008;
    let address = format!("{}:{}", target_ip, port);
    let socket_addr: SocketAddr = address.parse()?;

    println!("Attempting to connect to {}", socket_addr);

    let mut stream = TcpStream::connect(socket_addr).await?;
    println!("Connected to {}", socket_addr);

    // Send handshake
    let challenge: [u8; 32] = random();
    let signing_key = SigningKey::from_bytes(&private_key);
    let signature = signing_key.sign(&challenge);

    let hello = Message::Hello {
        peer_id: local_peer_id.id.clone(),
        public_key: general_purpose::STANDARD.encode(public_key),
        challenge: general_purpose::STANDARD.encode(challenge),
        signature: general_purpose::STANDARD.encode(signature.to_bytes()),
    };

    let mut handshake_json = serde_json::to_vec(&hello)?;
    handshake_json.push(b'\n');
    stream.write_all(&handshake_json).await?;
    stream.flush().await?;

    let (mut reader, mut writer) = stream.into_split();
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Message>(100);

    let peer = Peer {
        peer_id: local_peer_id.clone(),
        address: socket_addr,
        public_key,
        sender: Some(tx.clone()),
    };

    {
        let mut peers = shared_peers.lock().await;
        peers.insert(local_peer_id.clone(), peer);
    }

    // Spawn a task to handle outgoing messages
    tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            match serde_json::to_vec(&message) {
                Ok(mut serialized) => {
                    serialized.push(b'\n');
                    if let Err(e) = writer.write_all(&serialized).await {
                        eprintln!("Failed to send message to {}: {}", socket_addr, e);
                        break;
                    }
                    if let Err(e) = writer.flush().await {
                        eprintln!("Failed to flush writer for {}: {}", socket_addr, e);
                        break;
                    }
                }
                Err(e) => {
                    eprintln!("Serialization error for {}: {}", socket_addr, e);
                }
            }
        }
    });

    let tx_clone = tx.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        loop {
            interval.tick().await;
            if let Err(e) = tx_clone.send(Message::Ping).await {
                eprintln!("Failed to send ping to {}: {}", socket_addr, e);
                break;
            }
        }
    });

    let mut buffer = vec![0u8; 4096];
    let mut accumulated_data = Vec::new();

    loop {
        match reader.read(&mut buffer).await {
            Ok(0) => {
                println!("Connection closed by peer {}", socket_addr);
                break;
            }
            Ok(n) => {
                accumulated_data.extend_from_slice(&buffer[..n]);

                while let Some(pos) = accumulated_data.iter().position(|&b| b == b'\n') {
                    let msg_bytes = accumulated_data.drain(..pos).collect::<Vec<_>>();
                    accumulated_data.drain(..1); // Remove the newline

                    let msg_str = match str::from_utf8(&msg_bytes) {
                        Ok(s) => s,
                        Err(e) => {
                            eprintln!("Invalid UTF-8 from {}: {}", socket_addr, e);
                            continue;
                        }
                    };

                    let msg: Message = match serde_json::from_str(msg_str) {
                        Ok(m) => m,
                        Err(e) => {
                            eprintln!("Failed to parse message from {}: {}", socket_addr, e);
                            continue;
                        }
                    };

                    match msg {
                        Message::Hello { .. } => {
                            if verify_handshake(&msg, socket_addr, &tx, &shared_peers).await.is_some() {
                                // Trigger initial sync
                                let mut crdt = crdt.lock().await;
                                let mut sync_state = sync_state.lock().await;
                                crdt.send_changes(&mut sync_state, &shared_peers).await;
                            }
                        }
                        Message::Changes(raw_changes) => {
                            let mut crdt = crdt.lock().await;
                            let mut sync_state = sync_state.lock().await;
                            crdt.apply_changes_from_bytes(raw_changes, &mut sync_state).await;
                        }
                        Message::Ping => {
                            if let Err(e) = tx.send(Message::Pong).await {
                                eprintln!("Failed to send pong to {}: {}", socket_addr, e);
                            }
                        }
                        Message::Pong => {
                            println!("Received pong from {}", socket_addr);
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to read from socket {}: {}", socket_addr, e);
                break;
            }
        }
    }

    // Clean up
    let mut peers = shared_peers.lock().await;
    peers.retain(|_, peer| peer.address != socket_addr);
    println!("Unregistered peer at {}", socket_addr);
    Ok(())
}

pub async fn connections(
    shared_peers: SharedPeers,
    crdt: Arc<Mutex<CrdtToDoList>>,
    sync_state: Arc<Mutex<SyncState>>,
) -> Result<(), Box<dyn std::error::Error>> {
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
    socket: TcpStream,
    address: SocketAddr,
    shared_peers: SharedPeers,
    crdt: Arc<Mutex<CrdtToDoList>>,
    sync_state: Arc<Mutex<SyncState>>,
) {
    use tokio::sync::mpsc;

    let (mut reader, mut writer) = socket.into_split();
    let mut buffer = vec![0u8; 4096];
    let mut accumulated_data = Vec::new();

    let (tx, mut rx) = mpsc::channel::<Message>(100);

    tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            match serde_json::to_vec(&message) {
                Ok(mut serialized) => {
                    serialized.push(b'\n');
                    if let Err(e) = writer.write_all(&serialized).await {
                        eprintln!("Failed to send message to {}: {}", address, e);
                        break;
                    }
                    if let Err(e) = writer.flush().await {
                        eprintln!("Failed to flush writer for {}: {}", address, e);
                        break;
                    }
                }
                Err(e) => {
                    eprintln!("Serialization error for {}: {}", address, e);
                }
            }
        }
    });

    let tx_clone = tx.clone(); // Clone tx for the ping task
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        loop {
            interval.tick().await;
            if let Err(e) = tx_clone.send(Message::Ping).await {
                eprintln!("Failed to send ping to {}: {}", address, e);
                break;
            }
        }
    });

    loop {
        match reader.read(&mut buffer).await {
            Ok(0) => {
                println!("Connection closed by peer {}", address);
                break;
            }
            Ok(n) => {
                accumulated_data.extend_from_slice(&buffer[..n]);

                while let Some(pos) = accumulated_data.iter().position(|&b| b == b'\n') {
                    let msg_bytes = accumulated_data.drain(..pos).collect::<Vec<_>>();
                    accumulated_data.drain(..1); // Remove the newline

                    let msg_str = match str::from_utf8(&msg_bytes) {
                        Ok(s) => s,
                        Err(e) => {
                            eprintln!("Invalid UTF-8 from {}: {}", address, e);
                            continue;
                        }
                    };

                    let msg: Message = match serde_json::from_str(msg_str) {
                        Ok(m) => m,
                        Err(e) => {
                            eprintln!("Failed to parse message from {}: {}", address, e);
                            continue;
                        }
                    };

                    match msg {
                        Message::Hello { .. } => {
                            if verify_handshake(&msg, address, &tx, &shared_peers).await.is_some() {
                                // Trigger initial sync
                                let mut crdt = crdt.lock().await;
                                let mut sync_state = sync_state.lock().await;
                                crdt.send_changes(&mut sync_state, &shared_peers).await;
                            }
                        }
                        Message::Changes(raw_changes) => {
                            let mut crdt = crdt.lock().await;
                            let mut sync_state = sync_state.lock().await;
                            crdt.apply_changes_from_bytes(raw_changes, &mut sync_state).await;
                        }
                        Message::Ping => {
                            if let Err(e) = tx.send(Message::Pong).await {
                                eprintln!("Failed to send pong to {}: {}", address, e);
                            }
                        }
                        Message::Pong => {
                            println!("Received pong from {}", address);
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to read from socket {}: {}", address, e);
                break;
            }
        }
    }

    let mut peers = shared_peers.lock().await;
    peers.retain(|_, peer| peer.address != address);
    println!("Unregistered peer at {}", address);
}