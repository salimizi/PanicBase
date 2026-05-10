//! Récupère sur l’appareil les derniers rapports `.ips` (panic / crash) via idevicecrashreport.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::Serialize;
use tauri::State;

use crate::{analyzer, iphone, ips};

const MAX_FILE_BYTES: usize = 14 * 1024 * 1024;
const MAX_SNIPPET: usize = 220;
pub const MAX_PANICS: usize = 5;

/// Segments Apple typiques d’un rapport kernel panic téléchargé (évite les crashes userspace quelconques).
const PANIC_FULL_MARKER: &[u8] = b"panic-full";

fn pull_dir_active() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("PanicBase")
        .join("device_pull")
        .join("active")
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
        String::from("—")
    }
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
    let t = s.chars().take(MAX_SNIPPET).collect::<String>();
    let ell = if s.chars().count() > MAX_SNIPPET {
        "…"
    } else {
        ""
    };
    format!("{}{}", t.trim(), ell)
}

fn utf8_contains_panic_full(s: &str) -> bool {
    // Comparatif insensible à la casse ASCII sur le marqueur "panic-full"
    s.as_bytes()
        .windows(PANIC_FULL_MARKER.len())
        .any(|w| w.eq_ignore_ascii_case(PANIC_FULL_MARKER))
}

fn read_file_capped_utf8(path: &Path, max: usize) -> Result<String, String> {
    let mut f =
        fs::File::open(path).map_err(|e| format!("Ouverture « {} » : {}", path.display(), e))?;
    let mut buf = Vec::with_capacity(max.min(MAX_FILE_BYTES));
    let mut chunk = vec![0u8; 64 * 1024];
    while buf.len() < max {
        let n = f
            .read(&mut chunk)
            .map_err(|e| format!("Lecture « {} » : {}", path.display(), e))?;
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

/// Garde au plus `MAX_PANICS` fichiers les plus récents dont le **contenu** contient `panic-full`
/// (rapports panic noyau Apple), sans remplissage avec d’autres `.ips`/`.crash`.
fn pick_recent_panic_full_files(
    mut entries: Vec<(PathBuf, i64, String)>,
) -> Result<Vec<PickedPanicFull>, String> {
    entries.sort_by_key(|(_, ms, _)| -*ms);

    let mut out: Vec<PickedPanicFull> = Vec::new();
    for (path, ms, _) in entries.into_iter() {
        if out.len() >= MAX_PANICS {
            break;
        }
        let raw_text = read_file_capped_utf8(&path, MAX_FILE_BYTES)?;
        if utf8_contains_panic_full(&raw_text) {
            out.push(PickedPanicFull { path, ms, raw_text });
        }
    }
    out.sort_by_key(|p| -p.ms);
    Ok(out)
}

/// Vide le dossier puis lance idevicecrashreport, indexe jusqu’à 5 `.ips`/`.crash` les plus récents.
#[tauri::command]
pub fn pull_device_recent_panic_logs(state: State<PulledPanicState>) -> Result<PanicPullListResponse, String> {
    let dir = pull_dir_active();
    if dir.exists() {
        fs::remove_dir_all(&dir).map_err(|e| format!("Nettoyage pull : {}", e))?;
    }
    fs::create_dir_all(&dir).map_err(|e| format!("mkdir pull : {}", e))?;

    let exe = iphone::resolved_libimobile_tool("idevicecrashreport");
    let dir_str = dir.to_string_lossy().to_string();
    let output = std::process::Command::new(&exe)
        .arg(&dir_str)
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                return "idevicecrashreport introuvable — installe les binaires libimobiledevice.".to_string();
            }
            e.to_string()
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!(
            "idevicecrashreport a échoué ({}).\nstderr: {}\nstdout: {}",
            output.status,
            stderr.trim(),
            stdout.trim()
        ));
    }

    let raw = collect_ips_with_times(&dir);
    let chosen = pick_recent_panic_full_files(raw)?;

    if chosen.is_empty() {
        *state
            .inner
            .lock()
            .map_err(|_| "Erreur interne mémoire (mutex).".to_string())? = Some(PulledSession { items: vec![] });
        return Ok(PanicPullListResponse {
            count: 0,
            message: "Aucun rapport panic-full : parmi les fichiers téléchargés, aucun ne contient « panic-full » dans le texte (seuls ceux-là sont retenus). L’appareil peut aussi ne pas en exposer ici.".into(),
            logs: vec![],
        });
    }

    let mut items_vec = Vec::new();
    let mut rows = Vec::new();

    for PickedPanicFull { path, ms, raw_text } in chosen.into_iter() {
        let (extracted, _, _) = ips::extract_ips_body(&raw_text);
        let snip = truncate_snippet(&extracted);
        let index = rows.len();

        rows.push(PanicPullRow {
            index,
            filename: readable_filename(&path),
            modified_label: iso_label(ms),
            snippet: snip,
        });
        items_vec.push(PulledItem { path });
    }

    *state
        .inner
        .lock()
        .map_err(|_| "Erreur interne mémoire (mutex).".to_string())? = Some(PulledSession {
        items: items_vec,
    });

    let count = rows.len();
    let message = if count == 0 {
        "Les fichiers trouvés n’ont pas pu être décodés comme texte (format binaire ?). Réessaie après un tirage forcé.".into()
    } else {
        format!(
            "{count} rapport(s) panic-full (max {}), les plus récents parmi ceux contenant « panic-full » dans le fichier.",
            MAX_PANICS
        )
    };

    Ok(PanicPullListResponse {
        message,
        count,
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
            .ok_or_else(|| "Aucun tirage disponible — lance « Lire les 5 derniers » d’abord.".to_string())?;
        let item = s
            .items
            .get(index)
            .ok_or_else(|| format!("Index {index} invalide."))?;
        item.path.clone()
    };

    let raw = read_file_capped_utf8(&path, MAX_FILE_BYTES)?;
    if ips::ips_is_binary_plist(&raw) {
        return Err(
            "Ce rapport est au format bplist : PanicBase ne peut pas décoder tout le fichier ici.".into(),
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
