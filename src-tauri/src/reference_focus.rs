
use serde::{Deserialize, Serialize};

use crate::analyzer::AnalysisResult;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PanicReferenceFocus {
    pub nav_section: String,
    pub confidence: f64,
    pub initial_search: String,
}

fn product_type_section(pt: &str) -> Option<&'static str> {
    let pt = pt.trim().to_ascii_lowercase();
    match pt.as_str() {
        "iphone8,1" | "iphone8,2" => Some("iphone-6s"),
        "iphone8,4" => Some("iphone-se1"),
        "iphone9,1" | "iphone9,2" | "iphone9,3" | "iphone9,4" => Some("iphone-7"),
        "iphone10,1" | "iphone10,2" | "iphone10,4" | "iphone10,5" => Some("iphone-8"),
        "iphone10,3" | "iphone10,6" => Some("iphone-x"),
        "iphone11,2" | "iphone11,4" | "iphone11,6" => Some("iphone-xs"),
        "iphone11,8" => Some("iphone-xr"),
        "iphone12,8" => Some("iphone-se2"),
        "iphone12,1" | "iphone12,3" | "iphone12,5" => Some("iphone-11"),
        "iphone13,1" | "iphone13,2" => Some("iphone-12"),
        "iphone13,3" | "iphone13,4" => Some("iphone-12-pro"),
        "iphone14,4" | "iphone14,5" => Some("iphone-13"),
        "iphone14,2" | "iphone14,3" => Some("iphone-13-pro"),
        "iphone14,6" => Some("iphone-se3"),
        "iphone14,7" | "iphone14,8" => Some("iphone-14"),
        "iphone15,2" | "iphone15,3" => Some("iphone-14-pro"),
        "iphone15,4" | "iphone15,5" => Some("iphone-15"),
        "iphone16,1" | "iphone16,2" => Some("iphone-15-pro"),
        "iphone17,1" | "iphone17,2" => Some("iphone-16-pro"),
        "iphone17,3" | "iphone17,4" => Some("iphone-16"),
        "iphone18,1" | "iphone18,2" => Some("iphone-17-pro"),
        "iphone18,3" | "iphone18,4" => Some("iphone-17"),
        _ => None,
    }
}

fn section_from_marketing_name(name: &str) -> Option<&'static str> {
    let n = name.to_ascii_lowercase();
    let n = n.split_whitespace().collect::<Vec<_>>().join(" ");
    if n.contains("iphone 17 pro") {
        return Some("iphone-17-pro");
    }
    if n.contains("iphone 17") {
        return Some("iphone-17");
    }
    if n.contains("iphone 16 pro") {
        return Some("iphone-16-pro");
    }
    if n.contains("iphone 16") {
        return Some("iphone-16");
    }
    if n.contains("iphone 15 pro") {
        return Some("iphone-15-pro");
    }
    if n.contains("iphone 15 plus") || n.contains("iphone 15") {
        return Some("iphone-15");
    }
    if n.contains("iphone 14 pro") {
        return Some("iphone-14-pro");
    }
    if n.contains("iphone 14 plus") || n.contains("iphone 14") {
        return Some("iphone-14");
    }
    if n.contains("iphone 13 pro") {
        return Some("iphone-13-pro");
    }
    if n.contains("iphone 13 mini") || n.contains("iphone 13") {
        return Some("iphone-13");
    }
    if n.contains("iphone 12 pro") {
        return Some("iphone-12-pro");
    }
    if n.contains("iphone 12 mini") || n.contains("iphone 12") {
        return Some("iphone-12");
    }
    if n.contains("iphone 11 pro") || n.contains("iphone 11") {
        return Some("iphone-11");
    }
    if n.contains("iphone se") {
        if n.contains("3rd") {
            return Some("iphone-se3");
        }
        return Some("iphone-se2");
    }
    if n.contains("iphone xr") {
        return Some("iphone-xr");
    }
    if n.contains("iphone xs") {
        return Some("iphone-xs");
    }
    if n.contains("iphone x") {
        return Some("iphone-x");
    }
    if n.contains("iphone 8") {
        return Some("iphone-8");
    }
    if n.contains("iphone 7") {
        return Some("iphone-7");
    }
    None
}

pub fn infer_search_query_from_blob(blob_lower: &str) -> String {
    if let Some(caps) = regex::Regex::new(r"missing sensor\(s?\)?:?\s*([a-z0-9,\s]+)")
        .ok()
        .and_then(|re| re.captures(blob_lower))
    {
        if let Some(m) = caps.get(1) {
            for part in m.as_str().split([',', ' ', ';']) {
                let t = part.trim();
                if (2..=10).contains(&t.len()) {
                    return t.to_ascii_uppercase();
                }
            }
        }
    }
    if let Ok(re) = regex::Regex::new(r"(?i)s\.sensor\s+array[^\n]*?(?:is\s+)?(?:0x[0-9a-f]+|\d{4,})")
    {
        if let Some(caps) = re.captures(blob_lower) {
            let chunk = caps.get(0).map(|m| m.as_str()).unwrap_or("");
            if let Some(hex) = regex::Regex::new(r"(?i)0x[0-9a-f]+")
                .ok()
                .and_then(|re| re.find(chunk))
            {
                return hex.as_str().to_ascii_uppercase();
            }
            if let Some(dec) = regex::Regex::new(r"\d{4,}")
                .ok()
                .and_then(|re| re.find(chunk))
            {
                if let Ok(v) = dec.as_str().parse::<u64>() {
                    return format!("0x{:X}", v);
                }
            }
        }
    }
    const TOKENS: &[(&str, &str)] = &[
        ("thermalmonitord", "thermalmonitord"),
        ("smc panic", "SMC PANIC"),
        ("ans2", "ANS2"),
        ("sep panic", "SEP"),
        ("baseband", "Baseband"),
        ("applesochot", "AppleSocHot"),
        ("aop nmi", "AOP NMI"),
    ];
    for (needle, out) in TOKENS {
        if blob_lower.contains(needle) {
            return out.to_string();
        }
    }
    String::new()
}

fn build_blob_lower(panic_text: &str, analysis: &AnalysisResult) -> String {
    let mut parts = vec![panic_text.to_string(), analysis.probable_cause.clone()];
    parts.extend(analysis.keywords.iter().cloned());
    parts.extend(
        analysis
            .structured_diagnostic
            .critical_lines
            .iter()
            .cloned(),
    );
    parts.join("\n").to_ascii_lowercase()
}

fn with_search(
    panic_text: &str,
    analysis: &AnalysisResult,
    nav_section: &'static str,
    confidence: f64,
) -> PanicReferenceFocus {
    let blob_lower = build_blob_lower(panic_text, analysis);
    let mut initial_search = infer_search_query_from_blob(&blob_lower);
    if initial_search.is_empty() && blob_lower.contains("thermalmonitord") {
        initial_search = "thermalmonitord".to_string();
    }
    PanicReferenceFocus {
        nav_section: nav_section.to_string(),
        confidence,
        initial_search,
    }
}

pub fn infer_panic_reference_focus(
    panic_text: &str,
    analysis: &AnalysisResult,
    product_type: Option<&str>,
) -> PanicReferenceFocus {
    if let Some(pt) = product_type.filter(|s| !s.trim().is_empty()) {
        if let Some(section) = product_type_section(pt) {
            return with_search(panic_text, analysis, section, 1.0);
        }
    }

    let device = analysis.structured_diagnostic.device.trim();
    if !device.is_empty() && !device.eq_ignore_ascii_case("unknown") {
        if let Some(section) = product_type_section(device) {
            return with_search(panic_text, analysis, section, 0.97);
        }
    }

    for src in [
        analysis
            .structured_diagnostic
            .marketing_name
            .as_deref()
            .unwrap_or(""),
        analysis.device_model.as_str(),
    ] {
        if src.is_empty() {
            continue;
        }
        if let Some(section) = section_from_marketing_name(src) {
            return with_search(panic_text, analysis, section, 0.92);
        }
    }

    if let Ok(re) = regex::Regex::new(r"(?i)ProductType\s*[:=]\s*(iPhone\d+,\d+)") {
        if let Some(caps) = re.captures(panic_text) {
            if let Some(m) = caps.get(1) {
                if let Some(section) = product_type_section(m.as_str()) {
                    return with_search(panic_text, analysis, section, 0.95);
                }
            }
        }
    }

    if let Ok(re) =
        regex::Regex::new(r"(?i)iPhone\s*(?:SE\s*\(?\d+(?:st|nd|rd|th)?[^)]*\)?|\d+(?:\s*(?:Pro\s*Max|Pro|Plus|mini))?)")
    {
        if let Some(caps) = re.captures(panic_text) {
            if let Some(m) = caps.get(0) {
                if let Some(section) = section_from_marketing_name(m.as_str()) {
                    return with_search(panic_text, analysis, section, 0.8);
                }
            }
        }
    }

    let blob = panic_text.to_ascii_lowercase();
    const SOC: &[(&str, &str, f64)] = &[
        ("t8120", "iphone-15-pro", 0.6),
        ("t8122", "iphone-15-pro", 0.6),
        ("t8112", "iphone-15", 0.6),
        ("t8110", "iphone-15", 0.6),
        ("t8103", "iphone-14-pro", 0.6),
        ("t8101", "iphone-14-pro", 0.6),
        ("t8132", "iphone-14", 0.6),
        ("t8015", "iphone-11", 0.55),
        ("t8030", "iphone-x", 0.55),
    ];
    for (soc, section, conf) in SOC {
        if blob.contains(soc) {
            return with_search(panic_text, analysis, section, *conf);
        }
    }

    with_search(panic_text, analysis, "iphone-x", 0.0)
}
