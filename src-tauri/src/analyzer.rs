use serde::{Deserialize, Serialize};

use crate::iphone;
use crate::panic_diagnostic::{diagnose_structured, StructuredDiagnostic};
use crate::reference_focus::{infer_panic_reference_focus, PanicReferenceFocus};
use crate::signature::{build_signature, hash_signature};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AnalysisResult {
    pub device_model: String,
    pub detected: bool,
    pub panic_type: String,
    pub probable_cause: String,
    pub confidence: u8,
    pub keywords: Vec<String>,
    pub explanation: String,
    pub signature: String,
    pub signature_hash: String,
    pub structured_diagnostic: StructuredDiagnostic,
    #[serde(default)]
    pub reference_focus: PanicReferenceFocus,
}

fn is_apple_internal_product_token(dev: &str) -> bool {
    let t = dev.trim();
    let lc = t.to_ascii_lowercase();
    (lc.starts_with("iphone") || lc.starts_with("ipad")) && lc.contains(',')
}

pub fn analyze_panic_log(
    log: &str,
    device_model_hint: Option<&str>,
    ips_envelope: Option<&str>,
) -> AnalysisResult {
    let structured = diagnose_structured(log, device_model_hint, ips_envelope);

    let device_display = match (
        structured.marketing_name.as_deref(),
        structured.device.trim(),
    ) {
        (Some(m), dev)
            if !dev.is_empty()
                && !dev.eq_ignore_ascii_case("unknown")
                && is_apple_internal_product_token(dev) =>
        {
            m.to_string()
        }
        (Some(m), dev) if !dev.is_empty() && !dev.eq_ignore_ascii_case("unknown") => {
            if dev.trim().eq_ignore_ascii_case(m.trim()) {
                m.to_string()
            } else {
                format!("{m} Â· {dev}")
            }
        }
        (Some(m), _) => m.to_string(),
        (None, dev) if !dev.is_empty() && !dev.eq_ignore_ascii_case("unknown") => dev.to_string(),
        _ => {
            if let Some(h) = device_model_hint.map(str::trim).filter(|s| !s.is_empty()) {
                h.to_string()
            } else if let Some(m) = iphone::marketing_display_for_hints(device_model_hint) {
                m
            } else {
                "Non renseignÃ©".to_string()
            }
        }
    };

    let mut device_sig = structured.device.trim();
    if device_sig.is_empty() || device_sig.eq_ignore_ascii_case("unknown") {
        device_sig = device_model_hint.unwrap_or("unknown_model").trim();
    }
    let device_sig = if device_sig.is_empty() {
        "unknown_model"
    } else {
        device_sig
    };

    let panic_type = structured.panic_type.clone();
    let keywords = structured.normalized_signatures.clone();

    let signature = build_signature(device_sig, &panic_type, &keywords);
    let signature_hash = hash_signature(&signature);

    let probable_cause = structured
        .possible_causes
        .first()
        .map(|p| p.name.clone())
        .unwrap_or_else(|| "Non classifiÃ© (signatures insuffisantes)".into());

    let confidence = (structured.confidence_global * 100.0)
        .round()
        .clamp(0.0, 99.0) as u8;

    let detected = confidence > 0 || !structured.normalized_signatures.is_empty();

    let mut result = AnalysisResult {
        device_model: device_display,
        detected,
        panic_type,
        probable_cause,
        confidence,
        keywords,
        explanation: String::new(),
        signature,
        signature_hash,
        structured_diagnostic: structured,
        reference_focus: PanicReferenceFocus {
            nav_section: "iphone-x".into(),
            confidence: 0.0,
            initial_search: String::new(),
        },
    };
    result.reference_focus =
        infer_panic_reference_focus(log, &result, device_model_hint);
    result
}
