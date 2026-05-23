use std::io::Read;
use std::sync::OnceLock;
use chacha20poly1305::{aead::{Aead, KeyInit}, ChaCha20Poly1305, Nonce};

include!(concat!(env!("OUT_DIR"), "/kb_key.rs"));

pub fn kb_json_plaintext() -> &'static str {
    static PLAIN: OnceLock<String> = OnceLock::new();
    PLAIN.get_or_init(|| {
        let sealed = include_bytes!(concat!(env!("OUT_DIR"), "/kb.sealed"));
        let cipher = ChaCha20Poly1305::new((&KB_K).into());
        let nonce = Nonce::from_slice(&[0u8; 12]);
        let compressed = cipher
            .decrypt(nonce, sealed.as_ref())
            .expect("kb decrypt failed");
        let mut dec = flate2::read::GzDecoder::new(compressed.as_slice());
        let mut out = String::new();
        dec.read_to_string(&mut out).expect("kb decompress failed");
        out
    })
}
