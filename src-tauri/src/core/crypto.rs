use anyhow::Result;
use chacha20poly1305::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    ChaCha20Poly1305, Key, Nonce,
};
use hkdf::Hkdf;
use sha2::Sha256;
use x25519_dalek::{EphemeralSecret, PublicKey, SharedSecret};

pub const NONCE_SIZE: usize = 12;
pub const TAG_SIZE: usize = 16;

/// Ephemeral keypair for X25519 ECDH
pub struct EphemeralKeypair {
    secret: EphemeralSecret,
    pub public: PublicKey,
}

impl EphemeralKeypair {
    pub fn generate() -> Self {
        let secret = EphemeralSecret::random_from_rng(OsRng);
        let public = PublicKey::from(&secret);
        Self { secret, public }
    }

    pub fn diffie_hellman(self, their_public: &PublicKey) -> SharedSecret {
        self.secret.diffie_hellman(their_public)
    }
}

/// Derive a 32-byte session key from the X25519 shared secret using HKDF-SHA256
pub fn derive_session_key(shared_secret: &SharedSecret, session_code: &str) -> [u8; 32] {
    let hk = Hkdf::<Sha256>::new(Some(session_code.as_bytes()), shared_secret.as_bytes());
    let mut okm = [0u8; 32];
    hk.expand(b"inputsync-v1-session-key", &mut okm)
        .expect("HKDF expand failed");
    okm
}

/// Session cipher - wraps ChaCha20-Poly1305 with a session key
pub struct SessionCipher {
    cipher: ChaCha20Poly1305,
    // Base nonce XOR'd with packet counter to get per-packet nonce
    base_nonce: [u8; NONCE_SIZE],
}

impl SessionCipher {
    pub fn new(key_bytes: &[u8; 32], base_nonce: [u8; NONCE_SIZE]) -> Self {
        let key = Key::from_slice(key_bytes);
        Self {
            cipher: ChaCha20Poly1305::new(key),
            base_nonce,
        }
    }

    /// Generate a random base nonce to be exchanged during handshake
    pub fn generate_base_nonce() -> [u8; NONCE_SIZE] {
        ChaCha20Poly1305::generate_nonce(&mut OsRng).into()
    }

    /// Compute per-packet nonce: base_nonce XOR little-endian counter
    pub fn packet_nonce(&self, counter: u64) -> Nonce {
        let mut nonce = self.base_nonce;
        let counter_bytes = counter.to_le_bytes();
        for i in 0..8 {
            nonce[i] ^= counter_bytes[i];
        }
        Nonce::from(nonce)
    }

    pub fn encrypt(&self, plaintext: &[u8], counter: u64) -> Result<Vec<u8>> {
        let nonce = self.packet_nonce(counter);
        self.cipher
            .encrypt(&nonce, plaintext)
            .map_err(|e| anyhow::anyhow!("Encrypt failed: {:?}", e))
    }

    pub fn decrypt(&self, ciphertext: &[u8], counter: u64) -> Result<Vec<u8>> {
        let nonce = self.packet_nonce(counter);
        self.cipher
            .decrypt(&nonce, ciphertext)
            .map_err(|e| anyhow::anyhow!("Decrypt failed: {:?}", e))
    }
}

/// Combine two base nonces (server + client) by XOR for shared nonce
pub fn combine_nonces(a: &[u8; NONCE_SIZE], b: &[u8; NONCE_SIZE]) -> [u8; NONCE_SIZE] {
    let mut result = [0u8; NONCE_SIZE];
    for i in 0..NONCE_SIZE {
        result[i] = a[i] ^ b[i];
    }
    result
}
