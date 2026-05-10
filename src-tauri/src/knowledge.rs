//! Base « iphone_paniclog_knowledgebase » embarquée (`iphone_panic_kb.json`).

use std::collections::HashMap;
use std::sync::OnceLock;

use regex::Regex;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct KbFile {
    #[serde(rename = "iphone_paniclog_knowledgebase")]
    inner: KbInner,
}

#[derive(Debug, Deserialize)]
struct KbInner {
    common_panic_patterns: Vec<Pattern>,
    model_specific_failures: HashMap<String, Vec<String>>,
    bug_type_mapping: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct Pattern {
    signature: String,
    causes: Vec<String>,
    confidence: f64,
    models: Option<Vec<String>>,
}

fn inner() -> &'static KbInner {
    static CELL: OnceLock<KbInner> = OnceLock::new();
    CELL.get_or_init(|| {
        let f: KbFile =
            serde_json::from_str(include_str!("iphone_panic_kb.json")).expect("KB JSON invalide.");
        f.inner
    })
}

fn confidence_to_u8(c: f64) -> u8 {
    let v = (c * 100.0).round().clamp(1.0, 99.0) as u8;
    v
}

fn marketing_matches_key(marketing: &str, key: &str) -> bool {
    let m = marketing.trim();
    let k = key.trim();
    if m.eq_ignore_ascii_case(k) {
        return true;
    }
    let pref = format!("{} ", k);
    m.len() >= pref.len()
        && m
            .get(..pref.len())
            .map(|p| p.eq_ignore_ascii_case(&pref))
            .unwrap_or(false)
}

fn log_contains_signature(log_lc: &str, sig: &str) -> bool {
    let s = sig.to_lowercase();
    if s.contains("missing sensor") || s == "missing sensor" {
        return log_lc.contains("missing sensor");
    }
    log_lc.contains(&s)
}

fn model_allowed(pat: &Pattern, marketing: Option<&str>) -> bool {
    match (&pat.models, marketing) {
        (None, _) => true,
        (Some(models), Some(mkt)) => {
            let ml = mkt.to_lowercase();
            models.iter().any(|mo| ml.contains(&mo.to_lowercase()))
        }
        (Some(_), None) => true,
    }
}

fn bug_type_line(log: &str, mapping: &HashMap<String, String>) -> Option<String> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        Regex::new(r#"(?i)bug[_\s-]*type["'\s:=]+(\d{2,})"#).expect("regex bug_type")
    });
    let cap = re.captures(log)?;
    let code = cap.get(1)?.as_str().to_string();
    let label = mapping.get(&code)?.clone();
    Some(format!("Type bug système signalé : {code} ({label})."))
}

fn model_failures_addon(marketing: Option<&str>) -> Option<String> {
    let m = marketing?;
    let kb_map = &inner().model_specific_failures;
    let mut keys: Vec<&str> = kb_map.keys().map(|s| s.as_str()).collect();
    keys.sort_by(|a, b| b.len().cmp(&a.len()));
    for key in keys {
        if marketing_matches_key(m, key) {
            let joined = kb_map[key].join(", ");
            return Some(format!(" Sur {key}, failles souvent citées : {joined}."));
        }
    }
    None
}

#[derive(Debug, Clone)]
pub struct KbMatch {
    pub probable_cause: String,
    pub confidence: u8,
    pub explanation: String,
    pub matched_signature: String,
}

/// Trouve le meilleur motif KB (priorité aux signatures les plus longues, puis aux confidences).
pub fn match_panic_kb(log: &str, marketing: Option<&str>) -> Option<KbMatch> {
    let kb = inner();
    let log_lc = log.to_lowercase();
    let mut hits: Vec<(&Pattern, usize)> = kb
        .common_panic_patterns
        .iter()
        .filter(|p| !p.signature.is_empty() && !p.causes.is_empty())
        .filter(|p| log_contains_signature(&log_lc, &p.signature))
        .filter(|p| model_allowed(p, marketing))
        .map(|p| (p, p.signature.len()))
        .collect();

    let bug_txt = bug_type_line(log, &kb.bug_type_mapping);
    let model_txt = model_failures_addon(marketing);

    if hits.is_empty() {
        let mut tail = Vec::new();
        if let Some(ref b) = bug_txt {
            tail.push(b.clone());
        }
        if let Some(ref m) = model_txt {
            tail.push(m.clone());
        }
        if tail.is_empty() {
            return None;
        }
        return Some(KbMatch {
            probable_cause: "Voir détails système".to_string(),
            confidence: 42,
            explanation: tail.join(" "),
            matched_signature: "bug_type_or_model_hints".into(),
        });
    }

    hits.sort_by(|a, b| {
        let len_ord = b.1.cmp(&a.1);
        len_ord.then_with(|| {
            b.0.confidence
                .partial_cmp(&a.0.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    });

    let pat = hits[0].0;
    let probable = pat.causes[0].clone();
    let conf = confidence_to_u8(pat.confidence);

    let mut expl = format!(
        "Base PanicBase — motif « {} » : {}.",
        pat.signature,
        pat.causes.join(", ")
    );
    if let Some(s) = model_txt {
        expl.push_str(&s);
    }
    if let Some(b) = bug_txt {
        expl.push_str(" ");
        expl.push_str(&b);
    }

    Some(KbMatch {
        probable_cause: probable,
        confidence: conf,
        explanation: expl,
        matched_signature: pat.signature.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iphone6_not_matches_iphone16_keys() {
        assert!(!marketing_matches_key("iPhone 16 Pro", "iPhone 6"));
        assert!(marketing_matches_key("iPhone 6 Plus", "iPhone 6"));
    }
}
