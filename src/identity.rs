use rand::rngs::OsRng;
use sha2::{Digest, Sha256};
use base64::engine::general_purpose;
use base64::Engine;
use ed25519_dalek::{SigningKey, PUBLIC_KEY_LENGTH, SECRET_KEY_LENGTH};
use crate::peer::PeerId;

#[derive(Debug)]
pub struct Identity {
    pub public_key: [u8; PUBLIC_KEY_LENGTH],
    pub private_key: [u8; SECRET_KEY_LENGTH],
}

impl Identity {
    pub fn generate() -> Self {
        let signing_key = SigningKey::generate(&mut OsRng);
        let public_key = signing_key.verifying_key().to_bytes();
        let private_key = signing_key.to_bytes();
        Identity { public_key, private_key }
    }

    pub fn derive_peer_id(&self) -> PeerId {
        let mut hasher = Sha256::new();
        hasher.update(&self.public_key);
        let hash = hasher.finalize();
        let short_hash = &hash[..16];
        let encoded = general_purpose::URL_SAFE_NO_PAD.encode(short_hash);
        PeerId { id: format!("peer_{}", encoded) }
    }
}