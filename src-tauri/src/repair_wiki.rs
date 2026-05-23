
use std::collections::HashSet;

use regex::Regex;
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RwTier {
    LegacyTextMic,
    Se2020SpecialMic,
    Series13Identifiers,
    Iphone1414Plus,
    Iphone14ProSeries,
    Iphone1515Plus,
    Iphone15ProSeries,
    Iphone16Series,
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


#[inline]
fn is_iphone11_family(lc: &str) -> bool {
    matches!(lc, "iphone12,1" | "iphone12,3" | "iphone12,5")
}

#[inline]
fn is_iphone_x_only(lc: &str) -> bool {
    matches!(lc, "iphone10,3" | "iphone10,6")
}

#[inline]
fn is_iphone_xs_xs_max(lc: &str) -> bool {
    matches!(lc, "iphone11,2" | "iphone11,4" | "iphone11,6")
}

#[inline]
fn is_iphone_xr_only(lc: &str) -> bool {
    lc == "iphone11,8"
}

#[inline]
fn is_iphone12_apple_ids(lc: &str) -> bool {
    matches!(
        lc,
        "iphone13,1" | "iphone13,2" | "iphone13,3" | "iphone13,4"
    )
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

        // iPhone 16 series: iphone17,1 (Pro), iphone17,2 (Pro Max), iphone17,3 (16), iphone17,4 (16 Plus)
        "iphone17,1" | "iphone17,2" | "iphone17,3" | "iphone17,4" => RwTier::Iphone16Series,

        _ if lc.starts_with("iphone18,") => RwTier::UniversalFallback,

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


pub(crate) fn cause_label(msg: impl Into<String>) -> String {
    msg.into().trim().to_string()
}


pub(crate) fn append_ifixit_kernel_panic_hints(log_lc: &str, checks: &mut std::collections::HashSet<String>) {
    if log_lc.contains("thermalmonitord")
        && !log_lc.contains("missing sensor")
        && !log_lc.contains("missing sensor(s)")
    {
        checks.insert(
            "iFixit FR : thermalmonitord sans Â« missing sensor Â» peut Ãªtre logiciel (Springboard, logd, wifid) â€” mise Ã  jour / restore Ã  envisager."
                .into(),
        );
    }
    if log_lc.contains("i2c") || log_lc.contains("iÂ²c") {
        checks.insert(
            "iFixit : plusieurs bus IÂ²C par modÃ¨le â€” utiliser le nom de bus / pÃ©riphÃ©rique citÃ© dans le log ou un schÃ©ma."
                .into(),
        );
    }
    if log_lc.contains("smc panic") || log_lc.contains("smc panic assertion") || log_lc.contains("bsc failure") {
        checks.insert(
            "iFixit : SMC assertion (surtout iPhone 13+) â€” corrÃ©ler capteurs + codes sensor array ; reboot ~3 min si donnÃ©es absentes."
                .into(),
        );
    }
    if log_lc.contains("bosch") && log_lc.contains("aop") {
        checks.insert(
            "iFixit : panique AOP / canal Bosch (audio) â€” nappe port de charge (signaux HP), liquide, piÃ¨ce dâ€™origine ou premium."
                .into(),
        );
    }
}


#[inline]
fn missing_lists_sensor(missing: &[String], id: &str) -> bool {
    missing.iter().any(|s| s.eq_ignore_ascii_case(id))
}


pub(crate) const IPHONE11_TG_MISSING_CAUSE: &str =
    "TG0B ou TG0V Â· iPhone 11 Â· problÃ¨me de batterie";
pub(crate) const IPHONE11_PRS_MIC1_CAUSE: &str =
    "PRS0 ou MIC1 Â· iPhone 11 Â· nappe du connecteur de charge OEM/Premium";
pub(crate) const IPHONE11_MIC2_CAUSE: &str =
    "MIC2 Â· iPhone 11 Â· Nappe bouton power Â· Micro cÃ´tÃ© flash";
pub(crate) const IPHONE11_MIC2_THERMAL_FLASH_POWER_CAUSE: &str =
    "MIC2 + thermalmonitord Â· iPhone 11 Â· Nappe bouton power / flash";


pub(crate) const IPHONE15_BOTTOM_MIC_MODULE_CAUSE: &str =
    "Module micro du bas (MIC1) Â· iPhone 15 Â· PCB MEMS sur assemblage USB-C â€” oxydation / clip / joint acoustique";

pub(crate) const IPHONE15_BOTTOM_MIC_HARDWARE_NOTE: &str =
    "PCB indÃ©pendant (capsule MEMS), connecteur sur flex USB-C, emplacement chÃ¢ssis (grille mÃ©tal + joint mousse/caoutchouc)";

fn log_suggests_liquid_or_oxidation(log_lc: &str) -> bool {
    log_lc.contains("oxyd")
        || log_lc.contains("corros")
        || log_lc.contains("liquid")
        || log_lc.contains("eau")
        || log_lc.contains("humid")
        || log_lc.contains("moisture")
        || log_lc.contains("water damage")
}

fn iphone15_bottom_mic_signal(log_lc: &str, missing_sensors: &[String]) -> bool {
    missing_lists_sensor(missing_sensors, "mic1")
        || log_lc.contains("mic1")
        || (log_lc.contains("thermalmonitord")
            && (log_lc.contains("smc panic")
                || log_lc.contains("0x80000")
                || log_lc.contains("524288")
                || log_lc.contains("0x300000")
                || log_lc.contains("3145728")))
}


pub fn apply_repair_wiki_correlations(
    acc: &mut std::collections::HashMap<String, f64>,
    checks: &mut std::collections::HashSet<String>,
    product_from_log: Option<&str>,
    device_hint: Option<&str>,
    soc_id: Option<&str>,
    log_lc: &str,
    log_window: &str,
    missing_sensors: &[String],
) -> Vec<String> {
    let mut ui_lines = Vec::new();
    append_ifixit_kernel_panic_hints(log_lc, checks);

    let pc = product_lc(product_from_log, device_hint);
    let product_for_chart = if pc.is_empty() {
        None
    } else {
        Some(pc.as_str())
    };
    crate::panic_logs_chart::apply_panic_logs_chart_pdf(
        log_lc,
        product_for_chart,
        soc_id,
        acc,
        checks,
        &mut ui_lines,
    );
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

    // â”€â”€ Ancienne gÃ©nÃ©ration + SE 2020 : capteurs Â« Missing sensor(s): â€¦ Â» en texte â”€â”€
    if legacy_mic_tier {
        if is_iphone11_family(&pc) {
            if log_lc.contains("tg0b") || log_lc.contains("tg0v") {
                let n = cause_label(IPHONE11_TG_MISSING_CAUSE);
                push_acc(acc, &n, 0.92);
                ui_lines.push(format!("Capteur TG Â· {IPHONE11_TG_MISSING_CAUSE}"));
                checks.insert(
                    "TG0B/TG0V 11 : batterie/FPC pile Â· diode I2C (SCL/SDA) Â· R3201/R3202 Â· OL â†’ piste CPU ; CC â†’ CPU."
                        .into(),
                );
                if matches!(pc.as_str(), "iphone12,3" | "iphone12,5") {
                    checks.insert(
                        "TG0V/TG0B 11 Pro / Max : iFixit â€” capteurs batterie ; aussi ensemble port de charge sur carte."
                            .into(),
                    );
                }
            }
            if log_lc.contains("prs0") || log_lc.contains("mic1") {
                let n = cause_label(IPHONE11_PRS_MIC1_CAUSE);
                push_acc(acc, &n, 0.9);
                ui_lines.push(format!("Capteur PRS/MIC1 Â· {IPHONE11_PRS_MIC1_CAUSE}"));
                checks.insert(
                    "PRS0/MIC1 11 : cas iFixit/terrain â€” nappe charge OEM/Premium Â· FPC dock Â· liquide Â· R6822/R6823 I2C1 AOP."
                        .into(),
                );
            }
            if missing_lists_sensor(missing_sensors, "mic2") {
                let n = cause_label(IPHONE11_MIC2_CAUSE);
                push_acc(acc, &n, 0.93);
                ui_lines.push(format!("Capteur mic2 Â· {IPHONE11_MIC2_CAUSE}"));
                checks.insert(
                    "MIC2 Â· iPhone 11 : nappe bouton power (micro flash) â€” oxydation connecteur, FPC mal clipÃ©, vitre arriÃ¨re / flash, chute ou liquide. Pas l'Ã©couteur en premier."
                        .into(),
                );
                if log_lc.contains("thermalmonitord") {
                    let flash = cause_label(IPHONE11_MIC2_THERMAL_FLASH_POWER_CAUSE);
                    push_acc(acc, &flash, 0.65);
                    ui_lines.push(format!("Flash / power Â· {IPHONE11_MIC2_THERMAL_FLASH_POWER_CAUSE}"));
                    checks.insert("iPhone 11 + thermal : prioriser Power + flash.".into());
                }
            }
            checks.insert(
                "iPhone 11 : reboot â€” dock, puis Power+flash ; si carte double, interposer. SÃ©rie 12+ = autre profil Â« connecteur Â».".into(),
            );
            checks.insert(
                "iPhone 11 : capteur citÃ© dans le log en premier ; nappes avant carte.".into(),
            );
            checks.insert(
                "Liquide / oxy / pry : Ã©liminer avant de conclure carte mÃ¨re.".into(),
            );
            checks.insert(
                "Aftermarket 11 : fort taux dâ€™Ã©chec â€” valider avec OEM/Premium.".into(),
            );
            if log_lc.contains("boot") || log_lc.contains("loop") {
                checks.insert(
                    "iPhone 11 : boot ~3 min + capteur â€” dock puis Power+flash.".into(),
                );
            }
            if log_lc.contains("bsc failure") || log_lc.contains("bsc ") {
                checks.insert(
                    "11 Pro / Max : SMC BSC failure â€” souvent ligne batterie / fuel gauge ; ne pas assimiler Ã  PMIC mort sans preuve."
                        .into(),
                );
            }
        } else {
            if is_iphone_x_only(&pc)
                && log_lc.contains("thermalmonitord")
                && log_lc.contains("mic1")
            {
                let n = cause_label(
                    "iPhone X Â· thermalmonitord + MIC1 Â· nappe charge / dock / micro bas (trÃ¨s fiable atelier)",
                );
                push_acc(acc, &n, 0.91);
                ui_lines.push(n.clone());
                checks.insert(
                    "Faux ami : thermalmonitord â‰  chauffe CPU â€” souvent capteur absent ou bus dock (X)."
                        .into(),
                );
            }

            if is_iphone_x_only(&pc) && log_lc.contains("smc panic") {
                let n = cause_label(
                    "iPhone X Â· SMC PANIC ASSERT Â· batterie, dock ou court PPBUS (fiabilitÃ© moyenne â€” croiser gas gauge / capteurs)",
                );
                push_acc(acc, &n, 0.72);
                ui_lines.push(n.clone());
            }

            if is_iphone_xs_xs_max(&pc)
                && missing_lists_sensor(missing_sensors, "mic2")
                && (log_lc.contains("boot") || log_lc.contains("loop"))
            {
                checks.insert(
                    "XS / XS Max : bootloop aprÃ¨s changement Ã©couteur + missing mic2 = signature atelier trÃ¨s connue."
                        .into(),
                );
            }

            if log_lc.contains("prs0") {
                let (lab, wt): (&str, f64) = if is_iphone_xr_only(&pc) {
                    (
                        "PRS0 Â· iPhone XR Â· recharge sans fil / pression / flex pÃ©riphÃ©rique (atelier)",
                        0.87,
                    )
                } else {
                    ("PRS0 Â· problÃ¨me liÃ© Ã  la nappe du connecteur de charge", 0.86)
                };
                let n = cause_label(lab);
                push_acc(acc, &n, wt);
                ui_lines.push(format!("Capteur prs0 Â· {lab}"));
            }
            for (key, lab, wt) in [
                ("tg0b", "TG0B Â· problÃ¨me de batterie / fuel gauge", 0.84_f64),
                ("tg0v", "TG0V Â· problÃ¨me de batterie", 0.82),
            ] {
                if log_lc.contains(key) {
                    let n = cause_label(lab);
                    push_acc(acc, &n, wt);
                    ui_lines.push(format!("Capteur {key} Â· {lab}"));
                }
            }

            if tier == RwTier::Se2020SpecialMic && log_lc.contains("mic1") {
                let n = cause_label(
                    "SE 2020 MIC1 Â· nappe connecteur de charge d'abord, puis lignes I2C1 / tactile si connu bon",
                );
                push_acc(acc, &n, 0.92);
                ui_lines.push("SE2020 MIC1 Â· dock OEM/Premium connu bon puis diode I2C1_AP_SCL/SDA".into());
                checks.insert(
                    "SE 2020 / mic1 â€” cas Repair Wiki : tester dock connu bon, puis diode I2C1_AP_SCL/SDA ; ne pas condamner carte avant isolement.".into(),
                );
            } else if tier == RwTier::LegacyTextMic && log_lc.contains("mic1") {
                let mic1_lab = if is_iphone_xs_xs_max(&pc) || is_iphone_xr_only(&pc) || is_iphone12_apple_ids(&pc) {
                    "MIC1 Â· nappe dock / connecteur de charge (corrÃ©lation forte X / XS / XR / 12)"
                } else {
                    "MIC1 Â· problÃ¨me liÃ© Ã  la nappe du connecteur de charge"
                };
                let n = cause_label(mic1_lab);
                push_acc(acc, &n, 0.86);
                ui_lines.push(format!("MIC1 Â· {mic1_lab}"));
            }

            // iPhone 12 (ex. codename Â« Eiger Â» dans certains logs) : nappe connecteur de charge
            if tier == RwTier::LegacyTextMic
                && matches!(pc.as_str(), "iphone13,1" | "iphone13,2" | "iphone13,3" | "iphone13,4")
                && log_lc.contains("eiger")
            {
                let n = cause_label("Eiger Â· iPhone 12 Â· nappe du connecteur de charge");
                push_acc(acc, &n, 0.87);
                ui_lines.push("Eiger (iPhone 12) Â· nappe connecteur de charge".into());
            }

            let mic2_lab =
                if is_iphone12_apple_ids(&pc) {
                    "MIC2 Â· iPhone 12 (famille) Â· Ã©couteur interne / prÃ©-ensemble avant (corrÃ©lation atelier)"
                } else if is_iphone_x_only(&pc) {
                    "MIC2 Â· iPhone X Â· Ã©couteur interne / nappe capteurs avant"
                } else if is_iphone_xs_xs_max(&pc) || is_iphone_xr_only(&pc) {
                    "MIC2 Â· XS / XR Â· Ã©couteur interne / capteurs avant"
                } else {
                    "MIC2 Â· microphone sur nappe du bouton power (ou nappe dock selon modÃ¨le)"
                };
            let mic2_w = if is_iphone12_apple_ids(&pc) {
                0.9_f64
            } else if is_iphone_x_only(&pc) || is_iphone_xs_xs_max(&pc) || is_iphone_xr_only(&pc) {
                0.88
            } else {
                0.86
            };
            if missing_lists_sensor(missing_sensors, "mic2") {
                let n = cause_label(mic2_lab);
                push_acc(acc, &n, mic2_w);
                ui_lines.push(format!("MIC2 Â· {mic2_lab}"));
            }

            checks.insert("VÃ©rifier nappes OEM sur capteurs texte MIC/PRS/TG".into());

            if is_iphone12_apple_ids(&pc) {
                checks.insert(
                    "SÃ©rie 12 : reboot ~3 min + capteur â€” corrÃ©lation atelier forte (>80 %) dock ou Ã©couteur."
                        .into(),
                );
            }
        }

        if log_lc.contains("ans2") {
            let n = cause_label("ANS2 Â· NAND ou contrÃ´leur stockage / liaison CPUâ€“NAND (corrosion possible)");
            push_acc(acc, &n, 0.87);
            ui_lines.push("ANS2 prÃ©sent dans le log".into());
            if tier == RwTier::Series13Identifiers {
                checks.insert(
                    "SÃ©rie 13 : aprÃ¨s sÃ©paration carte, ANS2 â†’ isoler soudure NAND / interposer / rails donnÃ©es."
                        .into(),
                );
            }
        }
    }

    let masks = sensor_array_nonzero_decimal_masks(log_window);

    if log_lc.contains("smc panic") && log_lc.contains("taop") && log_lc.contains("taoj") {
        checks.insert(
            "SMC + TAOP/TAOJ + OUTBOX : souvent nappe Qi/MagSafe ou USBâ€‘C mal enclenchÃ©e â€” reprendre ces FPC avant carte."
                .into(),
        );
        ui_lines.push(
            "Motif SMC BSC + TAOP/TAOJ : penser charge sans fil / MagSafe et nappe USBâ€‘C (alignÃ© masque 0x280000)."
                .into(),
        );
    }

    if masks.is_empty() && legacy_mic_tier {
        ui_lines.push(
            "(Capteurs en texte seulement sur cette gÃ©nÃ©ration â€” pas de masque S.sensor lisible)".into(),
        );
    }

    if masks.is_empty() && !legacy_mic_tier {
        ui_lines.push("Aucune valeur bitmask non nulle lisible dans sensor array â€” coller panic-full complet aide.".into());
    }

    let mut decoded = HashSet::new();

    for &val in &masks {
        let explanations = explanations_for_mask(val, tier, log_lc, &pc);
        for (explain, wt) in explanations {
            if decoded.insert(explain.clone()) {
                ui_lines.push(format!("Mask 0x{val:x} ({val}) Â· {explain}"));
            }
            push_acc(acc, &cause_label(explain.clone()), wt);
        }
    }

    if tier == RwTier::Series13Identifiers && log_lc.contains("ans2") {
        checks.insert(
            "SÃ©rie 13 : aprÃ¨s sÃ©paration carte, ANS2 â†’ NAND / interposer / rails donnÃ©es (atelier)."
                .into(),
        );
    }

    if matches!(
        tier,
        RwTier::Iphone1414Plus | RwTier::Iphone14ProSeries | RwTier::Iphone1515Plus | RwTier::Iphone15ProSeries | RwTier::Iphone16Series
    ) {
        if log_lc.contains("userspace watchdog") {
            checks.insert(
                "SÃ©rie 14/15 : userspace watchdog â€” signature parfois piÃ©gÃ©e ; Ã©carter bug iOS / restauration avant hardware seul."
                    .into(),
            );
        }
        if log_lc.contains("baseband panic") {
            checks.insert(
                "Baseband panic 14/15 : RF, alim BB, eSIM ou iOS â€” ne pas conclure hardware sans contexte."
                    .into(),
            );
        }
    }

    if matches!(tier, RwTier::Iphone1515Plus | RwTier::Iphone15ProSeries)
        && iphone15_bottom_mic_signal(log_lc, missing_sensors)
    {
        let thermal = log_lc.contains("thermalmonitord");
        let mut w = if missing_lists_sensor(missing_sensors, "mic1") && thermal {
            0.97_f64
        } else if missing_lists_sensor(missing_sensors, "mic1") || log_lc.contains("mic1") {
            0.95
        } else {
            0.9
        };
        if log_suggests_liquid_or_oxidation(log_lc) {
            w = w.max(0.98);
            let ox = cause_label(
                "Oxydation module micro bas / connecteur flex USB-C (trÃ¨s frÃ©quent sÃ©rie 15)",
            );
            push_acc(acc, &ox, 0.94);
            ui_lines.push(format!("Liquide / oxy Â· {ox}"));
        }
        let n = cause_label(IPHONE15_BOTTOM_MIC_MODULE_CAUSE);
        push_acc(acc, &n, w);
        ui_lines.push(format!("Module micro bas Â· {IPHONE15_BOTTOM_MIC_MODULE_CAUSE}"));
        checks.insert(format!(
            "SÃ©rie 15 Â· hardware : {IPHONE15_BOTTOM_MIC_HARDWARE_NOTE}."
        ));
        checks.insert(
            "Module micro bas : reseat clip sur flex USB-C, joint acoustique, nettoyage ultrason si oxydÃ© ; assembler USB-C OEM.".into(),
        );
        if thermal {
            checks.insert(
                "thermalmonitord sur iPhone 15 â‰  surchauffe CPU : prioriser MIC1 / lignes capteurs du module bas.".into(),
            );
        }
        checks.insert(
            "Le tÃ©lÃ©phone peut encore charger (VBUS OK) alors que MIC1 / capteurs du flex bas sont absents.".into(),
        );
    }

    if masks.is_empty() {
        return ui_lines;
    }

    checks.insert("ContrÃ´ler tableau capteurs S.sensor avec masques hex.".into());
    if tier == RwTier::Series13Identifiers {
        checks.insert(
            "SÃ©rie 13 SMC + sensor array : aprÃ¨s chaque reboot ~3 min, noter le code hex sous Sensor Array.".into(),
        );
        checks.insert(
            "Ã€ contrÃ´ler : nappe flash Â· nappe bouton power Â· proximitÃ© Â· charge Â· batterie Â· liquide Â· connecteurs arrachÃ©s.".into(),
        );
        checks.insert(
            "Aftermarket dÃ©clenche souvent ces paniques â€” prÃ©fÃ©rer OEM ou Premium pour tester.".into(),
        );
    }
    if tier == RwTier::Iphone1515Plus {
        checks.insert(
            "15 / 15 Plus : lire le nombre exact sous S.sensor array (ex. 2621440 = 0x280000) â€” prioritÃ© sur dÃ©tail bitwise."
                .into(),
        );
        checks.insert(
            "AprÃ¨s swap : derniÃ¨re piÃ¨ce en cause en premier ; aftermarket nappes = risque."
                .into(),
        );
        checks.insert(
            "Avant carte : FPC recharge + USBâ€‘C + liquide Â· sandwich au doute.".into(),
        );
    }

    ui_lines
}

#[allow(clippy::too_many_lines)]
fn explanations_for_mask(val: u64, tier: RwTier, log_lc: &str, pc: &str) -> Vec<(String, f64)> {
    let mut out: Vec<(String, f64)> = Vec::new();

    // â”€â”€ Valeurs composites exactes (prioritÃ© Wiki par sÃ©rie) â”€â”€
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
                "0x800 (2048) Â· connecteur de charge",
                0.89,
            );
            try_push(
                val == 0x1000 || val == 4096,
                "0x1000 (4096) Â· capteur de proximitÃ©",
                0.88,
            );
            try_push(
                val == 0x1800 || val == 6144,
                "0x1800 (6144) Â· double dÃ©faut : proximitÃ© + connecteur de charge",
                0.91,
            );
            try_push(
                val == 0x4000 || val == 16_384,
                "0x4000 (16384) Â· capteur / donnÃ©es batterie",
                0.87,
            );
            if mini {
                try_push(
                    val == 0x400 || val == 1024,
                    "0x400 (1024) Â· iPhone 13 mini Â· gyroscope",
                    0.93,
                );
                try_push(
                    val == 0xc00 || val == 3072,
                    "0xC00 (3072) Â· iPhone 13 mini Â· bottom board + nappe port de charge",
                    0.93,
                );
            }
        }
        RwTier::Iphone1414Plus => {
            try_push(
                val == 0x400000 || val == 4_194_304,
                "0x400000 (4194304) Â· iPhone 14 / 14 Plus Â· nappe recharge sans fil (Qi) / vitre arriÃ¨re",
                0.91,
            );
            try_push(
                val == 0x100000 || val == 1_048_576,
                "0x100000 (1048576) Â· iPhone 14 / 14 Plus Â· nappe connecteur de charge (Lightning)",
                0.9,
            );
            try_push(
                val == 0x500000 || val == 5_242_880,
                "0x500000 (5242880) Â· iPhone 14 / 14 Plus Â· communication batterie â€” peut aussi Ãªtre Taptic Engine ou nappe de charge",
                0.86,
            );
            try_push(
                val == 0x200000 || val == 2_097_152,
                "0x200000 (2097152) Â· iPhone 14 / 14 Plus Â· nappe capteur de proximitÃ©",
                0.89,
            );
            try_push(
                val == 0x600000 || val == 6_291_456,
                "0x600000 (6291456) Â· iPhone 14 / 14 Plus Â· double dÃ©faut : recharge sans fil + proximitÃ©",
                0.92,
            );
            try_push(
                val == 0x20000 || val == 131_072,
                "0x20000 (131072) Â· iPhone 14 / 14 Plus Â· problÃ¨me carte mÃ¨re (logic board)",
                0.84,
            );
        }
        RwTier::Iphone14ProSeries => {
            try_push(
                val == 0x80000 || val == 524_288,
                "0x80000 (524288) Â· iPhone 14 Pro / Pro Max Â· nappe proximitÃ©",
                0.91,
            );
            try_push(
                val == 0x40000 || val == 262_144,
                "0x40000 (262144) Â· iPhone 14 Pro / Pro Max Â· nappe connecteur de charge",
                0.9,
            );
            try_push(
                val == 0x100000 || val == 1_048_576,
                "0x100000 (1048576) Â· iPhone 14 Pro / Pro Max Â· nappe bouton Power",
                0.9,
            );
            try_push(
                val == 0xC0000 || val == 786_432,
                "0xC0000 (786432) Â· iPhone 14 Pro / Pro Max Â· double dÃ©faut : proximitÃ© + connecteur de charge",
                0.93,
            );
            try_push(
                val == 0x180000 || val == 1_572_864,
                "0x180000 (1572864) Â· iPhone 14 Pro / Pro Max Â· double dÃ©faut : proximitÃ© + bouton Power",
                0.93,
            );
            try_push(
                val == 0x140000 || val == 1_310_720,
                "0x140000 (1310720) Â· iPhone 14 Pro / Pro Max Â· double dÃ©faut : bouton Power + connecteur de charge",
                0.93,
            );
            try_push(
                val == 0x1C0000 || val == 1_835_008,
                "0x1C0000 (1835008) Â· iPhone 14 Pro / Pro Max Â· triple dÃ©faut : proximitÃ© + bouton Power + connecteur de charge",
                0.94,
            );
            try_push(
                val == 0x20000 || val == 131_072,
                "0x20000 (131072) Â· iPhone 14 Pro / Pro Max Â· sandwich board / sÃ©paration carte mÃ¨re",
                0.85,
            );
            try_push(
                val == 0xA0000 || val == 655_360,
                "0xA0000 (655360) Â· iPhone 14 Pro / Pro Max Â· sandwich board + nappe proximitÃ©",
                0.88,
            );
            try_push(
                val == 0x41 || val == 65,
                "0x41 (65) Â· iPhone 14 Pro / Pro Max Â· donnÃ©es batterie",
                0.87,
            );
        }
        RwTier::Iphone1515Plus => {
            // Masques composites en premier (valeur exacte lue sous S.sensor array).
            try_push(
                val == 0x380000 || val == 3_670_016,
                "0x380000 (3670016) Â· iPhone 15 / 15 Plus Â· triple dÃ©faut : recharge sans fil + connecteur de charge + proximitÃ©",
                0.94,
            );
            try_push(
                val == 0x280000 || val == 2_621_440,
                "0x280000 (2621440) Â· iPhone 15 / 15 Plus Â· double dÃ©faut : recharge sans fil + connecteur de charge",
                0.93,
            );
            try_push(
                val == 0x200000 || val == 2_097_152,
                "0x200000 (2097152) Â· iPhone 15 / 15 Plus Â· recharge sans fil (nappe Qi / vitre arriÃ¨re)",
                0.9,
            );
            try_push(
                val == 0x80000 || val == 524_288,
                "0x80000 (524288) Â· iPhone 15 / 15 Plus Â· assemblage USB-C + module micro bas MIC1 (PCB MEMS) / baromÃ¨tre",
                0.93,
            );
            try_push(
                val == 0x100000 || val == 1_048_576,
                "0x100000 (1048576) Â· iPhone 15 / 15 Plus Â· nappe proximitÃ© / capteurs avant",
                0.88,
            );
            try_push(
                val == 0xa1 || val == 161,
                "0xA1 (161) Â· iPhone 15 / 15 Plus Â· capteur ou donnÃ©es batterie",
                0.9,
            );
        }
        RwTier::Iphone15ProSeries => {
            try_push(
                val == 0xa1 || val == 161,
                "0xA1 (161) Â· iPhone 15 Pro / Pro Max Â· capteur ou donnÃ©es batterie",
                0.9,
            );
            try_push(
                val == 0x700000 || val == 7_340_032,
                "0x700000 (7340032) Â· iPhone 15 Pro / Pro Max Â· double dÃ©faut : connecteur de charge + recharge sans fil",
                0.92,
            );
            try_push(
                val == 0x600000 || val == 6_291_456,
                "0x600000 (6291456) Â· iPhone 15 Pro / Pro Max Â· double dÃ©faut : proximitÃ© + recharge sans fil",
                0.91,
            );
            try_push(
                val == 0x400000 || val == 4_194_304,
                "0x400000 (4194304) Â· iPhone 15 Pro / Pro Max Â· recharge sans fil (nappe Qi arriÃ¨re)",
                0.9,
            );
            try_push(
                val == 0x300000 || val == 3_145_728,
                "0x300000 (3145728) Â· iPhone 15 Pro / Pro Max Â· assemblage USB-C + module micro bas MIC1 (PCB MEMS)",
                0.93,
            );
            try_push(
                val == 0x100000 || val == 1_048_576,
                "0x100000 (1048576) Â· iPhone 15 Pro / Pro Max Â· nappe connecteur de charge USBâ€‘C (autre bus mÃªme famille)",
                0.87,
            );
            try_push(
                val == 0x200000 || val == 2_097_152,
                "0x200000 (2097152) Â· iPhone 15 Pro / Pro Max Â· nappe proximitÃ©",
                0.88,
            );
            try_push(val == 0x280000 || val == 2_621_440, "0x280000 (2621440) Â· iPhone 15 Pro / Pro Max Â· Qi + ligne charge USBâ€‘C", 0.82);
        }
        RwTier::Iphone16Series => {
            try_push(
                val == 0xa1 || val == 161,
                "0xA1 (161) Â· iPhone 16 Â· bus donnÃ©es batterie",
                0.89,
            );
            try_push(
                val == 0x200000 || val == 2_097_152,
                "0x200000 (2097152) Â· iPhone 16 Â· bobine MagSafe / Qi arriÃ¨re",
                0.90,
            );
            try_push(
                val == 0x300000 || val == 3_145_728,
                "0x300000 (3145728) Â· iPhone 16 Â· assemblage USB-C + port de charge",
                0.89,
            );
            try_push(
                val == 0x400000 || val == 4_194_304,
                "0x400000 (4194304) Â· iPhone 16 Â· bobine Qi / MagSafe arriÃ¨re (nappe vitre)",
                0.91,
            );
            try_push(
                val == 0x700000 || val == 7_340_032,
                "0x700000 (7340032) Â· iPhone 16 Â· USB-C + Qi ensemble",
                0.92,
            );
            try_push(
                val == 0x280000 || val == 2_621_440,
                "0x280000 (2621440) Â· iPhone 16 Â· Qi / MagSafe + USB-C",
                0.91,
            );
            try_push(
                val == 0x380000 || val == 3_670_016,
                "0x380000 (3670016) Â· iPhone 16 Â· Qi + USB-C + proximitÃ© / capteurs faÃ§ade",
                0.93,
            );
            try_push(
                val == 0x500000 || val == 5_242_880,
                "0x500000 (5242880) Â· iPhone 16 Â· batterie / bus alimentation",
                0.87,
            );
            try_push(
                val == 0x100000 || val == 1_048_576,
                "0x100000 (1048576) Â· iPhone 16 Â· nappe capteurs avant / proximitÃ©",
                0.88,
            );
            try_push(
                val == 0x600000 || val == 6_291_456,
                "0x600000 (6291456) Â· iPhone 16 Â· double dÃ©faut : recharge sans fil + capteurs avant",
                0.90,
            );
            try_push(
                val == 0x80000 || val == 524_288,
                "0x80000 (524288) Â· iPhone 16 Â· nappe bouton / Action button",
                0.88,
            );
        }
        _ => {}
    }

    // Fallback universel 13+ (bitwise) â€” jamais pour gÃ©nÃ©rations Â« texte mic Â» ni SE 2020
    if !matches!(
        tier,
        RwTier::LegacyTextMic | RwTier::Se2020SpecialMic
    ) {
        universal_bitwise_fr(val, log_lc, &mut out);
    }

    out
}

fn universal_bitwise_fr(val: u64, _log_lc: &str, out: &mut Vec<(String, f64)>) {
    // Table Â« universelle Â» iPhone 13 et + (bitwise, si aucun masque composite sÃ©rie nâ€™a dÃ©jÃ  matchÃ©).
    const BITS: &[(u64, &str, f64)] = &[
        (0x20, "Circuit de charge", 0.62),
        (0x40, "Gas gauge batterie", 0.65),
        (0x41, "DonnÃ©es batterie", 0.66),
        (0xa1, "Capteur batterie", 0.68),
        (0xa9, "Variante erreur donnÃ©es batterie", 0.65),
        (0x400, "Gyroscope", 0.6),
        (0x800, "Connecteur de charge", 0.68),
        (0x1000, "Capteur proximitÃ©", 0.66),
        (0x4000, "Capteur batterie", 0.64),
        (0x20000, "Gyroscope (souvent iPhone 14 Pro et +) ou carte mÃ¨re selon modÃ¨le", 0.58),
        (0x40000, "Connecteur de charge (souvent iPhone 14 Pro et +)", 0.62),
        (0x80000, "Capteur proximitÃ© (iPhone 14 â†’ 17, selon contexte)", 0.65),
        (0x100000, "Bouton Power / bus associÃ© selon modÃ¨le", 0.62),
        (0x200000, "Capteur avant / recharge sans fil selon modÃ¨le", 0.68),
        (0x300000, "USBâ€‘C (modÃ¨les Pro)", 0.67),
        (0x400000, "Bobine recharge sans fil", 0.69),
        (0x500000, "Batterie", 0.61),
        (0x700000, "USBâ€‘C + recharge sans fil (bus commun)", 0.7),
    ];

    // Si aucune ligne exact composite nâ€™a rempli Â« out Â», dÃ©composer bitwise
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
        let sl = |s: &str| s.to_lowercase();
        assert!(
            ex.iter().any(|(s, _)| {
                s.contains("0x380000")
                    && sl(s).contains("triple")
                    && sl(s).contains("proximit")
                    && (sl(s).contains("sans fil") || sl(s).contains("recharge"))
            }),
            "{ex:?}"
        );
    }

    #[test]
    fn iphone15_plus_mask_0xa1_battery_data() {
        let ex = explanations_for_mask(161, RwTier::Iphone1515Plus, "", "");
        assert!(
            ex.iter().any(|(s, _)| {
                s.to_lowercase().contains("0xa1") && s.to_lowercase().contains("batter")
            }),
            "{ex:?}"
        );
    }

    #[test]
    fn iphone15_bottom_mic_module_when_mic1_and_thermal() {
        let mut acc = HashMap::new();
        let mut checks = HashSet::new();
        let log = concat!(
            "producttype iphone15,4\n",
            "no successful checkins from thermalmonitord\n",
            "missing sensor(s): mic1\n",
        );
        let missing = crate::panic_parser::parse_panic_log(log).missing_sensors;
        let ui = apply_repair_wiki_correlations(
            &mut acc,
            &mut checks,
            Some("iPhone15,4"),
            None,
            None,
            log,
            log,
            missing.as_slice(),
        );
        assert!(
            acc.keys().any(|k| k.contains("Module micro du bas")),
            "{acc:?}"
        );
        assert!(
            checks.iter().any(|c| c.contains("MEMS")),
            "{checks:?}"
        );
        assert!(ui.iter().any(|l| l.contains("Module micro bas")), "{ui:?}");
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
            None,
            "smc panic assertion failed",
            log,
            &[],
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
        let missing = crate::panic_parser::parse_panic_log(log_lc).missing_sensors;
        let ui = apply_repair_wiki_correlations(
            &mut acc,
            &mut checks,
            Some("iPhone12,1"),
            None,
            None,
            log_lc,
            "",
            missing.as_slice(),
        );
        assert!(acc.keys().any(|k| k.contains("TG0B") && k.contains("TG0V")), "{acc:?}");
        assert!(acc.keys().any(|k| k.contains("PRS0") && k.contains("MIC1")), "{acc:?}");
        assert!(
            acc.keys().any(|k| {
                let l = k.to_lowercase();
                l.contains("mic2")
                    && l.contains("iphone 11")
                    && (l.contains("power") || l.contains("flash") || l.contains("bouton") || l.contains("micro"))
            }),
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
    fn iphone11_mic2_not_triggered_by_mic2_substring_without_missing_line() {
        let mut acc = HashMap::new();
        let mut checks = HashSet::new();
        let log_lc = concat!(
            "thermalmonitord complained\n",
            "No successful checkins from thermalmonitord\n",
            "AppleSocHot\n",
            "producttype iphone12,1\n",
            "someblobmic2noise_without_missing_declaration\n",
            "missing sensor(s): tg0v\n",
        );
        let missing = crate::panic_parser::parse_panic_log(log_lc).missing_sensors;
        assert!(
            missing.iter().all(|s| s != "mic2"),
            "fixture doit Ãªtre tg0v seulement Â· missing={missing:?}"
        );
        apply_repair_wiki_correlations(
            &mut acc,
            &mut checks,
            Some("iPhone12,1"),
            None,
            None,
            log_lc,
            "",
            missing.as_slice(),
        );
        assert!(
            !acc.keys().any(|k| {
                let l = k.to_lowercase();
                l.contains("iphone 11")
                    && l.contains("mic2")
                    && (l.contains("power")
                        || l.contains("flash")
                        || l.contains("Ã©couteur")
                        || l.contains("earpiece"))
            }),
            "MIC2 iPhone 11 ne doit pas apparaÃ®tre sans mic2 sur Missing sensor(s): Â· acc={acc:?}"
        );
    }

    #[test]
    fn iphone13_mini_sensor_mask_0x400_bottom_board_not_regular_13() {
        let mini = explanations_for_mask(1024, RwTier::Series13Identifiers, "", "iphone14,4");
        assert!(
            mini.iter().any(|(s, _)| {
                s.to_lowercase().contains("iphone 13 mini") && s.to_lowercase().contains("gyroscope")
            }),
            "{mini:?}"
        );

        let regular = explanations_for_mask(1024, RwTier::Series13Identifiers, "", "iphone14,5");
        assert!(
            !regular
                .iter()
                .any(|(s, _)| s.to_lowercase().contains("iphone 13 mini")),
            "0x400 ne doit pas forcer scÃ©nario mini hors iPhone14,4: {regular:?}"
        );
    }

    #[test]
    fn iphone13_mini_0xc00_combo_bottom_and_dock() {
        let ex = explanations_for_mask(3072, RwTier::Series13Identifiers, "", "iphone14,4");
        let sl = |s: &str| s.to_lowercase();
        assert!(
            ex.iter().any(|(s, _)| {
                s.contains("0xC00")
                    && sl(s).contains("bottom board")
                    && sl(s).contains("port de charge")
            }),
            "{ex:?}"
        );
    }

    #[test]
    fn iphone13_battery_data_mask_0x4000() {
        let ex = explanations_for_mask(16_384, RwTier::Series13Identifiers, "", "iphone14,5");
        assert!(
            ex.iter().any(|(s, _)| s.contains("0x4000") && s.to_lowercase().contains("batter")),
            "{ex:?}"
        );
    }

    #[test]
    fn iphone11_mic2_strong_correlation() {
        let mut acc = HashMap::new();
        let mut checks = HashSet::new();
        let log_lc = "missing sensor(s): mic2";
        let missing = crate::panic_parser::parse_panic_log(log_lc).missing_sensors;
        apply_repair_wiki_correlations(
            &mut acc,
            &mut checks,
            Some("iPhone12,1"),
            None,
            None,
            log_lc,
            "",
            missing.as_slice(),
        );
        assert!(acc.iter().any(|(k, _)| {
            let l = k.to_lowercase();
            l.contains("mic2")
                && l.contains("iphone 11")
                && (l.contains("power") || l.contains("flash") || l.contains("bouton"))
        }));
    }

    #[test]
    fn iphone11_mic2_thermalmonitord_adds_flash_power_lead() {
        let mut acc = HashMap::new();
        let mut checks = HashSet::new();
        let log_lc = "missing sensor(s): mic2 thermalmonitord complained";
        let missing = crate::panic_parser::parse_panic_log(log_lc).missing_sensors;
        let ui = apply_repair_wiki_correlations(
            &mut acc,
            &mut checks,
            Some("iPhone12,1"),
            None,
            None,
            log_lc,
            "",
            missing.as_slice(),
        );
        let joined_acc = acc.keys().map(|k| k.to_lowercase()).collect::<Vec<_>>().join(" | ");
        assert!(
            joined_acc.contains("flash") || joined_acc.contains("bouton power"),
            "attendu piste flash/power secondaire Â· acc={acc:?}"
        );
        assert!(
            checks.iter().any(|c| {
                let l = c.to_lowercase();
                l.contains("flash") && (l.contains("thermalmonitord") || l.contains("thermal"))
            }),
            "{checks:?}"
        );
        assert!(
            ui.iter().any(|l| l.to_lowercase().contains("flash")),
            "{ui:?}"
        );
    }

    #[test]
    fn se2020_mic1_special_case() {
        let mut acc = HashMap::new();
        let mut checks = HashSet::new();
        let log_lc = "missing sensor(s): mic1";
        let missing = crate::panic_parser::parse_panic_log(log_lc).missing_sensors;
        let ui = apply_repair_wiki_correlations(
            &mut acc,
            &mut checks,
            Some("iPhone12,8"),
            None,
            None,
            log_lc,
            "",
            missing.as_slice(),
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
            None,
            "smc panic bsc taop taoj",
            "",
            &[],
        );
        assert!(
            checks.iter().any(|c| c.contains("Qi") || c.contains("MagSafe")),
            "{checks:?}"
        );
    }
}
