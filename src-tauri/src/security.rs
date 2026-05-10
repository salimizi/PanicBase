//! Consentement, détection de fuite de données sensibles avant upload, garde-fous futurs.

/// Données explicitement interdites côté upload (rappel produit / doc).
pub const FORBIDDEN_MARKERS: &[&str] = &[
    "IMEI",
    "UDID",
    "Serial Number",
    "Apple ID",
    "device name",
];
