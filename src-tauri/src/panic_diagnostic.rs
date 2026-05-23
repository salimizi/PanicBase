
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use regex::Regex;

use crate::iphone;
use crate::knowledge;

// â”€â”€â”€ Ã‰tape 1 : sous-signatures canoniques (ordre = plus longues dâ€™abord pour le match) â”€â”€â”€

const SUB_SIGNATURE_ROWS: &[(&str, &str)] = &[
    (
        "no successful checkins from thermalmonitord",
        "No successful checkins from thermalmonitord",
    ),
    ("no successful checkins", "No successful checkins"),
    (
        "userspace watchdog timeout",
        "Userspace watchdog timeout",
    ),
    ("mic2 interrupt watchdog", "mic2 interrupt watchdog"),
    ("undefined kernel instruction", "Undefined kernel instruction"),
    ("undefined instruction", "Undefined kernel instruction"),
    ("aop nmi power", "AOP NMI POWER"),
    ("applesochot", "AppleSocHot"),
    ("no valid cfg", "No valid CFG"),
    ("ans2 recoverable panic", "ANS2 Recoverable Panic"),
    ("outbox1 not ready", "OUTBOX1 not ready"),
    ("bsc failure", "BSC failure"),
    ("sep panic", "SEP Panic"),
    ("baseband panic", "Baseband Panic"),
    ("smc panic", "SMC PANIC"),
    ("aop panic", "AOP PANIC"),
    ("thermalmonitord", "thermalmonitord"),
    ("missing sensor", "Missing sensor"),
    ("gas gauge", "Gas gauge"),
    ("watchdog timeout", "watchdog timeout"),
];

#[derive(Debug, Clone, Default)]
pub struct ExtractedFields {
    pub panic_string_preview: Option<String>,
    pub bug_type: Option<String>,
    pub product: Option<String>,
    pub os_version: Option<String>,
    pub soc_id: Option<String>,
    pub uptime: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PossibleCauseDiag {
    pub name: String,
    pub confidence: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StructuredDiagnostic {
    pub device: String,
    pub marketing_name: Option<String>,
    pub panic_type: String,
    pub normalized_signatures: Vec<String>,
    pub possible_causes: Vec<PossibleCauseDiag>,
    pub confidence_global: f64,
    pub repair_priority: String,
    pub recommended_checks: Vec<String>,
    pub critical_lines: Vec<String>,
    pub wiki_hints: Vec<String>,
    pub action_plan: Vec<String>,
    pub danger_flags: Vec<String>,
    pub isolation_sequence: Vec<String>,
    pub likely_parts: Vec<String>,
    pub evidence_markers: Vec<String>,
    pub technician_summary: String,
    pub confidence_rationale: String,
    pub next_best_test: String,
}

pub(crate) fn scan_wide_blob(log: &str) -> &str {
    log.get(..240000.min(log.len())).unwrap_or(log)
}

pub fn extract_critical_signal_lines(log: &str) -> Vec<String> {
    let window = scan_wide_blob(log);
    let mut seen: HashSet<String> = HashSet::new();
    let mut out: Vec<String> = Vec::new();

    for raw in window.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        let lc = line.to_lowercase();
        if line.len() > 32000 {
            continue;
        }
        if line.len() > 12000
            && !lc.contains("sensor array")
            && !lc.contains("smc panic")
            && !lc.contains("bsc failure")
        {
            continue;
        }
        let hit = lc.contains("sensor array")
            || lc.contains("smc panic")
            || (lc.contains("smc") && lc.contains("assert"))
            || lc.contains("bsc failure")
            || (lc.contains("outbox") && lc.contains("not ready"))
            || lc.contains("missing sensor")
            || lc.contains("panic(cpu")
            || (lc.contains("panicstring")
                && (line.len() < 2200 || lc.contains("smc") || lc.contains("sensor array")));

        if !hit {
            continue;
        }

        let display = if line.len() > 1200 {
            format!("{}â€¦ [ligne tronquÃ©e]", line.chars().take(1200).collect::<String>())
        } else {
            line.to_string()
        };

        if seen.insert(display.clone()) {
            out.push(display);
            if out.len() >= 40 {
                break;
            }
        }
    }

    // Panic JSON monoligne : extraire coupures courtes mÃªme sans \n avant sensor array.
    let re_sf = Regex::new(r"(?i)([sf]\.sensor\s+array.{0,520})").unwrap();
    let mut bitmask_src: Vec<String> = Vec::new();
    for cap in re_sf.captures_iter(window) {
        if let Some(m) = cap.get(1) {
            let snip = m.as_str().trim().to_string();
            bitmask_src.push(snip.clone());
            if seen.insert(snip.clone()) {
                out.push(snip);
            }
        }
    }

    for full in bitmask_src
        .iter()
        .map(|s| s.as_str())
        .chain(window.lines().filter(|l| {
            let l = l.to_lowercase();
            l.contains("sensor array") && l.contains(" is ")
        }))
    {
        let lower = full.to_lowercase();
        let after_is = lower.split(" is ").nth(1).unwrap_or("");
        let until_noise = after_is
            .split("\\n")
            .next()
            .unwrap_or(after_is)
            .split('\n')
            .next()
            .unwrap_or(after_is);

        for part in until_noise.split(|c: char| c == ',' || c.is_whitespace()) {
            let t = part.trim().trim_matches(|c: char| !c.is_ascii_digit());
            if let Ok(v) = t.parse::<u64>() {
                if v > 0 {
                    let hint = format!("Sensor bitmask (extrait) : {v} â†’ 0x{v:x}");
                    if seen.insert(hint.clone()) {
                        out.push(hint);
                    }
                }
            }
        }
    }

    out
}

fn scan_window(log: &str) -> &str {
    log.get(..80000.min(log.len())).unwrap_or(log)
}

fn re(key: &'static str) -> &'static Regex {
    static MAP: OnceLock<HashMap<&'static str, Regex>> = OnceLock::new();
    MAP.get_or_init(|| {
        let mut m = HashMap::new();
        m.insert(
            "bug_type",
            Regex::new(r#"(?i)bug[_\s-]*type["'\s:=<]+"?(\d{2,})"#).unwrap(),
        );
        /* Inclure la virgule (iPhone15,4) â€” lâ€™ancienne classe exclus `,` et ne capturait que Â« iPhone15 Â». */
        m.insert(
            "product_type",
            Regex::new(r#"(?i)ProductType["'\s:=]+"?([^"'\\s\n\v\r>]+)"#).unwrap(),
        );
        /* IPS JSON utilise souvent "product":"iPhone15,2" sans clÃ© ProductType */
        m.insert(
            "apple_product_json",
            Regex::new(r#"(?is)"product"\s*:\s*"(iPhone\d+,\d+)""#).unwrap(),
        );
        m.insert(
            "os_version",
            Regex::new(r#"(?i)ProductVersion["'\s:=]+"?([^"'\\s,\n]+)"#).unwrap(),
        );
        m.insert(
            "soc_id",
            Regex::new(r#"(?i)soc[Ii][Dd]["'\s:=]+"?([^"'\\s,\n]+)"#).unwrap(),
        );
        m.insert(
            "uptime",
            Regex::new(r#"(?i)uptime["'\s:=]+"?([^"'\\s,\n]+)"#).unwrap(),
        );
        m.insert(
            "panic_preview",
            Regex::new(r#"(?is)panicString["'\s:=]+"?(.{12,960}?)"?(?:\n\n|\z|"\s|,)"#).unwrap(),
        );
        m
    })
    .get(key)
    .unwrap()
}

pub fn extract_fields(log: &str) -> ExtractedFields {
    let w = scan_window(log);
    let panic_string_preview = re("panic_preview")
        .captures(w)
        .and_then(|c| c.get(1))
        .map(|m| sanitize_preview(m.as_str()));
    ExtractedFields {
        panic_string_preview,
        bug_type: re("bug_type")
            .captures(w)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string()),
        product: re("apple_product_json")
            .captures(w)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
            .or_else(|| {
                re("product_type")
                    .captures(w)
                    .and_then(|c| c.get(1))
                    .map(|m| m.as_str().to_string())
            }),
        os_version: re("os_version")
            .captures(w)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string()),
        soc_id: re("soc_id")
            .captures(w)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string()),
        uptime: re("uptime")
            .captures(w)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string()),
    }
}

pub(crate) fn merge_extracted_fields(log_primary: &str, ips_envelope: Option<&str>) -> ExtractedFields {
    let mut e = extract_fields(log_primary);
    let Some(extra) = ips_envelope.filter(|s| !s.trim().is_empty()) else {
        return e;
    };
    let m = extract_fields(extra);
    if e.panic_string_preview.is_none() {
        e.panic_string_preview = m.panic_string_preview;
    }
    if e.product.is_none() {
        e.product.clone_from(&m.product);
    }
    if e.bug_type.is_none() {
        e.bug_type.clone_from(&m.bug_type);
    }
    if e.os_version.is_none() {
        e.os_version.clone_from(&m.os_version);
    }
    if e.soc_id.is_none() {
        e.soc_id.clone_from(&m.soc_id);
    }
    if e.uptime.is_none() {
        e.uptime.clone_from(&m.uptime);
    }
    e
}

fn sanitize_preview(s: &str) -> String {
    let t = s
        .chars()
        .filter(|c| !c.is_control() || *c == '\n')
        .collect::<String>();
    t.trim().chars().take(900).collect()
}

pub fn normalize_signatures(log: &str, extracted: &ExtractedFields) -> Vec<String> {
    let mut blob = String::new();
    blob.push_str(&log.to_lowercase());
    if let Some(p) = &extracted.panic_string_preview {
        blob.push(' ');
        blob.push_str(&p.to_lowercase());
    }

    let mut seen: HashSet<String> = HashSet::new();
    let mut out: Vec<String> = Vec::new();

    for (needle, label) in SUB_SIGNATURE_ROWS {
        if blob.contains(needle) && seen.insert(label.to_string()) {
            out.push((*label).to_string());
        }
    }

    out
}

fn bug_type_label(code: &str) -> Option<&'static str> {
    match code {
        "109" => Some("kernel panic"),
        "134" => Some("baseband panic"),
        "210" => Some("watchdog timeout"),
        "211" => Some("thermal issue"),
        "288" => Some("userspace reset"),
        _ => None,
    }
}

fn is_iphone_7_family(marketing: Option<&str>, product: Option<&str>) -> bool {
    let pack = format!(
        "{} {}",
        marketing.unwrap_or("").to_lowercase(),
        product.unwrap_or("").to_lowercase()
    );
    pack.contains("iphone 7")
        || pack.contains("iphone9,1")
        || pack.contains("iphone9,2")
        || pack.contains("iphone9,3")
        || pack.contains("iphone9,4")
}

fn is_iphone_x_class(marketing: Option<&str>) -> bool {
    marketing
        .map(|m| {
            let l = m.to_lowercase();
            l.contains("iphone x") && !l.contains("iphone xs") && !l.contains("iphone xr")
        })
        .unwrap_or(false)
}

fn missing_has(missing: &[String], id: &str) -> bool {
    missing.iter().any(|s| s == id)
}

fn missing_battery_hint(missing: &[String]) -> bool {
    missing.iter().any(|s| {
        s.starts_with("tg0") || s == "ncc" || s.contains("gauge") || s.contains("bms")
    })
}

fn missing_dock_pressure_hint(missing: &[String]) -> bool {
    missing.iter().any(|s| s == "prs0" || s == "prs" || s.starts_with("prs"))
}

#[inline]
fn is_iphone11_family_product(product: Option<&str>, device_hint: Option<&str>) -> bool {
    for s in [product, device_hint].into_iter().flatten() {
        let c = s.trim().to_ascii_lowercase();
        if matches!(c.as_str(), "iphone12,1" | "iphone12,3" | "iphone12,5") {
            return true;
        }
    }
    false
}

fn is_mic2_earpiece_generation(product: Option<&str>, device_hint: Option<&str>) -> bool {
    for s in [product, device_hint].into_iter().flatten() {
        let c = s.trim().to_ascii_lowercase();
        if matches!(
            c.as_str(),
            "iphone10,3" | "iphone10,6" | "iphone11,2" | "iphone11,4" | "iphone11,6" | "iphone11,8"
                | "iphone12,1" | "iphone12,3" | "iphone12,5"
                | "iphone13,1" | "iphone13,2" | "iphone13,3" | "iphone13,4"
        ) {
            return true;
        }
    }
    false
}

fn detect_i2c_stress(log_lc: &str) -> bool {
    if !log_lc.contains("i2c") && !log_lc.contains("iÂ²c") {
        return false;
    }
    log_lc.contains("timeout")
        || log_lc.contains("timed out")
        || log_lc.contains("time-out")
        || log_lc.contains("stall")
        || log_lc.contains("nack")
        || log_lc.contains("error recover")
}

fn is_iphone15_series_product(
    marketing: Option<&str>,
    product: Option<&str>,
    device_hint: Option<&str>,
) -> bool {
    for s in [product, device_hint].into_iter().flatten() {
        let c = s.trim().to_ascii_lowercase();
        if matches!(
            c.as_str(),
            "iphone15,4" | "iphone15,5" | "iphone16,1" | "iphone16,2"
        ) {
            return true;
        }
    }
    if let Some(m) = marketing {
        let m = m.to_ascii_lowercase();
        if m.contains("iphone 15") {
            return true;
        }
    }
    false
}

fn log_suggests_bottom_mic_oxidation(log_lc: &str) -> bool {
    log_lc.contains("oxyd")
        || log_lc.contains("corros")
        || log_lc.contains("liquid")
        || log_lc.contains("eau")
        || log_lc.contains("humid")
        || log_lc.contains("moisture")
        || log_lc.contains("water damage")
}

fn score_iphone15_bottom_mic_module(
    acc: &mut HashMap<String, f64>,
    checks: &mut HashSet<String>,
    log_lc: &str,
    missing: &[String],
    marketing: Option<&str>,
    product: Option<&str>,
    device_hint: Option<&str>,
) {
    if !is_iphone15_series_product(marketing, product, device_hint) {
        return;
    }
    let mic1 = missing_has(missing, "mic1") || log_lc.contains("mic1");
    let thermal = log_lc.contains("thermalmonitord");
    let smc_bottom = log_lc.contains("0x80000")
        || log_lc.contains("524288")
        || log_lc.contains("0x300000")
        || log_lc.contains("3145728")
        || log_lc.contains("smc panic");
    if !mic1 && !thermal && !smc_bottom {
        return;
    }
    let push = |acc: &mut HashMap<String, f64>, name: &str, w: f64| {
        acc.entry(name.to_string())
            .and_modify(|e| *e = e.max(w))
            .or_insert(w);
    };
    let mut w = if mic1 && thermal {
        0.97_f64
    } else if mic1 {
        0.95
    } else if thermal && smc_bottom {
        0.93
    } else {
        0.9
    };
    if log_suggests_bottom_mic_oxidation(log_lc) {
        w = w.max(0.98);
        push(
            acc,
            "Oxydation module micro bas / connecteur flex USB-C (trÃ¨s frÃ©quent sÃ©rie 15)",
            0.94,
        );
    }
    push(
        acc,
        crate::repair_wiki::IPHONE15_BOTTOM_MIC_MODULE_CAUSE,
        w,
    );
    checks.insert(
        "SÃ©rie 15 : module micro bas = PCB MEMS sur flex USB-C ; reseat clip + joint mousse ; ultrason si oxydÃ©.".into(),
    );
    if thermal {
        checks.insert(
            "thermalmonitord sur iPhone 15 â‰  chauffe CPU : prioriser MIC1 / capteurs du module bas.".into(),
        );
    }
    checks.insert(
        "Charge VBUS peut rester OK alors que MIC1 / lignes capteurs du flex bas sont absentes.".into(),
    );
}

fn is_iphone15_4_product(product: Option<&str>, device_hint: Option<&str>) -> bool {
    for s in [product, device_hint].into_iter().flatten() {
        let c = s.to_lowercase().replace(' ', "");
        if c.contains("iphone15,4") || c == "15,4" {
            return true;
        }
    }
    false
}

fn iphone15_4_wireless_smc_fingerprint(log_lc: &str) -> bool {
    let l = log_lc;
    let smc = l.contains("smc panic");
    let bsc = l.contains("bsc failure");
    let taop = l.contains("taop");
    let taoj = l.contains("taoj");
    let out = l.contains("outbox1 not ready")
        || (l.contains("outbox1") && l.contains("not ready"));
    smc && bsc && taop && taoj && out
}

fn demote_non_wireless_for_iphone15_4_smc(acc: &mut HashMap<String, f64>) {
    let keys: Vec<String> = acc.keys().cloned().collect();
    for k in keys {
        if let Some(v) = acc.get_mut(&k) {
            let lk = k.to_lowercase();
            if lk.contains("wireless charging")
                || lk.contains("magsafe")
                || lk.contains("qi â€”")
                || lk.contains("qi /")
            {
                continue;
            }
            if lk.contains("bsc / liaison batterie-bus")
                || (lk.contains("bms") && lk.contains("bsc"))
            {
                *v *= 0.26;
            } else if lk.contains("communication batterie")
                || lk.contains("gas gauge")
                || lk.contains("battery fpc")
            {
                *v *= 0.3;
            } else if lk.contains("chaÃ®ne thermique")
                || lk.contains("rapport tempÃ©ratures")
            {
                *v *= 0.36;
            } else if lk.contains("rails alim")
                && lk.contains("smc")
                && !lk.contains("qi")
            {
                *v *= 0.52;
            } else if lk.contains("pmic")
                || (lk.contains("pmu") && !lk.contains("wireless"))
            {
                *v *= 0.48;
            } else if lk.contains("usb-c")
                || (lk.contains("dock") && lk.contains("nappe charge"))
            {
                *v *= 0.42;
            }
        }
    }
}

fn score_named_missing_sensors(
    acc: &mut HashMap<String, f64>,
    checks: &mut HashSet<String>,
    missing: &[String],
    marketing: Option<&str>,
    product: Option<&str>,
    device_slug_hint: Option<&str>,
) {
    let push = |acc: &mut HashMap<String, f64>, name: &str, w: f64| {
        acc.entry(name.to_string())
            .and_modify(|e| *e = e.max(w))
            .or_insert(w);
    };

    let iphone11 = is_iphone11_family_product(product, device_slug_hint);

    if missing_has(missing, "mic2") {
        let u3500_bonus = if is_iphone_7_family(marketing, product) {
            0.96
        } else {
            0.74
        };
        if iphone11 {
            push(acc, crate::repair_wiki::IPHONE11_MIC2_CAUSE, 0.95);
            checks.insert(
                "iPhone 11 + MIC2 : nappe bouton power / micro flash (cf. fiche rÃ©fÃ©rence HTML)."
                    .into(),
            );
        } else {
            let slug12_1 = product
                .map(|p| p.trim().eq_ignore_ascii_case("iPhone12,1"))
                .unwrap_or(false)
                || device_slug_hint
                    .map(|p| p.trim().eq_ignore_ascii_case("iPhone12,1"))
                    .unwrap_or(false);
            if slug12_1 {
                push(
                    acc,
                    "MIC2 Â· assemblage Lightning (dock) puis flex volume / veille latÃ©ral",
                    0.95,
                );
            } else {
                push(
                    acc,
                    "MIC2 Â· prÃ©-ensemble haut-parleur + grille micro faÃ§ade",
                    0.94,
                );
            }
        }
        push(acc, "Bus audio CODEC Â· ligne MIC2 sans rÃ©ponse pile", 0.76);
        push(acc, "Audio IC (U3500) si court MIC2 avec nappes neuves", u3500_bonus);
        checks.insert("Prioritaire : FPC reliÃ© au MIC2 nominal sur ce modÃ¨le.".into());
        checks.insert("ContrÃ´ler siÃ¨ge FPC MIC2 / oxydation connecteur.".into());
    }

    if missing_has(missing, "mic1") && !iphone11 {
        if is_iphone15_series_product(marketing, product, device_slug_hint) {
            push(
                acc,
                crate::repair_wiki::IPHONE15_BOTTOM_MIC_MODULE_CAUSE,
                0.96,
            );
            push(
                acc,
                "Assemblage USB-C complet ou reseat clip micro bas + joint acoustique",
                0.9,
            );
            checks.insert(
                "iPhone 15 + MIC1 : module PCB MEMS sur flex USB-C â€” trÃ¨s paniqueux si oxydÃ©.".into(),
            );
        } else {
            push(acc, "MIC1 Â· nappe connecteur de charge / dock (cas terrain confirmÃ©)", 0.91);
            push(acc, "Ligne MIC1 / I2C audio-dock ou connecteur oxydÃ©", 0.74);
            checks.insert(
                "MIC1 : tester d'abord une nappe dock OEM/Premium connue bonne avant carte mÃ¨re.".into(),
            );
        }
    }

    if missing_dock_pressure_hint(missing) && !iphone11 {
        push(acc, "PRS0 Â· nappe connecteur de charge / capteur pression-dock", 0.92);
        push(acc, "Dock aftermarket ou FPC charge mal clipsÃ© / oxydÃ©", 0.82);
        checks.insert("PRS0 : cas terrain rÃ©current â€” remplacer/essayer dock OEM/Premium avant batterie ou carte.".into());
    }

    if missing_battery_hint(missing) && !iphone11 {
        push(acc, "Batterie / BMS Â· capteur courant ou thermique TG0", 0.9);
        push(acc, "Connecteur battery FPC / gas gauge si TG0/BMS impliquÃ©", 0.78);
        checks.insert("MÃ©trologie BMS Â· connecteur batterie Â· cellules si TG0/gauge manquants".into());
    }

    if !missing.is_empty() && !missing_has(missing, "mic2") && !missing_has(missing, "mic1") && !missing_battery_hint(missing) {
        push(acc, "Capteur(s) listÃ©(s) manquant(s) : nappe / connecteur correspondant(e)", 0.79);
        checks.insert("Pour chaque nom de capteur : FPC reliÃ© puis continuitÃ©s".into());
    }
}

fn apply_panic_kb_layer(
    log: &str,
    marketing: Option<&str>,
    acc: &mut HashMap<String, f64>,
    wiki_lines_out: &mut Vec<String>,
) {
    let Some(km) = knowledge::match_panic_kb(log, marketing) else {
        return;
    };
    wiki_lines_out.push(format!(
        "Base KB [{}] Â· {}",
        km.matched_signature, km.explanation
    ));
    let w = ((km.confidence as f64) / 100.0 * 0.82).clamp(0.28, 0.68);
    let key = format!("KB Â· {}", km.probable_cause.trim());
    acc.entry(key)
        .and_modify(|e| *e = e.max(w))
        .or_insert(w);
}

fn merge_redundant_cause_names(causes: &mut Vec<PossibleCauseDiag>) {
    if causes.len() < 2 {
        return;
    }
    causes.sort_by(|a, b| {
        b.confidence
            .partial_cmp(&a.confidence)
            .unwrap_or(Ordering::Equal)
    });
    let mut out: Vec<PossibleCauseDiag> = Vec::new();
    'next: for c in causes.drain(..) {
        let cl = c.name.to_lowercase();
        for o in &mut out {
            let ol = o.name.to_lowercase();
            let (short, long, wc) = if c.name.len() <= o.name.len() {
                (&cl, &ol, c.confidence)
            } else {
                (&ol, &cl, c.confidence)
            };
            if short.len() >= 32 && long.contains(short.as_str()) {
                o.confidence = o.confidence.max(wc).min(0.97);
                if c.name.len() > o.name.len() {
                    o.name = c.name.clone();
                }
                continue 'next;
            }
        }
        out.push(c);
    }
    *causes = out;
}

fn score_causes(
    normalized: &[String],
    extracted: &ExtractedFields,
    marketing: Option<&str>,
    missing_sensors: &[String],
    log_lc: &str,
    log_wide: &str,
    device_for_scoring: Option<&str>,
    soc_id: Option<&str>,
    wiki_lines_out: &mut Vec<String>,
) -> (Vec<PossibleCauseDiag>, Vec<String>, f64) {
    let mut acc: HashMap<String, f64> = HashMap::new();
    let mut checks: HashSet<String> = HashSet::new();
    let norm_lc: Vec<String> = normalized.iter().map(|s| s.to_lowercase()).collect();
    let joined = norm_lc.join(" | ");
    let product = extracted.product.as_deref();
    let mkt = marketing.map(|s| s.to_lowercase());

    let push = |acc: &mut HashMap<String, f64>, name: &str, w: f64| {
        acc.entry(name.to_string())
            .and_modify(|e| *e = e.max(w))
            .or_insert(w);
    };

    score_named_missing_sensors(
        &mut acc,
        &mut checks,
        missing_sensors,
        marketing,
        product,
        device_for_scoring,
    );

    score_iphone15_bottom_mic_module(
        &mut acc,
        &mut checks,
        log_lc,
        missing_sensors,
        marketing,
        product,
        device_for_scoring,
    );

    if is_iphone11_family_product(product, device_for_scoring)
        && missing_has(missing_sensors, "mic2")
        && (log_lc.contains("thermalmonitord")
            || log_lc.contains("no successful checkins from thermalmonitord"))
    {
        push(
            &mut acc,
            crate::repair_wiki::IPHONE11_MIC2_THERMAL_FLASH_POWER_CAUSE,
            0.72,
        );
        checks.insert(
            "iPhone 11 + thermalmonitord + MIC2 : vÃ©rifier nappe bouton power / flash.".into(),
        );
    }

    let iphone15_4_wireless_smc = is_iphone15_4_product(product, device_for_scoring)
        && iphone15_4_wireless_smc_fingerprint(log_lc);

    if iphone15_4_wireless_smc {
        push(
            &mut acc,
            "Nappe wireless charging / assemblage MagSafe (motif TAOPÂ·TAOJ + OUTBOX1 â€” prioritaire)",
            0.95,
        );
        push(
            &mut acc,
            "Capteurs thermiques ou bus SMC sur nappe Qi â€” pÃ©riph. sans rÃ©ponse (OUTBOX1)",
            0.88,
        );
        push(
            &mut acc,
            "Nappe aftermarket, FPC mal clipsÃ©, ou ligne I2C thermique rompue (avant carte mÃ¨re / batterie seule)",
            0.82,
        );
        checks.insert(
            "Historique remplacement MagSafe / bobine / flex wireless : reprendre piÃ¨ce OE ou ressertir avant BMS/CM"
                .into(),
        );
        checks.insert(
            "Ne pas classer en simple surchauffe gÃ©nÃ©rique, batterie seule, ou dÃ©faut carte mÃ¨re sans isoler nappe Qi"
                .into(),
        );
    }

    let thermal_with_audio_miss =
        missing_has(missing_sensors, "mic2") || missing_has(missing_sensors, "mic1");
    let thermal_battery_backed = missing_battery_hint(missing_sensors);

    if detect_i2c_stress(log_lc) {
        let stuck_low = log_lc.contains("checkbusstatus") && log_lc.contains("scl") && log_lc.contains("stuck low");
        if stuck_low {
            push(&mut acc, "I2C SCL stuck low Â· pÃ©riphÃ©rique/flex qui tire le bus Ã  la masse", 0.89);
            push(&mut acc, "Nappe dock/charge ou accessoire Lightning/USB-C Ã  isoler en premier", 0.83);
            checks.insert("I2C SCL stuck low : dÃ©brancher nappes une par une, commencer par dock/charge/accessoires, puis mesurer diode SCL/SDA.".into());
        } else {
            push(&mut acc, "Bus I2C bloquÃ© ou pÃ©riphÃ©rique sans rÃ©ponse (timeouts)", 0.82);
            checks.insert("Localiser lignes I2C citÃ©es puis periph coupÃ© ou rail associÃ©".into());
        }
    }

    if log_lc.contains("gas gauge") || log_lc.contains("gasgauge") {
        push(&mut acc, "Communication gas gauge / batterie (preuve dans le log)", 0.8);
        checks.insert("Gauge / BMS / connecteur bat si gas gauge nominal".into());
    }

    if log_lc.contains("panic-full") && log_lc.contains("3 minute") || log_lc.contains("3-minute") || log_lc.contains("restarts every 3") {
        checks.insert("Reboot ~3 minutes : penser watchdog thermalmonitord / capteur absent, pas batterie au hasard.".into());
    }

    for n in &norm_lc {
        match n.as_str() {
            s if s.contains("mic2 interrupt") => {
                if is_iphone_7_family(marketing, product) {
                    push(&mut acc, "Audio IC (U3500)", 0.97);
                    checks.insert("Mesure lignes MIC2 / ligne IÂ²S".into());
                    checks.insert("Reflow / repro Audio IC selon gabarit".into());
                } else if is_mic2_earpiece_generation(product, device_for_scoring) {
                    if is_iphone11_family_product(product, device_for_scoring) {
                        push(
                            &mut acc,
                            "MIC2 interrupt Â· iPhone 11 Â· Nappe bouton power Â· Micro cÃ´tÃ© flash",
                            0.86,
                        );
                        push(
                            &mut acc,
                            "Audio IC (U3500) seulement aprÃ¨s exclusion nappe power et bus audio",
                            0.42,
                        );
                        checks.insert(
                            "MIC2 interrupt sÃ©rie 11 : nappe bouton power (micro flash), pas Ã©couteur en premier."
                                .into(),
                        );
                    } else {
                        push(
                            &mut acc,
                            "Ã‰couteur interne / prÃ©-ensemble avant (MIC2 interrupt â€” corrÃ©lation forte Xâ†’12)",
                            0.85,
                        );
                        push(&mut acc, "Audio IC (U3500) seulement aprÃ¨s exclusion nappe Ã©couteur", 0.48);
                        checks.insert(
                            "MIC2 interrupt hors iPhone 7 : isoler prÃ©-ensemble Ã©couteur + FPC avant U3500."
                                .into(),
                        );
                    }
                } else {
                    push(&mut acc, "Audio IC (U3500)", 0.72);
                    checks.insert("Mesure lignes MIC2 / ligne IÂ²S".into());
                    checks.insert("Reflow / repro Audio IC selon gabarit".into());
                }
            }
            s if s.contains("thermalmonitord") || s.contains("no successful checkins") => {
                let (w_therm, w_dock, w_batt) =
                    if iphone15_4_wireless_smc {
                        (0.34_f64, 0.38, 0.22)
                    } else if thermal_with_audio_miss && !thermal_battery_backed {
                        (
                            0.52_f64,
                            0.44,
                            0.35,
                        )
                    } else if thermal_battery_backed {
                        (0.9, 0.81, 0.85)
                    } else {
                        (0.91, 0.84, 0.72)
                    };

                push(
                    &mut acc,
                    "ChaÃ®ne thermique / rapport tempÃ©ratures",
                    w_therm,
                );
                push(
                    &mut acc,
                    "Nappe charge / dock (chemins donnÃ©es capteurs)",
                    w_dock,
                );
                push(
                    &mut acc,
                    "Communication batterie (Gas gauge / BMS)",
                    w_batt,
                );

                if thermal_with_audio_miss && !thermal_battery_backed {
                    push(
                        &mut acc,
                        "thermalmonitord sans check-ins (effet capteur/bus â€” pas forcÃ©ment surchauffe)",
                        0.64,
                    );
                    checks.insert("Ne pas conclure surchauffe rÃ©elle sans autre motif thermique dans le panic".into());
                }

                if !thermal_with_audio_miss && !thermal_battery_backed {
                    checks.insert(
                        "thermalmonitord seul sans capteur manquant : souvent symptÃ´me bus â€” â‰  preuve panne thermique CPU (atelier)."
                            .into(),
                    );
                }

                checks.insert("Nappe charge et connecteur".into());
                checks.insert("Sonde thermique / lignes I2C capteurs".into());
                checks.insert("Connecteur battery FPC".into());
            }
            s if s.contains("userspace watchdog") || s.ends_with("watchdog timeout") => {
                push(&mut acc, "Blocage watchdog (periph I2C / capteurs)", 0.72);
                push(&mut acc, "PMIC / rails instables possibles", 0.58);
                checks.insert("Rails alim basse tension (mesure)".into());
                checks.insert(
                    "userspace watchdog : signature souvent gÃ©nÃ©rique â€” Ã©carter iOS / restore avant hardware seul (surtout 14/15)."
                        .into(),
                );
            }
            s if s.contains("aop nmi power") => {
                push(&mut acc, "Nappe bouton Power ou assemblage camÃ©ra avant (AOP NMI POWER â€” iFixit)", 0.82);
                checks.insert(
                    "AOP NMI POWER : iFixit â€” cÃ¢ble Power ou camÃ©ra avant ; isoler avant carte."
                        .into(),
                );
            }
            s if s.contains("applesochot") => {
                push(
                    &mut acc,
                    "Surchauffe SoC / ligne alim SoC ou dÃ©faut carte (AppleSocHot â€” souvent carte, pas flex)",
                    0.76,
                );
                checks.insert(
                    "AppleSocHot : iFixit â€” vÃ©rifier zones rÃ©parÃ©es ; Wiâ€‘Fi / audio carte souvent en cause."
                        .into(),
                );
            }
            s if s.contains("undefined kernel instruction") => {
                push(
                    &mut acc,
                    "Instruction noyau non dÃ©finie â€” cause logicielle frÃ©quente (mise Ã  jour / restore)",
                    0.55,
                );
                checks.insert(
                    "iFixit : si persiste aprÃ¨s restore complet â€” penser RAM / NAND / carte."
                        .into(),
                );
            }
            s if s.contains("aop panic") => {
                let fx = if is_iphone_x_class(marketing.as_deref()) {
                    0.9
                } else {
                    0.78
                };
                push(&mut acc, "Nappe Face ID / capteurs avant", fx);
                push(&mut acc, "Haut-parleur / flood illuminator (ligne avant)", 0.72);
                checks.insert("Nappe avant + proximity / flood".into());
                checks.insert(
                    "AOP PANIC : plusieurs pÃ©riphÃ©riques avant possibles â€” ne pas arrÃªter un seul composant sans isolement."
                        .into(),
                );
            }
            s if s.contains("smc panic") || joined.contains("bsc failure") => {
                let (w_rails, w_batt_smc, w_out) = if iphone15_4_wireless_smc {
                    (0.52_f64, 0.35, 0.58)
                } else {
                    (0.86_f64, 0.74, 0.82)
                };
                push(&mut acc, "Rails alim / SMC / PMU", w_rails);
                push(
                    &mut acc,
                    "Communication batterie ou thermique vers SMC",
                    w_batt_smc,
                );
                if joined.contains("outbox1") || norm_lc.iter().any(|x| x.contains("outbox1")) {
                    push(&mut acc, "Lignes SMC / donnÃ©es capteurs (OUTBOX)", w_out);
                }
                checks.insert("USB-C ou nappe charge (rails)".into());
                checks.insert("Capteurs thermiques â†’ SMC".into());
            }
            s if s.contains("outbox1") => {
                let w_ob = if iphone15_4_wireless_smc { 0.48_f64 } else { 0.8 };
                push(&mut acc, "File SMC â€” capteurs / charge non prÃªts", w_ob);
                checks.insert("Bloc charge + bus capteurs basse niveau".into());
            }
            s if s.contains("bsc failure") => {
                let w_bsc = if iphone15_4_wireless_smc { 0.32_f64 } else { 0.77 };
                push(&mut acc, "BSC / liaison batterie-bus systÃ¨me", w_bsc);
                checks.insert("Battery BMS et connecteur batterie".into());
                if !missing_sensors.is_empty() || joined.contains("missing sensor") {
                    checks.insert(
                        "SMC BSC + capteur / missing sensor : privilÃ©gier pÃ©riphÃ©rique absent avant PMIC mort (atelier)."
                            .into(),
                    );
                }
            }
            s if s.contains("ans2") => {
                push(&mut acc, "NAND / contrÃ´leur stockage / interposer CPUâ€“NAND", 0.92);
                checks.insert("Stress stockage / health NAND".into());
                checks.insert(
                    "ANS2 : sur 13+ aprÃ¨s sÃ©paration carte â€” penser soudure NAND / rails donnÃ©es (pas seulement Â« SSD Â» gÃ©nÃ©rique)."
                        .into(),
                );
            }
            s if s.contains("no valid cfg") => {
                push(
                    &mut acc,
                    "NAND / corruption stockage Â· Â« No valid CFG Â» (corrÃ©lation atelier forte)",
                    0.88,
                );
                checks.insert("No valid CFG : NAND / config stockage â€” croiser ANS2 et historique swap.".into());
            }
            s if s.contains("baseband panic") => {
                push(&mut acc, "Modem baseband / BB_CPU", 0.82);
                push(&mut acc, "SÃ©paration interposer / RF (mÃ©canique)", 0.52);
                push(&mut acc, "Cause logicielle / eSIM / profil opÃ©rateur (Ã  Ã©carter avant RF)", 0.45);
                checks.insert("Zone baseband / RF shield".into());
                checks.insert(
                    "Baseband panic : piÃ¨ge atelier â€” RF, alim BB, eSIM ou iOS ; ne pas conclure hardware seul."
                        .into(),
                );
            }
            s if s.contains("sep panic") => {
                push(&mut acc, "Secure Enclave / Face ID pairÃ©e", 0.9);
                push(&mut acc, "IncompatibilitÃ© NAND sans repro SE (swap)", 0.68);
                checks.insert("Historique Face ID / TrueDepth".into());
            }
            s if s.contains("missing sensor")
                || (s.contains("capteur absent") && !missing_sensors.is_empty()) =>
            {
                let nappe = if missing_has(missing_sensors, "mic2")
                    || missing_has(missing_sensors, "mic1")
                    || s.contains("mic2")
                    || s.contains("mic1")
                {
                    0.58
                } else {
                    0.76
                };
                push(&mut acc, "Nappe / dock ou FPC reliÃ© aux capteurs listÃ©s manquants", nappe);
                push(&mut acc, "Module camÃ©ra / lidar si capteur optique nominÃ©", 0.61);
                checks.insert("Isolation capteurs listÃ©s comme manquants".into());
                checks.insert(
                    "Table capteurs / thermalmonitord : iFixit FR â€” lire la ligne Â« missing sensor Â» au-delÃ  du premier panicString."
                        .into(),
                );
            }
            _ => {}
        }
    }

    if acc.is_empty() {
        // bug_type uniquement ou rien vu
        if let Some(bt) = &extracted.bug_type {
            if let Some(label) = bug_type_label(bt) {
                let thermal_bug_demote =
                    label == "thermal issue" && (thermal_with_audio_miss && !thermal_battery_backed);
                let wt = match label {
                    "watchdog timeout" => 0.7,
                    "thermal issue" => {
                        if thermal_bug_demote {
                            0.48
                        } else {
                            0.75
                        }
                    }
                    "baseband panic" => 0.82,
                    "kernel panic" => 0.55,
                    _ => 0.5,
                };
                push(
                    &mut acc,
                    &format!("Type bug {bt} ({label})"),
                    wt,
                );
                checks.insert("Relire panicString aprÃ¨s extraction courte".into());
                if thermal_bug_demote {
                    checks.insert("Bug thermique IOS : corroborer avec capteurs manquants/bus avant axe batterie dock".into());
                }
            }
        }
    }

    // ModÃ¨le gÃ©nÃ©rique depuis KB-lite : gÃ©nÃ©ration
    if let Some(m) = &mkt {
        if m.contains("iphone 15") || m.contains("iphone 16") || m.contains("iphone 17") {
            if (joined.contains("smc panic") || joined.contains("thermal")) && !iphone15_4_wireless_smc {
                push(&mut acc, "Sous-systÃ¨me USB-C / PMU ( PMIC )", 0.68);
            }
        }
    }

    if iphone15_4_wireless_smc {
        demote_non_wireless_for_iphone15_4_smc(&mut acc);
    }

    let wiki_ui = crate::repair_wiki::apply_repair_wiki_correlations(
        &mut acc,
        &mut checks,
        extracted.product.as_deref(),
        device_for_scoring,
        soc_id,
        log_lc,
        log_wide,
        missing_sensors,
    );
    wiki_lines_out.extend(wiki_ui);
    apply_panic_kb_layer(log_wide, marketing, &mut acc, wiki_lines_out);

    let mut causes: Vec<PossibleCauseDiag> = acc
        .into_iter()
        .map(|(name, confidence)| PossibleCauseDiag { name, confidence })
        .collect();

    merge_redundant_cause_names(&mut causes);

    // Plus le rang est Ã©levÃ©, plus la cause est Â« actionnable Â» / spÃ©cifique.
    // (Le tri doit Ãªtre dÃ©croissant sur ce rang, sinon tout ce qui vaut 0 â€” ex. ChaÃ®ne thermique â€” passe devant SMC/masques.)
    fn hardware_precision_rank(name: &str) -> u8 {
        let n = name.to_lowercase();
        let trimmed = name.trim_start();
        if trimmed.starts_with("0x") {
            return 5;
        }
        // CorrÃ©lations capteurs manquants nommÃ©es (prioritÃ© surThermalMonitorD gÃ©nÃ©rique ou explicatif).
        if n.starts_with("mic2 Â·")
            || n.starts_with("mic1 Â·")
            || (n.contains("mic2")
                && (n.contains("lightning")
                    || n.contains("dock")
                    || n.contains("nappe power")
                    || n.contains("iphone 11")))
            || (n.contains("prs0") && n.contains("nappe"))
            || (n.contains("iphone 11") && n.contains("prs0") && n.contains("mic1"))
            || (n.contains("iphone 11") && (n.contains("tg0b") || n.contains("tg0v")))
        {
            return 7;
        }
        if (n.contains("rails alim") && n.contains("smc"))
            || n.contains("communication batterie ou thermique vers smc")
            || (n.contains("usb-c") && (n.contains("pmu") || n.contains("pmic")))
        {
            return 4;
        }
        if n.contains("lignes smc") || n.contains("(outbox)") || n.contains("file smc") {
            return 3;
        }
        if n.contains("bsc / liaison batterie-bus") || n.contains("thermalmonitord sans check") {
            return 2;
        }
        0
    }

    causes.sort_by(|a, b| {
        hardware_precision_rank(&b.name)
            .cmp(&hardware_precision_rank(&a.name))
            .then_with(|| {
                b.confidence
                    .partial_cmp(&a.confidence)
                    .unwrap_or(Ordering::Equal)
            })
    });
    causes.truncate(6);

    let confidence_global = if causes.is_empty() {
        0.0
    } else if causes.len() == 1 {
        causes[0].confidence.min(0.97)
    } else {
        let c1 = causes[0].confidence;
        let c2 = causes.get(1).map(|x| x.confidence).unwrap_or(0.0);
        let c3 = causes.get(2).map(|x| x.confidence).unwrap_or(0.0);
        ((c1 + 0.65 * c2 + 0.35 * c3) / 2.0).min(0.97)
    };

    let mut rec: Vec<String> = checks.into_iter().collect();
    rec.sort();
    (causes, rec, confidence_global)
}


fn push_unique_vec(out: &mut Vec<String>, value: impl Into<String>) {
    let v = value.into();
    if !out.iter().any(|x| x == &v) {
        out.push(v);
    }
}

fn contains_any(hay: &str, needles: &[&str]) -> bool {
    needles.iter().any(|n| hay.contains(n))
}


fn extract_i2c_bus_names(log_lc: &str) -> Vec<String> {
    let mut out = Vec::new();
    let re = Regex::new(r"(?i)\b(i2c\d+|i2c[-_ ]?[a-z0-9]{1,8}|scl|sda|spi\d+|uart\d+)\b").unwrap();
    for m in re.find_iter(log_lc).take(12) {
        let v = m.as_str().trim().replace(' ', "-");
        if !out.iter().any(|x: &String| x.eq_ignore_ascii_case(&v)) {
            out.push(v);
        }
    }
    out
}

fn evidence_score_label(confidence: f64, cause_count: usize, signatures: usize, critical: usize) -> String {
    let pct = (confidence * 100.0).round().clamp(0.0, 99.0) as u8;
    if confidence >= 0.78 && cause_count > 0 {
        format!("Score fort ({pct}%) : signature prÃ©cise + piste actionnable, Ã  valider par test piÃ¨ce/mesure.")
    } else if confidence >= 0.48 {
        format!("Score moyen ({pct}%) : indices cohÃ©rents mais concurrence entre plusieurs causes ; isolation obligatoire.")
    } else if signatures > 0 || critical > 0 {
        format!("Score prudent ({pct}%) : panic reconnu mais preuve incomplÃ¨te ; comparer avec un deuxiÃ¨me log rÃ©cent.")
    } else {
        "Score faible : pas assez de signatures exploitables dans le panic fourni.".into()
    }
}

fn build_evidence_markers(
    diagnostic: &StructuredDiagnostic,
    log_lc: &str,
    missing_sensors: &[String],
) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(m) = &diagnostic.marketing_name {
        push_unique_vec(&mut out, format!("ModÃ¨le rÃ©solu : {m}"));
    } else if diagnostic.device != "unknown" {
        push_unique_vec(&mut out, format!("Identifiant produit : {}", diagnostic.device));
    }
    if !missing_sensors.is_empty() {
        push_unique_vec(&mut out, format!("Capteurs manquants lus : {}", missing_sensors.join(", ")));
    }
    for s in diagnostic.normalized_signatures.iter().take(6) {
        push_unique_vec(&mut out, format!("Signature : {s}"));
    }
    for bus in extract_i2c_bus_names(log_lc).into_iter().take(4) {
        push_unique_vec(&mut out, format!("Bus citÃ© : {bus}"));
    }
    for line in diagnostic.critical_lines.iter().take(4) {
        let clean = line.replace('\\', "");
        let snip: String = clean.chars().take(150).collect();
        push_unique_vec(&mut out, format!("Ligne critique : {snip}"));
    }
    if out.is_empty() {
        push_unique_vec(&mut out, "Aucune preuve forte extraite : importer le panic-full le plus rÃ©cent.".to_string());
    }
    out.truncate(10);
    out
}

fn choose_next_best_test(
    diagnostic: &StructuredDiagnostic,
    missing_sensors: &[String],
    log_lc: &str,
) -> String {
    let pack = diagnostic
        .possible_causes
        .iter()
        .map(|c| c.name.to_lowercase())
        .chain(diagnostic.normalized_signatures.iter().map(|s| s.to_lowercase()))
        .collect::<Vec<_>>()
        .join(" | ");

    if missing_sensors.iter().any(|s| s == "mic2") && diagnostic.device.eq_ignore_ascii_case("iphone12,1") {
        return "Tester une nappe bouton power/volume iPhone 11 OEM connue bonne (micro cÃ´tÃ© flash).".into();
    }
    if missing_sensors.iter().any(|s| s == "mic1" || s == "prs0" || s.starts_with("prs")) {
        return "Monter une nappe dock/charge OEM ou premium connue bonne avant toute intervention carte mÃ¨re.".into();
    }
    if missing_sensors.iter().any(|s| s.starts_with("tg0") || s.contains("gauge")) || pack.contains("gas gauge") {
        return "Tester batterie connue bonne, puis mesurer lignes BMS/gauge au connecteur batterie.".into();
    }
    if pack.contains("magsafe") || pack.contains("wireless") || (log_lc.contains("taop") && log_lc.contains("taoj")) {
        return "DÃ©brancher/remplacer la nappe Qi/MagSafe puis relancer un boot dâ€™observation.".into();
    }
    if pack.contains("i2c") || log_lc.contains("scl") || log_lc.contains("sda") {
        return "Isoler le bus IÂ²C : dÃ©brancher pÃ©riphÃ©riques un par un, puis comparer diode SCL/SDA.".into();
    }
    if pack.contains("ans2") || pack.contains("nand") || pack.contains("no valid cfg") {
        return "Sauvegarder/extraire les donnÃ©es avant restore ; contrÃ´ler zone NAND/stockage et interposer.".into();
    }
    if pack.contains("baseband") {
        return "ContrÃ´ler IMEI/baseband dans iOS, puis alimentation BB/RF avant de conclure carte mÃ¨re.".into();
    }
    if let Some(first) = diagnostic.action_plan.first() {
        return first.trim_start_matches(|c: char| c.is_ascii_digit() || c == '.' || c.is_whitespace()).to_string();
    }
    "Importer un second panic-full rÃ©cent et refaire lâ€™analyse avec le modÃ¨le exact.".into()
}

fn build_technician_summary(diagnostic: &StructuredDiagnostic) -> String {
    let cause = diagnostic
        .possible_causes
        .first()
        .map(|c| c.name.as_str())
        .unwrap_or("Panne non classÃ©e");
    let pct = (diagnostic.confidence_global * 100.0).round().clamp(0.0, 99.0) as u8;
    let test = diagnostic.next_best_test.trim();
    if test.is_empty() {
        format!("{cause} Â· confiance {pct}% Â· valider par isolation avant devis.")
    } else {
        format!("{cause} Â· confiance {pct}% Â· premier test : {test}")
    }
}

fn recalibrate_confidence(causes: &mut [PossibleCauseDiag], signatures: &[String], critical_lines: &[String], missing_sensors: &[String]) -> f64 {
    if causes.is_empty() {
        return 0.0;
    }
    for c in causes.iter_mut() {
        let n = c.name.to_lowercase();
        if !missing_sensors.is_empty() && (n.contains("capteur") || n.contains("mic") || n.contains("prs") || n.contains("tg0") || n.contains("bms")) {
            c.confidence = (c.confidence + 0.06).min(0.97);
        }
        if n.contains("gÃ©nÃ©rique") || n.contains("type bug") || n.contains("logicielle frÃ©quente") {
            c.confidence = (c.confidence * 0.86).max(0.22);
        }
    }
    let c1 = causes.get(0).map(|x| x.confidence).unwrap_or(0.0);
    let c2 = causes.get(1).map(|x| x.confidence).unwrap_or(0.0);
    let gap = (c1 - c2).max(0.0);
    let evidence_bonus = (signatures.len() as f64 * 0.012 + critical_lines.len() as f64 * 0.006).min(0.06);
    let ambiguity_penalty = if causes.len() >= 4 && gap < 0.10 { 0.08 } else if gap < 0.05 { 0.05 } else { 0.0 };
    (c1 + evidence_bonus + gap.min(0.08) - ambiguity_penalty).clamp(0.18, 0.97)
}

fn build_dangerous_workflow(
    diagnostic: &StructuredDiagnostic,
    log_lc: &str,
    missing_sensors: &[String],
) -> (Vec<String>, Vec<String>, Vec<String>, Vec<String>) {
    let mut action_plan = Vec::new();
    let mut danger_flags = Vec::new();
    let mut isolation = Vec::new();
    let mut parts = Vec::new();

    let causes_lc = diagnostic
        .possible_causes
        .iter()
        .map(|c| c.name.to_lowercase())
        .collect::<Vec<_>>()
        .join(" | ");
    let sig_lc = diagnostic
        .normalized_signatures
        .iter()
        .map(|s| s.to_lowercase())
        .collect::<Vec<_>>()
        .join(" | ");
    let pack = format!("{log_lc} | {causes_lc} | {sig_lc}");

    let iphone11_chassis = {
        let d = diagnostic.device.trim().to_ascii_lowercase();
        matches!(d.as_str(), "iphone12,1" | "iphone12,3" | "iphone12,5")
            || diagnostic.marketing_name.as_ref().is_some_and(|m| {
                let l = m.to_lowercase();
                l.contains("iphone 11") && !l.contains("iphone 12")
            })
    };

    push_unique_vec(&mut action_plan, "0. Sauvegarde client si lâ€™iPhone tient.");
    push_unique_vec(&mut action_plan, "1. Utiliser le panic-full le plus rÃ©cent.");

    let has_thermal = pack.contains("thermalmonitord") || pack.contains("no successful checkins");
    let has_missing = !missing_sensors.is_empty() || pack.contains("missing sensor");
    if has_thermal || has_missing {
        push_unique_vec(&mut danger_flags, "Reboot ~3 min : souvent un capteur absent, pas une batterie HS.");
        push_unique_vec(&mut action_plan, "2. Noter le capteur ou le masque indiquÃ© dans le log.");
        push_unique_vec(&mut isolation, "Boot minimal, puis remonter une nappe Ã  la fois.");
    }

    if contains_any(&pack, &["mic1", "prs0", "prs ", "dock", "connecteur de charge", "charging port"]) {
        push_unique_vec(&mut parts, "Nappe charge / dock OEM");
        push_unique_vec(&mut action_plan, "Tester une nappe charge connue bonne.");
        push_unique_vec(&mut isolation, "Inspecter FPC dock (pli, oxy).");
        push_unique_vec(&mut danger_flags, "Dock aftermarket : charge OK, capteurs KO.");
    }

    let mic2_on_missing = missing_sensors
        .iter()
        .any(|s| s.eq_ignore_ascii_case("mic2"));
    if iphone11_chassis && mic2_on_missing {
        push_unique_vec(
            &mut danger_flags,
            "MIC2 iPhone 11 : nappe bouton power + micro flash â€” oxydation connecteur, FPC mal clipÃ©, vitre arriÃ¨re / flash, chute ou liquide (reboot ~3 min).",
        );
        push_unique_vec(
            &mut action_plan,
            "iPhone 11 MIC2 : connecteur power (oxydation), historique vitre arriÃ¨re / flash / chute, puis nappe power OEM.",
        );
        push_unique_vec(&mut parts, "Nappe bouton power iPhone 11 (micro cÃ´tÃ© flash)");
        push_unique_vec(&mut isolation, "Ne pas partir sur lâ€™Ã©couteur en premier sur sÃ©rie 11.");
    } else if iphone11_chassis
        && (has_missing
            || has_thermal
            || contains_any(
                &pack,
                &["mic1", "prs0", "mic2", "dock", "connecteur de charge", "charging port"],
            ))
    {
        push_unique_vec(
            &mut danger_flags,
            "iPhone 11 : surtout dock, Power+flash, puis interposer â€” prÃ©-ensemble Ã©couteur facultatif Â· pas le mÃªme profil Â« connecteur Â» quâ€™en sÃ©rie 12.",
        );
        push_unique_vec(
            &mut action_plan,
            "iPhone 11 : Power + flash, puis dock ; si carte double, tester interposer.",
        );
        push_unique_vec(&mut parts, "iPhone 11 : nappe Power + flash de test");
    }

    if contains_any(&pack, &["mic2", "Ã©couteur", "earpiece", "capteurs avant", "prÃ©-ensemble avant"])
        && !iphone11_chassis
    {
        push_unique_vec(&mut parts, "Ã‰couteur / avant connu bon");
        push_unique_vec(&mut action_plan, "MIC2 : Ã©couteur ou power-flex selon modÃ¨le.");
        push_unique_vec(&mut isolation, "Swap prÃ©-ensemble avant, voir grille micro et liquide.");
    }

    if contains_any(&pack, &["tg0", "bms", "gas gauge", "batterie", "battery", "fuel gauge"]) {
        push_unique_vec(&mut parts, "Batterie + FPC batterie");
        push_unique_vec(&mut action_plan, "Batterie connue bonne, puis lignes BMS.");
        push_unique_vec(&mut isolation, "Diode mode BMS, pression lÃ©gÃ¨re FPC.");
    }

    if contains_any(&pack, &["i2c", "iÂ²c", "scl stuck low", "stuck low", "nack", "timed out", "timeout"]) {
        push_unique_vec(&mut parts, "Nappes du bus citÃ© + schÃ©ma");
        push_unique_vec(&mut action_plan, "IÂ²C : une nappe Ã  la fois.");
        push_unique_vec(&mut isolation, "SCL/SDA en diode, pull-up.");
        push_unique_vec(&mut danger_flags, "IÂ²C : une seule nappe peut bloquer tout le bus.");
    }

    if contains_any(&pack, &["smc panic", "bsc failure", "sensor array", "outbox1", "taop", "taoj"]) {
        push_unique_vec(&mut action_plan, "SMC : lire le masque capteur, pas seulement le titre.");
        push_unique_vec(&mut isolation, "Wireless / USB-C selon le log.");
        push_unique_vec(&mut danger_flags, "SMC â‰  PMIC mort sans preuve.");
    }

    if contains_any(&pack, &["magsafe", "wireless", "qi", "taop", "taoj", "outbox1"]) {
        push_unique_vec(&mut parts, "Nappe Qi / MagSafe");
        push_unique_vec(&mut action_plan, "Resserrer ou changer la nappe sans fil.");
    }

    if contains_any(&pack, &["ans2", "nvme", "nand", "no valid cfg", "ememory", "apcie", "invalid queue"]) {
        push_unique_vec(&mut parts, "Zone stockage / NAND");
        push_unique_vec(&mut action_plan, "DonnÃ©es client : Ã©viter restore destructif.");
        push_unique_vec(&mut danger_flags, "NAND : risque perte de donnÃ©es.");
    }

    if contains_any(&pack, &["baseband", "bb_cpu", "modem", "rf"]) {
        push_unique_vec(&mut parts, "Zone modem / RF");
        push_unique_vec(&mut action_plan, "VÃ©rifier rÃ©seau, eSIM, puis hardware BB.");
        push_unique_vec(&mut danger_flags, "Baseband : pas tout mettre sur la carte sans contexte rÃ©seau.");
    }

    if contains_any(&pack, &["aop panic", "aop nmi", "bosch", "flood", "face id", "proximity"]) {
        push_unique_vec(&mut parts, "Nappes avant (Face ID, prox, Ã©couteur)");
        push_unique_vec(&mut action_plan, "AOP : nappes avant et liquide.");
        push_unique_vec(&mut danger_flags, "Face ID : piÃ¨ces pairÃ©es.");
    }

    if diagnostic.confidence_global < 0.45 {
        push_unique_vec(&mut action_plan, "Score bas : second log + photos connecteurs.");
        push_unique_vec(&mut danger_flags, "Ne pas promettre une piÃ¨ce unique au client.");
    } else if diagnostic.confidence_global >= 0.78 {
        push_unique_vec(&mut action_plan, "Score haut : prÃ©parer la piÃ¨ce, valider au multimÃ¨tre.");
    }

    if parts.is_empty() {
        push_unique_vec(&mut parts, "Ã‰cran + batterie + dock de test");
        push_unique_vec(&mut parts, "MultimÃ¨tre + schÃ©ma");
    }
    if isolation.is_empty() {
        push_unique_vec(&mut isolation, "Minimal puis une nappe Ã  la fois.");
        push_unique_vec(&mut isolation, "Microscope : connecteurs, liquide, pins.");
    }

    action_plan.truncate(10);
    danger_flags.truncate(8);
    isolation.truncate(8);
    parts.truncate(8);
    (action_plan, danger_flags, isolation, parts)
}

fn resolve_panic_type(extracted: &ExtractedFields, normalized: &[String]) -> String {
    let pack = normalized
        .iter()
        .map(|s| s.to_lowercase())
        .collect::<Vec<_>>()
        .join(" ");

    if pack.contains("smc panic") || pack.contains("bsc failure") || pack.contains("outbox1") {
        return "smc_bsc_outbox_chain".into();
    }
    if pack.contains("no valid cfg") {
        return "no_valid_cfg_nand".into();
    }
    if pack.contains("applesochot") {
        return "applesochot_soc_thermal".into();
    }
    if pack.contains("aop nmi power") {
        return "aop_nmi_power".into();
    }
    if pack.contains("undefined kernel instruction") {
        return "undefined_kernel_instruction".into();
    }
    if pack.contains("ans2") {
        return "ans2_storage".into();
    }
    if pack.contains("baseband panic") {
        return "baseband_panic".into();
    }
    if pack.contains("sep panic") {
        return "sep_panic".into();
    }
    if pack.contains("aop panic") {
        return "aop_panic".into();
    }

    if let Some(bt) = &extracted.bug_type {
        if let Some(l) = bug_type_label(bt) {
            return format!("bug_{bt}_{}", l.replace(' ', "_"));
        }
    }
    if let Some(first) = normalized.first() {
        return first.replace([' ', '/', '-'], "_").to_lowercase();
    }
    if extracted.panic_string_preview.is_some() {
        return "unclassified_panicstring".into();
    }
    "unknown".into()
}

pub fn diagnose_structured(
    log: &str,
    device_model_hint: Option<&str>,
    ips_envelope: Option<&str>,
) -> StructuredDiagnostic {
    let extracted = merge_extracted_fields(log, ips_envelope);
    let wide_combined = match ips_envelope {
        Some(env) if !env.trim().is_empty() => format!("{}\n{}", env.trim(), log),
        _ => log.to_string(),
    };
    let mut parsed_miss = crate::panic_parser::parse_panic_log(&wide_combined);
    let parsed_primary = crate::panic_parser::parse_panic_log(log);
    // IPS complet + extrait : ne pas agrÃ©ger toutes les lignes Â« Missing sensor Â» du fichier
    // (sinon un vieux bloc mic2 pollue un panic courant tg0v/prs0 dans lâ€™extrait).
    if !parsed_primary.missing_sensors.is_empty() {
        parsed_miss.missing_sensors = parsed_primary.missing_sensors;
    } else if ips_envelope.is_some_and(|s| !s.trim().is_empty()) {
        parsed_miss.missing_sensors =
            crate::panic_parser::extract_missing_sensors_last(&wide_combined);
    }
    let mut normalized = normalize_signatures(log, &extracted);

    for sens in &parsed_miss.missing_sensors {
        let label = format!("Capteur absent ({})", sens);
        if !normalized.iter().any(|x| x == &label) {
            normalized.push(label);
        }
    }

    if !parsed_miss.missing_sensors.is_empty()
        && !normalized
            .iter()
            .any(|s| s.eq_ignore_ascii_case("Missing sensor"))
    {
        normalized.push("Missing sensor".into());
    }

    let device = extracted
        .product
        .clone()
        .or_else(|| device_model_hint.map(|s| s.trim().to_string()))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".to_string());

    let resolved_hint = if device != "unknown" {
        Some(device.as_str())
    } else {
        device_model_hint.map(str::trim).filter(|s| !s.is_empty())
    };
    let marketing_display = iphone::marketing_display_for_hints(resolved_hint);
    let marketing_opt_str = marketing_display.clone();
    let marketing = marketing_display.as_deref();

    let log_lc = scan_window(&wide_combined).to_lowercase();
    let log_wide = scan_wide_blob(&wide_combined);
    let mut wiki_hints: Vec<String> = Vec::new();

    let (mut possible_causes, recommended_checks, mut confidence_global) = score_causes(
        &normalized,
        &extracted,
        marketing,
        &parsed_miss.missing_sensors,
        &log_lc,
        log_wide,
        resolved_hint,
        extracted.soc_id.as_deref(),
        &mut wiki_hints,
    );

    let critical_lines = extract_critical_signal_lines(&wide_combined);
    confidence_global = recalibrate_confidence(
        &mut possible_causes,
        &normalized,
        &critical_lines,
        &parsed_miss.missing_sensors,
    );

    let panic_type = resolve_panic_type(&extracted, &normalized);

    if possible_causes.is_empty() {
        confidence_global = if extracted.bug_type.is_some() {
            0.42
        } else {
            0.0
        };
    }

    let repair_priority = if confidence_global >= 0.76 {
        "high"
    } else if confidence_global >= 0.48 {
        "medium"
    } else if confidence_global > 0.001 {
        "low"
    } else {
        "unknown"
    }
    .to_string();

    let mut diagnostic = StructuredDiagnostic {
        device: device.clone(),
        marketing_name: marketing_opt_str,
        panic_type,
        normalized_signatures: normalized,
        possible_causes,
        confidence_global,
        repair_priority,
        recommended_checks,
        critical_lines,
        wiki_hints,
        action_plan: Vec::new(),
        danger_flags: Vec::new(),
        isolation_sequence: Vec::new(),
        likely_parts: Vec::new(),
        evidence_markers: Vec::new(),
        technician_summary: String::new(),
        confidence_rationale: String::new(),
        next_best_test: String::new(),
    };

    let (action_plan, danger_flags, isolation_sequence, likely_parts) =
        build_dangerous_workflow(&diagnostic, &log_lc, &parsed_miss.missing_sensors);
    diagnostic.action_plan = action_plan;
    diagnostic.danger_flags = danger_flags;
    diagnostic.isolation_sequence = isolation_sequence;
    diagnostic.likely_parts = likely_parts;
    diagnostic.evidence_markers = build_evidence_markers(&diagnostic, &log_lc, &parsed_miss.missing_sensors);
    diagnostic.next_best_test = choose_next_best_test(&diagnostic, &parsed_miss.missing_sensors, &log_lc);
    diagnostic.confidence_rationale = evidence_score_label(
        diagnostic.confidence_global,
        diagnostic.possible_causes.len(),
        diagnostic.normalized_signatures.len(),
        diagnostic.critical_lines.len(),
    );
    diagnostic.technician_summary = build_technician_summary(&diagnostic);
    diagnostic
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE_ANS2: &str = r#"ProductType: iPhone14,5
panicString "ANS2 Recoverable Panic test""#;

    const FIXTURE_NVME_CHART: &str =
        r#"product "iPhone15,2" nvme controller reset kernel nvme error"#;

    #[test]
    fn regression_fixture_ans2_normalizes_storage() {
        let d = diagnose_structured(FIXTURE_ANS2, Some("iPhone14,5"), None);
        assert!(
            d.normalized_signatures
                .iter()
                .any(|s| s.contains("ANS2") || s.to_lowercase().contains("ans2")),
            "{:?}",
            d.normalized_signatures
        );
    }

    #[test]
    fn regression_fixture_nvme_triggers_chart_cause() {
        let d = diagnose_structured(FIXTURE_NVME_CHART, Some("iPhone15,2"), None);
        assert!(
            d.possible_causes.iter().any(|c| c.name.to_lowercase().contains("nvme")),
            "{:?}",
            d.possible_causes
        );
    }

    #[test]
    fn regression_fixture_userspace_watchdog_kb_or_core() {
        let log = r#"bug_type 210
userspace watchdog timeout in Springboard"#;
        let d = diagnose_structured(log, Some("iPhone 14"), None);
        assert!(
            !d.possible_causes.is_empty() || !d.recommended_checks.is_empty(),
            "expected some diagnostic output"
        );
    }

    #[test]
    fn missing_mic2_with_thermal_daemon_prioritizes_audio_path() {
        let log = concat!(
            "panicString \"foo Missing sensor(s): mic2 bar\"\n",
            "thermalmonitord complained\n",
            "No successful checkins from thermalmonitord\n",
        );

        let d = diagnose_structured(log, Some("iPhone12,1"), None);
        let top = &d.possible_causes[0];

        let tl = top.name.to_lowercase();
        assert!(
            tl.contains("mic2")
                && (tl.contains("power")
                    || tl.contains("flash")
                    || tl.contains("bouton")
                    || tl.contains("micro")),
            "iPhone 11 / MIC2 : attendu piste Power+flash en tÃªte Â· {:?}",
            top.name
        );

        assert!(
            d.possible_causes.iter().any(|c| {
                c.name.contains("thermalmonitord")
                    || c.name.contains("sans check-ins")
                    || (c.name.contains("thermique") && c.confidence <= 0.62)
            }),
            "devrait garder une piste thermalmonitord sous-corrÃ©lÃ©e ou explicative"
        );

        assert!(
            d.possible_causes.iter().any(|c| {
                let n = c.name.to_lowercase();
                n.contains("mic2") && n.contains("flash")
            }) || d.recommended_checks.iter().any(|c| {
                let l = c.to_lowercase();
                l.contains("flash") && (l.contains("thermalmonitord") || l.contains("power"))
            }),
            "attendu piste Power+flash avec mic2 + thermalmonitord Â· causes={:?} checks={:?}",
            d.possible_causes,
            d.recommended_checks
        );

        assert!(
            d.normalized_signatures.iter().any(|s| *s == "Capteur absent (mic2)"),
            "{:?}",
            d.normalized_signatures
        );
    }

    #[test]
    fn tg0_missing_boosts_battery_path() {
        let log =
            "panicString \"Missing sensor(s): tg0v, prs0\"\nthermalmonitord timeout\nProductType iPhone12,1";

        let d = diagnose_structured(log, None, None);
        assert!(
            d.possible_causes.iter().any(|c| {
                c.name.contains("BMS") || c.name.contains("batterie") || c.name.contains("PRS")
            }),
            "{:?}",
            d.possible_causes
        );
    }

    #[test]
    fn i2c_timeout_boosts_bus_cause() {
        let log = concat!(
            "panicString watchdog\n",
            "AppleARMII2CEthernetController i2c timed out stall\n",
        );

        let d = diagnose_structured(log, None, None);
        assert!(
            d.possible_causes
                .iter()
                .any(|c| c.name.contains("I2C") || c.name.contains("IÂ²C")),
            "{:?}",
            d.possible_causes
        );
    }

    #[test]
    fn iphone15_4_smc_taop_wireless_priority() {
        let log = r#"ProductType iPhone15,4
panicString "SMC PANIC BSC failure TAOP TAOJ
OUTBOX1 not ready
S.sensor array 0 - 5 is 0, 2621440, 0, 0, 0"
"#;

        let d = diagnose_structured(log, Some("iPhone15,4"), None);

        assert!(
            d.possible_causes.iter().any(|c| {
                let x = c.name.to_lowercase();
                x.contains("qi") || x.contains("combo") || x.contains("bobine") || x.contains("280")
            }),
            "masque combinÃ© Qi / tableau attendu pour SMC+masques 15,4 Â· {:?}",
            d.possible_causes
        );
        assert!(
            d.possible_causes.iter().any(|c| {
                c.name.contains("wireless") || c.name.contains("MagSafe") || c.name.contains("Qi")
            }),
            "motif TAOPÂ·TAOJ + SMC doit garder une piste Qi/MagSafe explicite: {:?}",
            d.possible_causes
        );
        assert!(
            !d.possible_causes
                .iter()
                .take(6)
                .any(|c| c.name.contains("USB-C / PMU")),
            "ligne gÃ©nÃ©rique USB-C/PMU (famille 15 hors motif complet) absent ici : {:?}",
            d.possible_causes
        );
    }

    #[test]
    fn iphone15_4_sensor_mask_prioritized_over_thermal_daemon_banner() {
        let log = r#"ProductType iPhone15,4
SMC PANIC - ASSERTION FAILED
thermalmonitord complained
No successful checkins from thermalmonitord
S.sensor array 0 - 5 is 0, 2621440, 0, 0, 0"#;

        let d = diagnose_structured(log, Some("iPhone15,4"), None);
        let top = d.possible_causes.first().expect("possible_causes empty");
        let n = top.name.to_lowercase();
        assert!(
            n.contains("0x280000")
                || (n.contains("sans fil") && n.contains("usb"))
                || n.contains("magsafe"),
            "le masque Sensor Array doit dominer ThermalMonitorD affichÃ© seul Â· top={:?} Â· {:?}",
            top.name,
            d.possible_causes
        );
        assert!(
            !top.name.contains("ChaÃ®ne thermique"),
            "premiÃ¨re ligne : pas la bulle Â« ChaÃ®ne thermique Â» si un 0xâ€¦ Repair Wiki existe Â· {:?}",
            top.name
        );
    }

    #[test]
    fn ips_envelope_injects_product_for_scoring() {
        let env = r#"{"product":"iPhone14,7","bug_type":"210"}"#;
        let d = diagnose_structured("SMC PANIC BSC", None, Some(env));
        assert_eq!(d.device, "iPhone14,7");
        assert!(
            d.marketing_name
                .as_ref()
                .map(|s| s.contains('4'))
                .unwrap_or(false),
            "marketing attendu pour iPhone14,7, got {:?}",
            d.marketing_name
        );
    }

    #[test]
    fn smc_panic_extracts_sensor_lines_and_mask() {
        let log = r#"{"product" : "iPhone15,4","panicString" : "panic(cpu 0): SMC PANIC - ASSERT: BSC failure
S.sensor array 0 - 5 is 0, 2621440, 0, 0, 0
F.sensor array 0 - 1 is 0
OUTBOX1 not ready"}"#;

        let lines = extract_critical_signal_lines(log);
        assert!(
            lines.iter().any(|l| l.contains("S.sensor array") && l.contains("2621440")),
            "{lines:?}"
        );
        assert!(lines.iter().any(|l| l.contains("0x280000")), "{lines:?}");

        let d = diagnose_structured(log, Some("iPhone15,4"), None);
        assert_eq!(d.panic_type, "smc_bsc_outbox_chain");
        assert!(!d.critical_lines.is_empty());
    }

    #[test]
    fn excerpt_missing_sensor_line_overrides_stale_mic2_from_ips_envelope() {
        let env = concat!(
            "legacy panic block\n",
            "Missing sensor(s): mic2\n",
            "--- separator ---\n",
        );
        let excerpt = concat!(
            "panicString \"â€¦\"\n",
            "Missing sensor(s): tg0v\n",
            "ProductType: iPhone12,1\n",
        );
        let d = diagnose_structured(excerpt, None, Some(env));
        assert!(
            !d.normalized_signatures
                .iter()
                .any(|s| s.to_lowercase().contains("mic2") && s.contains("Capteur absent")),
            "pas de Capteur absent (mic2) issu dâ€™un vieux bloc IPS Â· sig={:?}",
            d.normalized_signatures
        );
        assert!(
            d.normalized_signatures
                .iter()
                .any(|s| s.to_lowercase().contains("tg0v")),
            "tg0v de lâ€™extrait doit rester Â· sig={:?}",
            d.normalized_signatures
        );
        let blob = d
            .possible_causes
            .iter()
            .map(|c| c.name.as_str())
            .collect::<Vec<_>>()
            .join(" | ")
            .to_lowercase();
        assert!(
            !blob.contains("mic2 Â· iphone 11"),
            "MIC2 iPhone 11 ne doit pas venir dâ€™un autre bloc du fichier IPS Â· causes={:?}",
            d.possible_causes
        );
    }
}
