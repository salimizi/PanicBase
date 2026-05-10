use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;

use crate::database;

/// État UX USB : brancher → faire confiance → connecté avec modèle.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")] // marketingName, productType, iosVersion pour le front
pub struct IphoneUsbStatus {
    pub phase: String,
    pub detail: String,
    pub udids: Vec<String>,
    pub marketing_name: Option<String>,
    pub product_type: Option<String>,
    pub ios_version: Option<String>,
}

/// Correspondance `ProductType` → nom commercial Apple (liste locale).
pub fn marketing_lookup(product_type: &str) -> &'static str {
    match product_type {
        "iPhone8,1" => "iPhone 6s",
        "iPhone9,3" => "iPhone 7",
        "iPhone10,6" => "iPhone X",
        "iPhone11,2" => "iPhone XS",
        "iPhone11,8" => "iPhone XR",
        "iPhone12,1" => "iPhone 11",
        "iPhone12,3" => "iPhone 11 Pro",
        "iPhone12,5" => "iPhone 11 Pro Max",
        "iPhone13,1" => "iPhone 12 mini",
        "iPhone13,2" => "iPhone 12",
        "iPhone13,3" => "iPhone 12 Pro",
        "iPhone13,4" => "iPhone 12 Pro Max",
        "iPhone14,4" => "iPhone 13 mini",
        "iPhone14,5" => "iPhone 13",
        "iPhone14,2" => "iPhone 13 Pro",
        "iPhone14,3" => "iPhone 13 Pro Max",
        "iPhone14,7" => "iPhone 14",
        "iPhone14,8" => "iPhone 14 Plus",
        "iPhone15,2" => "iPhone 14 Pro",
        "iPhone15,3" => "iPhone 14 Pro Max",
        "iPhone15,4" => "iPhone 15",
        "iPhone15,5" => "iPhone 15 Plus",
        "iPhone16,1" => "iPhone 15 Pro",
        "iPhone16,2" => "iPhone 15 Pro Max",
        "iPhone17,1" => "iPhone 16 Pro",
        "iPhone17,2" => "iPhone 16 Pro Max",
        "iPhone17,3" => "iPhone 16",
        "iPhone17,4" => "iPhone 16 Plus",
        // iPhone 17 (noms indicatifs — à ajuster selon grille Apple officielle)
        "iPhone18,1" => "iPhone 17 Pro",
        "iPhone18,2" => "iPhone 17 Pro Max",
        "iPhone18,3" => "iPhone 17",
        "iPhone18,4" => "iPhone 17 Plus",
        _ => "",
    }
}

/// Pour la base panic : résout `iPhone17,2` → marketing, sinon repasse la chaîne telle quelle.
pub fn marketing_display_for_hints(hint: Option<&str>) -> Option<String> {
    let t = hint?.trim();
    if t.is_empty() {
        return None;
    }
    if t.contains(',') && t.to_lowercase().starts_with("iphone") {
        let m = marketing_lookup(t);
        if m.is_empty() {
            Some(t.to_string())
        } else {
            Some(m.to_string())
        }
    } else {
        Some(t.to_string())
    }
}

pub(crate) fn resolved_libimobile_tool(tool_stem: &str) -> PathBuf {
    #[cfg(windows)]
    let filename = format!("{tool_stem}.exe");
    #[cfg(not(windows))]
    let filename = tool_stem.to_string();

    #[cfg(windows)]
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        let p = PathBuf::from(local).join("libimobiledevice").join(&filename);
        if p.exists() {
            return p;
        }
    }

    #[cfg(windows)]
    {
        PathBuf::from(&filename)
    }
    #[cfg(not(windows))]
    {
        PathBuf::from(tool_stem)
    }
}

fn idevice_read_key(tool: &Path, udid: Option<&str>, key: &str) -> Option<String> {
    let mut c = Command::new(tool);
    if let Some(u) = udid {
        if !u.trim().is_empty() {
            c.arg("-u").arg(u.trim());
        }
    }
    c.arg("-k").arg(key);
    let ok = c.output().ok().filter(|o| o.status.success())?;
    let v = String::from_utf8_lossy(&ok.stdout).trim().to_string();
    if v.is_empty() {
        None
    } else {
        Some(v)
    }
}

fn parse_udids(stdout: &str) -> Vec<String> {
    stdout
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .map(|l| l.to_string())
        .collect()
}

pub fn detect_iphone_usb() -> IphoneUsbStatus {
    let exe_id = resolved_libimobile_tool("idevice_id");
    let exe_info = resolved_libimobile_tool("ideviceinfo");

    let out = match Command::new(&exe_id).arg("-l").output() {
        Ok(o) => o,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return IphoneUsbStatus {
                phase: "no_tools".into(),
                detail: "Outils libimobiledevice introuvables. Place idevice_id.exe dans %LOCALAPPDATA%\\libimobiledevice\\ ou dans le PATH.".into(),
                udids: vec![],
                marketing_name: None,
                product_type: None,
                ios_version: None,
            };
        }
        Err(e) => {
            return IphoneUsbStatus {
                phase: "error".into(),
                detail: format!("Impossible de lancer idevice_id : {e}"),
                udids: vec![],
                marketing_name: None,
                product_type: None,
                ios_version: None,
            };
        }
    };

    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return IphoneUsbStatus {
            phase: "error".into(),
            detail: format!(
                "idevice_id a échoué ({}) — {}",
                out.status,
                stderr.trim()
            ),
            udids: vec![],
            marketing_name: None,
            product_type: None,
            ios_version: None,
        };
    }

    let stdout = String::from_utf8_lossy(&out.stdout);
    let udids = parse_udids(&stdout);

    if udids.is_empty() {
        return IphoneUsbStatus {
            phase: "unplugged".into(),
            detail: "".into(),
            udids: vec![],
            marketing_name: None,
            product_type: None,
            ios_version: None,
        };
    }

    let first_udid = udids[0].as_str();
    let product = idevice_read_key(&exe_info, Some(first_udid), "ProductType");
    let ios = idevice_read_key(&exe_info, Some(first_udid), "ProductVersion");

    if product.is_none() {
        return IphoneUsbStatus {
            phase: "awaiting_trust".into(),
            detail: String::new(),
            udids,
            marketing_name: None,
            product_type: None,
            ios_version: None,
        };
    }

    let product_type_val = product.unwrap();
    let mapped = marketing_lookup(&product_type_val);
    let mapped_opt = if mapped.is_empty() {
        None
    } else {
        Some(mapped.to_string())
    };
    IphoneUsbStatus {
        phase: "connected".into(),
        detail: String::new(),
        udids,
        marketing_name: mapped_opt,
        product_type: Some(product_type_val),
        ios_version: ios,
    }
}

fn count_crash_like_files(root: &Path) -> usize {
    let mut n = 0;
    let Ok(rd) = std::fs::read_dir(root) else {
        return 0;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            n += count_crash_like_files(&p);
            continue;
        }
        if let Some(ext) = p.extension() {
            if ext.eq_ignore_ascii_case("ips") || ext.eq_ignore_ascii_case("crash") {
                n += 1;
            }
        }
    }
    n
}

pub fn extract_panic_logs() -> Result<String, String> {
    let exe = resolved_libimobile_tool("idevicecrashreport");
    let out = database::ensure_crash_reports_dir()?;
    let out_str = out.to_string_lossy().to_string();

    let output = Command::new(&exe).arg(&out_str).output().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            return "idevicecrashreport introuvable — même dossier qu’idevice_id.".to_string();
        }
        e.to_string()
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!(
            "idevicecrashreport ({}) stdout: {} stderr: {}",
            output.status,
            stdout.trim(),
            stderr.trim()
        ));
    }

    let n = count_crash_like_files(&out);
    Ok(format!(
        "Extraction terminée dans « {} » · {} fichier(s) .ips / .crash visibles.",
        out_str, n
    ))
}
