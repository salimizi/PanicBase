use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::Connection;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContactsExportResult {
    pub count: u32,
    pub dest_path: String,
}

struct Person {
    row_id: i64,
    first: Option<String>,
    last: Option<String>,
    organization: Option<String>,
    department: Option<String>,
    job_title: Option<String>,
    note: Option<String>,
}

struct MultiValue {
    property: i64,
    value: String,
    label: Option<String>,
}

fn find_addressbook_path(backup_root: &Path, udid: &str) -> Option<PathBuf> {
    let manifest_db = backup_root.join(udid).join("Manifest.db");
    let conn = Connection::open(&manifest_db).ok()?;

    let file_id: String = conn
        .query_row(
            "SELECT fileID FROM Files
             WHERE domain = 'HomeDomain'
               AND relativePath = 'Library/AddressBook/AddressBook.sqlitedb'
             LIMIT 1",
            [],
            |row| row.get(0),
        )
        .ok()?;

    if file_id.len() < 2 {
        return None;
    }
    Some(backup_root.join(udid).join(&file_id[..2]).join(&file_id))
}

fn escape_vcard(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace(',', "\\,")
        .replace(';', "\\;")
        .replace('\n', "\\n")
}

fn build_vcard(person: &Person, multi_values: &[MultiValue]) -> String {
    let mut lines: Vec<String> = vec![
        "BEGIN:VCARD".into(),
        "VERSION:3.0".into(),
    ];

    let first = person.first.as_deref().unwrap_or("");
    let last = person.last.as_deref().unwrap_or("");
    let full_name = format!("{} {}", first, last).trim().to_string();

    if full_name.is_empty() {
        if let Some(org) = &person.organization {
            lines.push(format!("FN:{}", escape_vcard(org)));
        } else {
            lines.push("FN:Inconnu".into());
        }
    } else {
        lines.push(format!("FN:{}", escape_vcard(&full_name)));
    }

    lines.push(format!(
        "N:{};{};;;",
        escape_vcard(last),
        escape_vcard(first)
    ));

    if let Some(org) = &person.organization {
        if !org.trim().is_empty() {
            let dept = person.department.as_deref().unwrap_or("");
            lines.push(format!("ORG:{};{}", escape_vcard(org), escape_vcard(dept)));
        }
    }

    if let Some(title) = &person.job_title {
        if !title.trim().is_empty() {
            lines.push(format!("TITLE:{}", escape_vcard(title)));
        }
    }

    for mv in multi_values {
        match mv.property {
            3 => {
                // Phone
                let label_type = match mv.label.as_deref() {
                    Some("_$!<Mobile>!$_") => "CELL",
                    Some("_$!<Home>!$_") => "HOME",
                    Some("_$!<Work>!$_") => "WORK",
                    _ => "VOICE",
                };
                lines.push(format!(
                    "TEL;TYPE={}:{}",
                    label_type,
                    escape_vcard(&mv.value)
                ));
            }
            4 => {
                // Email
                let label_type = match mv.label.as_deref() {
                    Some("_$!<Home>!$_") => "HOME",
                    Some("_$!<Work>!$_") => "WORK",
                    _ => "INTERNET",
                };
                lines.push(format!(
                    "EMAIL;TYPE={}:{}",
                    label_type,
                    escape_vcard(&mv.value)
                ));
            }
            5 => {
                // URL
                lines.push(format!("URL:{}", escape_vcard(&mv.value)));
            }
            _ => {}
        }
    }

    if let Some(note) = &person.note {
        if !note.trim().is_empty() {
            lines.push(format!("NOTE:{}", escape_vcard(note)));
        }
    }

    lines.push("END:VCARD".into());
    lines.join("\r\n") + "\r\n"
}

pub fn export_contacts_to_vcf(udid: &str) -> Result<ContactsExportResult, String> {
    let backup_root = crate::media_transfer::backup_dir_pub();

    let ab_path = find_addressbook_path(&backup_root, udid)
        .ok_or("AddressBook introuvable dans le backup. Lancez d'abord un backup.")?;

    let ab_conn =
        Connection::open(&ab_path).map_err(|e| format!("AddressBook.sqlitedb : {e}"))?;

    let mut person_stmt = ab_conn
        .prepare(
            "SELECT ROWID, First, Last, Organization, Department, JobTitle, Note
             FROM ABPerson ORDER BY Last, First",
        )
        .map_err(|e| e.to_string())?;

    let persons: Vec<Person> = person_stmt
        .query_map([], |row| {
            Ok(Person {
                row_id: row.get(0)?,
                first: row.get(1)?,
                last: row.get(2)?,
                organization: row.get(3)?,
                department: row.get(4)?,
                job_title: row.get(5)?,
                note: row.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect();

    let mut vcf_content = String::new();
    let mut count = 0u32;

    for person in &persons {
        let multi_values: Vec<MultiValue> = {
            let mut mv_stmt = ab_conn
                .prepare(
                    "SELECT mv.property, mv.value, mvl.value
                     FROM ABMultiValue mv
                     LEFT JOIN ABMultiValueLabel mvl ON mv.label = mvl.ROWID
                     WHERE mv.record_id = ?1
                     ORDER BY mv.ordering",
                )
                .map_err(|e| e.to_string())?;

            let rows: Vec<MultiValue> = mv_stmt
                .query_map([person.row_id], |row| {
                    Ok(MultiValue {
                        property: row.get(0)?,
                        value: row.get::<_, String>(1).unwrap_or_default(),
                        label: row.get(2)?,
                    })
                })
                .map_err(|e| e.to_string())?
                .filter_map(Result::ok)
                .collect();
            rows
        };

        vcf_content.push_str(&build_vcard(person, &multi_values));
        count += 1;
    }

    let dest_path = rfd::FileDialog::new()
        .set_title("Enregistrer les contacts")
        .set_file_name("contacts_iphone.vcf")
        .add_filter("vCard", &["vcf"])
        .save_file()
        .map(|p| p.to_string_lossy().into_owned());

    let dest_path = dest_path.ok_or("Export annulÃ©")?;

    fs::write(&dest_path, vcf_content.as_bytes()).map_err(|e| e.to_string())?;

    Ok(ContactsExportResult {
        count,
        dest_path,
    })
}

pub fn count_contacts(udid: &str) -> Result<u32, String> {
    let backup_root = crate::media_transfer::backup_dir_pub();
    let ab_path = find_addressbook_path(&backup_root, udid)
        .ok_or("AddressBook introuvable dans le backup")?;

    let ab_conn =
        Connection::open(&ab_path).map_err(|e| format!("AddressBook.sqlitedb : {e}"))?;

    ab_conn
        .query_row("SELECT COUNT(*) FROM ABPerson", [], |row| row.get::<_, u32>(0))
        .map_err(|e| e.to_string())
}

