//! Client HTTP vers l’API privée PanicBase (HTTPS, rate limit) — **non branché** avant MVP 0.3.

/// URL de base réservée ; aucun appel réel tant que le module n’est pas activé côté produit.
pub const API_BASE_PLACEHOLDER: &str = "https://api.panicbase.local/v1";
