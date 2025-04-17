use std::net::SocketAddr;
use tokio::sync::mpsc::Sender;
use crate::network::Message;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PeerId {
    pub id: String,
}

pub struct Peer {
    pub peer_id: PeerId,
    pub address: SocketAddr,
    pub public_key: [u8; 32],
    pub sender: Option<Sender<Message>>,
}

pub type SharedPeers = Arc<Mutex<HashMap<PeerId, Peer>>>;