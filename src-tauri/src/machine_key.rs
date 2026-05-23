use base64::{engine::general_purpose::STANDARD as B64, Engine};
use chacha20poly1305::aead::rand_core::RngCore;
use chacha20poly1305::aead::OsRng;

const SERVICE: &str = "com.panicbase.app";
const ACCOUNT: &str = "db_master_key_v1";

pub fn machine_storage_key() -> [u8; 32] {
    static KEY: std::sync::OnceLock<[u8; 32]> = std::sync::OnceLock::new();
    *KEY.get_or_init(load_or_create_machine_key)
}

fn load_or_create_machine_key() -> [u8; 32] {
    let entry = match keyring::Entry::new(SERVICE, ACCOUNT) {
        Ok(e) => e,
        Err(_) => return random_key(),
    };
    match entry.get_password() {
        Ok(stored) => decode_key(&stored).unwrap_or_else(|_| {
            let fresh = random_key();
            let _ = entry.set_password(&encode_key(&fresh));
            fresh
        }),
        Err(_) => {
            let fresh = random_key();
            let _ = entry.set_password(&encode_key(&fresh));
            fresh
        }
    }
}

fn random_key() -> [u8; 32] {
    let mut k = [0u8; 32];
    OsRng.fill_bytes(&mut k);
    k
}

fn encode_key(key: &[u8; 32]) -> String {
    B64.encode(key)
}

fn decode_key(s: &str) -> Result<[u8; 32], ()> {
    let bytes = B64.decode(s.trim()).map_err(|_| ())?;
    if bytes.len() != 32 {
        return Err(());
    }
    let mut k = [0u8; 32];
    k.copy_from_slice(&bytes);
    Ok(k)
}
