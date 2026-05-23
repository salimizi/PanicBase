
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
    #[serde(default)]
    source_registry: Option<HashMap<String, SourceEntry>>,
    #[serde(default)]
    model_panic_rules: Option<Vec<ModelPanicRule>>,
}

#[derive(Debug, Deserialize)]
struct ModelPanicRule {
    models: Vec<String>,
    #[serde(rename = "match")]
    match_spec: ModelPanicMatch,
    likely_component: String,
    confidence: f64,
    #[serde(default)]
    repair_order: Option<Vec<String>>,
    #[serde(default)]
    source_ids: Option<Vec<String>>,
    #[serde(default)]
    evidence_level: Option<String>,
    #[serde(default)]
    note: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct ModelPanicMatch {
    #[serde(default)]
    panic_contains: Option<Vec<String>>,
    #[serde(default)]
    missing_sensor_any: Option<Vec<String>>,
    #[serde(default)]
    smc_sensor_array_any: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct SourceEntry {
    title: String,
    url: String,
    #[serde(default)]
    quality: Option<String>,
    #[serde(default)]
    notes: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Pattern {
    signature: String,
    causes: Vec<String>,
    confidence: f64,
    #[serde(default, alias = "models_affected")]
    models: Option<Vec<String>>,
    #[serde(default)]
    reliability_note: Option<String>,
    #[serde(default)]
    evidence_level: Option<String>,
    #[serde(default)]
    source_ids: Option<Vec<String>>,
    #[serde(default)]
    require_all_substrings: Option<Vec<String>>,
    #[serde(default)]
    require_any_substrings: Option<Vec<String>>,
    #[serde(default)]
    symptoms: Option<Vec<String>>,
    #[serde(default)]
    repair_priority: Option<Vec<String>>,
    #[serde(default)]
    aftermarket_part_risk: Option<bool>,
    #[serde(default)]
    false_positive_common: Option<String>,
    #[serde(default)]
    requires_known_good_test: Option<bool>,
    #[serde(default)]
    confirmed_by_repair: Option<bool>,
    #[serde(default, alias = "workshop_note")]
    repair_notes: Option<String>,
    #[serde(default, rename = "source")]
    kb_source_line: Option<String>,
}

fn inner() -> &'static KbInner {
    static CELL: OnceLock<KbInner> = OnceLock::new();
    CELL.get_or_init(|| {
        let f: KbFile =
            serde_json::from_str(crate::kb_seal::kb_json_plaintext()).expect("KB JSON invalide.");
        f.inner
    })
}

fn append_pattern_provenance(expl: &mut String, pat: &Pattern, reg: &Option<HashMap<String, SourceEntry>>) {
    if let Some(ref note) = pat.reliability_note {
        expl.push(' ');
        expl.push_str(note.trim());
    }
    if let Some(ref ev) = pat.evidence_level {
        expl.push_str(" â€” Niveau de preuve (KB) : ");
        expl.push_str(ev.trim());
    }
    let Some(map) = reg else {
        return;
    };
    let Some(ids) = pat.source_ids.as_ref() else {
        return;
    };
    if ids.is_empty() {
        return;
    }
    expl.push_str(" â€” Sources : ");
    let mut first = true;
    for id in ids.iter().take(8) {
        let Some(ent) = map.get(id) else {
            continue;
        };
        if !first {
            expl.push_str(" Â· ");
        }
        first = false;
        let q = ent.quality.as_deref().unwrap_or("?");
        expl.push_str(&format!("{} [{}] {}", ent.title, q, ent.url));
        if let Some(n) = ent.notes.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty() && s.len() < 200) {
            expl.push_str(&format!(" ({})", n));
        }
    }
}

fn append_pattern_repair_notes(expl: &mut String, pat: &Pattern) {
    if let Some(ref n) = pat.repair_notes {
        let t = n.trim();
        if !t.is_empty() {
            expl.push_str(" â€” Note atelier (KB) : ");
            expl.push_str(t);
        }
    }
    if let Some(ref s) = pat.kb_source_line {
        let t = s.trim();
        if !t.is_empty() {
            expl.push_str(" â€” Sources (KB texte) : ");
            expl.push_str(t);
        }
    }
}

fn append_pattern_clinical(expl: &mut String, pat: &Pattern) {
    if let Some(ref sxs) = pat.symptoms {
        if !sxs.is_empty() {
            expl.push_str(" â€” SymptÃ´mes KB : ");
            expl.push_str(&sxs.join(" ; "));
        }
    }
    if let Some(ref steps) = pat.repair_priority {
        if !steps.is_empty() {
            expl.push_str(" â€” Ordre atelier : ");
            expl.push_str(&steps.join(" â†’ "));
        }
    }
    if pat.aftermarket_part_risk == Some(true) {
        expl.push_str(" â€” Risque piÃ¨ce aftermarket : Ã©levÃ© (capteurs faux / array incomplet / panics erratiques).");
    }
    if pat.requires_known_good_test == Some(true) {
        expl.push_str(" â€” Requiert test avec flex OEM ou connu bon avant diagnostic plaque.");
    }
    if let Some(ref fp) = pat.false_positive_common {
        let t = fp.trim();
        if !t.is_empty() {
            expl.push_str(" â€” Faux positifs frÃ©quents : ");
            expl.push_str(t);
        }
    }
    if pat.confirmed_by_repair == Some(true) {
        expl.push_str(" â€” CorrÃ©lation confirmÃ©e atelier (niveau KB).");
    }
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

fn log_meets_all_substrings(log_lc: &str, parts: &[String]) -> bool {
    parts
        .iter()
        .all(|s| log_lc.contains(&s.to_lowercase()))
}

fn log_meets_any_substrings(log_lc: &str, parts: &[String]) -> bool {
    parts
        .iter()
        .any(|s| log_lc.contains(&s.to_lowercase()))
}

fn pattern_matches_log(log_lc: &str, pat: &Pattern) -> bool {
    if let Some(req) = pat.require_all_substrings.as_ref() {
        if !req.is_empty() && !log_meets_all_substrings(log_lc, req) {
            return false;
        }
    }
    if let Some(req) = pat.require_any_substrings.as_ref() {
        if !req.is_empty() && !log_meets_any_substrings(log_lc, req) {
            return false;
        }
    }
    let sig = pat.signature.trim();
    if sig.is_empty() {
        return pat
            .require_all_substrings
            .as_ref()
            .map(|v| !v.is_empty())
            .unwrap_or(false);
    }
    log_contains_signature(log_lc, sig)
}

fn pattern_sort_len(pat: &Pattern) -> usize {
    let mut len = pat.signature.chars().count();
    if pat
        .require_all_substrings
        .as_ref()
        .map(|v| !v.is_empty())
        .unwrap_or(false)
    {
        len = len.saturating_add(500);
    }
    len
}

fn matched_signature_label(pat: &Pattern) -> String {
    let sig = pat.signature.trim();
    if let Some(req) = pat.require_all_substrings.as_ref().filter(|v| !v.is_empty()) {
        if sig.is_empty() {
            format!("req:{}", req.join("+"))
        } else {
            format!("{sig}+{}", req.join("+"))
        }
    } else {
        pat.signature.clone()
    }
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
    Some(format!("Type bug systÃ¨me signalÃ© : {code} ({label})."))
}

fn sensor_token_in_log(log_lc: &str, token: &str) -> bool {
    let t = token.trim();
    if t.is_empty() {
        return false;
    }
    let tl = t.to_lowercase();
    if log_lc.contains(&tl) {
        return true;
    }
    if let Ok(n) = t.parse::<u64>() {
        let hex = format!("0x{n:x}");
        if log_lc.contains(&hex) {
            return true;
        }
        if log_lc.contains(t) {
            return true;
        }
    }
    false
}

fn model_panic_rule_applies(
    log_lc: &str,
    marketing: Option<&str>,
    rule: &ModelPanicRule,
) -> bool {
    let Some(mkt) = marketing else {
        return false;
    };
    let model_ok = rule.models.iter().any(|m| {
        let m = m.trim();
        m.eq_ignore_ascii_case("all") || marketing_matches_key(mkt, m)
    });
    if !model_ok {
        return false;
    }
    let spec = &rule.match_spec;
    let mut used = false;
    if let Some(p) = spec.panic_contains.as_ref().filter(|v| !v.is_empty()) {
        used = true;
        if !p.iter().all(|x| log_lc.contains(&x.to_lowercase())) {
            return false;
        }
    }
    if let Some(ms) = spec.missing_sensor_any.as_ref().filter(|v| !v.is_empty()) {
        used = true;
        if !ms.iter().any(|x| log_lc.contains(&x.to_lowercase())) {
            return false;
        }
    }
    if let Some(arr) = spec.smc_sensor_array_any.as_ref().filter(|v| !v.is_empty()) {
        used = true;
        if !arr.iter().any(|tok| sensor_token_in_log(log_lc, tok)) {
            return false;
        }
    }
    used
}

fn match_from_model_panic_rules(log: &str, marketing: Option<&str>) -> Option<KbMatch> {
    let kb = inner();
    let rules = kb.model_panic_rules.as_ref()?;
    if rules.is_empty() {
        return None;
    }
    let log_lc = log.to_lowercase();
    let mut best: Option<(&ModelPanicRule, f64)> = None;
    for rule in rules {
        if !model_panic_rule_applies(&log_lc, marketing, rule) {
            continue;
        }
        let score = rule.confidence
            + 0.01 * (rule.match_spec.smc_sensor_array_any.as_ref().map(|v| v.len()).unwrap_or(0) as f64);
        if best.map(|(_, s)| score > s).unwrap_or(true) {
            best = Some((rule, score));
        }
    }
    let (rule, _) = best?;
    let conf = confidence_to_u8(rule.confidence);
    let mut expl = format!(
        "Base PanicBase â€” rÃ¨gle modÃ¨le Â« {} Â» : {}.",
        rule.likely_component.trim(),
        rule
            .repair_order
            .as_ref()
            .map(|v| v.join(" â†’ "))
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "voir ordre atelier dans la KB".into())
    );
    if let Some(ref n) = rule.note {
        let t = n.trim();
        if !t.is_empty() {
            expl.push_str(" â€” ");
            expl.push_str(t);
        }
    }
    if let Some(ref ev) = rule.evidence_level {
        expl.push_str(" â€” Niveau de preuve (KB) : ");
        expl.push_str(ev.trim());
    }
    if let Some(ids) = rule.source_ids.as_ref() {
        if !ids.is_empty() {
            expl.push_str(" â€” Sources : ");
            let mut first = true;
            if let Some(map) = kb.source_registry.as_ref() {
                for id in ids.iter().take(6) {
                    let Some(ent) = map.get(id) else {
                        continue;
                    };
                    if !first {
                        expl.push_str(" Â· ");
                    }
                    first = false;
                    let q = ent.quality.as_deref().unwrap_or("?");
                    expl.push_str(&format!("{} [{}] {}", ent.title, q, ent.url));
                }
            }
        }
    }
    Some(KbMatch {
        probable_cause: rule.likely_component.trim().to_string(),
        confidence: conf,
        explanation: expl,
        matched_signature: format!("model_rule:{}", rule.likely_component.trim()),
    })
}

fn model_failures_addon(marketing: Option<&str>) -> Option<String> {
    let m = marketing?;
    let kb_map = &inner().model_specific_failures;
    let mut keys: Vec<&str> = kb_map.keys().map(|s| s.as_str()).collect();
    keys.sort_by(|a, b| b.len().cmp(&a.len()));
    for key in keys {
        if marketing_matches_key(m, key) {
            let joined = kb_map[key].join(", ");
            return Some(format!(" Sur {key}, failles souvent citÃ©es : {joined}."));
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

pub fn match_panic_kb(log: &str, marketing: Option<&str>) -> Option<KbMatch> {
    let kb = inner();
    let log_lc = log.to_lowercase();
    let mut hits: Vec<(&Pattern, usize)> = kb
        .common_panic_patterns
        .iter()
        .filter(|p| !p.causes.is_empty())
        .filter(|p| {
            let sig = p.signature.trim();
            let req = p
                .require_all_substrings
                .as_ref()
                .map(|v| !v.is_empty())
                .unwrap_or(false);
            !sig.is_empty() || req
        })
        .filter(|p| pattern_matches_log(&log_lc, p))
        .filter(|p| model_allowed(p, marketing))
        .map(|p| (p, pattern_sort_len(p)))
        .collect();

    let bug_txt = bug_type_line(log, &kb.bug_type_mapping);
    let model_txt = model_failures_addon(marketing);

    if hits.is_empty() {
        if let Some(mr) = match_from_model_panic_rules(log, marketing) {
            let mut expl = mr.explanation.clone();
            if let Some(ref m) = model_txt {
                expl.push_str(m);
            }
            if let Some(ref b) = bug_txt {
                expl.push(' ');
                expl.push_str(b);
            }
            return Some(KbMatch {
                probable_cause: mr.probable_cause,
                confidence: mr.confidence,
                explanation: expl,
                matched_signature: mr.matched_signature,
            });
        }
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
            probable_cause: "Voir dÃ©tails systÃ¨me".to_string(),
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
        "Base PanicBase â€” motif Â« {} Â» : {}.",
        matched_signature_label(pat),
        pat.causes.join(", ")
    );
    append_pattern_provenance(&mut expl, pat, &kb.source_registry);
    append_pattern_clinical(&mut expl, pat);
    append_pattern_repair_notes(&mut expl, pat);
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
        matched_signature: matched_signature_label(pat),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_kb_json_loads() {
        let kb = inner();
        assert!(
            !kb.common_panic_patterns.is_empty(),
            "common_panic_patterns vide"
        );
        assert!(
            match_panic_kb(
                "No successful checkins from thermalmonitord",
                Some("iPhone 11")
            )
            .is_some(),
            "exemple thermalmonitord + iPhone 11"
        );
    }

    #[test]
    fn iphone15_thermal_mic1_compound_matches() {
        let log = "panic â€¦ no successful checkins from thermalmonitord â€¦ mic1 â€¦";
        let m = match_panic_kb(log, Some("iPhone 15 Pro"));
        assert!(m.is_some());
        let m = m.expect("kb");
        assert!(m.matched_signature.contains("thermalmonitord"));
        assert!(m.matched_signature.contains("mic1"));
    }

    #[test]
    fn iphone15_hex_mask_matches() {
        let m = match_panic_kb("sensor array 0x300000", Some("iPhone 15"));
        assert!(m.is_some());
    }

    #[test]
    fn iphone15_non_pro_sensor_mask_0x80000_matches() {
        let log = "SMC PANIC â€¦ S.sensor array 0 - 4 is 0x0, 0x80000, 0x0, 0x0";
        let m = match_panic_kb(log, Some("iPhone 15"));
        assert!(m.is_some());
        let m = m.expect("kb");
        assert!(
            m.matched_signature.contains("0x80000") || m.explanation.contains("0x80000"),
            "{}",
            m.matched_signature
        );
    }

    #[test]
    fn source_registry_has_required_keys() {
        let kb = inner();
        let reg = kb
            .source_registry
            .as_ref()
            .expect("source_registry must be present in KB");
        assert!(!reg.is_empty());
    }

    #[test]
    fn pmap_ppl_pattern_matches() {
        let log = "panic(cpu 0 â€¦): pmap_mark_page_as_ppl_page_internal: page still has mappings, pa=0x8bf880000";
        let m = match_panic_kb(log, None);
        assert!(m.is_some());
        assert!(
            m.expect("kb")
                .matched_signature
                .to_lowercase()
                .contains("pmap_mark")
        );
    }

    #[test]
    fn gfx_endpoint_pattern_matches() {
        let log = "panic(cpu 1): GFX NMI: GFXEndpoint2: send msg=0x83000000000008 error=0xe00002db";
        let m = match_panic_kb(log, Some("iPhone 11 Pro Max"));
        assert!(m.is_some());
        let m = m.expect("kb");
        assert!(m.matched_signature.to_lowercase().contains("gfxendpoint"));
    }

    #[test]
    fn mipi_dsi_pattern_matches() {
        let log = "panic â€¦ @AppleSynopsysMIPIDSIController.cpp:842 â€¦ Timing out on commit";
        let m = match_panic_kb(log, None);
        assert!(m.is_some());
        assert!(m.expect("kb").matched_signature.contains("MIPIDSI"));
    }

    #[test]
    fn watchdog_wifid_compound_prefers_over_generic() {
        let log = "panic(cpu 1): userspace watchdog timeout: no successful checkins from wifid in 180 seconds\nwifid has not exited";
        let m = match_panic_kb(log, Some("iPhone 12"));
        assert!(m.is_some());
        let m = m.expect("kb");
        assert!(m.matched_signature.contains("wifid") || m.explanation.to_lowercase().contains("wifid"));
    }

    #[test]
    fn iphone6_not_matches_iphone16_keys() {
        assert!(!marketing_matches_key("iPhone 16 Pro", "iPhone 6"));
        assert!(marketing_matches_key("iPhone 6 Plus", "iPhone 6"));
    }

    #[test]
    fn model_panic_rule_fallback_dock_prs0() {
        let log = "panicString â€¦ thermalmonitord â€¦ prs0 â€¦";
        let m = match_panic_kb(log, Some("iPhone X")).expect("model rule");
        assert!(
            m.matched_signature.starts_with("model_rule:"),
            "{}",
            m.matched_signature
        );
        assert!(m.probable_cause.to_lowercase().contains("charg"));
    }

    #[test]
    fn repair_notes_appended_for_long_thermal_watchdog() {
        let log = "userspace watchdog timeout: no successful checkins from thermalmonitord";
        let m = match_panic_kb(log, Some("iPhone 12")).expect("kb");
        assert!(m.explanation.contains("Note atelier (KB)"));
    }
}
