//! Base SQLite locale (`%LOCALAPPDATA%\\PanicBase\\panicbase.db` sur Windows).
//! Schéma aligné roadmap — remplissage complet en MVP 0.2+.

use std::path::PathBuf;

use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;

use crate::{analyzer, anonymizer};

pub fn db_path() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("PanicBase")
        .join("panicbase.db")
}

pub fn open_connection() -> Result<Connection, String> {
    let path = db_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    Connection::open(&path).map_err(|e| e.to_string())
}

const INIT_SQL: &str = r"
CREATE TABLE IF NOT EXISTS panic_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    device_model TEXT,
    ios_version TEXT,
    panic_date TEXT,
    raw_path TEXT,
    anonymized_text TEXT,
    signature TEXT,
    signature_hash TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS analysis_results (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    panic_log_id INTEGER NOT NULL REFERENCES panic_logs(id),
    probable_cause TEXT,
    confidence INTEGER,
    explanation TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS repair_confirmations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    panic_log_id INTEGER NOT NULL REFERENCES panic_logs(id),
    repair_type TEXT,
    success INTEGER NOT NULL,
    technician_note TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS community_matches (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    panic_log_id INTEGER NOT NULL REFERENCES panic_logs(id),
    signature_hash TEXT NOT NULL,
    model TEXT,
    similar_count INTEGER,
    top_cause TEXT,
    top_cause_percent REAL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_panic_logs_signature ON panic_logs(signature_hash);
";

/// Crée les tables si besoin. À appeler au démarrage de l’application.
pub fn init_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch("PRAGMA foreign_keys = ON;")
        .map_err(|e| e.to_string())?;
    conn.execute_batch(INIT_SQL).map_err(|e| e.to_string())
}

/// Initialise le fichier et le schéma.
pub fn bootstrap() -> Result<(), String> {
    let c = open_connection()?;
    init_schema(&c)
}

#[derive(Debug, Serialize)]
pub struct PanicLogSummary {
    pub id: i64,
    pub device_model: Option<String>,
    pub ios_version: Option<String>,
    pub panic_date: Option<String>,
    pub signature_hash: Option<String>,
    pub created_at: String,
}

pub fn list_local_panic_logs() -> Result<Vec<PanicLogSummary>, String> {
    let conn = open_connection()?;
    init_schema(&conn)?;
    let mut stmt = conn
        .prepare(
            "SELECT id, device_model, ios_version, panic_date, signature_hash, created_at
             FROM panic_logs ORDER BY id DESC LIMIT 200",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok(PanicLogSummary {
                id: row.get(0)?,
                device_model: row.get(1)?,
                ios_version: row.get(2)?,
                panic_date: row.get(3)?,
                signature_hash: row.get(4)?,
                created_at: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(|e| e.to_string())?);
    }
    Ok(out)
}

pub fn insert_repair_confirmation(
    panic_log_id: i64,
    repair_type: &str,
    success: bool,
    technician_note: Option<&str>,
) -> Result<(), String> {
    let conn = open_connection()?;
    init_schema(&conn)?;
    conn.execute(
        "INSERT INTO repair_confirmations (panic_log_id, repair_type, success, technician_note) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![panic_log_id, repair_type, success as i32, technician_note],
    )
    .map_err(|e| format!("confirmation impossible (log #{panic_log_id} existe-t‑il dans panic_logs?) — {e}"))?;
    Ok(())
}

pub fn read_panic_log_text(log_id: i64) -> Result<String, String> {
    let conn = open_connection()?;
    init_schema(&conn)?;
    let text: Option<String> = conn
        .query_row(
            "SELECT COALESCE(anonymized_text, '') FROM panic_logs WHERE id = ?1",
            [log_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    text.filter(|s| !s.is_empty())
        .ok_or_else(|| format!("Aucun enregistrement PanicBase #{log_id} dans la base locale (MVP 0.2 : importer / enregistrer un log)."))
}

/// Chemin du dossier où `idevicecrashreport` écrit les crashs (MVP 0.2).
pub fn crash_reports_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("PanicBase")
        .join("crash_reports")
}

pub fn ensure_crash_reports_dir() -> Result<PathBuf, String> {
    let p = crash_reports_dir();
    std::fs::create_dir_all(&p).map_err(|e| e.to_string())?;
    Ok(p)
}

/// Enregistre un panic importé (IPS) : texte anonymisé, signature, ligne d’analyse liée.
pub fn insert_panic_with_analysis_local(
    panic_plaintext: &str,
    device_hint: Option<&str>,
    raw_path_label: Option<&str>,
    ios_version_hint: Option<&str>,
) -> Result<i64, String> {
    let analysis = analyzer::analyze_panic_log(panic_plaintext, device_hint, None);
    let anonymized = anonymizer::anonymize_panic_log(panic_plaintext);

    let mut conn = open_connection()?;
    init_schema(&conn)?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    tx.execute(
        "INSERT INTO panic_logs (device_model, ios_version, panic_date, raw_path, anonymized_text, signature, signature_hash) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![
            analysis.device_model,
            ios_version_hint,
            Option::<String>::None,
            raw_path_label,
            anonymized,
            analysis.signature,
            analysis.signature_hash,
        ],
    )
    .map_err(|e| e.to_string())?;

    let id = tx.last_insert_rowid();

    tx.execute(
        "INSERT INTO analysis_results (panic_log_id, probable_cause, confidence, explanation) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![
            id,
            analysis.probable_cause,
            i64::from(analysis.confidence),
            analysis.explanation,
        ],
    )
    .map_err(|e| e.to_string())?;

    tx.commit().map_err(|e| e.to_string())?;
    Ok(id)
}

