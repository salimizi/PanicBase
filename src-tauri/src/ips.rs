use std::sync::OnceLock;

use regex::Regex;
use serde::Serialize;
use serde_json::{Map, Value};

use crate::analyzer::{analyze_panic_log, AnalysisResult};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IpsInterpretOutcome {
    pub analysis: AnalysisResult,
    pub panic_text: String,
    pub extraction_method: String,
    pub device_hint: Option<String>,
}

fn plist_panic_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?is)<key>\s*panicString\s*</key>\s*(?:<string>|!\[CDATA\[)(.*?)(?:\]\]>|</string>)")
            .expect("plist panic regex")
    })
}

pub fn ips_is_binary_plist(raw: &str) -> bool {
    raw.as_bytes().starts_with(b"bplist")
}

fn guess_device_hint(obj: &Map<String, Value>) -> Option<String> {
    for key in ["product", "hardwareModel", "model", "machine_name", "build"] {
        if let Some(Value::String(s)) = obj.get(key) {
            let t = s.trim();
            if !t.is_empty() {
                return Some(t.to_string());
            }
        }
    }
    None
}

fn find_panic_string_json(val: &Value) -> Option<String> {
    match val {
        Value::Object(map) => {
            for name in ["panicString", "panic_string"] {
                if let Some(Value::String(s)) = map.get(name) {
                    if !s.trim().is_empty() {
                        return Some(s.clone());
                    }
                }
            }
            for v in map.values() {
                if let Some(s) = find_panic_string_json(v) {
                    return Some(s);
                }
            }
            None
        }
        Value::Array(items) => {
            for v in items {
                if let Some(s) = find_panic_string_json(v) {
                    return Some(s);
                }
            }
            None
        }
        _ => None,
    }
}

fn find_device_hint_json(val: &Value) -> Option<String> {
    if let Value::Object(map) = val {
        if let Some(h) = guess_device_hint(map) {
            return Some(h);
        }
        for v in map.values() {
            if let Some(h) = find_device_hint_json(v) {
                return Some(h);
            }
        }
    } else if let Value::Array(items) = val {
        for v in items {
            if let Some(h) = find_device_hint_json(v) {
                return Some(h);
            }
        }
    }
    None
}

fn extract_from_json(trimmed: &str) -> Option<(String, Option<String>, &'static str)> {
    let val: Value = serde_json::from_str(trimmed).ok()?;
    let device = find_device_hint_json(&val);
    let panic = find_panic_string_json(&val)?;
    Some((panic, device, "json"))
}

fn extract_from_plist_xml(raw: &str) -> Option<(String, Option<String>, &'static str)> {
    let caps = plist_panic_regex().captures(raw)?;
    let body = caps.get(1)?.as_str().trim();
    if body.is_empty() {
        return None;
    }
    Some((body.to_string(), None, "plist-xml"))
}

/// Extrait le texte de panic utile pour l’analyseur, à partir d’un fichier .ips (JSON, plist XML ou texte brut).
pub fn extract_ips_body(raw: &str) -> (String, Option<String>, &'static str) {
    let trimmed = raw.trim_start_matches('\u{feff}').trim();

    if let Some((text, dev, method)) = extract_from_json(trimmed) {
        return (text, dev, method);
    }

    if let Some((text, dev, method)) = extract_from_plist_xml(trimmed) {
        return (text, dev, method);
    }

    let lower = trimmed.to_lowercase();
    if lower.contains("panicstring") || trimmed.contains("panic(cpu") {
        return (trimmed.to_string(), None, "plaintext");
    }

    (trimmed.to_string(), None, "raw")
}

pub fn interpret_ips_file(raw: &str) -> Result<IpsInterpretOutcome, String> {
    if ips_is_binary_plist(raw) {
        return Err(
            "Ce fichier est un binaire plist (bplist), non lisible ici. Sur Mac, ouvre le .ips dans \
             Console et exporte / copie le texte, ou utilise un .ips en JSON / XML issu d’un export texte."
                .to_string(),
        );
    }

    let raw_trimmed = raw.trim_start_matches('\u{feff}').trim();
    let (panic_text, device_hint, method) = extract_ips_body(raw);
    let analysis = analyze_panic_log(&panic_text, device_hint.as_deref(), Some(raw_trimmed));

    Ok(IpsInterpretOutcome {
        analysis,
        panic_text,
        extraction_method: method.to_string(),
        device_hint,
    })
}
