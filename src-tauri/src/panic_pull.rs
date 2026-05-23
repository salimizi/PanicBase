
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use regex::Regex;
use rfd::FileDialog;
use serde::Serialize;
use std::time::Duration;
use tauri::async_runtime;
use tauri::{AppHandle, Manager, State};

use crate::{afc::AfcSession, analyzer, iphone, ips};

const MAX_FILE_BYTES: usize = 14 * 1024 * 1024;
const MAX_SNIPPET: usize = 220;
pub const MAX_PANICS: usize = 5;

const PANIC_FULL_MARKER: &[u8] = b"panic-full";
const PANICSTRING_MARKER: &[u8] = b"panicstring";
const PRESCAN_BYTES: usize = 4 * 1024;
const FAST_PULL_TIMEOUT: Duration = Duration::from_secs(75);
const AFC_SCAN_LIMIT: usize = 2500;
const AFC_SCAN_DEPTH: usize = 4;
const CRASHREPORT_DIRS: &[&str] = &[
    "/Library/Logs/CrashReporter",
    "/Logs/CrashReporter",
    "/CrashReporter",
    "/DiagnosticLogs",
];

fn pull_dir_active() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("PanicBase")
        .join("device_pull")
        .join("active")
}

fn remove_dir_robust(dir: &PathBuf) -> std::io::Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    let backoffs_ms = [40_u64, 80, 160, 320, 640];
    let mut last_err: Option<std::io::Error> = None;
    for ms in backoffs_ms.iter() {
        match fs::remove_dir_all(dir) {
            Ok(()) => return Ok(()),
            Err(e) => {
                let retriable = matches!(
                    e.raw_os_error(),
                    Some(32)   // ERROR_SHARING_VIOLATION
                    | Some(33) // ERROR_LOCK_VIOLATION
                    | Some(5)  // ERROR_ACCESS_DENIED
                    | Some(145) // ERROR_DIR_NOT_EMPTY (re-occurs when Defender drops a file mid-deletion)
                    | Some(2)  // ERROR_FILE_NOT_FOUND (concurrent enum/delete race)
                );
                last_err = Some(e);
                if !retriable {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(*ms));
            }
        }
    }
    // Fallback : rename â†’ delete plus tard. Le nouveau pull peut dÃ©marrer.
    let stamp = chrono::Utc::now().format("%Y%m%d%H%M%S").to_string();
    let dying = dir
        .parent()
        .map(|p| p.join(format!("active.dying-{stamp}")))
        .unwrap_or_else(|| dir.with_extension(format!("dying-{stamp}")));
    match fs::rename(dir, &dying) {
        Ok(()) => {
            // Best-effort: tente une suppression diffÃ©rÃ©e en thread dÃ©tachÃ©.
            let dying_owned = dying.clone();
            std::thread::spawn(move || {
                for _ in 0..6 {
                    std::thread::sleep(std::time::Duration::from_secs(2));
                    if fs::remove_dir_all(&dying_owned).is_ok() {
                        return;
                    }
                }
            });
            Ok(())
        }
        Err(e2) => {
            // Si on a une erreur d'origine, on la priorise (plus parlante).
            Err(last_err.unwrap_or(e2))
        }
    }
}

pub struct PulledPanicState {
    inner: Mutex<Option<PulledSession>>,
}

impl Default for PulledPanicState {
    fn default() -> Self {
        Self {
            inner: Mutex::new(None),
        }
    }
}

#[derive(Clone)]
struct PulledItem {
    path: PathBuf,
}

struct PulledSession {
    items: Vec<PulledItem>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PanicPullListResponse {
    pub count: usize,
    pub total_downloaded: usize,
    pub message: String,
    pub logs: Vec<PanicPullRow>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PanicPullRow {
    pub index: usize,
    pub filename: String,
    pub modified_label: String,
    pub snippet: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PulledPanicDetailResponse {
    pub panic_text: String,
    pub analysis: analyzer::AnalysisResult,
}

fn system_time_ms(t: Result<std::time::SystemTime, std::io::Error>) -> i64 {
    t.ok()
        .and_then(|st| st.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn iso_label(ms: i64) -> String {
    let secs = ms / 1000;
    if let Some(dt) = chrono::DateTime::<chrono::Utc>::from_timestamp(secs, 0) {
        dt.format("%Y-%m-%d %H:%M UTC").to_string()
    } else {
        String::from("â€”")
    }
}

fn embedded_panic_full_timestamp_ms(filename: &str) -> Option<i64> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        Regex::new(r"(?i)panic-full-(\d{4})-(\d{2})-(\d{2})-(\d{6})").expect("regex ok")
    });
    let cap = re.captures(filename)?;
    let y: i32 = cap.get(1)?.as_str().parse().ok()?;
    let mo: u32 = cap.get(2)?.as_str().parse().ok()?;
    let d: u32 = cap.get(3)?.as_str().parse().ok()?;
    let hms: u32 = cap.get(4)?.as_str().parse().ok()?;
    let hh = hms / 10000;
    let mm = (hms / 100) % 100;
    let ss = hms % 100;
    let date = chrono::NaiveDate::from_ymd_opt(y, mo, d)?;
    let naive = date.and_hms_opt(hh, mm, ss)?;
    Some(chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(
        naive,
        chrono::Utc,
    )
    .timestamp_millis())
}

fn effective_panic_sort_ms(path: &Path, mtime_ms: i64) -> i64 {
    let fname = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");
    embedded_panic_full_timestamp_ms(fname).unwrap_or(mtime_ms)
}

fn readable_filename(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

fn collect_ips_with_times(root: &Path) -> Vec<(PathBuf, i64, String)> {
    let mut out = Vec::new();
    walk_ips(root, root, &mut out);
    out
}

fn walk_ips(base: &Path, dir: &Path, acc: &mut Vec<(PathBuf, i64, String)>) {
    let Ok(rd) = fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            walk_ips(base, &p, acc);
            continue;
        }
        let ext_ok = p
            .extension()
            .map(|x| x.eq_ignore_ascii_case("ips") || x.eq_ignore_ascii_case("crash"))
            .unwrap_or(false);
        if !ext_ok {
            continue;
        }
        let mt = fs::metadata(&p).and_then(|m| m.modified());
        let ms = system_time_ms(mt);
        let rel = p.strip_prefix(base).unwrap_or(&p).to_string_lossy().replace('\\', "/");
        acc.push((p.clone(), ms, rel));
    }
}

fn truncate_snippet(s: &str) -> String {
    let mut chars = s.chars();
    let t: String = chars.by_ref().take(MAX_SNIPPET).collect();
    let ell = if chars.next().is_some() { "â€¦" } else { "" };
    format!("{}{}", t.trim(), ell)
}

fn utf8_is_kernel_panic(s: &str) -> bool {
    let bytes = s.as_bytes();
    bytes.windows(PANIC_FULL_MARKER.len()).any(|w| w.eq_ignore_ascii_case(PANIC_FULL_MARKER))
        || bytes.windows(PANICSTRING_MARKER.len()).any(|w| w.eq_ignore_ascii_case(PANICSTRING_MARKER))
}

fn filename_is_likely_panic(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|n| {
            let lo = n.to_lowercase();
            lo.contains("panic-full") || lo.contains("panic_full")
        })
        .unwrap_or(false)
}

fn read_file_capped_utf8(path: &Path, max: usize) -> Result<String, String> {
    let mut f =
        fs::File::open(path).map_err(|e| format!("Ouverture Â« {} Â» : {}", path.display(), e))?;
    let mut buf = Vec::with_capacity(max.min(MAX_FILE_BYTES));
    let mut chunk = vec![0u8; 64 * 1024];
    while buf.len() < max {
        let n = f
            .read(&mut chunk)
            .map_err(|e| format!("Lecture Â« {} Â» : {}", path.display(), e))?;
        if n == 0 {
            break;
        }
        let take = n.min(max - buf.len());
        buf.extend_from_slice(&chunk[..take]);
        if buf.len() >= max {
            break;
        }
    }
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

struct PickedPanicFull {
    path: PathBuf,
    ms: i64,
    raw_text: String,
}

#[derive(Clone)]
struct AfcPanicCandidate {
    path: String,
    filename: String,
    ms: i64,
}

fn safe_local_filename(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    if cleaned.trim().is_empty() {
        "panic-full.ips".to_string()
    } else {
        cleaned
    }
}

fn afc_candidate_sort_ms(filename: &str, mtime_ns: i64, birth_ns: i64) -> i64 {
    embedded_panic_full_timestamp_ms(filename).unwrap_or_else(|| {
        let ns = if mtime_ns > 0 { mtime_ns } else { birth_ns };
        if ns > 1_000_000_000_000 {
            ns / 1_000_000
        } else {
            ns
        }
    })
}

fn collect_afc_panic_candidates(
    s: &AfcSession,
    dir: &str,
    depth: usize,
    seen: &mut usize,
    out: &mut Vec<AfcPanicCandidate>,
) {
    if depth > AFC_SCAN_DEPTH || *seen >= AFC_SCAN_LIMIT {
        return;
    }
    let Ok(entries) = s.read_directory(dir) else {
        return;
    };
    for entry in entries {
        if *seen >= AFC_SCAN_LIMIT {
            return;
        }
        *seen += 1;
        let path = if dir == "/" {
            format!("/{entry}")
        } else {
            format!("{dir}/{entry}")
        };
        let lower = entry.to_ascii_lowercase();
        if lower.contains("panic-full") || lower.contains("panic_full") {
            if lower.ends_with(".ips") || lower.ends_with(".crash") || lower.ends_with(".panic") {
                let info = s.file_info(&path).unwrap_or_default();
                out.push(AfcPanicCandidate {
                    path,
                    filename: entry,
                    ms: afc_candidate_sort_ms(&lower, info.mtime_ns, info.birth_ns),
                });
                continue;
            }
        }
        if depth < AFC_SCAN_DEPTH {
            if s.file_info(&path).map(|i| i.is_dir()).unwrap_or(false) {
                collect_afc_panic_candidates(s, &path, depth + 1, seen, out);
            }
        }
    }
}

fn try_pull_recent_panic_full_afc(udid: Option<&str>, dest: &Path) -> Result<Option<Vec<PickedPanicFull>>, String> {
    let s = match AfcSession::open_service(udid, "com.apple.crashreportcopymobile") {
        Ok(session) => session,
        Err(_) => return Ok(None),
    };

    let mut candidates = Vec::new();
    let mut seen = 0usize;

    // Racine "/" du service = dossier CrashReporter directement.
    // On scanne aussi les sous-dossiers Ã©ventuels (Retired, HighMemory, etc.).
    collect_afc_panic_candidates(&s, "/", 0, &mut seen, &mut candidates);

    // Fallback : tenter les chemins connus au cas oÃ¹ le service monte un arbre plus profond.
    if candidates.is_empty() {
        for root in CRASHREPORT_DIRS {
            collect_afc_panic_candidates(&s, root, 0, &mut seen, &mut candidates);
            if candidates.len() >= MAX_PANICS { break; }
        }
    }

    if candidates.is_empty() {
        return Ok(None);
    }

    candidates.sort_by(|a, b| b.ms.cmp(&a.ms));
    let mut picked = Vec::new();
    for c in candidates.into_iter().take(MAX_PANICS) {
        let bytes = match s.read_file(&c.path, MAX_FILE_BYTES) {
            Ok(b) => b,
            Err(_) => continue,
        };
        // Fichiers nommÃ©s "panic-full-*" : on fait confiance au nom Apple.
        // Pour les autres noms on vÃ©rifie le contenu.
        let is_panic_name = {
            let lo = c.filename.to_ascii_lowercase();
            lo.contains("panic-full") || lo.contains("panic_full")
        };
        let raw_text = String::from_utf8_lossy(&bytes).into_owned();
        if !is_panic_name && !utf8_is_kernel_panic(&raw_text) {
            continue;
        }
        let local = dest.join(safe_local_filename(&c.filename));
        fs::write(&local, &bytes).map_err(|e| format!("Copie AFC panic-full : {e}"))?;
        picked.push(PickedPanicFull {
            path: local,
            ms: c.ms,
            raw_text,
        });
    }

    if picked.is_empty() {
        Ok(None)
    } else {
        Ok(Some(picked))
    }
}

fn pick_recent_panic_full_files(
    mut entries: Vec<(PathBuf, i64, String)>,
) -> Result<Vec<PickedPanicFull>, String> {
    entries.sort_by(|a, b| {
        let ka = effective_panic_sort_ms(&a.0, a.1);
        let kb = effective_panic_sort_ms(&b.0, b.1);
        kb.cmp(&ka)
    });

    let mut out: Vec<PickedPanicFull> = Vec::new();
    for (path, ms, _) in entries.into_iter() {
        if out.len() >= MAX_PANICS {
            break;
        }
        if filename_is_likely_panic(&path) {
            // Nom Apple "panic-full-*" ou "panic_full-*" â†’ on fait confiance au nom.
            // On charge le contenu complet directement sans prÃ©-scan ni filtre contenu.
            let raw_text = read_file_capped_utf8(&path, MAX_FILE_BYTES)?;
            out.push(PickedPanicFull { path, ms, raw_text });
        } else {
            // Autres noms (Jetsam, ANR, app crashesâ€¦) : prÃ©-scan 4 Ko pour filtrer rapidement,
            // puis vÃ©rification complÃ¨te avant d'accepter le fichier.
            let prescan = read_file_capped_utf8(&path, PRESCAN_BYTES)?;
            if !utf8_is_kernel_panic(&prescan) {
                continue;
            }
            let raw_text = read_file_capped_utf8(&path, MAX_FILE_BYTES)?;
            if utf8_is_kernel_panic(&raw_text) {
                out.push(PickedPanicFull { path, ms, raw_text });
            }
        }
    }
    // entries Ã©tait dÃ©jÃ  triÃ©e par timestamp dÃ©croissant â†’ out lâ€™est aussi.
    Ok(out)
}

fn response_from_picked(
    state: &PulledPanicState,
    chosen: Vec<PickedPanicFull>,
    total_downloaded: usize,
    source_label: &str,
) -> Result<PanicPullListResponse, String> {
    let mut items_vec = Vec::new();
    let mut rows = Vec::new();

    for PickedPanicFull { path, ms, raw_text } in chosen.into_iter() {
        let (extracted, _, _) = ips::extract_ips_body(&raw_text);
        let snip = truncate_snippet(&extracted);
        let index = rows.len();
        let display_ms = effective_panic_sort_ms(&path, ms);

        rows.push(PanicPullRow {
            index,
            filename: readable_filename(&path),
            modified_label: iso_label(display_ms),
            snippet: snip,
        });
        items_vec.push(PulledItem { path });
    }

    *state
        .inner
        .lock()
        .map_err(|_| "Erreur interne mÃ©moire (mutex).".to_string())? = Some(PulledSession {
        items: items_vec,
    });

    let count = rows.len();
    let message = format!(
        "{count} panic(s) noyau (max {MAX_PANICS}), les plus rÃ©cents via {source_label} sur {total_downloaded} fichier(s)."
    );

    Ok(PanicPullListResponse {
        message,
        count,
        total_downloaded,
        logs: rows,
    })
}

#[tauri::command]
pub async fn pull_device_recent_panic_logs(app: AppHandle, udid: Option<String>) -> Result<PanicPullListResponse, String> {
    async_runtime::spawn_blocking(move || pull_device_recent_panic_logs_blocking(app, udid))
        .await
        .map_err(|e| format!("TÃ¢che USB interrompue : {e}"))?
}

fn pull_device_recent_panic_logs_blocking(app: AppHandle, udid: Option<String>) -> Result<PanicPullListResponse, String> {
    let state = app.state::<PulledPanicState>();
    let dir = pull_dir_active();
    remove_dir_robust(&dir).map_err(|e| format!("Nettoyage pull : {}", e))?;
    fs::create_dir_all(&dir).map_err(|e| format!("mkdir pull : {}", e))?;

    if let Ok(Some(chosen)) = try_pull_recent_panic_full_afc(udid.as_deref(), &dir) {
        let total = chosen.len();
        return response_from_picked(&state, chosen, total, "AFC direct");
    }

    let exe = iphone::resolved_libimobile_tool("idevicecrashreport");
    let dir_str = dir.to_string_lossy().to_string();
    let mut crash_cmd = iphone::command_for_tool(&exe);
    // UDID explicite : Ã©vite toute ambiguÃ¯tÃ© si WiFi-sync + USB actifs simultanÃ©ment.
    if let Some(ref u) = udid {
        let u = u.trim();
        if !u.is_empty() {
            crash_cmd.arg("-u").arg(u);
        }
    }
    // -k : conserver les logs sur lâ€™appareil aprÃ¨s extraction (non-destructif).
    // Sans ce flag, idevicecrashreport supprime les fichiers de lâ€™iPhone â†’ scan suivant = vide.
    // -e : extraire les archives ZIP (iOS rÃ©cent compresse certains crash reports).
    crash_cmd.arg("-k").arg("-e").arg(&dir_str);
    let output = iphone::command_output_with_timeout(crash_cmd, FAST_PULL_TIMEOUT).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            return "idevicecrashreport introuvable â€” installe les binaires libimobiledevice.".to_string();
        }
        if e.kind() == std::io::ErrorKind::TimedOut {
            return "Lecture panic-full arrÃªtÃ©e aprÃ¨s 75 secondes : idevicecrashreport nâ€™a pas terminÃ©. VÃ©rifie que lâ€™iPhone est dÃ©verrouillÃ©, que Â« Faire confiance Â» a Ã©tÃ© acceptÃ©, puis rÃ©essaie.".to_string();
        }
        e.to_string()
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!(
            "idevicecrashreport a Ã©chouÃ© ({}).\nstderr: {}\nstdout: {}",
            output.status,
            stderr.trim(),
            stdout.trim()
        ));
    }

    let raw = collect_ips_with_times(&dir);
    let total_downloaded = raw.len();
    let chosen = pick_recent_panic_full_files(raw)?;

    if chosen.is_empty() {
        *state
            .inner
            .lock()
            .map_err(|_| "Erreur interne mÃ©moire (mutex).".to_string())? = Some(PulledSession { items: vec![] });
        let msg = if total_downloaded == 0 {
            "Aucun fichier .ips/.crash tÃ©lÃ©chargÃ© â€” lâ€™appareil nâ€™a peut-Ãªtre pas de logs accessibles. VÃ©rifie que lâ€™iPhone est dÃ©verrouillÃ© et a acceptÃ© Â« Faire confiance Â».".into()
        } else {
            format!(
                "Aucun panic noyau parmi les {total_downloaded} fichier(s) tÃ©lÃ©chargÃ©s. \
                 Seuls les rapports contenant Â« panic-full Â» ou Â« panicString Â» sont retenus."
            )
        };
        return Ok(PanicPullListResponse {
            count: 0,
            total_downloaded,
            message: msg,
            logs: vec![],
        });
    }

    let mut items_vec = Vec::new();
    let mut rows = Vec::new();

    for PickedPanicFull { path, ms, raw_text } in chosen.into_iter() {
        let (extracted, _, _) = ips::extract_ips_body(&raw_text);
        let snip = truncate_snippet(&extracted);
        let index = rows.len();
        let display_ms = effective_panic_sort_ms(&path, ms);

        rows.push(PanicPullRow {
            index,
            filename: readable_filename(&path),
            modified_label: iso_label(display_ms),
            snippet: snip,
        });
        items_vec.push(PulledItem { path });
    }

    *state
        .inner
        .lock()
        .map_err(|_| "Erreur interne mÃ©moire (mutex).".to_string())? = Some(PulledSession {
        items: items_vec,
    });

    let count = rows.len();
    let message = format!(
        "{count} panic(s) noyau (max {MAX_PANICS}), les plus rÃ©cents sur {total_downloaded} fichier(s) tÃ©lÃ©chargÃ©s."
    );

    Ok(PanicPullListResponse {
        message,
        count,
        total_downloaded,
        logs: rows,
    })
}

#[tauri::command]
pub fn analyze_pulled_device_panic(
    state: State<PulledPanicState>,
    index: usize,
    device_hint: Option<String>,
) -> Result<PulledPanicDetailResponse, String> {
    let path = {
        let session = state
            .inner
            .lock()
            .map_err(|_| "Session pull indisponible.".to_string())?;
        let s = session
            .as_ref()
            .ok_or_else(|| "Aucun tirage disponible â€” lance Â« Lire les 5 derniers Â» dâ€™abord.".to_string())?;
        let item = s
            .items
            .get(index)
            .ok_or_else(|| format!("Index {index} invalide."))?;
        item.path.clone()
    };

    let raw = read_file_capped_utf8(&path, MAX_FILE_BYTES)?;
    if ips::ips_is_binary_plist(&raw) {
        return Err(
            "Ce rapport est au format bplist : PanicBase ne peut pas dÃ©coder tout le fichier ici.".into(),
        );
    }
    let (panic_text, _, _) = ips::extract_ips_body(&raw);
    let hint = device_hint.as_deref();
    let analysis = analyzer::analyze_panic_log(&panic_text, hint, Some(raw.as_str()));
    Ok(PulledPanicDetailResponse {
        panic_text,
        analysis,
    })
}

#[tauri::command]
pub fn export_pulled_device_panic_file(
    state: State<PulledPanicState>,
    index: usize,
    default_filename: String,
) -> Result<Option<String>, String> {
    let source = {
        let session = state
            .inner
            .lock()
            .map_err(|_| "Session pull indisponible.".to_string())?;
        let s = session
            .as_ref()
            .ok_or_else(|| "Aucun tirage â€” reconnecte lâ€™iPhone et attends le scan.".to_string())?;
        let item = s
            .items
            .get(index)
            .ok_or_else(|| format!("Index {index} invalide â€” resÃ©lectionne un log."))?;
        item.path.clone()
    };

    let Some(dest) = FileDialog::new().set_file_name(&default_filename).save_file() else {
        return Ok(None);
    };

    fs::copy(&source, &dest).map_err(|e| format!("Copie du fichier : {e}"))?;
    Ok(Some(dest.to_string_lossy().into_owned()))
}

#[tauri::command]
pub fn read_pulled_device_panic_raw(
    state: State<PulledPanicState>,
    index: usize,
) -> Result<String, String> {
    let path = {
        let session = state
            .inner
            .lock()
            .map_err(|_| "Session pull indisponible.".to_string())?;
        let s = session
            .as_ref()
            .ok_or_else(|| "Aucun tirage â€” reconnecte lâ€™iPhone.".to_string())?;
        let item = s
            .items
            .get(index)
            .ok_or_else(|| format!("Index {index} invalide."))?;
        item.path.clone()
    };
    read_file_capped_utf8(&path, MAX_FILE_BYTES)
}
