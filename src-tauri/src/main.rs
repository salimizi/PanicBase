#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::fs;

mod analyzer;
mod anonymizer;
mod api_client;
mod community;
mod database;
mod knowledge;
mod panic_diagnostic;
mod repair_wiki;
mod ips;
mod iphone;
mod panic_parser;
mod panic_pull;
mod security;
mod signature;
mod trust_score;

use analyzer::AnalysisResult;
use ips::IpsInterpretOutcome;
use panic_pull::{
    analyze_pulled_device_panic, pull_device_recent_panic_logs, PulledPanicState,
};

#[tauri::command]
fn detect_iphone() -> iphone::IphoneUsbStatus {
    iphone::detect_iphone_usb()
}

#[tauri::command]
fn extract_panic_logs() -> Result<String, String> {
    iphone::extract_panic_logs()
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
    let _ = (log_id, api_client::API_BASE_PLACEHOLDER, security::FORBIDDEN_MARKERS);
    Err(
        "Envoi vers la base communautaire : MVP 0.3 — consentement explicite obligatoire, aucune requête pour l’instant."
            .into(),
    )
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

/// Enregistre en SQLite un panic importé (IPS) · texte anonymisé + analyse locale.
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

/// Enregistre un fichier texte où l’utilisateur choisit (annulation → `Ok(None)`).
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

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(PulledPanicState::default())
        .setup(|_| {
            if let Err(e) = database::bootstrap() {
                eprintln!("[PanicBase] SQLite : {e}");
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            detect_iphone,
            pull_device_recent_panic_logs,
            analyze_pulled_device_panic,
            extract_panic_logs,
            list_local_panic_logs,
            read_panic_log,
            analyze_panic_log,
            interpret_ips_file,
            anonymize_panic_log,
            generate_signature,
            get_community_stats,
            submit_anonymized_log,
            confirm_repair,
            export_text_file,
            save_imported_panic_to_db
        ])
        .run(tauri::generate_context!())
        .expect("error while running PanicBase");
}
