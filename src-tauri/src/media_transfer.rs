use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use base64::Engine;
use serde::Serialize;

use crate::iphone::resolved_libimobile_tool;

// â”€â”€ Paths â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

pub fn backup_dir_pub() -> PathBuf {
    backup_dir()
}

pub fn backup_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("PanicBase")
        .join("device_backup")
}

// â”€â”€ Types â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaItem {
    pub file_id: String,
    pub relative_path: String,
    pub filename: String,
    pub extension: String,
    pub is_video: bool,
    pub folder: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupStatus {
    pub exists: bool,
    pub udid: Option<String>,
    pub media_count: usize,
    pub backup_timestamp: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupResult {
    pub udid: String,
    pub media_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResult {
    pub exported: usize,
    pub dest_dir: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaFileExport {
    pub file_id: String,
    pub filename: String,
}

// â”€â”€ Backup directory helpers â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

fn find_udid_dir(backup_root: &Path, preferred_udid: Option<&str>) -> Option<PathBuf> {
    if let Some(u) = preferred_udid {
        let dir = backup_root.join(u);
        if dir.join("Manifest.db").exists() {
            return Some(dir);
        }
    }
    fs::read_dir(backup_root).ok()?.flatten().find_map(|entry| {
        let path = entry.path();
        if path.is_dir() && path.join("Manifest.db").exists() {
            Some(path)
        } else {
            None
        }
    })
}

// â”€â”€ Check / backup â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

pub fn check_existing_backup(preferred_udid: Option<&str>) -> BackupStatus {
    let backup_root = backup_dir();
    let Some(udid_dir) = find_udid_dir(&backup_root, preferred_udid) else {
        return BackupStatus { exists: false, udid: None, media_count: 0, backup_timestamp: None };
    };

    let actual_udid = udid_dir
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let media_count = list_media_from_manifest(&backup_root, &actual_udid)
        .map(|v| v.len())
        .unwrap_or(0);

    let backup_timestamp = fs::metadata(&udid_dir)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs());

    BackupStatus { exists: true, udid: Some(actual_udid), media_count, backup_timestamp }
}

pub fn run_backup(udid: Option<&str>) -> Result<BackupResult, String> {
    let backup_root = backup_dir();
    fs::create_dir_all(&backup_root).map_err(|e| e.to_string())?;

    let tool = resolved_libimobile_tool("idevicebackup2");
    let mut args: Vec<String> = vec![];
    if let Some(u) = udid {
        args.extend(["-u".to_string(), u.to_string()]);
    }
    args.push("backup".to_string());
    args.push(backup_root.to_string_lossy().to_string());

    let status = Command::new(&tool)
        .args(&args)
        .current_dir(tool.parent().unwrap_or(Path::new(".")))
        .status()
        .map_err(|e| format!("idevicebackup2 introuvable : {e}"))?;

    if !status.success() {
        return Err(
            "Backup iPhone Ã©chouÃ©. VÃ©rifiez que l'iPhone est dÃ©verrouillÃ© et approuvÃ© sur ce PC."
                .to_string(),
        );
    }

    let udid_dir = find_udid_dir(&backup_root, udid)
        .ok_or("Dossier de backup introuvable aprÃ¨s l'opÃ©ration")?;

    let actual_udid = udid_dir
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .ok_or("UDID introuvable")?;

    let items = list_media_from_manifest(&backup_root, &actual_udid)?;
    Ok(BackupResult { udid: actual_udid, media_count: items.len() })
}

// â”€â”€ List media â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

pub fn list_media_from_manifest(backup_root: &Path, udid: &str) -> Result<Vec<MediaItem>, String> {
    let manifest_db = backup_root.join(udid).join("Manifest.db");
    let conn = rusqlite::Connection::open(&manifest_db)
        .map_err(|e| format!("Manifest.db : {e}"))?;

    let mut stmt = conn
        .prepare(
            "SELECT fileID, relativePath, domain FROM Files
             WHERE (relativePath LIKE 'Media/DCIM/%'
                 OR relativePath LIKE 'Media/PhotoData/%')
               AND flags != 2
             ORDER BY relativePath",
        )
        .map_err(|e| e.to_string())?;

    let items: Vec<MediaItem> = stmt
        .query_map([], |row| {
            let file_id: String = row.get(0)?;
            let path: String = row.get(1)?;
            let _domain: String = row.get(2)?;
            let filename = path.rsplit('/').next().unwrap_or("").to_string();
            let ext = filename.rsplit('.').next().unwrap_or("").to_lowercase();
            let is_video = matches!(ext.as_str(), "mov" | "mp4" | "m4v");
            // Dossier = avant-dernier segment du chemin (ex: 100APPLE)
            let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
            let folder = if parts.len() >= 2 {
                parts[parts.len() - 2].to_string()
            } else {
                "Camera Roll".to_string()
            };
            Ok(MediaItem {
                file_id,
                relative_path: path,
                filename,
                extension: ext,
                is_video,
                folder,
                size_bytes: 0,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .filter(|item| {
            matches!(
                item.extension.as_str(),
                "jpg" | "jpeg" | "png" | "heic" | "heif" | "gif" | "tif" | "tiff"
                    | "mov" | "mp4" | "m4v"
            )
        })
        .collect();

    // Enrichit avec la taille de fichier (lecture des mÃ©tadonnÃ©es sur disque).
    let items: Vec<MediaItem> = items
        .into_iter()
        .map(|mut item| {
            if item.file_id.len() >= 2 {
                let p = backup_root.join(udid).join(&item.file_id[..2]).join(&item.file_id);
                if let Ok(meta) = fs::metadata(&p) {
                    item.size_bytes = meta.len();
                }
            }
            item
        })
        .collect();

    Ok(items)
}

pub fn list_media(udid: &str) -> Result<Vec<MediaItem>, String> {
    list_media_from_manifest(&backup_dir(), udid)
}

// â”€â”€ Thumbnails â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

fn thumbnail_via_image(file_path: &Path) -> Result<Vec<u8>, String> {
    let img = image::open(file_path).map_err(|e| format!("image::open : {e}"))?;
    let thumb = img.thumbnail(200, 200);
    let mut buf = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut buf);
    thumb.write_to(&mut cursor, image::ImageFormat::Jpeg).map_err(|e| e.to_string())?;
    Ok(buf)
}

#[cfg(windows)]
fn thumbnail_via_wic(file_path: &Path) -> Result<Vec<u8>, String> {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::GENERIC_READ;
    use windows::Win32::Graphics::Imaging::{
        CLSID_WICImagingFactory, GUID_WICPixelFormat24bppBGR,
        IWICImagingFactory,
        WICBitmapDitherTypeNone,
        WICBitmapInterpolationModeHighQualityCubic,
        WICBitmapPaletteTypeMedianCut,
        WICDecodeMetadataCacheOnDemand,
    };
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoUninitialize,
        CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED,
    };

    let path_wide: Vec<u16> = file_path
        .to_string_lossy()
        .encode_utf16()
        .chain(std::iter::once(0u16))
        .collect();

    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);

        let factory: IWICImagingFactory = CoCreateInstance(
            &CLSID_WICImagingFactory,
            None,
            CLSCTX_INPROC_SERVER,
        )
        .map_err(|e| format!("WIC factory : {e}"))?;

        let decoder = factory
            .CreateDecoderFromFilename(
                PCWSTR(path_wide.as_ptr()),
                None,
                GENERIC_READ,
                WICDecodeMetadataCacheOnDemand,
            )
            .map_err(|e| format!("WIC decode : {e}"))?;

        let frame = decoder.GetFrame(0)
            .map_err(|e| format!("WIC GetFrame : {e}"))?;

        // Dimensions originales pour conserver le ratio
        let mut orig_w = 0u32;
        let mut orig_h = 0u32;
        frame.GetSize(&mut orig_w, &mut orig_h)
            .map_err(|e| format!("WIC GetSize : {e}"))?;

        let (tw, th) = if orig_w == 0 || orig_h == 0 {
            (200u32, 200u32)
        } else if orig_w >= orig_h {
            (200, ((200u64 * orig_h as u64) / orig_w as u64).max(1) as u32)
        } else {
            (((200u64 * orig_w as u64) / orig_h as u64).max(1) as u32, 200)
        };

        // Scale
        let scaler = factory.CreateBitmapScaler()
            .map_err(|e| format!("WIC scaler : {e}"))?;
        scaler
            .Initialize(&frame, tw, th, WICBitmapInterpolationModeHighQualityCubic)
            .map_err(|e| format!("WIC scaler init : {e}"))?;

        // Convertit en BGR24 pour lire les pixels bruts
        let converter = factory.CreateFormatConverter()
            .map_err(|e| format!("WIC converter : {e}"))?;
        converter
            .Initialize(
                &scaler,
                &GUID_WICPixelFormat24bppBGR,
                WICBitmapDitherTypeNone,
                None,
                0.0,
                WICBitmapPaletteTypeMedianCut,
            )
            .map_err(|e| format!("WIC converter init : {e}"))?;

        let stride = tw * 3;
        let mut pixel_buf = vec![0u8; (stride * th) as usize];
        converter
            .CopyPixels(std::ptr::null(), stride, &mut pixel_buf)
            .map_err(|e| format!("WIC CopyPixels : {e}"))?;

        CoUninitialize();

        // WIC restitue en BGR â€” inverse en RGB pour le crate image
        for chunk in pixel_buf.chunks_exact_mut(3) {
            chunk.swap(0, 2);
        }

        // Encode en JPEG via le crate image (dÃ©jÃ  prÃ©sent dans le projet)
        let img = image::RgbImage::from_raw(tw, th, pixel_buf)
            .ok_or("WIC : buffer RGB invalide (dimensions incohÃ©rentes)")?;
        let mut buf = Vec::new();
        let mut cursor = std::io::Cursor::new(&mut buf);
        image::DynamicImage::ImageRgb8(img)
            .write_to(&mut cursor, image::ImageFormat::Jpeg)
            .map_err(|e| format!("JPEG encode : {e}"))?;
        Ok(buf)
    }
}

#[cfg(not(windows))]
fn thumbnail_via_wic(_file_path: &Path) -> Result<Vec<u8>, String> {
    Err("WIC non disponible hors Windows".to_string())
}

pub fn get_thumbnail(udid: &str, file_id: &str, extension: &str) -> Result<String, String> {
    if file_id.len() < 2 {
        return Err("file_id invalide".to_string());
    }
    let file_path = backup_dir().join(udid).join(&file_id[..2]).join(file_id);

    let bytes = match extension.to_lowercase().as_str() {
        "jpg" | "jpeg" | "png" | "gif" | "tif" | "tiff" => thumbnail_via_image(&file_path)?,
        "heic" | "heif" => thumbnail_via_wic(&file_path)
            .or_else(|_| thumbnail_via_image(&file_path))?,
        _ => return Err(format!("Format {extension} non supportÃ©")),
    };

    Ok(base64::engine::general_purpose::STANDARD.encode(&bytes))
}

// â”€â”€ Export â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

pub fn export_media(
    udid: &str,
    files: Vec<MediaFileExport>,
    dest_dir: &str,
) -> Result<ExportResult, String> {
    let backup_root = backup_dir();
    let dest = Path::new(dest_dir);
    fs::create_dir_all(dest).map_err(|e| e.to_string())?;

    let mut exported = 0usize;
    for f in &files {
        if f.file_id.len() < 2 {
            continue;
        }
        let src = backup_root.join(udid).join(&f.file_id[..2]).join(&f.file_id);
        let safe_name = Path::new(&f.filename)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| format!("file_{exported}"));
        let mut dst = dest.join(&safe_name);
        if dst.exists() {
            let stem = Path::new(&safe_name).file_stem().and_then(|s| s.to_str()).unwrap_or("file");
            let ext = Path::new(&safe_name).extension().and_then(|e| e.to_str()).unwrap_or("");
            dst = dest.join(format!("{}_{}.{}", stem, exported, ext));
        }
        if fs::copy(&src, &dst).is_ok() {
            exported += 1;
        }
    }
    Ok(ExportResult { exported, dest_dir: dest_dir.to_string() })
}

pub fn pick_export_folder() -> Option<String> {
    rfd::FileDialog::new()
        .set_title("Choisir le dossier d'export")
        .pick_folder()
        .map(|p| p.to_string_lossy().into_owned())
}

