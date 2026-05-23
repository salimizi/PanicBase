use base64::Engine as _;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Emitter};

use crate::afc::{AfcSession, FileInfo};

// ── Types publics ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AfcMediaItem {
    pub object_id: String,
    pub filename: String,
    pub extension: String,
    pub is_video: bool,
    pub folder: String,
    pub size_bytes: u64,
    pub mtime_ns: i64,
}

#[derive(serde::Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AfcFileExport {
    pub object_id: String,
    pub filename: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AfcExportResult {
    pub exported: usize,
    pub icloud_only: usize,
    pub errors: usize,
    pub error_samples: Vec<String>,
    pub dest_dir: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProgressEvent {
    current: usize,
    total: usize,
    filename: String,
    exported: usize,
    failed: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PausedEvent {
    current: usize,
    total: usize,
    exported: usize,
    failed: usize,
    skipped_cloud: usize,
    completed_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DoneEvent {
    exported: usize,
    failed: usize,
    skipped_cloud: usize,
    dest_dir: String,
}

// ── Constantes ────────────────────────────────────────────────────────────────

const VALID_EXTS: &[&str] = &[
    "jpg", "jpeg", "png", "heic", "heif", "gif", "tif", "tiff", "webp", "bmp",
    "mov", "mp4", "m4v", "hevc", "avi", "mkv", "aae",
];

const VIDEO_EXTS: &[&str] = &["mov", "mp4", "m4v", "hevc", "avi", "mkv"];

const MEDIA_ROOT: &str = "/DCIM";
const THUMB_ROOT: &str = "/PhotoData/Thumbnails/V2/DCIM";

fn ext_of(name: &str) -> String {
    name.rsplit('.').next().unwrap_or("").to_lowercase()
}

fn is_valid_media(name: &str) -> bool {
    let e = ext_of(name);
    VALID_EXTS.iter().any(|x| *x == e)
}

fn is_video(ext: &str) -> bool {
    VIDEO_EXTS.iter().any(|x| *x == ext)
}

fn folder_label(raw: &str) -> String {
    if raw.is_empty() { return "Camera Roll".into(); }
    if raw.eq_ignore_ascii_case("dcim") { return "Camera Roll".into(); }
    if raw.len() == 8
        && raw[3..].eq_ignore_ascii_case("APPLE")
        && raw[..3].chars().all(|c| c.is_ascii_digit())
    {
        return "Camera Roll".into();
    }
    raw.to_string()
}

// ── Scan rapide — sans file_info, multi-thread ────────────────────────────────

pub fn list_media(udid: Option<&str>) -> Result<Vec<AfcMediaItem>, String> {
    let main = AfcSession::open(udid)?;
    let subdirs = main.read_directory(MEDIA_ROOT).unwrap_or_default();
    drop(main);

    if subdirs.is_empty() {
        return Ok(Vec::new());
    }

    let max_threads = subdirs.len().min(4);
    let queue = Arc::new(Mutex::new(subdirs));
    let results: Arc<Mutex<Vec<AfcMediaItem>>> = Arc::new(Mutex::new(Vec::with_capacity(4096)));

    let mut handles = Vec::new();
    for _ in 0..max_threads {
        let queue = Arc::clone(&queue);
        let results = Arc::clone(&results);
        let udid_owned = udid.map(|s| s.to_string());
        let handle = thread::Builder::new()
            .name("afc-walker".into())
            .spawn(move || -> Result<(), String> {
                let s = AfcSession::open(udid_owned.as_deref())?;
                loop {
                    let next = { let mut q = queue.lock().unwrap(); q.pop() };
                    let Some(sub) = next else { break };
                    let sub_path = format!("{MEDIA_ROOT}/{sub}");
                    let entries = s.read_directory(&sub_path).unwrap_or_default();
                    let label = folder_label(&sub);
                    let mut local: Vec<AfcMediaItem> = Vec::with_capacity(entries.len());
                    for entry in entries {
                        if !is_valid_media(&entry) { continue; }
                        let ext = ext_of(&entry);
                        let is_vid = is_video(&ext);
                        local.push(AfcMediaItem {
                            object_id: format!("{sub_path}/{entry}"),
                            filename: entry,
                            extension: ext,
                            is_video: is_vid,
                            folder: label.clone(),
                            size_bytes: 0,
                            mtime_ns: 0,
                        });
                    }
                    let mut g = results.lock().unwrap();
                    g.extend(local);
                }
                Ok(())
            })
            .map_err(|e| format!("spawn AFC : {e}"))?;
        handles.push(handle);
    }

    for h in handles { let _ = h.join(); }

    let mut items = Arc::try_unwrap(results)
        .map_err(|_| "Arc occupé".to_string())?
        .into_inner()
        .map_err(|e| format!("Mutex empoisonné : {e}"))?;
    items.sort_by(|a, b| a.folder.cmp(&b.folder).then(a.filename.cmp(&b.filename)));
    Ok(items)
}

// ── Thumbnails ────────────────────────────────────────────────────────────────

pub fn get_thumbnail(udid: Option<&str>, object_id: &str) -> Result<String, String> {
    let s = AfcSession::open(udid)?;

    if let Some(thumb_path) = derive_thumb_path(object_id) {
        if let Ok(bytes) = s.read_file(&thumb_path, 512 * 1024) {
            if looks_like_jpeg(&bytes) {
                return Ok(base64::engine::general_purpose::STANDARD.encode(&bytes));
            }
        }
        let small = thumb_path.replace("5005.JPG", "5003.JPG");
        if small != thumb_path {
            if let Ok(bytes) = s.read_file(&small, 512 * 1024) {
                if looks_like_jpeg(&bytes) {
                    return Ok(base64::engine::general_purpose::STANDARD.encode(&bytes));
                }
            }
        }
    }

    let bytes = s.read_file(object_id, 4 * 1024 * 1024)?;
    if bytes.is_empty() { return Err("iCloud-only".to_string()); }
    let img = image::load_from_memory(&bytes).map_err(|e| format!("image::load : {e}"))?;
    let thumb = img.thumbnail(240, 240);
    let mut out = Vec::new();
    let mut cur = std::io::Cursor::new(&mut out);
    thumb.write_to(&mut cur, image::ImageFormat::Jpeg).map_err(|e| format!("encode : {e}"))?;
    Ok(base64::engine::general_purpose::STANDARD.encode(&out))
}

fn derive_thumb_path(object_id: &str) -> Option<String> {
    let parts: Vec<&str> = object_id.split('/').filter(|s| !s.is_empty()).collect();
    if parts.len() < 3 { return None; }
    if !parts[0].eq_ignore_ascii_case("DCIM") { return None; }
    Some(format!("{}/{}/{}/5005.JPG", THUMB_ROOT, parts[1], parts[2]))
}

fn looks_like_jpeg(b: &[u8]) -> bool {
    b.len() >= 3 && b[0] == 0xFF && b[1] == 0xD8 && b[2] == 0xFF
}

// ── Export synchrone (legacy) ─────────────────────────────────────────────────

pub fn export_media(
    udid: Option<&str>,
    files: Vec<AfcFileExport>,
    dest_dir: &str,
) -> Result<AfcExportResult, String> {
    let s = AfcSession::open(udid)?;
    let dest = Path::new(dest_dir).to_path_buf();
    fs::create_dir_all(&dest).map_err(|e| e.to_string())?;

    let mut exported = 0usize;
    let mut icloud = 0usize;
    let mut errors = 0usize;
    let mut samples = Vec::new();

    for f in &files {
        match copy_one(&s, f, &dest, exported) {
            Ok(CopyOutcome::Copied) => exported += 1,
            Ok(CopyOutcome::ICloudOnly) => icloud += 1,
            Err(e) => {
                errors += 1;
                if samples.len() < 5 { samples.push(format!("{}: {}", f.filename, e)); }
            }
        }
    }

    Ok(AfcExportResult {
        exported, icloud_only: icloud, errors, error_samples: samples,
        dest_dir: dest_dir.to_string(),
    })
}

fn safe_preview_filename(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    let trimmed = cleaned.trim().trim_matches('.').trim().to_string();
    if trimmed.is_empty() { "preview_media".to_string() } else { trimmed }
}

pub fn open_media_preview(udid: Option<&str>, file: AfcFileExport) -> Result<String, String> {
    let s = AfcSession::open(udid)?;
    let info = s.file_info(&file.object_id).unwrap_or(FileInfo::default());
    if info.is_file() && info.size == 0 {
        return Err("Ce fichier est uniquement sur iCloud et n'est pas disponible localement sur l'iPhone.".to_string());
    }

    let dir = std::env::temp_dir().join("PanicBasePreview");
    fs::create_dir_all(&dir).map_err(|e| format!("preview dir: {e}"))?;

    let filename = safe_preview_filename(&file.filename);
    let dst = dir.join(filename);
    let part = dst.with_extension(format!(
        "{}part",
        dst.extension()
            .and_then(|e| e.to_str())
            .map(|e| format!("{e}."))
            .unwrap_or_default()
    ));

    let mut fp = fs::File::create(&part).map_err(|e| format!("preview create: {e}"))?;
    match s.copy_file_to(&file.object_id, &mut fp) {
        Ok(0) => {
            let _ = fs::remove_file(&part);
            return Err("Ce fichier est uniquement sur iCloud et n'est pas disponible localement sur l'iPhone.".to_string());
        }
        Ok(_) => {
            drop(fp);
            if dst.exists() {
                let _ = fs::remove_file(&dst);
            }
            fs::rename(&part, &dst).map_err(|e| {
                let _ = fs::remove_file(&part);
                format!("preview rename: {e}")
            })?;
        }
        Err(e) => {
            drop(fp);
            let _ = fs::remove_file(&part);
            return Err(e);
        }
    }

    #[cfg(target_os = "windows")]
    std::process::Command::new("cmd")
        .args(["/C", "start", "", &dst.to_string_lossy()])
        .spawn()
        .map_err(|e| e.to_string())?;
    #[cfg(target_os = "macos")]
    std::process::Command::new("open").arg(&dst).spawn().map_err(|e| e.to_string())?;
    #[cfg(target_os = "linux")]
    std::process::Command::new("xdg-open").arg(&dst).spawn().map_err(|e| e.to_string())?;

    Ok(dst.to_string_lossy().to_string())
}

enum CopyOutcome { Copied, ICloudOnly }

fn copy_one(s: &AfcSession, f: &AfcFileExport, dest: &Path, exported_count: usize) -> Result<CopyOutcome, String> {
    // 1) iCloud-only ? Détecté via taille = 0
    let info = s.file_info(&f.object_id).unwrap_or(FileInfo::default());
    if info.is_file() && info.size == 0 {
        return Ok(CopyOutcome::ICloudOnly);
    }

    // 2) destination — strip directory components to prevent path traversal
    let safe_name = Path::new(&f.filename)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| format!("file_{exported_count}"));
    let mut dst = dest.join(&safe_name);
    if dst.exists() {
        let stem = Path::new(&safe_name).file_stem().and_then(|x| x.to_str()).unwrap_or("file");
        let ext  = Path::new(&safe_name).extension().and_then(|x| x.to_str()).unwrap_or("");
        dst = dest.join(format!("{stem}_{exported_count}.{ext}"));
    }

    // 3) Stream direct disque vers un fichier temporaire.
    // Si l'iPhone est debranche au milieu, on ne laisse pas un fichier final corrompu.
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let part = dest.join(format!(".panicbase_export_{exported_count}_{stamp}.part"));
    let mut fp = fs::File::create(&part).map_err(|e| format!("create: {e}"))?;
    match s.copy_file_to(&f.object_id, &mut fp) {
        Ok(0) => {
            let _ = fs::remove_file(&part);
            Ok(CopyOutcome::ICloudOnly)
        }
        Ok(_) => {
            drop(fp);
            fs::rename(&part, &dst).map_err(|e| {
                let _ = fs::remove_file(&part);
                format!("rename: {e}")
            })?;
            Ok(CopyOutcome::Copied)
        }
        Err(e) => {
            drop(fp);
            let _ = fs::remove_file(&part);
            Err(e)
        }
    }
}

enum MonitoredCopy {
    Finished(Result<CopyOutcome, String>),
    Disconnected,
}

fn copy_one_monitored(
    udid: Option<String>,
    f: AfcFileExport,
    dest: PathBuf,
    exported_count: usize,
    cancel: Arc<AtomicBool>,
) -> MonitoredCopy {
    let (tx, rx) = mpsc::sync_channel(1);
    let udid_for_worker = udid.clone();
    let _ = thread::Builder::new()
        .name("afc-copy-one".into())
        .spawn(move || {
            let result = AfcSession::open(udid_for_worker.as_deref())
                .and_then(|s| copy_one(&s, &f, &dest, exported_count));
            let _ = tx.send(result);
        });

    loop {
        if cancel.load(Ordering::Relaxed) {
            return MonitoredCopy::Finished(Err("cancelled".to_string()));
        }

        match rx.recv_timeout(Duration::from_millis(900)) {
            Ok(result) => return MonitoredCopy::Finished(result),
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if !ping(udid.as_deref()) {
                    return MonitoredCopy::Disconnected;
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return MonitoredCopy::Finished(Err("thread copie interrompu".to_string()));
            }
        }
    }
}

// ── Export progressif (events Tauri + cancel + détection disconnect) ──────────

pub fn export_media_progressive(
    app: AppHandle,
    udid: Option<String>,
    files: Vec<AfcFileExport>,
    dest_dir: String,
    cancel: Arc<AtomicBool>,
) {
    let total = files.len();
    let dest_path = Path::new(&dest_dir).to_path_buf();
    if let Err(e) = fs::create_dir_all(&dest_path) {
        let _ = app.emit("afc-export-error", format!("Impossible de créer le dossier : {e}"));
        return;
    }

    // Verification initiale. Les copies sont ensuite executees sous watchdog :
    // si libimobiledevice reste bloquee pendant un debranchement USB, le thread
    // principal d'export peut quand meme mettre l'UI en pause.
    if let Err(e) = AfcSession::open(udid.as_deref()) {
        let _ = app.emit("afc-export-error", e);
        return;
    }

    let mut exported = 0usize;
    let mut failed = 0usize;
    let mut icloud = 0usize;
    let mut completed_ids: Vec<String> = Vec::with_capacity(total);
    let mut consecutive_errors = 0usize;

    for (i, f) in files.iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            let _ = app.emit("afc-export-done", DoneEvent {
                exported, failed, skipped_cloud: icloud, dest_dir: dest_dir.clone(),
            });
            return;
        }

        // Progress event (à chaque fichier — UI fluide)
        let _ = app.emit("afc-export-progress", ProgressEvent {
            current: i + 1, total,
            filename: f.filename.clone(),
            exported, failed,
        });

        match copy_one_monitored(
            udid.clone(),
            f.clone(),
            dest_path.clone(),
            exported,
            Arc::clone(&cancel),
        ) {
            MonitoredCopy::Finished(Ok(CopyOutcome::Copied)) => {
                exported += 1;
                completed_ids.push(f.object_id.clone());
                consecutive_errors = 0;
            }
            MonitoredCopy::Finished(Ok(CopyOutcome::ICloudOnly)) => {
                icloud += 1;
                completed_ids.push(f.object_id.clone());
                consecutive_errors = 0;
            }
            MonitoredCopy::Disconnected => {
                let _ = app.emit("afc-export-paused", PausedEvent {
                    current: i + 1, total, exported,
                    failed,
                    skipped_cloud: icloud,
                    completed_ids: completed_ids.clone(),
                });
                return;
            }
            MonitoredCopy::Finished(Err(e)) => {
                if e == "cancelled" {
                    let _ = app.emit("afc-export-done", DoneEvent {
                        exported, failed, skipped_cloud: icloud, dest_dir: dest_dir.clone(),
                    });
                    return;
                }

                consecutive_errors += 1;

                // Échec → on teste IMMÉDIATEMENT si l'iPhone est encore joignable.
                // Si non → pause sans attendre l'accumulation de 3 erreurs (gain de
                // réactivité énorme côté UI quand on débranche en plein export).
                let alive = ping(udid.as_deref());
                if !alive {
                    let _ = app.emit("afc-export-paused", PausedEvent {
                        current: i + 1, total, exported,
                        failed,
                        skipped_cloud: icloud,
                        completed_ids: completed_ids.clone(),
                    });
                    return;
                }

                // iPhone toujours joignable → c'est bien un échec individuel
                failed += 1;

                if consecutive_errors >= 3 {
                    thread::sleep(Duration::from_millis(300));
                    match AfcSession::open(udid.as_deref()) {
                        Ok(_) => {
                            consecutive_errors = 0;
                        }
                        Err(_) => {
                            let _ = app.emit("afc-export-paused", PausedEvent {
                                current: i + 1, total, exported,
                                failed,
                                skipped_cloud: icloud,
                                completed_ids: completed_ids.clone(),
                            });
                            return;
                        }
                    }
                }
            }
        }
    }

    let _ = app.emit("afc-export-done", DoneEvent {
        exported, failed, skipped_cloud: icloud, dest_dir,
    });
}

// ── Prefetch des tailles (async, après l'affichage initial) ───────────────────

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AfcSizeEntry {
    pub object_id: String,
    pub size_bytes: u64,
    pub mtime_ns: i64,
}

pub fn prefetch_sizes(udid: Option<&str>, ids: Vec<String>) -> Result<Vec<AfcSizeEntry>, String> {
    if ids.is_empty() { return Ok(Vec::new()); }
    let max_threads = ids.len().min(4);
    let queue: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(ids));
    let out: Arc<Mutex<Vec<AfcSizeEntry>>> = Arc::new(Mutex::new(Vec::new()));

    let mut handles = Vec::new();
    for _ in 0..max_threads {
        let queue = Arc::clone(&queue);
        let out = Arc::clone(&out);
        let udid_owned = udid.map(|s| s.to_string());
        let h = thread::Builder::new()
            .name("afc-sizes".into())
            .spawn(move || -> Result<(), String> {
                let s = AfcSession::open(udid_owned.as_deref())?;
                loop {
                    let next = { let mut q = queue.lock().unwrap(); q.pop() };
                    let Some(id) = next else { break };
                    let info = s.file_info(&id).unwrap_or(FileInfo::default());
                    let mut g = out.lock().unwrap();
                    g.push(AfcSizeEntry {
                        object_id: id,
                        size_bytes: info.size,
                        mtime_ns: info.mtime_ns,
                    });
                }
                Ok(())
            })
            .map_err(|e| format!("spawn sizes : {e}"))?;
        handles.push(h);
    }
    for h in handles { let _ = h.join(); }
    Ok(Arc::try_unwrap(out).map_err(|_| "Arc occupé".to_string())?
        .into_inner().map_err(|e| format!("Mutex empoisonné : {e}"))?)
}

// ── Ping AFC (utilisé par la modale de reconnexion) ───────────────────────────

pub fn ping(udid: Option<&str>) -> bool {
    let (tx, rx) = mpsc::sync_channel(1);
    let udid_owned = udid.map(|s| s.to_string());
    let _ = thread::Builder::new()
        .name("afc-ping".into())
        .spawn(move || {
            let _ = tx.send(AfcSession::open(udid_owned.as_deref()).is_ok());
        });
    rx.recv_timeout(Duration::from_millis(900)).unwrap_or(false)
}

// ── Suppression ──────────────────────────────────────────────────────────────

pub fn delete_items(udid: Option<&str>, object_ids: Vec<String>) -> Result<u32, String> {
    let s = AfcSession::open(udid)?;
    let mut n = 0u32;
    for id in &object_ids {
        if s.remove_path(id).is_ok() {
            n += 1;
            // Tente aussi de retirer le thumb pré-rendu correspondant pour libérer de l'espace.
            if let Some(t) = derive_thumb_path(id) {
                let _ = s.remove_path(&t);
                let _ = s.remove_path(&t.replace("5005.JPG", "5003.JPG"));
            }
        }
    }
    Ok(n)
}
