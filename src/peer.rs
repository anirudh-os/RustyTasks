use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use automerge::sync::Message;

pub struct Peer {
    pub peer_id: PeerId,
    pub address: SocketAddr,
    pub public_key: [u8; 32],
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PeerId {
   pub id: String,
}

pub type SharedPeers = Arc<Mutex<HashMap<PeerId, Peer>>>;
type SharedPeerSenders = Arc<Mutex<HashMap<PeerId, tokio::sync::mpsc::Sender<Message>>>>;