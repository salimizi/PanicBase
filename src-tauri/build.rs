use chacha20poly1305::{aead::{Aead, KeyInit}, ChaCha20Poly1305, Nonce};
use std::{env, fs, io::Write, path::Path};

fn load_key() -> [u8; 32] {
    let hex = env::var("PANICBASE_KB_KEY")
        .expect("PANICBASE_KB_KEY not set — provide a 64-char hex key (32 bytes)");
    assert_eq!(hex.len(), 64, "PANICBASE_KB_KEY must be 64 hex chars");
    let mut k = [0u8; 32];
    for (i, chunk) in hex.as_bytes().chunks(2).enumerate() {
        let s = std::str::from_utf8(chunk).unwrap();
        k[i] = u8::from_str_radix(s, 16).expect("PANICBASE_KB_KEY: invalid hex char");
    }
    k
}

fn main() {
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR");
    let kb_path = Path::new("src").join("kb.json");
    println!("cargo:rerun-if-changed={}", kb_path.display());
    println!("cargo:rerun-if-env-changed=PANICBASE_KB_KEY");

    let key = load_key();

    let key_src = format!("pub(crate) const KB_K: [u8; 32] = {:?};", key);
    fs::write(Path::new(&out_dir).join("kb_key.rs"), key_src).expect("write kb_key.rs");

    let raw = fs::read(&kb_path)
        .unwrap_or_else(|e| panic!("KB not found ({}): {e}", kb_path.display()));

    let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
    enc.write_all(&raw).expect("compress KB");
    let compressed = enc.finish().expect("gzip KB");

    let cipher = ChaCha20Poly1305::new((&key).into());
    let nonce = Nonce::from_slice(&[0u8; 12]);
    let ct = cipher.encrypt(nonce, compressed.as_slice()).expect("seal KB");

    fs::write(Path::new(&out_dir).join("kb.sealed"), &ct).expect("write kb.sealed");

    tauri_build::build();
}
