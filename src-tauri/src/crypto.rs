use base64::{engine::general_purpose::STANDARD as B64, Engine};
use chacha20poly1305::{
    aead::{Aead, KeyInit, OsRng},
    ChaCha20Poly1305, Nonce,
};

use crate::machine_key;

fn storage_cipher() -> ChaCha20Poly1305 {
    ChaCha20Poly1305::new((&machine_key::machine_storage_key()).into())
}

pub fn encrypt(plaintext: &str) -> String {
    let cipher = storage_cipher();
    let nonce_bytes: [u8; 12] = {
        use chacha20poly1305::aead::rand_core::RngCore;
        let mut n = [0u8; 12];
        OsRng.fill_bytes(&mut n);
        n
    };
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ct = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .expect("encrypt failed");
    let mut combined = nonce_bytes.to_vec();
    combined.extend_from_slice(&ct);
    B64.encode(&combined)
}

pub fn decrypt(stored: &str) -> String {
    let bytes = match B64.decode(stored) {
        Ok(b) => b,
        Err(_) => return stored.to_owned(),
    };
    if bytes.len() < 13 {
        return stored.to_owned();
    }
    let (nonce_bytes, ct) = bytes.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);
    if let Ok(plain) = storage_cipher().decrypt(nonce, ct) {
        return String::from_utf8(plain).unwrap_or_else(|_| stored.to_owned());
    }
    stored.to_owned()
}
