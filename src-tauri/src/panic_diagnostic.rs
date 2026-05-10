//! Pipeline obligatoire : extraction limitée → normalisation → corrélations → scoring multi-causes.
//! Ne pas raisonner sur le dump complet : uniquement champs critiques + sous-signatures.

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use regex::Regex;

use crate::iphone;

// ─── Étape 1 : sous-signatures canoniques (ordre = plus longues d’abord pour le match) ───

/// (needle lowercase, étiquette d’affichage canonique)
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

#[derive(Debug, Clone, serde::Serialize)]
pub struct PossibleCauseDiag {
    pub name: String,
    pub confidence: f64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct StructuredDiagnostic {
    /// Identifiant Apple (ProductType), ex. iPhone14,7
    pub device: String,
    /// Nom commercial si connu localement — corrélations hardware différentes par génération
    pub marketing_name: Option<String>,
    pub panic_type: String,
    pub normalized_signatures: Vec<String>,
    pub possible_causes: Vec<PossibleCauseDiag>,
    pub confidence_global: f64,
    pub repair_priority: String,
    pub recommended_checks: Vec<String>,
    /// Lignes brutes les plus parlantes extrait du log (sans stack complète).
    pub critical_lines: Vec<String>,
    /// Indices extraits masques / capteurs (texte brut outil export).
    pub wiki_hints: Vec<String>,
}

/// Fenêtre élargie pour capter tout le bloc panicString même JSON sur une ligne.
pub(crate) fn scan_wide_blob(log: &str) -> &str {
    log.get(..240000.min(log.len())).unwrap_or(log)
}

/// Étapes 1–2 : isoler les lignes utiles (capteurs SMC, OUTBOX, missing sensor, entête panic).
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
            format!("{}… [ligne tronquée]", line.chars().take(1200).collect::<String>())
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

    // Panic JSON monoligne : extraire coupures courtes même sans \n avant sensor array.
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
                    let hint = format!("Sensor bitmask (extrait) : {v} → 0x{v:x}");
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
        /* Inclure la virgule (iPhone15,4) — l’ancienne classe exclus `,` et ne capturait que « iPhone15 ». */
        m.insert(
            "product_type",
            Regex::new(r#"(?i)ProductType["'\s:=]+"?([^"'\\s\n\v\r>]+)"#).unwrap(),
        );
        /* IPS JSON utilise souvent "product":"iPhone15,2" sans clé ProductType */
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

/// Étape 1 — EXTRACTION : champs ciblés + ignore le reste du bruit.
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

/// Fusionne métadonnées du fichier IPS (JSON enveloppe) lorsque `panic_text` seul les omet encore.
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

/// Étape 2 — NORMALISATION : phrases simples à partir du texte utile uniquement.
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
        s.starts_with("prs") || s.starts_with("tg0") || s == "ncc" || s.contains("gauge")
    })
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

/// Indices bus / gaz : fenêtre courte pour éviter le bruit du dump.
fn detect_i2c_stress(log_lc: &str) -> bool {
    if !log_lc.contains("i2c") && !log_lc.contains("i²c") {
        return false;
    }
    log_lc.contains("timeout")
        || log_lc.contains("timed out")
        || log_lc.contains("time-out")
        || log_lc.contains("stall")
        || log_lc.contains("nack")
        || log_lc.contains("error recover")
}

/// **iPhone15,4** (identifiant produit Apple — souvent commercialisé « iPhone 15 » selon génération).
fn is_iphone15_4_product(product: Option<&str>, device_hint: Option<&str>) -> bool {
    for s in [product, device_hint].into_iter().flatten() {
        let c = s.to_lowercase().replace(' ', "");
        if c.contains("iphone15,4") || c == "15,4" {
            return true;
        }
    }
    false
}

/// Motif repair : SMC + BSC + clés thermiques TAOP/TAOJ + OUTBOX1 (nappe wireless / MagSafe fréquente après remplacement).
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
                || lk.contains("qi —")
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
            } else if lk.contains("chaîne thermique")
                || lk.contains("rapport températures")
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

/// Cause explicites « Missing sensor(s): … » : poids max, avant interprétation générique thermique/BMS.
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
                    "MIC2 · assemblage Lightning (dock) puis flex volume / veille latéral",
                    0.95,
                );
            } else {
                push(
                    acc,
                    "MIC2 · pré-ensemble haut-parleur + grille micro façade",
                    0.94,
                );
            }
        }
        push(acc, "Bus audio CODEC · ligne MIC2 sans réponse pile", 0.76);
        push(acc, "Audio IC (U3500) si court MIC2 avec nappes neuves", u3500_bonus);
        checks.insert("Prioritaire : FPC relié au MIC2 nominal sur ce modèle.".into());
        checks.insert("Contrôler siège FPC MIC2 / oxydation connecteur.".into());
    }

    if missing_has(missing, "mic1") && !iphone11 {
        push(acc, "Micro primaire MIC1 / nappe ou connecteur RF audio", 0.88);
        push(acc, "Ligne MIC1 ou bus audio associé", 0.69);
        checks.insert("Isolation nappe où MIC1 est routée (modèle) · faux contact".into());
    }

    if missing_battery_hint(missing) && !iphone11 {
        push(acc, "Batterie / BMS · capteur courant ou thermique TG/PRS", 0.9);
        push(acc, "Connecteur battery FPC / gas gauge si PRS‑TG impliqué", 0.78);
        checks.insert("Métrologie BMS · connecteur batterie · cellules si TG/PRS manquants".into());
    }

    if !missing.is_empty() && !missing_has(missing, "mic2") && !missing_has(missing, "mic1") && !missing_battery_hint(missing) {
        push(acc, "Capteur(s) listé(s) manquant(s) : nappe / connecteur correspondant(e)", 0.79);
        checks.insert("Pour chaque nom de capteur : FPC relié puis continuités".into());
    }
}

/// Étape 3–4 : corrélations hardware + scoring multi-causes (jamais une seule cause en interne).
fn score_causes(
    normalized: &[String],
    extracted: &ExtractedFields,
    marketing: Option<&str>,
    missing_sensors: &[String],
    log_lc: &str,
    log_wide: &str,
    device_for_scoring: Option<&str>,
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

    let iphone15_4_wireless_smc = is_iphone15_4_product(product, device_for_scoring)
        && iphone15_4_wireless_smc_fingerprint(log_lc);

    if iphone15_4_wireless_smc {
        push(
            &mut acc,
            "Nappe wireless charging / assemblage MagSafe (motif TAOP·TAOJ + OUTBOX1 — prioritaire)",
            0.95,
        );
        push(
            &mut acc,
            "Capteurs thermiques ou bus SMC sur nappe Qi — périph. sans réponse (OUTBOX1)",
            0.88,
        );
        push(
            &mut acc,
            "Nappe aftermarket, FPC mal clipsé, ou ligne I2C thermique rompue (avant carte mère / batterie seule)",
            0.82,
        );
        checks.insert(
            "Historique remplacement MagSafe / bobine / flex wireless : reprendre pièce OE ou ressertir avant BMS/CM"
                .into(),
        );
        checks.insert(
            "Ne pas classer en simple surchauffe générique, batterie seule, ou défaut carte mère sans isoler nappe Qi"
                .into(),
        );
    }

    let thermal_with_audio_miss =
        missing_has(missing_sensors, "mic2") || missing_has(missing_sensors, "mic1");
    let thermal_battery_backed = missing_battery_hint(missing_sensors);

    if detect_i2c_stress(log_lc) {
        push(&mut acc, "Bus I2C bloqué ou périphérique sans réponse (timeouts)", 0.82);
        checks.insert("Localiser lignes I2C citées puis periph coupé ou rail associé".into());
    }

    if log_lc.contains("gas gauge") || log_lc.contains("gasgauge") {
        push(&mut acc, "Communication gas gauge / batterie (preuve dans le log)", 0.8);
        checks.insert("Gauge / BMS / connecteur bat si gas gauge nominal".into());
    }

    for n in &norm_lc {
        match n.as_str() {
            s if s.contains("mic2 interrupt") => {
                let base = if is_iphone_7_family(marketing, product) {
                    0.97
                } else {
                    0.72
                };
                push(&mut acc, "Audio IC (U3500)", base);
                checks.insert("Mesure lignes MIC2 / ligne I²S".into());
                checks.insert("Reflow / repro Audio IC selon gabarit".into());
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
                    "Chaîne thermique / rapport températures",
                    w_therm,
                );
                push(
                    &mut acc,
                    "Nappe charge / dock (chemins données capteurs)",
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
                        "thermalmonitord sans check-ins (effet capteur/bus — pas forcément surchauffe)",
                        0.64,
                    );
                    checks.insert("Ne pas conclure surchauffe réelle sans autre motif thermique dans le panic".into());
                }

                checks.insert("Nappe charge et connecteur".into());
                checks.insert("Sonde thermique / lignes I2C capteurs".into());
                checks.insert("Connecteur battery FPC".into());
            }
            s if s.contains("userspace watchdog") || s.ends_with("watchdog timeout") => {
                push(&mut acc, "Blocage watchdog (periph I2C / capteurs)", 0.78);
                push(&mut acc, "PMIC / rails instables possibles", 0.62);
                checks.insert("Rails alim basse tension (mesure)".into());
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
                    push(&mut acc, "Lignes SMC / données capteurs (OUTBOX)", w_out);
                }
                checks.insert("USB-C ou nappe charge (rails)".into());
                checks.insert("Capteurs thermiques → SMC".into());
            }
            s if s.contains("outbox1") => {
                let w_ob = if iphone15_4_wireless_smc { 0.48_f64 } else { 0.8 };
                push(&mut acc, "File SMC — capteurs / charge non prêts", w_ob);
                checks.insert("Bloc charge + bus capteurs basse niveau".into());
            }
            s if s.contains("bsc failure") => {
                let w_bsc = if iphone15_4_wireless_smc { 0.32_f64 } else { 0.77 };
                push(&mut acc, "BSC / liaison batterie-bus système", w_bsc);
                checks.insert("Battery BMS et connecteur batterie".into());
            }
            s if s.contains("ans2") => {
                push(&mut acc, "NAND / contrôleur stockage", 0.92);
                checks.insert("Stress stockage / health NAND".into());
            }
            s if s.contains("baseband panic") => {
                push(&mut acc, "Modem baseband / BB_CPU", 0.88);
                push(&mut acc, "Séparation interposer / RF (mécanique)", 0.55);
                checks.insert("Zone baseband / RF shield".into());
            }
            s if s.contains("sep panic") => {
                push(&mut acc, "Secure Enclave / Face ID pairée", 0.9);
                push(&mut acc, "Incompatibilité NAND sans repro SE (swap)", 0.68);
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
                push(&mut acc, "Nappe / dock ou FPC relié aux capteurs listés manquants", nappe);
                push(&mut acc, "Module caméra / lidar si capteur optique nominé", 0.61);
                checks.insert("Isolation capteurs listés comme manquants".into());
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
                checks.insert("Relire panicString après extraction courte".into());
                if thermal_bug_demote {
                    checks.insert("Bug thermique IOS : corroborer avec capteurs manquants/bus avant axe batterie dock".into());
                }
            }
        }
    }

    // Modèle générique depuis KB-lite : génération
    if let Some(m) = &mkt {
        if m.contains("iphone 15") || m.contains("iphone 16") || m.contains("iphone 17") {
            if (joined.contains("smc panic") || joined.contains("thermal")) && !iphone15_4_wireless_smc {
                push(&mut acc, "Sous-système USB-C / PMU ( PMIC )", 0.68);
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
        log_lc,
        log_wide,
    );
    wiki_lines_out.extend(wiki_ui);

    let causes_vec: Vec<PossibleCauseDiag> = acc
        .into_iter()
        .map(|(name, confidence)| PossibleCauseDiag { name, confidence })
        .collect();

    // Plus le rang est élevé, plus la cause est « actionnable » / spécifique.
    // (Le tri doit être décroissant sur ce rang, sinon tout ce qui vaut 0 — ex. Chaîne thermique — passe devant SMC/masques.)
    fn hardware_precision_rank(name: &str) -> u8 {
        let n = name.to_lowercase();
        let trimmed = name.trim_start();
        if trimmed.starts_with("0x") {
            return 5;
        }
        // Corrélations capteurs manquants nommées (priorité surThermalMonitorD générique ou explicatif).
        if n.starts_with("mic2 ·")
            || n.starts_with("mic1 ·")
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

    let mut causes = causes_vec;
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

fn resolve_panic_type(extracted: &ExtractedFields, normalized: &[String]) -> String {
    let pack = normalized
        .iter()
        .map(|s| s.to_lowercase())
        .collect::<Vec<_>>()
        .join(" ");

    if pack.contains("smc panic") || pack.contains("bsc failure") || pack.contains("outbox1") {
        return "smc_bsc_outbox_chain".into();
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

/// Pipeline complet jusqu’au JSON métier obligatoire.
/// `ips_envelope` = fichier IPS brut (.ips) lorsque `log` est seulement l’extrait panic — pour ProductType / product / bug_type.
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
    let parsed_miss = crate::panic_parser::parse_panic_log(&wide_combined);
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

    let (possible_causes, recommended_checks, mut confidence_global) = score_causes(
        &normalized,
        &extracted,
        marketing,
        &parsed_miss.missing_sensors,
        &log_lc,
        log_wide,
        resolved_hint,
        &mut wiki_hints,
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

    let critical_lines = extract_critical_signal_lines(&wide_combined);

    StructuredDiagnostic {
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
                && (tl.contains("flex bouton power") || tl.contains("bouton power")),
            "iPhone 11 / MIC2 : attendu flex power avant piste thermal générique · {:?}",
            top.name
        );

        assert!(
            d.possible_causes.iter().any(|c| {
                c.name.contains("thermalmonitord")
                    || c.name.contains("sans check-ins")
                    || (c.name.contains("thermique") && c.confidence <= 0.62)
            }),
            "devrait garder une piste thermalmonitord sous-corrélée ou explicative"
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
                .any(|c| c.name.contains("I2C") || c.name.contains("I²C")),
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
            "masque combiné Qi / tableau attendu pour SMC+masques 15,4 · {:?}",
            d.possible_causes
        );
        assert!(
            d.possible_causes.iter().any(|c| {
                c.name.contains("wireless") || c.name.contains("MagSafe") || c.name.contains("Qi")
            }),
            "motif TAOP·TAOJ + SMC doit garder une piste Qi/MagSafe explicite: {:?}",
            d.possible_causes
        );
        assert!(
            !d.possible_causes
                .iter()
                .take(6)
                .any(|c| c.name.contains("USB-C / PMU")),
            "ligne générique USB-C/PMU (famille 15 hors motif complet) absent ici : {:?}",
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
            "le masque Sensor Array doit dominer ThermalMonitorD affiché seul · top={:?} · {:?}",
            top.name,
            d.possible_causes
        );
        assert!(
            !top.name.contains("Chaîne thermique"),
            "première ligne : pas la bulle « Chaîne thermique » si un 0x… Repair Wiki existe · {:?}",
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
}
