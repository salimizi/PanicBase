//! Statistiques « base communautaire » — implémentation réseau au MVP 0.3 uniquement après consentement explicite.

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct RepairBucket {
    pub label: String,
    pub count: u32,
    pub percent: u8,
}

#[derive(Debug, Serialize)]
pub struct CommunityStats {
    pub available: bool,
    pub similar_count: u64,
    pub message: String,
    pub buckets: Vec<RepairBucket>,
}

pub fn get_community_stats(_signature_hash: &str, _model: Option<&str>) -> CommunityStats {
    CommunityStats {
        available: false,
        similar_count: 0,
        message: "Base communautaire absente en V0.1 — aucune requête réseau. Connexion opt-in prévue au MVP 0.3."
            .to_string(),
        buckets: vec![],
    }
}
