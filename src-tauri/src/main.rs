#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::fs;

mod analyzer;
mod anonymizer;
mod crypto;
mod kb_seal;
mod machine_key;
mod reference_focus;
mod api_client;
mod community;
mod database;
mod knowledge;
mod panic_diagnostic;
mod panic_logs_chart;
mod repair_wiki;
mod ips;
mod iphone;
mod panic_parser;
mod panic_pull;
mod security;
mod signature;
mod trust_score;
mod media_transfer;
mod contacts_transfer;
mod afc;
mod gallery_afc;
mod icloud;
mod icloud_photos;

use std::sync::{Arc, atomic::AtomicBool};

struct ExportCancelState(Arc<AtomicBool>);
impl Default for ExportCancelState {
    fn default() -> Self { ExportCancelState(Arc::new(AtomicBool::new(false))) }
}

use analyzer::AnalysisResult;
use ips::IpsInterpretOutcome;
use panic_pull::{
    analyze_pulled_device_panic, export_pulled_device_panic_file, pull_device_recent_panic_logs,
    read_pulled_device_panic_raw, PulledPanicState,
};

#[tauri::command]
async fn exit_iphone_recovery_boot() -> Result<String, String> {
    match tauri::async_runtime::spawn_blocking(iphone::exit_recovery_boot).await {
        Ok(inner) => inner,
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
async fn detect_iphone() -> iphone::IphoneUsbStatus {
    match tauri::async_runtime::spawn_blocking(iphone::detect_iphone_usb).await {
        Ok(s) => s,
        Err(_) => iphone::usb_status(
            "error",
            "Tâche USB interrompue ou plantage interne. Relance PanicBase.",
            vec![],
            None,
            None,
            None,
        ),
    }
}

#[tauri::command]
async fn extract_panic_logs() -> Result<String, String> {
    match tauri::async_runtime::spawn_blocking(iphone::extract_panic_logs).await {
        Ok(inner) => inner,
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
fn list_local_panic_logs() -> Result<Vec<database::PanicLogSummary>, String> {
    database::list_local_panic_logs()
}

#[tauri::command]
fn read_panic_log(log_id: i64) -> Result<String, String> {
    database::read_panic_log_text(log_id)
}

#[tauri::command]
fn analyze_panic_log(log: String, device_hint: Option<String>) -> AnalysisResult {
    analyzer::analyze_panic_log(&log, device_hint.as_deref(), None)
}

#[tauri::command]
fn infer_panic_reference_focus(
    panic_text: String,
    analysis: AnalysisResult,
    product_type: Option<String>,
) -> reference_focus::PanicReferenceFocus {
    reference_focus::infer_panic_reference_focus(
        &panic_text,
        &analysis,
        product_type.as_deref(),
    )
}

#[tauri::command]
fn interpret_ips_file(content: String) -> Result<IpsInterpretOutcome, String> {
    ips::interpret_ips_file(&content)
}

#[tauri::command]
fn anonymize_panic_log(log: String) -> String {
    anonymizer::anonymize_panic_log(&log)
}

#[tauri::command]
fn generate_signature(log: String, model: String) -> signature::SignatureBundle {
    signature::bundle_from_log(&log, model)
}

#[tauri::command]
fn get_community_stats(signature_hash: String, model: Option<String>) -> community::CommunityStats {
    community::get_community_stats(&signature_hash, model.as_deref())
}

#[tauri::command]
fn submit_anonymized_log(log_id: i64) -> Result<(), String> {
    let _ = (log_id, api_client::API_BASE, security::FORBIDDEN_MARKERS);
    Err("unavailable".into())
}

#[tauri::command]
fn confirm_repair(
    log_id: i64,
    repair_type: String,
    success: bool,
    technician_note: Option<String>,
) -> Result<(), String> {
    let _ = trust_score::workshop_trust_placeholder();
    database::insert_repair_confirmation(
        log_id,
        &repair_type,
        success,
        technician_note.as_deref(),
    )
}

#[tauri::command]
fn save_imported_panic_to_db(
    panic_text: String,
    device_hint: Option<String>,
    source_filename: Option<String>,
    ios_version_hint: Option<String>,
) -> Result<i64, String> {
    database::insert_panic_with_analysis_local(
        &panic_text,
        device_hint.as_deref(),
        source_filename.as_deref(),
        ios_version_hint.as_deref(),
    )
}

#[tauri::command]
fn export_text_file(content: String, default_filename: String) -> Result<Option<String>, String> {
    let path = rfd::FileDialog::new()
        .set_file_name(&default_filename)
        .save_file();
    let Some(path) = path else {
        return Ok(None);
    };
    fs::write(&path, content.as_bytes()).map_err(|e| e.to_string())?;
    Ok(Some(path.to_string_lossy().into_owned()))
}

#[tauri::command]
async fn get_iphone_device_details(udid: Option<String>) -> Result<iphone::IphoneDeviceDetails, String> {
    match tauri::async_runtime::spawn_blocking(move || iphone::fetch_iphone_device_details(udid)).await {
        Ok(Ok(d)) => Ok(d),
        Ok(Err(e)) => Err(e),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
async fn get_iphone_device_identifiers(udid: Option<String>) -> Result<iphone::IphoneDeviceDetails, String> {
    match tauri::async_runtime::spawn_blocking(move || iphone::fetch_iphone_device_identifiers(udid)).await {
        Ok(Ok(d)) => Ok(d),
        Ok(Err(e)) => Err(e),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
async fn iphone_device_restart(udid: Option<String>) -> Result<(), String> {
    match tauri::async_runtime::spawn_blocking(move || iphone::idevice_diagnostics_action(udid.as_deref(), "restart")).await {
        Ok(r) => r,
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
async fn iphone_device_shutdown(udid: Option<String>) -> Result<(), String> {
    match tauri::async_runtime::spawn_blocking(move || iphone::idevice_diagnostics_action(udid.as_deref(), "shutdown")).await {
        Ok(r) => r,
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
fn check_device_backup(udid: Option<String>) -> media_transfer::BackupStatus {
    media_transfer::check_existing_backup(udid.as_deref())
}

#[tauri::command]
async fn run_device_backup(udid: Option<String>) -> Result<media_transfer::BackupResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        media_transfer::run_backup(udid.as_deref())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
fn list_device_media(udid: String) -> Result<Vec<media_transfer::MediaItem>, String> {
    media_transfer::list_media(&udid)
}

#[tauri::command]
async fn get_media_thumbnail(
    udid: String,
    file_id: String,
    extension: String,
) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        media_transfer::get_thumbnail(&udid, &file_id, &extension)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn export_selected_media(
    udid: String,
    files: Vec<media_transfer::MediaFileExport>,
) -> Result<media_transfer::ExportResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let dest = media_transfer::pick_export_folder()
            .ok_or_else(|| "Export annulé".to_string())?;
        media_transfer::export_media(&udid, files, &dest)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn count_device_contacts(udid: String) -> Result<u32, String> {
    tauri::async_runtime::spawn_blocking(move || contacts_transfer::count_contacts(&udid))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn export_device_contacts_vcf(
    udid: String,
) -> Result<contacts_transfer::ContactsExportResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        contacts_transfer::export_contacts_to_vcf(&udid)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
fn afc_available() -> bool {
    afc::afc_available()
}

#[tauri::command]
async fn list_afc_gallery(udid: Option<String>) -> Result<Vec<gallery_afc::AfcMediaItem>, String> {
    tauri::async_runtime::spawn_blocking(move || gallery_afc::list_media(udid.as_deref()))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn get_afc_thumbnail(udid: Option<String>, object_id: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || gallery_afc::get_thumbnail(udid.as_deref(), &object_id))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn export_afc_media(
    udid: Option<String>,
    files: Vec<gallery_afc::AfcFileExport>,
) -> Result<gallery_afc::AfcExportResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let dest = media_transfer::pick_export_folder()
            .ok_or_else(|| "Export annulé".to_string())?;
        gallery_afc::export_media(udid.as_deref(), files, &dest)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn open_afc_media_preview(
    udid: Option<String>,
    file: gallery_afc::AfcFileExport,
) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        gallery_afc::open_media_preview(udid.as_deref(), file)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn open_icloud_media_preview(
    filename: String,
    download_url: String,
) -> Result<String, String> {
    use std::fs;
    let bytes = {
        let client = reqwest::Client::builder()
            .user_agent(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
                 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36",
            )
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .map_err(|e| format!("http client: {e}"))?;
        let resp = client
            .get(&download_url)
            .header("Origin", "https://www.icloud.com")
            .header("Referer", "https://www.icloud.com/")
            .send()
            .await
            .map_err(|e| format!("download GET: {e}"))?;
        if !resp.status().is_success() {
            return Err(format!(
                "Apple a refusé le téléchargement ({}). L'URL a peut-être expiré (TTL ~5 min) — recharge la photothèque.",
                resp.status()
            ));
        }
        resp.bytes()
            .await
            .map_err(|e| format!("download body: {e}"))?
            .to_vec()
    };

    // Nom de fichier sain : on garde l'extension, on neutralise les caractères
    // Windows interdits dans le stem. Évite aussi qu'un nom comme `../IMG.JPG`
    // serve à écrire en dehors du dossier preview.
    let safe = {
        let stem = std::path::Path::new(&filename)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("preview");
        let ext = std::path::Path::new(&filename)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("bin");
        let clean: String = stem
            .chars()
            .map(|c| if matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') { '_' } else { c })
            .collect();
        format!("{clean}.{ext}")
    };

    let dir = std::env::temp_dir().join("PanicBasePreview");
    fs::create_dir_all(&dir).map_err(|e| format!("preview dir: {e}"))?;
    let dst = dir.join(&safe);
    if dst.exists() {
        let _ = fs::remove_file(&dst);
    }
    fs::write(&dst, &bytes).map_err(|e| format!("preview write: {e}"))?;

    #[cfg(target_os = "windows")]
    std::process::Command::new("cmd")
        .args(["/C", "start", "", &dst.to_string_lossy()])
        .spawn()
        .map_err(|e| e.to_string())?;
    #[cfg(target_os = "macos")]
    std::process::Command::new("open")
        .arg(&dst)
        .spawn()
        .map_err(|e| e.to_string())?;
    #[cfg(target_os = "linux")]
    std::process::Command::new("xdg-open")
        .arg(&dst)
        .spawn()
        .map_err(|e| e.to_string())?;

    Ok(dst.to_string_lossy().to_string())
}

#[tauri::command]
async fn pick_export_folder_cmd() -> Option<String> {
    tauri::async_runtime::spawn_blocking(media_transfer::pick_export_folder)
        .await
        .unwrap_or(None)
}

#[tauri::command]
async fn start_afc_export(
    app: tauri::AppHandle,
    state: tauri::State<'_, ExportCancelState>,
    udid: Option<String>,
    files: Vec<gallery_afc::AfcFileExport>,
    dest_dir: String,
) -> Result<(), String> {
    let cancel = state.0.clone();
    cancel.store(false, std::sync::atomic::Ordering::Relaxed);
    std::thread::spawn(move || {
        gallery_afc::export_media_progressive(app, udid, files, dest_dir, cancel);
    });
    Ok(())
}

#[tauri::command]
fn cancel_afc_export(state: tauri::State<'_, ExportCancelState>) {
    state.0.store(true, std::sync::atomic::Ordering::Relaxed);
}

#[tauri::command]
async fn delete_afc_items(
    udid: Option<String>,
    object_ids: Vec<String>,
) -> Result<u32, String> {
    tauri::async_runtime::spawn_blocking(move || {
        gallery_afc::delete_items(udid.as_deref(), object_ids)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn prefetch_afc_sizes(
    udid: Option<String>,
    object_ids: Vec<String>,
) -> Result<Vec<gallery_afc::AfcSizeEntry>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        gallery_afc::prefetch_sizes(udid.as_deref(), object_ids)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn ping_afc(udid: Option<String>) -> bool {
    tauri::async_runtime::spawn_blocking(move || gallery_afc::ping(udid.as_deref()))
        .await
        .unwrap_or(false)
}

#[tauri::command]
async fn get_iphone_disk_usage(udid: Option<String>) -> Result<iphone::DiskUsage, String> {
    tauri::async_runtime::spawn_blocking(move || iphone::get_disk_usage(udid.as_deref()))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn open_icloud_window(
    app: tauri::AppHandle,
    state: tauri::State<'_, icloud::SessionState>,
) -> Result<(), String> {
    use tauri::{Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

    if let Some(w) = app.get_webview_window("icloud") {
        let _ = w.show();
        let _ = w.set_focus();
        let _ = w.unminimize();
        return Ok(());
    }

    let url: tauri::Url = "https://www.icloud.com/photos/"
        .parse()
        .map_err(|e| format!("URL invalide : {e}"))?;

    let bridge = app.state::<icloud::ICloudBridge>();
    let responses = bridge.responses.clone();
    let notify = bridge.notify.clone();

    let window = WebviewWindowBuilder::new(&app, "icloud", WebviewUrl::External(url))
        .title("iCloud Photos · PanicBase")
        .inner_size(1180.0, 820.0)
        .min_inner_size(900.0, 600.0)
        .resizable(true)
        .center()
        .devtools(cfg!(debug_assertions))
        .on_navigation(move |target| {
            if target.host_str() == Some(icloud::BRIDGE_HOST) {
                let mut marker = String::new();
                let mut status: u16 = 0;
                let mut body = String::new();
                let mut seq: usize = 0;
                let mut total: usize = 1;
                for (k, v) in target.query_pairs() {
                    match k.as_ref() {
                        "marker" => marker = v.into_owned(),
                        "status" => status = v.parse().unwrap_or(0),
                        "body" => body = v.into_owned(),
                        "seq" => seq = v.parse().unwrap_or(0),
                        "total" => total = v.parse().unwrap_or(1).max(1),
                        _ => {}
                    }
                }
                if !marker.is_empty() {
                    if let Ok(mut g) = responses.lock() {
                        let entry = g.entry(marker.clone()).or_default();
                        entry.status = status;
                        entry.total = total;
                        entry.chunks.insert(seq, body);
                    }
                    notify.notify_waiters();
                }
                return false;
            }
            let host = target.host_str().unwrap_or("");
            if !host.ends_with("icloud.com")
                && !host.ends_with("apple.com")
                && !host.is_empty()
            {
            }
            true
        })
        .build()
        .map_err(|e| format!("Création fenêtre iCloud : {e}"))?;

    let watcher_window = window.clone();
    let watcher_app = app.clone();
    let watcher_state = state.inner().clone();
    tauri::async_runtime::spawn(async move {
        let max_attempts: u32 = 600;
        let mut signaled = false;
        let mut auth_cycles: u32 = 0;
        for _ in 0..max_attempts {
            tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
            if watcher_app.get_webview_window("icloud").is_none() {
                let has_session = watcher_state
                    .lock()
                    .map(|g| g.is_some())
                    .unwrap_or(false);
                if !has_session {
                    let _ = watcher_app.emit("icloud-session-cancelled", ());
                }
                return;
            }
            if watcher_state.lock().map(|g| g.is_some()).unwrap_or(false) {
                return;
            }
            let url = watcher_window
                .url()
                .map(|u| u.to_string())
                .unwrap_or_default();
            let on_icloud = url.contains("icloud.com") && !url.contains("idmsa.apple.com");
            if !signaled && on_icloud {
                let _ = watcher_app.emit("icloud-session-pending", ());
                signaled = true;
            }
            if !on_icloud {
                auth_cycles = 0;
                continue;
            }
            let cookies: Vec<icloud::CookiePair> = watcher_window
                .cookies()
                .ok()
                .map(|list| {
                    list.into_iter()
                        .map(|c| icloud::CookiePair {
                            name: c.name().to_string(),
                            value: c.value().to_string(),
                        })
                        .collect()
                })
                .unwrap_or_default();
            if !icloud::is_authenticated(&cookies) {
                auth_cycles = 0;
                continue;
            }
            auth_cycles += 1;
            if auth_cycles < 2 {
                continue;
            }
            let bridge = watcher_app.state::<icloud::ICloudBridge>();
            match icloud::validate_session_via_webview(&watcher_window, bridge.inner()).await {
                Ok(session) => {
                    let public = session.to_public();
                    if let Ok(mut g) = watcher_state.lock() {
                        *g = Some(session);
                    }
                    let _ = watcher_app.emit("icloud-session-ready", public);
                    let _ = watcher_window.hide();
                    return;
                }
                Err(_e) => {
                    auth_cycles = 0;
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                }
            }
        }
        let _ = watcher_app.emit("icloud-session-timeout", ());
    });

    Ok(())
}

#[tauri::command]
fn get_icloud_session(
    state: tauri::State<'_, icloud::SessionState>,
) -> Option<icloud::ICloudSessionPublic> {
    state
        .lock()
        .ok()
        .and_then(|g| g.as_ref().map(|s| s.to_public()))
}

#[tauri::command]
fn icloud_sign_out(
    app: tauri::AppHandle,
    state: tauri::State<'_, icloud::SessionState>,
) -> Result<(), String> {
    use tauri::Manager;
    let mut g = state.lock().map_err(|e| e.to_string())?;
    *g = None;
    if let Some(w) = app.get_webview_window("icloud") {
        let _ = w.close();
    }
    Ok(())
}

#[tauri::command]
async fn icloud_complete_login(
    app: tauri::AppHandle,
    state: tauri::State<'_, icloud::SessionState>,
    bridge: tauri::State<'_, icloud::ICloudBridge>,
) -> Result<icloud::ICloudSessionPublic, String> {
    use tauri::{Emitter, Manager};

    let window = app
        .get_webview_window("icloud")
        .ok_or_else(|| "Fenêtre iCloud introuvable — ouvre-la d'abord.".to_string())?;

    let session = icloud::validate_session_via_webview(&window, bridge.inner()).await?;
    let public = session.to_public();

    {
        let mut guard = state.lock().map_err(|e| e.to_string())?;
        *guard = Some(session);
    }

    let _ = app.emit("icloud-session-ready", public.clone());
    let _ = window.hide();
    Ok(public)
}

#[tauri::command]
async fn icloud_list_photos(
    app: tauri::AppHandle,
    state: tauri::State<'_, icloud::SessionState>,
    bridge: tauri::State<'_, icloud::ICloudBridge>,
) -> Result<Vec<icloud_photos::ICloudAsset>, String> {
    use tauri::{Emitter, Manager};
    let session = {
        let guard = state.lock().map_err(|e| e.to_string())?;
        guard
            .as_ref()
            .ok_or("Aucune session iCloud active")?
            .clone()
    };
    let window = app
        .get_webview_window("icloud")
        .ok_or_else(|| {
            "La fenêtre iCloud doit rester ouverte pendant l'import — relance \
             la connexion."
                .to_string()
        })?;
    let app_clone = app.clone();
    let result = icloud_photos::list_assets(
        &window,
        bridge.inner(),
        &session,
        move |n| {
            let _ = app_clone.emit("icloud-list-progress", n);
        },
        &app,
    )
    .await;

    match result {
        Ok(assets) => {
            let _ = app.emit("icloud-list-done", assets.len());

            let bg_window = window.clone();
            let bg_session = session.clone();
            let bg_bridge = icloud::ICloudBridge {
                responses: bridge.inner().responses.clone(),
                notify: bridge.inner().notify.clone(),
            };
            let bg_app = app.clone();
            tauri::async_runtime::spawn(async move {
                icloud_photos::scan_albums_in_background(
                    bg_window,
                    bg_bridge,
                    bg_session,
                    bg_app,
                )
                .await;
            });

            Ok(assets)
        }
        Err(e) => Err(e),
    }
}

#[tauri::command]
async fn start_icloud_export(
    app: tauri::AppHandle,
    state: tauri::State<'_, ExportCancelState>,
    bridge: tauri::State<'_, icloud::ICloudBridge>,
    files: Vec<icloud_photos::ICloudFileExport>,
    dest_dir: String,
) -> Result<(), String> {
    use tauri::Manager;
    let window = app
        .get_webview_window("icloud")
        .ok_or_else(|| {
            "Session iCloud fermée — reconnecte-toi avant d'exporter.".to_string()
        })?;
    let cancel = state.0.clone();
    cancel.store(false, std::sync::atomic::Ordering::Relaxed);
    let responses = bridge.responses.clone();
    let notify = bridge.notify.clone();
    let app_clone = app.clone();
    tauri::async_runtime::spawn(async move {
        icloud_photos::export_assets_progressive(
            app_clone, window, responses, notify, files, dest_dir, cancel,
        )
        .await;
    });
    Ok(())
}

#[tauri::command]
fn cancel_icloud_export(state: tauri::State<'_, ExportCancelState>) {
    state.0.store(true, std::sync::atomic::Ordering::Relaxed);
}

#[tauri::command]
async fn icloud_thumbnail_data(
    app: tauri::AppHandle,
    _state: tauri::State<'_, icloud::SessionState>,
    bridge: tauri::State<'_, icloud::ICloudBridge>,
    url: String,
) -> Result<String, String> {
    use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
    use tauri::Manager;
    let window = app
        .get_webview_window("icloud")
        .ok_or_else(|| "Fenêtre iCloud fermée — reconnecte-toi.".to_string())?;
    let bytes = icloud_photos::download_url_binary(&window, bridge.inner(), &url).await?;
    Ok(B64.encode(&bytes))
}

#[tauri::command]
fn open_folder(path: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    std::process::Command::new("explorer").arg(&path).spawn().map_err(|e| e.to_string())?;
    #[cfg(target_os = "macos")]
    std::process::Command::new("open").arg(&path).spawn().map_err(|e| e.to_string())?;
    #[cfg(target_os = "linux")]
    std::process::Command::new("xdg-open").arg(&path).spawn().map_err(|e| e.to_string())?;
    Ok(())
}

fn init_bundled_libimobiledevice_from_resources<R: tauri::Runtime>(app: &impl tauri::Manager<R>) {
    use tauri::path::BaseDirectory;

    let rel = if cfg!(windows) {
        "libimobiledevice/idevice_id.exe"
    } else {
        "libimobiledevice/idevice_id"
    };
    if let Ok(p) = app.path().resolve(rel, BaseDirectory::Resource) {
        if p.is_file() {
            if let Some(dir) = p.parent() {
                iphone::set_bundled_libimobiledevice_dir(dir.to_path_buf());
            }
        }
    }
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(PulledPanicState::default())
        .manage(ExportCancelState::default())
        .manage(icloud::new_session_state())
        .manage(icloud::ICloudBridge::default())
        .setup(|app| {
            if let Err(e) = database::bootstrap() {
                eprintln!("[PanicBase] SQLite : {e}");
            }
            init_bundled_libimobiledevice_from_resources(app);
            iphone::try_init_bundled_neighbor_if_unset();
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            detect_iphone,
            exit_iphone_recovery_boot,
            get_iphone_device_details,
            get_iphone_device_identifiers,
            iphone_device_restart,
            iphone_device_shutdown,
            pull_device_recent_panic_logs,
            analyze_pulled_device_panic,
            export_pulled_device_panic_file,
            read_pulled_device_panic_raw,
            extract_panic_logs,
            list_local_panic_logs,
            read_panic_log,
            analyze_panic_log,
            infer_panic_reference_focus,
            interpret_ips_file,
            anonymize_panic_log,
            generate_signature,
            get_community_stats,
            submit_anonymized_log,
            confirm_repair,
            export_text_file,
            save_imported_panic_to_db,
            check_device_backup,
            run_device_backup,
            list_device_media,
            get_media_thumbnail,
            export_selected_media,
            count_device_contacts,
            export_device_contacts_vcf,
            afc_available,
            list_afc_gallery,
            get_afc_thumbnail,
            export_afc_media,
            open_afc_media_preview,
            open_icloud_media_preview,
            pick_export_folder_cmd,
            start_afc_export,
            cancel_afc_export,
            delete_afc_items,
            prefetch_afc_sizes,
            ping_afc,
            get_iphone_disk_usage,
            open_icloud_window,
            get_icloud_session,
            icloud_sign_out,
            icloud_complete_login,
            icloud_list_photos,
            icloud_thumbnail_data,
            start_icloud_export,
            cancel_icloud_export,
            open_folder
        ])
        .run(tauri::generate_context!())
        .expect("error while running PanicBase");
}
