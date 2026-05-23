
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use regex::Regex;

fn i2c_bus_regex() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"(?i)\bi2c[012]\b").expect("regex i2c"))
}

fn append_i2c_chart_bus_hint(
    log_lc: &str,
    product_apple_lc: Option<&str>,
    soc_id: Option<&str>,
    checks: &mut HashSet<String>,
) {
    if !i2c_bus_regex().is_match(log_lc) {
        return;
    }
    let mut msg = "Chart PANIC LOGS : bus IÂ²C citÃ© dans le log â€” croiser tableau PDF (nappe / connecteurs par SoC).".to_string();
    if let Some(p) = product_apple_lc.map(str::trim).filter(|s| !s.is_empty()) {
        msg.push_str(&format!(" ProductType: {p}."));
    }
    if let Some(s) = soc_id.map(str::trim).filter(|s| !s.is_empty()) {
        msg.push_str(&format!(" socId: {s}."));
    }
    checks.insert(msg);
}

struct Row {
    needle: &'static str,
    cause_fr: &'static str,
    weight: f64,
    check_fr: &'static str,
}

const ROWS: &[Row] = &[
    Row {
        needle: "ememory",
        cause_fr: "eMemory / stockage NAND ou circuit disque (PANIC LOGS chart)",
        weight: 0.68,
        check_fr: "Chart : eMemory â†’ NAND / disque ; croiser ANS2 / NVMe.",
    },
    Row {
        needle: "anc-postnand",
        cause_fr: "Lien NAND post-init (anc-postnand â€” chart)",
        weight: 0.7,
        check_fr: "Chart : anc-postnand â†’ erreur NAND / liaison ; microsoudure / interposer.",
    },
    Row {
        needle: "applebcmwlan",
        cause_fr: "Wiâ€‘Fi / Bluetooth (AppleBCMWLAN â€” chart)",
        weight: 0.64,
        check_fr: "Chart : AppleBCMWLAN â†’ module Wiâ€‘Fi/BT, antenne, RF.",
    },
    Row {
        needle: "amcc error",
        cause_fr: "Capteur luminositÃ© / AMCC (chart)",
        weight: 0.58,
        check_fr: "Chart : AMCC Error â†’ capteur lumiÃ¨re / ligne associÃ©e.",
    },
    Row {
        needle: "pmp nmi fiq",
        cause_fr: "Alimentation CPU (PMP NMI FIQ â€” chart)",
        weight: 0.64,
        check_fr: "Chart : PMP NMI FIQ â†’ rails CPU, inductances autour SoC.",
    },
    Row {
        needle: "apple ppm",
        cause_fr: "Apple PPM â€” charge / dÃ©charge (chart)",
        weight: 0.6,
        check_fr: "Chart : Apple PPM â†’ IC charge, conversion batterie, BMS.",
    },
    Row {
        needle: "apple pmgr",
        cause_fr: "Apple PMGR â€” gestion alim (chart)",
        weight: 0.6,
        check_fr: "Chart : Apple PMGR â†’ rails PMU/PMGR, buck CPU.",
    },
    Row {
        needle: "cp_com_norm",
        cause_fr: "CP_COM_NORM â€” CPU / stockage / camÃ©ra (chart)",
        weight: 0.58,
        check_fr: "Chart : CP_COM_NORM REQUEST â†’ tri CPU, disque, bus camÃ©ra selon contexte.",
    },
    Row {
        needle: "cp com norm",
        cause_fr: "CP com NORM â€” CPU / stockage / camÃ©ra (chart)",
        weight: 0.58,
        check_fr: "Chart : variante CP com NORM â€” mÃªme logique multi-pistes.",
    },
    Row {
        needle: "dart-disp",
        cause_fr: "Dart display / SMMU â€” camÃ©ra arriÃ¨re ou bus (chart)",
        weight: 0.66,
        check_fr: "Chart : Dart-disp SMMU â†’ camÃ©ra arriÃ¨re, bus camÃ©ra, flex arriÃ¨re.",
    },
    Row {
        needle: "dart-dispo",
        cause_fr: "Dart display / SMMU â€” camÃ©ra (chart)",
        weight: 0.66,
        check_fr: "Chart : Dart-dispo â†’ camÃ©ra principale / bus.",
    },
    Row {
        needle: "invalid queue element",
        cause_fr: "Invalid queue element â€” soudure / NAND (chart)",
        weight: 0.65,
        check_fr: "Chart : invalid queue â†’ disque mal soudÃ© ou NAND ; refaire BGA si doute.",
    },
    Row {
        needle: "invaild queue element",
        cause_fr: "Invalid queue (typo Â« invaild Â» dans log) â€” NAND / soudure (chart)",
        weight: 0.65,
        check_fr: "Chart : invaild queue element (typo frÃ©quente) â†’ idem NAND/soudure.",
    },
    Row {
        needle: "agxg10p",
        cause_fr: "AGXG10P BO NMI â€” dÃ©faut couche carte (chart)",
        weight: 0.62,
        check_fr: "Chart : AGXG10P â†’ couche carte / sandwich, pas simple flex.",
    },
    Row {
        needle: "agxaccelerator",
        cause_fr: "AGX / gyro / accÃ©lÃ©romÃ¨tre / coprocesseur (chart)",
        weight: 0.6,
        check_fr: "Chart : AGXAccelerator â†’ gyro, accÃ©lÃ©ro, copro graphique.",
    },
    Row {
        needle: "sks request timeout",
        cause_fr: "SKS request timeout â€” ligne CPU â†” puce logique (chart)",
        weight: 0.62,
        check_fr: "Chart : sks timeout â†’ continuitÃ© CPUâ€“logic board / interposer.",
    },
    Row {
        needle: "initproc exited",
        cause_fr: "initproc exited â€” quartz / horloge principale (chart)",
        weight: 0.56,
        check_fr: "Chart : initproc exited â†’ quartz principal, oscillateur, rails horloge.",
    },
    Row {
        needle: "bad tailq elm",
        cause_fr: "Bad tailq elm â€” quartz / horloge (chart)",
        weight: 0.56,
        check_fr: "Chart : bad tailq â†’ mÃªme famille que initproc (horloge).",
    },
    Row {
        needle: "prev- next",
        cause_fr: "Prev-next â€” quartz (chart)",
        weight: 0.54,
        check_fr: "Chart : prev-next / prew-next â†’ quartz (variante OCR).",
    },
    Row {
        needle: "prew-next",
        cause_fr: "Prew-next â€” quartz (chart)",
        weight: 0.54,
        check_fr: "Chart : prew-next â†’ quartz.",
    },
    Row {
        needle: "firmware fatal",
        cause_fr: "Firmware fatal â€” couche basse / restore (chart)",
        weight: 0.52,
        check_fr: "Chart : firmware fatal â†’ restore / iOS ; puis NAND si persistant.",
    },
    Row {
        needle: "nvme",
        cause_fr: "NVMe â€” stockage soudÃ© / contrÃ´leur (chart)",
        weight: 0.7,
        check_fr: "Chart : nvme â†’ disque NVMe, lignes donnÃ©es, soudure.",
    },
    Row {
        needle: "tristar2",
        cause_fr: "Tristar2 / USB â€” connecteur charge (chart)",
        weight: 0.68,
        check_fr: "Chart : Apple Tristar2 / system id â†’ dock / connecteur interne.",
    },
    Row {
        needle: "wkdmd error",
        cause_fr: "WKDMD error â€” flash / stockage (chart cite code 0x2)",
        weight: 0.58,
        check_fr: "Chart : WKDMD ERROR â†’ NAND/flash ; noter code 0x2 si prÃ©sent.",
    },
    Row {
        needle: "mic-temp-sens2",
        cause_fr: "mic-temp-sens2 â€” nappe power / MIC thermique (chart)",
        weight: 0.64,
        check_fr: "Chart : mic-temp-sens2 â†’ nappe alimentation, MIC, sonde.",
    },
    Row {
        needle: "applesynopsysmipi",
        cause_fr: "AppleSynopsysMIPIDSI â€” nappe camÃ©ra avant / Ã©cran (chart)",
        weight: 0.62,
        check_fr: "Chart : Synopsys MIPI DSI â†’ flex avant, Ã©cran, camÃ©ra avant.",
    },
    Row {
        needle: "smc data abort",
        cause_fr: "SMC DATA ABORT â€” communication CPU anormale (chart)",
        weight: 0.63,
        check_fr: "Chart : SMC DATA ABORT â†’ rails CPU, SMC, pas seulement flex.",
    },
    Row {
        needle: "sleep\\wake hang",
        cause_fr: "Sleep/wake hang â€” audio puis alim CPU (chart)",
        weight: 0.59,
        check_fr: "Chart : sleep/wake hang â†’ circuits audio dâ€™abord, puis alim CPU.",
    },
    Row {
        needle: "sleep/wake hang",
        cause_fr: "Sleep/wake hang â€” audio puis alim CPU (chart)",
        weight: 0.59,
        check_fr: "Chart : sleep/wake hang â†’ audio puis rails CPU.",
    },
    Row {
        needle: "kernel command stop",
        cause_fr: "Kernel command stop â€” quick charge IC ou bus CPU (chart)",
        weight: 0.57,
        check_fr: "Chart : kernel command stop â†’ IC charge rapide, communication CPU.",
    },
    Row {
        needle: "a freed zone element has been",
        cause_fr: "Freed zone element modified â€” soudure CPU (chart)",
        weight: 0.6,
        check_fr: "Chart : freed zone element â†’ soudure CPU / corruption mÃ©moire noyau.",
    },
    Row {
        needle: "l2c llc",
        cause_fr: "L2C / LLC â€” audio, camÃ©ra avant (chart)",
        weight: 0.58,
        check_fr: "Chart : L2C LLC Mux â†’ audio, nappe camÃ©ra avant.",
    },
    Row {
        needle: "gfx gpu",
        cause_fr: "GFX / GPU â€” GPU ou carte (chart)",
        weight: 0.58,
        check_fr: "Chart : GFX GPU â†’ GPU soudÃ©, couche carte (8/8s citÃ©s dans doc).",
    },
    Row {
        needle: "apcie(",
        cause_fr: "aPCIe / stockage PCIe (chart)",
        weight: 0.66,
        check_fr: "Chart : apcie(â€¦ â†’ disque / lignes PCIe NAND.",
    },
    Row {
        needle: "aop data abort",
        cause_fr: "AOP DATA ABORT â€” stockage ou soudure CPU (chart ; croiser contexte)",
        weight: 0.62,
        check_fr: "Chart : AOP DATA ABORT â†’ disque dâ€™abord (doc), puis soudure CPU si stockage OK.",
    },
];

pub fn apply_panic_logs_chart_pdf(
    log_lc: &str,
    product_apple_lc: Option<&str>,
    soc_id: Option<&str>,
    acc: &mut HashMap<String, f64>,
    checks: &mut HashSet<String>,
    ui_lines: &mut Vec<String>,
) {
    let push_acc = |acc: &mut HashMap<String, f64>, name: &str, w: f64| {
        acc.entry(name.to_string())
            .and_modify(|e| *e = e.max(w))
            .or_insert(w);
    };

    let mut hits = 0usize;
    for row in ROWS {
        if log_lc.contains(row.needle) {
            hits += 1;
            checks.insert(row.check_fr.to_string());
            push_acc(acc, row.cause_fr, row.weight);
        }
    }
    if hits > 0 {
        ui_lines.push(format!(
            "PANIC LOGS chart : {hits} motif(s) du PDF reconnus â€” voir recommandations dÃ©taillÃ©es."
        ));
    }
    append_i2c_chart_bus_hint(log_lc, product_apple_lc, soc_id, checks);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chart_nvme_adds_cause() {
        let mut acc = HashMap::new();
        let mut checks = HashSet::new();
        let mut ui = Vec::new();
        apply_panic_logs_chart_pdf(
            "kernel nvme controller error",
            None,
            None,
            &mut acc,
            &mut checks,
            &mut ui,
        );
        assert!(acc.keys().any(|k| k.to_lowercase().contains("nvme")), "{acc:?}");
        assert!(!ui.is_empty());
    }

    #[test]
    fn chart_i2c_hint_with_product() {
        let mut acc = HashMap::new();
        let mut checks = HashSet::new();
        let mut ui = Vec::new();
        apply_panic_logs_chart_pdf(
            "timeout on i2c1 bus",
            Some("iphone14,5"),
            Some("s8000"),
            &mut acc,
            &mut checks,
            &mut ui,
        );
        assert!(
            checks.iter().any(|c| c.contains("IÂ²C") && c.contains("iphone14,5")),
            "{checks:?}"
        );
    }
}
