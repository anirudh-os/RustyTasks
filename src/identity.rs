use ed25519_dalek::{SigningKey, PUBLIC_KEY_LENGTH, SECRET_KEY_LENGTH};
use rand::rngs::OsRng;
use rand::RngCore;

#[derive(Debug)]
pub struct Identity {
    pub public_key: [u8; PUBLIC_KEY_LENGTH],
    pub private_key: [u8; SECRET_KEY_LENGTH],
}

impl Identity {
    pub fn generate() -> Self {
        let mut csprng = OsRng;
        let mut secret_key = [0u8; SECRET_KEY_LENGTH];
        csprng.fill_bytes(&mut secret_key);

        let signing_key = SigningKey::from_bytes(&secret_key);

        let public_key = signing_key.verifying_key().to_bytes();

        Identity {
            public_key,
            private_key: secret_key,
        }
    }

    pub fn get_public_key(&self) -> &[u8; PUBLIC_KEY_LENGTH] {
        &self.public_key
    }

    pub fn get_private_key(&self) -> &[u8; SECRET_KEY_LENGTH] {
        &self.private_key
    }
}