use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use automerge::sync::Message;

pub struct Peer {
    peer_id: PeerId,
    address: SocketAddr,
    public_key: [u8; 32],
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct PeerId {
   id: String,
}

type SharedPeers = Arc<Mutex<HashMap<PeerId, Peer>>>;
type SharedPeerSenders = Arc<Mutex<HashMap<PeerId, tokio::sync::mpsc::Sender<Message>>>>;