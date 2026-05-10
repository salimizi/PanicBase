//! Corrélations panic par ProductType — capteurs texte et masques S.sensor → **nom de pièce** court.
//! Pas une source Apple officielle.

use std::collections::HashSet;

use regex::Regex;
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RwTier {
    /// X → 12 (incl.) sauf SE 2020 : capteurs texte mic1/mic2/prs/tg0b dans le wiki
    LegacyTextMic,
    /// iPhone SE 2020 (`iPhone12,8`) — mic1 : logique différente du port charge simple
    Se2020SpecialMic,
    /// Gamme « iPhone 13 » (identifiers iPhone14,4 … 14,3 en notation Apple)
    Series13Identifiers,
    Iphone1414Plus,
    Iphone14ProSeries,
    Iphone1515Plus,
    Iphone15ProSeries,
    /// Tableau universel bitmask 13+
    UniversalFallback,
}

fn product_lc(product_log: Option<&str>, hint: Option<&str>) -> String {
    let p = product_log
        .map(|s| s.trim())
        .filter(|s| {
            let l = s.to_lowercase();
            l.starts_with("iphone") && l.contains(',')
        })
        .map(|s| s.to_ascii_lowercase());
    if let Some(ref v) = p {
        if !v.is_empty() {
            return v.clone();
        }
    }
    hint.unwrap_or("").trim().to_ascii_lowercase()
}

/// iPhone 11 / 11 Pro / 11 Pro Max (`iPhone12,1` …) — Missing sensor texte MIC/PRS/TG.
#[inline]
fn is_iphone11_family(lc: &str) -> bool {
    matches!(lc, "iphone12,1" | "iphone12,3" | "iphone12,5")
}

fn tier_for_product(lc: &str) -> RwTier {
    match lc {
        "iphone12,8" => RwTier::Se2020SpecialMic,
        // iPhone 12 fam
        "iphone13,1" | "iphone13,2" | "iphone13,3" | "iphone13,4"
        | "iphone12,1" | "iphone12,3" | "iphone12,5"
        // X / XR / XS / 11 family
        | "iphone10,3" | "iphone10,6"
        | "iphone11,2" | "iphone11,4" | "iphone11,6" | "iphone11,8"
        | "iphone9,1" | "iphone9,2" | "iphone9,3" | "iphone9,4"
        | "iphone8,1" | "iphone8,2"
        | "iphone8,4" => RwTier::LegacyTextMic,

        // 13 mini / 13 / 13 Pro / 13 Max
        "iphone14,4" | "iphone14,5" | "iphone14,2" | "iphone14,3" => RwTier::Series13Identifiers,

        "iphone14,7" | "iphone14,8" => RwTier::Iphone1414Plus,
        "iphone15,2" | "iphone15,3" => RwTier::Iphone14ProSeries,
        "iphone15,4" | "iphone15,5" => RwTier::Iphone1515Plus,
        "iphone16,1" | "iphone16,2" => RwTier::Iphone15ProSeries,

        _ if lc.starts_with("iphone17,") || lc.starts_with("iphone18,") => RwTier::UniversalFallback,

        "" => RwTier::UniversalFallback,
        _ => RwTier::UniversalFallback,
    }
}

fn re_sf_array() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"(?is)([sf])\.sensor\s+array[^\n]{0,1200}").unwrap())
}

pub fn sensor_array_nonzero_decimal_masks(log_fragment: &str) -> Vec<u64> {
    let mut vals = Vec::new();
    for cap in re_sf_array().captures_iter(log_fragment) {
        if let Some(m) = cap.get(0) {
            let low = m.as_str().to_lowercase();
            let after = low.split(" is ").nth(1).unwrap_or("");
            let head = after
                .split("\\n")
                .next()
                .unwrap_or(after)
                .split('\n')
                .next()
                .unwrap_or(after);
            for raw in head.split(|c: char| c == ',' || c.is_whitespace()) {
                if let Some(v) = parse_sensor_mask_token(raw.trim()) {
                    if v > 0 {
                        vals.push(v);
                    }
                }
            }
        }
    }
    vals
}

/// Tolère `1048576`, `0x100000`, espaces — format courant dans panicString Apple.
fn parse_sensor_mask_token(raw: &str) -> Option<u64> {
    if raw.is_empty() {
        return None;
    }
    let lc = raw.to_ascii_lowercase();
    if let Some(hex) = lc.strip_prefix("0x") {
        let clean: String = hex.chars().filter(|c| c.is_ascii_hexdigit()).collect();
        return u64::from_str_radix(clean.as_str(), 16).ok();
    }
    let digits: String = raw.chars().filter(|c| c.is_ascii_digit()).collect();
    digits.parse().ok()
}

/// Libellé atelier uniquement (pas de source / pas de suffixe).
pub(crate) fn cause_label(msg: impl Into<String>) -> String {
    msg.into().trim().to_string()
}

/// Causes « Missing sensor » — iPhone 11 / 11 Pro / 11 Pro Max (partagées avec panic_diagnostic). Court pour l’UI « pièce probable ».
pub(crate) const IPHONE11_TG_MISSING_CAUSE: &str =
    "TG0B / TG0V · iPhone 11 · pile + connecteur + ligne DATA";
pub(crate) const IPHONE11_PRS_MIC1_CAUSE: &str =
    "PRS0 / MIC1 · iPhone 11 · nappe Lightning / charge";
pub(crate) const IPHONE11_MIC2_CAUSE: &str =
    "MIC2 · iPhone 11 · flex bouton power (micro MIC2)";

/// Pousse des causes pondérées + renvoie des lignes lisibles pour l’UI.
pub fn apply_repair_wiki_correlations(
    acc: &mut std::collections::HashMap<String, f64>,
    checks: &mut std::collections::HashSet<String>,
    product_from_log: Option<&str>,
    device_hint: Option<&str>,
    log_lc: &str,
    log_window: &str,
) -> Vec<String> {
    let mut ui_lines = Vec::new();

    let pc = product_lc(product_from_log, device_hint);
    let tier = tier_for_product(&pc);
    let push_acc = |acc: &mut std::collections::HashMap<String, f64>,
                    name: &str,
                    w: f64| {
        acc.entry(name.to_string())
            .and_modify(|e| *e = e.max(w))
            .or_insert(w);
    };

    let legacy_mic_tier =
        tier == RwTier::LegacyTextMic || tier == RwTier::Se2020SpecialMic;

    // ── Ancienne génération + SE 2020 : capteurs « Missing sensor(s): … » en texte ──
    if legacy_mic_tier {
        if is_iphone11_family(&pc) {
            if log_lc.contains("tg0b") || log_lc.contains("tg0v") {
                let n = cause_label(IPHONE11_TG_MISSING_CAUSE);
                push_acc(acc, &n, 0.92);
                ui_lines.push(format!("Capteur TG · {IPHONE11_TG_MISSING_CAUSE}"));
                checks.insert(
                    "TG0B/TG0V 11 : batterie/FPC pile · diode I2C (SCL/SDA) · R3201/R3202 · OL → piste CPU ; CC → CPU."
                        .into(),
                );
            }
            if log_lc.contains("prs0") || log_lc.contains("mic1") {
                let n = cause_label(IPHONE11_PRS_MIC1_CAUSE);
                push_acc(acc, &n, 0.9);
                ui_lines.push(format!("Capteur PRS/MIC1 · {IPHONE11_PRS_MIC1_CAUSE}"));
                checks.insert(
                    "PRS0/MIC1 11 : nappe charge OEM si doute · FPC dock · liquide · R6822/R6823 I2C1 AOP."
                        .into(),
                );
            }
            if log_lc.contains("mic2") {
                let n = cause_label(IPHONE11_MIC2_CAUSE);
                push_acc(acc, &n, 0.93);
                ui_lines.push(format!("Capteur mic2 · {IPHONE11_MIC2_CAUSE}"));
                checks.insert(
                    "MIC2 11 : nappe power clipsée · pry/lever · OL FPC → sandwich ; sinon continuités."
                        .into(),
                );
            }
            checks.insert(
                "iPhone 11 : capteur cité dans le log en premier ; nappes avant carte.".into(),
            );
            checks.insert(
                "Liquide / oxy / pry : éliminer avant de conclure carte mère.".into(),
            );
            checks.insert(
                "Aftermarket 11 : fort taux d’échec — valider avec OEM/Premium.".into(),
            );
        } else {
            let map = [
                ("prs0", "PRS0 · mic nappe Lightning / dock embout charge", 0.86_f64),
                ("tg0b", "TG0B · pack batterie + connecteur battery FPC", 0.84),
                ("tg0v", "TG0/TG0v · lignes données batterie (FPC pile)", 0.82),
            ];
            for (key, lab, wt) in map {
                if log_lc.contains(key) {
                    let n = cause_label(lab);
                    push_acc(acc, &n, wt);
                    ui_lines.push(format!("Capteur {key} · {lab}"));
                }
            }

            if tier == RwTier::Se2020SpecialMic && log_lc.contains("mic1") {
                let n = cause_label(
                    "SE 2020 MIC1 · dalle tactile / tactile IC / carte (prioritaire vs seule nappe dock)",
                );
                push_acc(acc, &n, 0.91);
                ui_lines.push("SE2020 MIC1 · vérifier tactile avant dock seul".into());
                checks.insert(
                    "SE 2020 / mic1 — isoler tactile + plateau avant de blâmer dock charge.".into(),
                );
            } else if tier == RwTier::LegacyTextMic && log_lc.contains("mic1") {
                let n = cause_label("MIC1 · micro bas / nappe dock Lightning complète");
                push_acc(acc, &n, 0.86);
                ui_lines.push("MIC1 · nappe dock".into());
            }

            let mic2_lab = if pc == "iphone12,1" {
                "MIC2 · assemblage Lightning (dock) puis flex volume / veille latéral"
            } else {
                "MIC2 · nappe dock Lightning ou flex bouton power selon modèle"
            };
            let mic2_w = if pc == "iphone12,1" { 0.92_f64 } else { 0.86 };
            if log_lc.contains("mic2") {
                let n = cause_label(mic2_lab);
                push_acc(acc, &n, mic2_w);
                ui_lines.push(format!("MIC2 · {mic2_lab}"));
            }

            checks.insert("Vérifier nappes OEM sur capteurs texte MIC/PRS/TG".into());
        }

        if log_lc.contains("ans2") {
            let n = cause_label("ANS2 · NAND ou contrôleur stockage monté NAND");
            push_acc(acc, &n, 0.87);
            ui_lines.push("ANS2 présent dans le log".into());
        }
    }

    let masks = sensor_array_nonzero_decimal_masks(log_window);

    if log_lc.contains("smc panic") && log_lc.contains("taop") && log_lc.contains("taoj") {
        checks.insert(
            "SMC + TAOP/TAOJ + OUTBOX : souvent nappe Qi/MagSafe ou USB‑C mal enclenchée — reprendre ces FPC avant carte."
                .into(),
        );
        ui_lines.push(
            "Motif SMC BSC + TAOP/TAOJ : penser charge sans fil / MagSafe et nappe USB‑C (aligné masque 0x280000)."
                .into(),
        );
    }

    if masks.is_empty() && legacy_mic_tier {
        ui_lines.push(
            "(Capteurs en texte seulement sur cette génération — pas de masque S.sensor lisible)".into(),
        );
    }

    if masks.is_empty() && !legacy_mic_tier {
        ui_lines.push("Aucune valeur bitmask non nulle lisible dans sensor array — coller panic-full complet aide.".into());
    }

    let mut decoded = HashSet::new();

    for &val in &masks {
        let explanations = explanations_for_mask(val, tier, log_lc, &pc);
        for (explain, wt) in explanations {
            if decoded.insert(explain.clone()) {
                ui_lines.push(format!("Mask 0x{val:x} ({val}) · {explain}"));
            }
            push_acc(acc, &cause_label(explain.clone()), wt);
        }
    }

    if masks.is_empty() {
        return ui_lines;
    }

    checks.insert("Contrôler tableau capteurs S.sensor avec masques hex.".into());
    if tier == RwTier::Series13Identifiers {
        checks.insert(
            "Série 13 SMC + sensor array : après chaque reboot ~3 min, noter le code hex sous Sensor Array.".into(),
        );
        checks.insert(
            "À contrôler : nappe flash · nappe bouton power · proximité · charge · batterie · liquide · connecteurs arrachés.".into(),
        );
        checks.insert(
            "Aftermarket déclenche souvent ces paniques — préférer OEM ou Premium pour tester.".into(),
        );
    }
    if tier == RwTier::Iphone1515Plus {
        checks.insert(
            "15 / 15 Plus : lire le nombre exact sous S.sensor array (ex. 2621440 = 0x280000) — priorité sur détail bitwise."
                .into(),
        );
        checks.insert(
            "Après swap : dernière pièce en cause en premier ; aftermarket nappes = risque."
                .into(),
        );
        checks.insert(
            "Avant carte : FPC recharge + USB‑C + liquide · sandwich au doute.".into(),
        );
    }
    ui_lines
}

#[allow(clippy::too_many_lines)]
fn explanations_for_mask(val: u64, tier: RwTier, log_lc: &str, pc: &str) -> Vec<(String, f64)> {
    let mut out: Vec<(String, f64)> = Vec::new();

    // ── Valeurs composites exactes (priorité Wiki par série) ──
    let mut try_push = |cond: bool, txt: &'static str, w: f64| {
        if cond {
            out.push((txt.to_string(), w));
        }
    };

    match tier {
        RwTier::Series13Identifiers => {
            let mini = pc == "iphone14,4";
            try_push(
                val == 0x800 || val == 2048,
                "0x800 · problème de nappe du port de charge",
                0.89,
            );
            try_push(
                val == 0x1000 || val == 4096,
                "0x1000 · problème nappe capteur de proximité — reconnecter, vérif déchirure ou oxydation, OEM/Premium, connecteur sur carte",
                0.88,
            );
            try_push(
                val == 0x1800 || val == 6144,
                "0x1800 · double problème : capteur de proximité + nappe du port de charge",
                0.91,
            );
            try_push(
                val == 0x4000 || val == 16_384,
                "0x4000 · problème ligne DATA batterie",
                0.87,
            );
            if mini {
                try_push(
                    val == 0x400 || val == 1024,
                    "0x400 · iPhone 13 Mini uniquement — problème Gyroscope",
                    0.93,
                );
                try_push(
                    val == 0xc00 || val == 3072,
                    "0xC00 · iPhone 13 Mini uniquement — problème Bottom Board et nappe du port de charge",
                    0.93,
                );
            }
        }
        RwTier::Iphone1414Plus => {
            try_push(val == 0x400000 || val == 4_194_304, "0x400000 · bobine Qi / module recharge sans fil (vitre arrière)", 0.9);
            try_push(val == 0x100000 || val == 1_048_576, "0x100000 · assemblage Lightning + MIC bas + flex infra", 0.88);
            try_push(val == 0x200000 || val == 2_097_152, "0x200000 · flex avant supérieur (proximity / ambiant)", 0.86);
            try_push(val == 0x500000 || val == 5_242_880, "0x500000 · bundle Taptic + zone charge bas", 0.83);
        }
        RwTier::Iphone14ProSeries => {
            try_push(
                val == 0x80000 || val == 524_288,
                "0x80000 · Dynamic Island · flex TrueDepth / capteurs façade",
                0.92,
            );
            try_push(val == 0x40000 || val == 262_144, "0x40000 · flex Lightning (assemblage port charge)", 0.88);
            try_push(val == 0x10000 || val == 65_536, "0x10000 · flex bouton veille latéral", 0.87);
            try_push(val == 0x20000 || val == 131_072, "0x20000 · fixations cadre / plaque médiane", 0.83);
        }
        RwTier::Iphone1515Plus => {
            // Masques composites en premier (priorité valeur exacte SMC / Sensor Array).
            try_push(
                val == 0x380000 || val == 3_670_016,
                "0x380000 · iPhone 15/15 Plus · Qi/MagSafe + USB‑C + façade (proximité)",
                0.94,
            );
            try_push(
                val == 0x280000 || val == 2_621_440,
                "0x280000 · iPhone 15/15 Plus · charge sans fil + USB‑C (les deux nappes)",
                0.93,
            );
            try_push(
                val == 0x200000 || val == 2_097_152,
                "0x200000 · iPhone 15/15 Plus · charge sans fil / bobine (arrière)",
                0.9,
            );
            try_push(
                val == 0x80000 || val == 524_288,
                "0x80000 · iPhone 15/15 Plus · nappe USB‑C (+ baromètre sur la ligne)",
                0.89,
            );
            try_push(
                val == 0x100000 || val == 1_048_576,
                "0x100000 · iPhone 15/15 Plus · proximité / capteurs avant",
                0.88,
            );
            try_push(
                val == 0xa1 || val == 161,
                "0xa1 · iPhone 15/15 Plus · données batterie (FPC pile)",
                0.9,
            );
        }
        RwTier::Iphone15ProSeries => {
            try_push(val == 0xa1 || val == 161, "0xa1 · bus données batterie (BMS côté FPC pile)", 0.88);
            try_push(val == 0x300000 || val == 3_145_728, "0x300000 · flexible USB‑C + port complet", 0.89);
            try_push(val == 0x400000 || val == 4_194_304, "0x400000 · bobine MagSafe/Qi montée vitre arrière", 0.9);
            try_push(
                val == 0x700000 || val == 7_340_032,
                "0x700000 · charge filaire USB‑C + bobine Qi (ensemble à reprendre si swap)",
                0.92,
            );
            try_push(val == 0x280000 || val == 2_621_440, "0x280000 · zone Qi mélangée ligne charge USB‑C", 0.82);
        }
        _ => {}
    }

    // Fallback universel 13+ (bitwise) — jamais pour générations « texte mic » ni SE 2020
    if !matches!(
        tier,
        RwTier::LegacyTextMic | RwTier::Se2020SpecialMic
    ) {
        universal_bitwise_fr(val, log_lc, &mut out);
    }

    out
}

fn universal_bitwise_fr(val: u64, _log_lc: &str, out: &mut Vec<(String, f64)>) {
    // Table Repair Wiki « Universal » simplifiée (hex → FR)
    const BITS: &[(u64, &str, f64)] = &[
        (0x20, "Ligne alim charge PMU", 0.62),
        (0x40, "Circuit gas gauge pile", 0.65),
        (0x41, "Bus I2C batterie / données cellule", 0.66),
        (0xa1, "SON / temp interne pile 0xa1", 0.68),
        (0xa9, "Ligne SON batterie variant 0xa9", 0.65),
        (0x400, "Gyromètre (IMU) monté carte", 0.6),
        (0x800, "Bloc dock / Lightning bas", 0.68),
        (0x1000, "Capteurs proximité façade", 0.66),
        (0x4000, "Bus données / authent batterie (SON)", 0.64),
        (0x20000, "IMU gyro / lignes mouvement", 0.58),
        (0x40000, "Flex port charge Pro", 0.62),
        (0x80000, "Capteurs TrueDepth avant", 0.65),
        (0x100000, "Bloc alim bas / ou flex avant suivant série", 0.62),
        (0x200000, "Antenne Qi / flex charge sans fil", 0.68),
        (0x300000, "Ensemble USB‑C + PMIC associé", 0.67),
        (0x400000, "Bobine recharge sans fil", 0.69),
        (0x500000, "Taptic + bundle bas", 0.61),
        (0x700000, "USB‑C + Qi sur même bus", 0.7),
    ];

    // Si aucune ligne exact composite n’a rempli « out », décomposer bitwise
    if out.is_empty() {
        let mut pushed = HashSet::new();
        for &(bit, label, wt) in BITS {
            if bit != 0 && (val & bit) == bit {
                let line = format!("{label} (sous-mask 0x{bit:x})");
                if pushed.insert(line.clone()) {
                    out.push((line, wt));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use super::*;

    #[test]
    fn parses_hex_sensor_masks() {
        let text = r#"S.sensor array 0 - 4 is 0x0, 0x100000, 0x0, 0x0"#;
        assert_eq!(sensor_array_nonzero_decimal_masks(text), vec![0x100000]);
    }

    #[test]
    fn parses_iphone15_combo_mask() {
        let text = r#"S.sensor array 0 - 5 is 0, 2621440, 0, 0, 0"#;
        assert_eq!(sensor_array_nonzero_decimal_masks(text), vec![2621440]);
        let ex =
            explanations_for_mask(2621440, RwTier::Iphone1515Plus, "", "");
        assert!(ex.iter().any(|(s, _)| {
            let l = s.to_lowercase();
            l.contains("280000")
                || l.contains("magsafe")
                || l.contains("sans fil")
                || l.contains("bobine")
                || l.contains("combo")
                || l.contains("qi")
        }), "{ex:?}");
    }

    #[test]
    fn iphone15_plus_mask_0x380000_triple_wireless_charge_proximity() {
        let ex = explanations_for_mask(3_670_016, RwTier::Iphone1515Plus, "", "");
        assert!(
            ex.iter().any(|(s, _)| {
                s.contains("0x380000")
                    && s.contains("USB")
                    && (s.contains("proximit") || s.contains("avant"))
            }),
            "{ex:?}"
        );
    }

    #[test]
    fn iphone15_plus_mask_0xa1_battery_data() {
        let ex = explanations_for_mask(161, RwTier::Iphone1515Plus, "", "");
        assert!(
            ex.iter().any(|(s, _)| s.contains("0xa1") && s.contains("batter")),
            "{ex:?}"
        );
    }

    #[test]
    fn iphone15_plus_correlation_adds_workshop_checks_when_mask_present() {
        let mut acc = HashMap::new();
        let mut checks = HashSet::new();
        let log = r#"S.sensor array 0 - 5 is 0, 3670016, 0, 0, 0"#;
        let _ui = apply_repair_wiki_correlations(
            &mut acc,
            &mut checks,
            Some("iPhone15,4"),
            None,
            "smc panic assertion failed",
            log,
        );
        assert!(
            checks.iter().any(|c| c.contains("sandwich")),
            "{checks:?}"
        );
    }

    #[test]
    fn iphone11_missing_sensor_text_hooks_tg_and_dock_rules() {
        let mut acc = HashMap::new();
        let mut checks = HashSet::new();
        let log_lc = concat!(
            "missing sensor(s): tg0b, prs0, mic2\n",
            "producttype iphone12,1\n",
        );
        let ui = apply_repair_wiki_correlations(
            &mut acc,
            &mut checks,
            Some("iPhone12,1"),
            None,
            log_lc,
            "",
        );
        assert!(acc.keys().any(|k| k.contains("TG0B") && k.contains("TG0V")), "{acc:?}");
        assert!(acc.keys().any(|k| k.contains("PRS0") && k.contains("MIC1")), "{acc:?}");
        assert!(
            acc.keys()
                .any(|k| k.contains("MIC2") && k.contains("iPhone 11") && k.contains("bouton power")),
            "{acc:?}"
        );
        assert!(
            checks
                .iter()
                .any(|c| c.to_lowercase().contains("aftermarket")),
            "{checks:?}"
        );
        assert!(
            ui.iter().any(|l| l.contains("Capteur TG")),
            "{ui:?}"
        );
    }

    #[test]
    fn iphone13_mini_sensor_mask_0x400_bottom_board_not_regular_13() {
        let mini = explanations_for_mask(1024, RwTier::Series13Identifiers, "", "iphone14,4");
        assert!(
            mini.iter().any(|(s, _)| {
                s.contains("iPhone 13 Mini uniquement") && s.to_lowercase().contains("gyroscope")
            }),
            "{mini:?}"
        );

        let regular = explanations_for_mask(1024, RwTier::Series13Identifiers, "", "iphone14,5");
        assert!(
            !regular
                .iter()
                .any(|(s, _)| s.contains("iPhone 13 Mini uniquement")),
            "0x400 ne doit pas forcer scénario mini hors iPhone14,4: {regular:?}"
        );
    }

    #[test]
    fn iphone13_mini_0xc00_combo_bottom_and_dock() {
        let ex = explanations_for_mask(3072, RwTier::Series13Identifiers, "", "iphone14,4");
        assert!(
            ex.iter().any(|(s, _)| {
                s.contains("0xC00")
                    && s.contains("Bottom Board")
                    && s.contains("nappe du port de charge")
            }),
            "{ex:?}"
        );
    }

    #[test]
    fn iphone13_battery_data_mask_0x4000() {
        let ex = explanations_for_mask(16_384, RwTier::Series13Identifiers, "", "iphone14,5");
        assert!(
            ex.iter().any(|(s, _)| s.contains("0x4000") && s.contains("DATA")),
            "{ex:?}"
        );
    }

    #[test]
    fn iphone11_mic2_strong_correlation() {
        let mut acc = HashMap::new();
        let mut checks = HashSet::new();
        apply_repair_wiki_correlations(
            &mut acc,
            &mut checks,
            Some("iPhone12,1"),
            None,
            "missing sensor(s): mic2",
            "",
        );
        assert!(acc.iter().any(|(k, _)| {
            let l = k.to_lowercase();
            l.contains("mic2")
                && l.contains("iphone 11")
                && l.contains("bouton power")
        }));
    }

    #[test]
    fn se2020_mic1_special_case() {
        let mut acc = HashMap::new();
        let mut checks = HashSet::new();
        let ui = apply_repair_wiki_correlations(
            &mut acc,
            &mut checks,
            Some("iPhone12,8"),
            None,
            "missing sensor(s): mic1",
            "",
        );
        assert!(
            ui.iter().any(|l| l.to_lowercase().contains("mic1")),
            "{ui:?}"
        );
        assert!(acc.keys().any(|k| k.contains("SE 2020") || k.contains("MIC1")));
    }

    #[test]
    fn smc_taop_taoj_hints_post_wireless_intervention() {
        let mut acc = HashMap::new();
        let mut checks = HashSet::new();
        apply_repair_wiki_correlations(
            &mut acc,
            &mut checks,
            None,
            None,
            "smc panic bsc taop taoj",
            "",
        );
        assert!(
            checks.iter().any(|c| c.contains("Qi") || c.contains("MagSafe")),
            "{checks:?}"
        );
    }
}
