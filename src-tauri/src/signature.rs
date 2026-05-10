use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::panic_parser;

#[derive(Debug, Serialize)]
pub struct SignatureBundle {
    pub signature: String,
    pub signature_hash: String,
    pub panic_type: String,
    pub keywords: Vec<String>,
}

pub fn build_signature(device_model: &str, panic_type: &str, keywords: &[String]) -> String {
    let mut sorted_keywords = keywords.to_vec();
    sorted_keywords.sort();
    sorted_keywords.dedup();
    format!("{}|{}|{}", device_model, panic_type, sorted_keywords.join("|"))
}

pub fn hash_signature(signature: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(signature.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Recalcule signature + SHA256 hors pipeline d’analyse (commande dédiée).
pub fn bundle_from_log(log: &str, device_model_arg: String) -> SignatureBundle {
    let parsed = panic_parser::parse_panic_log(log);
    let m = device_model_arg.trim();
    let model = if m.is_empty() { "unknown_model" } else { m };
    let signature = build_signature(model, &parsed.panic_type, &parsed.keywords);
    let signature_hash = hash_signature(&signature);
    SignatureBundle {
        signature,
        signature_hash,
        panic_type: parsed.panic_type,
        keywords: parsed.keywords,
    }
}
