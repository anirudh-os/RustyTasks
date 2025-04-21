use std::net::SocketAddr;
use std::str;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tokio::sync::mpsc;
use base64::engine::general_purpose;
use base64::Engine as _;
use crate::crdt::CrdtToDoList;
use crate::peer::{Peer, PeerId, SharedPeers};
use crate::sync::SyncState;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", content = "data")]
pub enum Message {
    Hello {
        peer_id: String,
        public_key: String,
    },
    Changes(Vec<Vec<u8>>),
    // Ping,
    // Pong,
}

async fn verify_handshake(
    msg: &Message,
    socket_addr: SocketAddr,
    tx: &mpsc::Sender<Message>,
    shared_peers: &SharedPeers,
) -> Option<()> {
    if let Message::Hello { peer_id, public_key } = msg {
        // Decode base64 public key (optional step)
        let pk_bytes = general_purpose::STANDARD
            .decode(public_key)
            .ok()
            .filter(|b| b.len() == 32)?;
        let mut key = [0u8; 32];
        key.copy_from_slice(&pk_bytes);

        let peer = Peer {
            peer_id: PeerId { id: peer_id.clone() },
            address: socket_addr,
            public_key: key,
            sender: Some(tx.clone()),
        };

        let mut peers = shared_peers.lock().await;
        peers.insert(peer.peer_id.clone(), peer);

        println!("Registered peer '{}' from {}", peer_id, socket_addr);
        Some(())
    } else {
        None
    }
}

pub async fn connect_to_peer(
    target_ip: String,
    local_peer_id: PeerId,
    public_key: [u8; 32],
    shared_peers: SharedPeers,
    crdt: Arc<Mutex<CrdtToDoList>>,
    sync_state: Arc<Mutex<SyncState>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let port = 58008;
    let addr: SocketAddr = format!("{}:{}", target_ip, port).parse()?;
    println!("Connecting to {}", addr);

    let stream = TcpStream::connect(addr).await?;
    let (mut reader, mut writer) = stream.into_split();
    let (tx, mut rx) = mpsc::channel::<Message>(100);

    // Send a simple Hello
    let hello = Message::Hello {
        peer_id: local_peer_id.id.clone(),
        public_key: general_purpose::STANDARD.encode(public_key),
    };
    let mut buf = serde_json::to_vec(&hello)?;
    buf.push(b'\n');
    writer.write_all(&buf).await?;
    writer.flush().await?;

    // Register ourselves locally
    {
        let mut peers = shared_peers.lock().await;
        peers.insert(local_peer_id.clone(), Peer {
            peer_id: local_peer_id.clone(),
            address: addr,
            public_key,
            sender: Some(tx.clone()),
        });
    }

    // Task to send outgoing messages
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let mut out = serde_json::to_vec(&msg).unwrap();
            out.push(b'\n');
            if writer.write_all(&out).await.is_err() { break; }
        }
    });

    // Send periodic Pings
    // {
    //     let ping_tx = tx.clone();
    //     tokio::spawn(async move {
    //         let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
    //         loop {
    //             interval.tick().await;
    //             if ping_tx.send(Message::Ping).await.is_err() { break; }
    //         }
    //     });
    // }

    // Read & handle incoming messages
    let mut buffer = vec![0; 4096];
    let mut acc = Vec::new();
    loop {
        let n = reader.read(&mut buffer).await?;
        if n == 0 {
            println!("Disconnected from {}", addr);
            break;
        }
        acc.extend_from_slice(&buffer[..n]);
        while let Some(pos) = acc.iter().position(|&b| b == b'\n') {
            let line = acc.drain(..pos).collect::<Vec<_>>();
            acc.drain(..1); // remove newline
            if let Ok(text) = str::from_utf8(&line) {
                if let Ok(msg) = serde_json::from_str::<Message>(text) {
                    match msg {
                        Message::Hello { .. } => {
                            if verify_handshake(&msg, addr, &tx, &shared_peers).await.is_some() {
                                let mut crdt = crdt.lock().await;
                                let mut st   = sync_state.lock().await;
                                crdt.send_changes(&mut st, &shared_peers).await;
                            }
                        }
                        Message::Changes(chs) => {
                            let mut crdt = crdt.lock().await;
                            let mut st   = sync_state.lock().await;
                            crdt.apply_changes_from_bytes(chs, &mut st).await;
                        }
                        // Message::Ping => {
                        //     let _ = tx.send(Message::Pong).await;
                        // }
                        // Message::Pong => {
                        //     println!("Pong from {}", addr);
                        // }
                    }
                }
            }
        }
    }

    // on disconnect, remove peer
    let mut peers = shared_peers.lock().await;
    peers.retain(|_, p| p.address != addr);
    println!("Unregistered peer {}", addr);
    Ok(())
}

pub async fn connections(
    shared_peers: SharedPeers,
    crdt: Arc<Mutex<CrdtToDoList>>,
    sync_state: Arc<Mutex<SyncState>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("0.0.0.0:58008").await?;
    loop {
        let (socket, addr) = listener.accept().await?;
        let sp = shared_peers.clone();
        let cd = crdt.clone();
        let ss = sync_state.clone();
        tokio::spawn(async move {
            handle_connection(socket, addr, sp, cd, ss).await;
        });
    }
}

async fn handle_connection(
    stream: TcpStream,
    addr: SocketAddr,
    shared_peers: SharedPeers,
    crdt: Arc<Mutex<CrdtToDoList>>,
    sync_state: Arc<Mutex<SyncState>>,
) {
    // identical to the read/write loop in connect_to_peer,
    // minus the initial "send Hello". Just call verify_handshake
    // on the first Hello you receive.
    use tokio::io::AsyncReadExt;
    use tokio::io::AsyncWriteExt;

    let (mut reader, mut writer) = stream.into_split();
    let (tx, mut rx) = mpsc::channel::<Message>(100);

    // spawn writer task
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let mut out = serde_json::to_vec(&msg).unwrap();
            out.push(b'\n');
            if writer.write_all(&out).await.is_err() { break; }
        }
    });

    // spawn ping task
    // {
    //     let ping_tx = tx.clone();
    //     tokio::spawn(async move {
    //         let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
    //         loop {
    //             interval.tick().await;
    //             if ping_tx.send(Message::Ping).await.is_err() { break; }
    //         }
    //     });
    // }

    // reader loop (same as above)
    let mut buffer = vec![0; 4096];
    let mut acc = Vec::new();
    loop {
        match reader.read(&mut buffer).await {
            Ok(0) => break,
            Ok(n) => {
                acc.extend_from_slice(&buffer[..n]);
                while let Some(pos) = acc.iter().position(|&b| b == b'\n') {
                    let line = acc.drain(..pos).collect::<Vec<_>>();
                    acc.drain(..1);
                    if let Ok(text) = str::from_utf8(&line) {
                        if let Ok(msg) = serde_json::from_str::<Message>(text) {
                            match msg {
                                Message::Hello { .. } => {
                                    if verify_handshake(&msg, addr, &tx, &shared_peers).await.is_some() {
                                        let mut crdt = crdt.lock().await;
                                        let mut st   = sync_state.lock().await;
                                        crdt.send_changes(&mut st, &shared_peers).await;
                                    }
                                }
                                Message::Changes(chs) => {
                                    let mut crdt = crdt.lock().await;
                                    let mut st   = sync_state.lock().await;
                                    crdt.apply_changes_from_bytes(chs, &mut st).await;
                                }
                                // Message::Ping => {
                                //     let _ = tx.send(Message::Pong).await;
                                // }
                                // Message::Pong => {}
                            }
                        }
                    }
                }
            }
            Err(_) => break,
        }
    }

    // cleanup
    let mut peers = shared_peers.lock().await;
    peers.retain(|_, p| p.address != addr);
    println!("Unregistered peer {}", addr);
}