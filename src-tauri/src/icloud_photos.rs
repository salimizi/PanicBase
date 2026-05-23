use crate::icloud::{webview_fetch, ICloudBridge, ICloudSession};
use base64::{engine::general_purpose::STANDARD as BASE64_STD, Engine as _};
use serde::Serialize;
use serde_json::{json, Value};
use tauri::WebviewWindow;

const CLIENT_BUILD: &str = "2421Project53";
const CLIENT_MASTER: &str = "2421B25";
const CLIENT_VERSION: &str = "5.4";
const CKJS_VERSION: &str = "2.6.4";
const PAGE_SIZE: u32 = 500;

const PARALLEL_PAGES: usize = 4;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ICloudAsset {
    pub record_name: String,
    pub master_ref: Option<String>,
    pub filename: String,
    pub extension: String,
    pub is_video: bool,
    pub date_created_ms: i64,
    pub duration_ms: i64,
    pub size_bytes: u64,
    pub thumb_url: Option<String>,
    pub original_url: Option<String>,
    pub is_hidden: bool,
    pub is_favorite: bool,
    pub is_in_trash: bool,
    pub folder: String,
}

#[derive(Default, Debug, Clone, Copy)]
struct AssetClassificationMeta {
    pub subtype: Option<i64>,
    pub image_width: Option<i64>,
    pub image_height: Option<i64>,
    pub video_frame_rate: Option<f64>,
}

// Bits du champ PHAssetMediaSubtype (iOS / PhotoKit).
// Source : https://developer.apple.com/documentation/photokit/phassetmediasubtype
const PHA_SUBTYPE_PHOTO_PANORAMA: i64       = 1 << 0;  // 1
const PHA_SUBTYPE_PHOTO_HDR: i64            = 1 << 1;  // 2
const PHA_SUBTYPE_PHOTO_SCREENSHOT: i64     = 1 << 2;  // 4
const PHA_SUBTYPE_PHOTO_LIVE: i64           = 1 << 3;  // 8
const PHA_SUBTYPE_PHOTO_DEPTH_EFFECT: i64   = 1 << 4;  // 16  â€” Portraits
const PHA_SUBTYPE_VIDEO_STREAMED: i64       = 1 << 16; // 65536
const PHA_SUBTYPE_VIDEO_HIGH_FRAMERATE: i64 = 1 << 17; // 131072 â€” Slo-mo
const PHA_SUBTYPE_VIDEO_TIMELAPSE: i64      = 1 << 18; // 262144
#[allow(dead_code)]
const PHA_SUBTYPE_VIDEO_CINEMATIC: i64      = 1 << 21; // 2097152 â€” Mode CinÃ©ma

fn classify_folder(
    filename: &str,
    extension: &str,
    is_video: bool,
    has_live_video: bool,
    meta: AssetClassificationMeta,
) -> String {
    let lower = filename.to_lowercase();
    let ext = extension.to_lowercase();
    if let Some(sub) = meta.subtype {
        if sub & PHA_SUBTYPE_PHOTO_DEPTH_EFFECT != 0 {
            return "Portraits".to_string();
        }
        if sub & PHA_SUBTYPE_PHOTO_PANORAMA != 0 {
            return "Panoramas".to_string();
        }
        if sub & PHA_SUBTYPE_VIDEO_HIGH_FRAMERATE != 0 {
            return "Ralentis".to_string();
        }
        if sub & PHA_SUBTYPE_VIDEO_TIMELAPSE != 0 {
            return "Time-lapse".to_string();
        }
        // PHA_SUBTYPE_PHOTO_LIVE / PHA_SUBTYPE_PHOTO_SCREENSHOT sont dÃ©jÃ 
        // gÃ©rÃ©s par les heuristiques ci-dessous, mais on peut les confirmer
        // ici si le bit est explicitement prÃ©sent.
        if sub & PHA_SUBTYPE_PHOTO_SCREENSHOT != 0 {
            return "Captures d'Ã©cran".to_string();
        }
        if sub & PHA_SUBTYPE_PHOTO_LIVE != 0 && !is_video {
            return "Live Photos".to_string();
        }
        // Streamed video / HDR ne sont pas exposÃ©s dans iCloud.com sidebar,
        // on les laisse retomber sur les heuristiques.
        let _ = sub & PHA_SUBTYPE_VIDEO_STREAMED;
        let _ = sub & PHA_SUBTYPE_PHOTO_HDR;
    }

    // 2. WhatsApp â€” pattern IMG-YYYYMMDD-WAxxxx (10+ ans, unique au monde)
    if (lower.starts_with("img-") || lower.starts_with("vid-")) && lower.contains("-wa") {
        return "WhatsApp".to_string();
    }

    // 3. Screen recording â€” RPReplay (ReplayKit iOS) ou nom natif
    if lower.starts_with("rpreplay") || lower.starts_with("screen recording") {
        return "Enregistrements d'Ã©cran".to_string();
    }

    // 4. Animation GIF
    if ext == "gif" {
        return "Animations".to_string();
    }

    // 5. Panorama heuristique : aspect ratio extrÃªme (Apple Camera produit
    //    typiquement 2:1 Ã  3:1 voire plus en mode Pano).
    if let (Some(w), Some(h)) = (meta.image_width, meta.image_height) {
        if w > 0 && h > 0 {
            let ratio = (w as f64 / h as f64).max(h as f64 / w as f64);
            if ratio >= 2.2 && !is_video {
                return "Panoramas".to_string();
            }
        }
    }

    // 5b. Slo-mo heuristique : si Apple expose videoFrameRate > 60.
    if is_video {
        if let Some(fps) = meta.video_frame_rate {
            if fps >= 100.0 {
                return "Ralentis".to_string();
            }
        }
    }

    // 6. Capture d'Ã©cran iPhone â€” toujours en PNG, l'app CamÃ©ra ne fait
    //    jamais de PNG donc c'est un signal fiable.
    if ext == "png" {
        return "Captures d'Ã©cran".to_string();
    }

    // 7. FaceTime / Photo Booth captures
    if lower.starts_with("photo on ") || lower.starts_with("facetime") {
        return "FaceTime".to_string();
    }

    // 8. Live Photo â€” HEIC avec composante vidÃ©o associÃ©e cÃ´tÃ© master
    if ext == "heic" && has_live_video {
        return "Live Photos".to_string();
    }

    // 9. VidÃ©o standard
    if is_video {
        return "VidÃ©os camÃ©ra".to_string();
    }

    // 10. Fallback : photo classique
    "CamÃ©ra".to_string()
}

fn resolve_download_url(rendition: serde_json::Value) -> Option<String> {
    let value = rendition.get("value")?;
    let url_template = value.get("downloadURL").and_then(|u| u.as_str())?;
    if !url_template.contains("${f}") {
        return Some(url_template.to_string());
    }
    let raw_token = value
        .get("fileChecksum")
        .or_else(|| value.get("referenceChecksum"))
        .or_else(|| value.get("wrappingKey"))
        .and_then(|c| c.as_str())?;
    let token = raw_token
        .replace('/', "_")
        .replace('+', "-")
        .trim_end_matches('=')
        .to_string();
    Some(url_template.replace("${f}", &token))
}

fn ck_query_url(session: &ICloudSession) -> Result<String, String> {
    let root = session
        .webservices
        .get("ckdatabasews")
        .ok_or("ckdatabasews missing from session.webservices")?;
    Ok(format!(
        "{}/database/1/com.apple.photos.cloud/production/private/records/query\
         ?remapEnums=True&getCurrentSyncToken=True&dsid={}&clientId={}\
         &clientBuildNumber={}&clientMasteringNumber={}\
         &clientVersion={}&ckjsBuildVersion={}&ckjsVersion={}",
        root.trim_end_matches('/'),
        session.dsid,
        session.client_id,
        CLIENT_BUILD,
        CLIENT_MASTER,
        CLIENT_VERSION,
        CLIENT_BUILD,
        CKJS_VERSION,
    ))
}

pub async fn fetch_photo_count(
    window: &WebviewWindow,
    bridge: &ICloudBridge,
    session: &ICloudSession,
) -> Result<u64, String> {
    let url = ck_query_url(session)?;
    let body = json!({
        "query": {
            "recordType": "HyperionIndexCountLookup",
            "filterBy": [{
                "fieldName": "indexCountID",
                "fieldValue": {
                    "type": "STRING_LIST",
                    "value": ["CPLAssetAndMasterByAssetDate"]
                },
                "comparator": "IN"
            }]
        },
        "zoneWide": true,
        "zoneID": { "zoneName": "PrimarySync" }
    });
    let req_body = serde_json::to_string(&body).unwrap();
    let (status, text) = webview_fetch(window, bridge, &url, Some(&req_body)).await?;
    if status != 200 {
        return Err(format!(
            "count HTTP {status} â€” {}",
            text.chars().take(300).collect::<String>()
        ));
    }
    let v: Value = serde_json::from_str(&text)
        .map_err(|e| format!("count parse: {e}"))?;
    let count = v
        .get("records")
        .and_then(|r| r.as_array())
        .and_then(|r| r.first())
        .and_then(|r| r.get("fields"))
        .and_then(|f| f.get("itemCount"))
        .and_then(|c| c.get("value"))
        .and_then(|v| v.as_u64())
        .ok_or_else(|| {
            format!(
                "count response missing records[0].fields.itemCount.value â€” body={}",
                text.chars().take(600).collect::<String>()
            )
        })?;
    Ok(count)
}

// â”€â”€ Scan des albums CloudKit â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
//
// iCloud Photos organise les mÃ©dias en deux niveaux :
//   1. Les **albums utilisateur** (Snapchat, Instagram, WhatsApp, â€¦) qui
//      sont crÃ©Ã©s par les apps tierces via PhotoKit ou par l'utilisateur.
//   2. Les **smart albums systÃ¨me** (Live Photos, Portraits, Panoramas,
//      Ralentis, Captures d'Ã©cran, Animations) qui sont alimentÃ©s
//      automatiquement par iOS selon les caractÃ©ristiques techniques de
//      l'asset.
//
// Pour reconstruire la vue de `icloud.com/photos`, on a besoin de la
// par `classify_folder` cÃ´tÃ© heuristique).
//
// Workflow :
//   a) Une requÃªte `CPLAlbumByPositionLive` liste tous les albums du
//      compte. On rÃ©cupÃ¨re pour chaque album son `recordName` + son nom
//      base64-encodÃ© (`albumNameEnc`).
//   b) Pour chaque album, on pagine `CPLContainerRelationLiveByAssetDate`
//      filtrÃ© sur `parentId = <album.recordName>` pour obtenir les
//      `itemId` (= asset.recordName) des assets qui appartiennent Ã 
//      l'album.
//   c) On agrÃ¨ge dans une map `assetRecordName â†’ albumName` qu'on Ã©met vers
//      le frontend via l'Ã©vÃ©nement `icloud-album-assignments`.

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ICloudAlbumAssignment {
    pub asset_record_name: String,
    pub album_name: String,
}

#[derive(Debug, Clone)]
struct AlbumMeta {
    record_name: String,
    name: String,
}

async fn fetch_albums(
    window: &WebviewWindow,
    bridge: &ICloudBridge,
    session: &ICloudSession,
) -> Result<Vec<AlbumMeta>, String> {
    let url = ck_query_url(session)?;
    let body = json!({
        "query": {
            "recordType": "CPLAlbumByPositionLive"
        },
        "resultsLimit": 500,
        "desiredKeys": ["albumNameEnc", "albumType", "isDeleted", "position", "sortAscending", "sortType"],
        "zoneID": { "zoneName": "PrimarySync" }
    });
    let req_body = serde_json::to_string(&body).unwrap();
    let (status, text) = webview_fetch(window, bridge, &url, Some(&req_body)).await?;
    if status != 200 {
        return Err(format!(
            "albums HTTP {status}: {}",
            text.chars().take(300).collect::<String>()
        ));
    }
    let v: Value =
        serde_json::from_str(&text).map_err(|e| format!("albums JSON parse: {e}"))?;
    let arr = v
        .get("records")
        .and_then(|r| r.as_array())
        .cloned()
        .unwrap_or_default();
    let mut out = Vec::with_capacity(arr.len());
    let mut dumped = false;
    for rec in arr.iter() {
        let record_name = match rec.get("recordName").and_then(|n| n.as_str()) {
            Some(s) => s.to_string(),
            None => continue,
        };
        // On dump le premier album pour diag (au cas oÃ¹ l'encodage change).
        if !dumped {
            dumped = true;
            let pretty = serde_json::to_string_pretty(rec).unwrap_or_default();
        }
        let is_deleted = rec
            .get("fields")
            .and_then(|f| f.get("isDeleted"))
            .and_then(|v| v.get("value"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0)
            != 0;
        if is_deleted {
            continue;
        }
        let name_enc = rec
            .get("fields")
            .and_then(|f| f.get("albumNameEnc"))
            .and_then(|v| v.get("value"))
            .and_then(|v| v.as_str());
        let name = name_enc
            .and_then(|s| BASE64_STD.decode(s.as_bytes()).ok())
            .and_then(|b| String::from_utf8(b).ok())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| format!("Album {}", &record_name[..record_name.len().min(8)]));
        out.push(AlbumMeta { record_name, name });
    }
    Ok(out)
}

async fn fetch_album_assets(
    window: &WebviewWindow,
    bridge: &ICloudBridge,
    session: &ICloudSession,
    album_record: &str,
) -> Result<Vec<String>, String> {
    let url = ck_query_url(session)?;
    let mut assets: Vec<String> = Vec::new();
    let mut start_rank: i64 = 0;
    loop {
        let body = json!({
            "query": {
                "recordType": "CPLContainerRelationLiveByAssetDate",
                "filterBy": [
                    {
                        "fieldName": "startRank",
                        "fieldValue": { "value": start_rank, "type": "INT64" },
                        "comparator": "EQUALS"
                    },
                    {
                        "fieldName": "direction",
                        "fieldValue": { "value": "ASCENDING", "type": "STRING" },
                        "comparator": "EQUALS"
                    },
                    {
                        "fieldName": "parentId",
                        "fieldValue": { "value": album_record, "type": "STRING" },
                        "comparator": "EQUALS"
                    }
                ]
            },
            "resultsLimit": 500,
            "desiredKeys": ["itemId", "parentId"],
            "zoneID": { "zoneName": "PrimarySync" }
        });
        let req_body = serde_json::to_string(&body).unwrap();
        let (status, text) = webview_fetch(window, bridge, &url, Some(&req_body)).await?;
        if status != 200 {
            return Err(format!(
                "album {} HTTP {}: {}",
                album_record,
                status,
                text.chars().take(200).collect::<String>()
            ));
        }
        let v: Value = serde_json::from_str(&text)
            .map_err(|e| format!("album {} JSON: {e}", album_record))?;
        let arr = v
            .get("records")
            .and_then(|r| r.as_array())
            .cloned()
            .unwrap_or_default();
        if arr.is_empty() {
            break;
        }
        let page_len = arr.len();
        let mut added = 0usize;
        for rec in arr.iter() {
            // Les relations exposent l'asset cible via le champ `itemId` qui
            // est soit un REFERENCE soit une STRING selon les versions
            // CloudKit. On essaie les deux formats.
            let item_id = rec
                .get("fields")
                .and_then(|f| f.get("itemId"))
                .and_then(|v| v.get("value"))
                .and_then(|v| {
                    // Cas REFERENCE : { recordName: "..." }
                    v.get("recordName")
                        .and_then(|n| n.as_str())
                        // Cas STRING direct
                        .or_else(|| v.as_str())
                })
                .map(String::from);
            if let Some(id) = item_id {
                assets.push(id);
                added += 1;
            }
        }
        if added == 0 {
            break;
        }
        start_rank += page_len as i64;
        // Safety cap par album : aucun album iOS n'a >50k items en pratique.
        if assets.len() > 50_000 {
            break;
        }
    }
    Ok(assets)
}

const ALBUM_PARALLELISM: usize = 8;

#[allow(dead_code)]
async fn fetch_smart_album_assets(
    window: &WebviewWindow,
    bridge: &ICloudBridge,
    session: &ICloudSession,
    record_type: &str,
) -> Result<Vec<String>, String> {
    let url = ck_query_url(session)?;
    let mut assets: Vec<String> = Vec::new();
    let mut start_rank: i64 = 0;
    let mut seen: std::collections::HashSet<String> =
        std::collections::HashSet::new();
    loop {
        let body = json!({
            "query": {
                "recordType": record_type,
                "filterBy": [
                    {
                        "fieldName": "startRank",
                        "fieldValue": { "value": start_rank, "type": "INT64" },
                        "comparator": "EQUALS"
                    },
                    {
                        "fieldName": "direction",
                        "fieldValue": { "value": "ASCENDING", "type": "STRING" },
                        "comparator": "EQUALS"
                    }
                ]
            },
            "resultsLimit": 200,
            "desiredKeys": ["recordName"],
            "zoneID": { "zoneName": "PrimarySync" }
        });
        let req_body = serde_json::to_string(&body).unwrap();
        let (status, text) = webview_fetch(window, bridge, &url, Some(&req_body)).await?;
        if status != 200 {
            // Dump le body sur la premiÃ¨re page (= soit query invalide,
            // soit zone non accessible). Au-delÃ  on tronque.
            let preview: String = text.chars().take(500).collect();
            return Err(format!("HTTP {} body={}", status, preview));
        }
        let v: Value = serde_json::from_str(&text)
            .map_err(|e| format!("JSON parse: {e}"))?;
        let arr = v
            .get("records")
            .and_then(|r| r.as_array())
            .cloned()
            .unwrap_or_default();
        if arr.is_empty() {
            // Ã€ la 1Ã¨re page, si records=[] on log la rÃ©ponse complÃ¨te pour
            // savoir si Apple a fait un fallback silencieux (ex: serverError
            // inline). C'est ce qu'on veut absolument voir.
            if start_rank == 0 {
            }
            break;
        }
        let page_len = arr.len();
        let mut new_in_page = 0usize;
        for rec in arr.iter() {
            let rtype = rec
                .get("recordType")
                .and_then(|t| t.as_str())
                .unwrap_or("");
            if rtype != "CPLAsset" {
                continue;
            }
            if let Some(name) = rec.get("recordName").and_then(|n| n.as_str()) {
                if seen.insert(name.to_string()) {
                    assets.push(name.to_string());
                    new_in_page += 1;
                }
            }
        }
        if new_in_page == 0 {
            break;
        }
        start_rank += page_len as i64;
        if assets.len() > 50_000 {
            break;
        }
    }
    Ok(assets)
}

pub async fn scan_albums_in_background(
    window: WebviewWindow,
    bridge: ICloudBridge,
    session: ICloudSession,
    app: tauri::AppHandle,
) {
    use std::sync::Arc;
    use tauri::Emitter;
    use tokio::sync::Semaphore;

    let started = std::time::Instant::now();
    let albums = match fetch_albums(&window, &bridge, &session).await {
        Ok(a) => a,
        Err(e) => {
            let _ = app.emit("icloud-albums-error", e);
            return;
        }
    };

    let total_albums = albums.len();
    let sem = Arc::new(Semaphore::new(ALBUM_PARALLELISM));
    let mut joinset = tokio::task::JoinSet::new();

    // â”€â”€ Ã‰tape A : albums utilisateur (Snapchat, Instagram, WhatsApp, â€¦) â”€â”€
    for (i, album) in albums.into_iter().enumerate() {
        let permit = sem.clone().acquire_owned().await.expect("semaphore closed");
        let window = window.clone();
        let bridge = ICloudBridge {
            responses: bridge.responses.clone(),
            notify: bridge.notify.clone(),
        };
        let session = session.clone();
        let app = app.clone();
        joinset.spawn(async move {
            let _permit = permit;
            match fetch_album_assets(&window, &bridge, &session, &album.record_name).await {
                Ok(asset_ids) => {
                    let batch: Vec<ICloudAlbumAssignment> = asset_ids
                        .into_iter()
                        .map(|id| ICloudAlbumAssignment {
                            asset_record_name: id,
                            album_name: album.name.clone(),
                        })
                        .collect();
                    let len = batch.len();
                    if !batch.is_empty() {
                        let _ = app.emit("icloud-album-assignments", &batch);
                    }
                    len
                }
                Err(e) => {
                    0
                }
            }
        });
    }

    // â”€â”€ Ã‰tape B : smart albums systÃ¨me â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    //
    // /!\ DÃ‰SACTIVÃ‰ aprÃ¨s confirmation par log (tauri.log 2026-05-21) qu'Apple
    // rÃ©pond HTTP 404 "Did not find record type: â€¦" sur tous les noms
    // candidats (DepthEffectGroup, PanoramasGroup, SlomoGroup, AnimatedGroup,
    // TimelapseGroup, BurstsGroup, SelfiesGroup, FavoriteGroup). Le schÃ©ma
    // CloudKit du compte exposÃ© via la WebView 2024+ ne contient plus ces
    // record types prÃ©-agrÃ©gÃ©s.
    //
    // Approche actuelle : dÃ©tection mÃ©tadonnÃ©e pure (cf. `classify_folder`)
    // Ã  partir des champs que CloudKit accepte de renvoyer dans la query
    // principale (subtype, originalAssetSubtype, imageWidth/Height,
    // videoFrameRate, â€¦). Le dump du 1er asset montre ce qui est dispo.

    let mut total_assignments = 0usize;
    while let Some(res) = joinset.join_next().await {
        match res {
            Ok(n) => total_assignments += n,
            Err(_) => {}
        }
    }
    let _ = app.emit("icloud-albums-done", total_assignments);
}

fn build_query_body(desired_keys: &Value, start_rank: i64, batch: u32) -> String {
    let body = json!({
        "query": {
            "recordType": "CPLAssetAndMasterByAssetDate",
            "filterBy": [
                {
                    "fieldName": "startRank",
                    "fieldValue": { "value": start_rank, "type": "INT64" },
                    "comparator": "EQUALS"
                },
                {
                    "fieldName": "direction",
                    "fieldValue": { "value": "ASCENDING", "type": "STRING" },
                    "comparator": "EQUALS"
                }
            ]
        },
        "resultsLimit": (batch * 2) as i64,
        "desiredKeys": desired_keys,
        "zoneID": { "zoneName": "PrimarySync" }
    });
    serde_json::to_string(&body).unwrap_or_default()
}

async fn fetch_and_parse_page(
    window: &WebviewWindow,
    bridge: &ICloudBridge,
    url: &str,
    body: &str,
    start_rank: i64,
    dump_first: bool,
) -> Result<(Vec<ICloudAsset>, usize), String> {
    let (status, text) = webview_fetch(window, bridge, url, Some(body)).await?;

    if status != 200 {
        let preview: String = text.chars().take(500).collect();
        let hint = match status {
            421 => " â€” Apple a refusÃ© la session CloudKit. Active Â« AccÃ©der Ã  iCloud sur le Web Â» sur ton iPhone (RÃ©glages â†’ ton nom â†’ iCloud â†’ tout en bas), reconnecte-toi puis rÃ©essaie.",
            401 | 403 => " â€” Session iCloud expirÃ©e ou non autorisÃ©e. DÃ©connecte puis reconnecte iCloud dans PanicBase.",
            500..=599 => " â€” Erreur cÃ´tÃ© serveur Apple. RÃ©essaie dans quelques minutes.",
            _ => "",
        };
        return Err(format!("CloudKit HTTP {status} â€” {preview}{hint}"));
    }

    let v: Value = serde_json::from_str(&text)
        .map_err(|e| format!("CloudKit JSON parse: {e}"))?;
    let records_arr = v
        .get("records")
        .and_then(|r| r.as_array())
        .cloned()
        .unwrap_or_default();

    if records_arr.is_empty() {
        return Ok((Vec::new(), 0));
    }

    if dump_first {
        let mut dumped_asset = false;
        let mut dumped_master = false;
        for rec in &records_arr {
            let rtype = rec
                .get("recordType")
                .and_then(|t| t.as_str())
                .unwrap_or("");
            let should_dump = (rtype == "CPLAsset" && !dumped_asset)
                || (rtype == "CPLMaster" && !dumped_master);
            if !should_dump {
                continue;
            }
            if rtype == "CPLAsset" {
                dumped_asset = true;
            } else {
                dumped_master = true;
            }
            let pretty = serde_json::to_string_pretty(rec).unwrap_or_default();
            if dumped_asset && dumped_master {
                break;
            }
        }
    }

    let mut masters: std::collections::HashMap<String, &Value> =
        std::collections::HashMap::new();
    for rec in &records_arr {
        let rtype = rec.get("recordType").and_then(|t| t.as_str()).unwrap_or("");
        if rtype == "CPLMaster" {
            if let Some(name) = rec.get("recordName").and_then(|n| n.as_str()) {
                masters.insert(name.to_string(), rec);
            }
        }
    }

    let mut parsed: Vec<ICloudAsset> = Vec::new();
    let mut page_assets = 0usize;
    let mut with_thumb = 0usize;
    let mut with_original = 0usize;
    let mut first_no_thumb_dumped = false;
    for rec in &records_arr {
        let rtype = rec.get("recordType").and_then(|t| t.as_str()).unwrap_or("");
        if rtype != "CPLAsset" {
            continue;
        }
        page_assets += 1;
        let master_ref = rec
            .get("fields")
            .and_then(|f| f.get("masterRef"))
            .and_then(|v| v.get("value"))
            .and_then(|v| v.get("recordName"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let master = masters.get(master_ref).copied();
        if let Some(asset) = parse_asset_pair(rec, master) {
            if asset.is_in_trash {
                continue;
            }
            if asset.thumb_url.is_some() {
                with_thumb += 1;
            } else if !first_no_thumb_dumped {
                // Diagnostic auto : dÃ¨s qu'un asset n'a pas de thumb_url,
                // Ã‡a donne exactement la structure JSON qui manque pour
                // qu'on sache oÃ¹ Apple a planquÃ© la rendition.
                first_no_thumb_dumped = true;
                let asset_pretty =
                    serde_json::to_string_pretty(rec).unwrap_or_default();
                let master_pretty = master
                    .map(|m| serde_json::to_string_pretty(m).unwrap_or_default())
                    .unwrap_or_else(|| "<no master found>".to_string());
            }
            if asset.original_url.is_some() {
                with_original += 1;
            }
            parsed.push(asset);
        }
    }
    Ok((parsed, page_assets))
}

pub async fn list_assets<F>(
    window: &WebviewWindow,
    bridge: &ICloudBridge,
    session: &ICloudSession,
    mut on_progress: F,
    app: &tauri::AppHandle,
) -> Result<Vec<ICloudAsset>, String>
where
    F: FnMut(usize),
{
    use tauri::Emitter;
    let url = ck_query_url(session)?;

    // Ã‰tape 0 : rÃ©cupÃ¨re le total avant de paginer (cf commentaire dans la
    // version sÃ©quentielle prÃ©cÃ©dente). Sans ce comptage, on ne peut pas
    // parallÃ©liser : on doit savoir oÃ¹ sont les bornes.
    let total = match fetch_photo_count(window, bridge, session).await {
        Ok(n) => {
            n
        }
        Err(e) => {
            0
        }
    };

        let desired_keys = json!([
        "assetDate",
        "addedDate",
        "isHidden",
        "isFavorite",
        "isDeleted",
        "masterRef",
        "itemType",
        "adjustmentType",
        "burstId",
        "captionEnc",
        "locationEnc",
        "mediaMetaDataType",
        "originalOrientation",
        "filenameEnc",
        "originalAssetSize",
        "originalCreationDate",
        "resOriginalRes",
        "resOriginalFileType",
        "resOriginalVidComplRes",
        "resOriginalVidComplFileType",
        "resJPEGThumbRes",
        "resJPEGThumbFileType",
        "resJPEGMedRes",
        "resJPEGMedFileType",
        "resJPEGFullRes",
        "resJPEGFullFileType",
        "resVidFullRes",
        "resVidFullFileType",
        // Candidats pour la classification smart-album (Portraits, Panoramas,
        // Ralentis, Animationsâ€¦). Si Apple n'expose pas l'un de ces champs
        // sur le schÃ©ma CloudKit du compte, il est silencieusement omis de
        // la rÃ©ponse â€” pas d'erreur. On ne sait pas a priori lequel est
        // utilisÃ© : le dump diagnostique nous le dira au 1er asset.
        "subtype",
        "kindSubType",
        "kindSubTypeEnum",
        "mediaSubType",
        "originalAssetSubtype",
        "assetSubtype",
        "assetSubtypes",
        // Dimensions (panorama = ratio > 2:1)
        "imageWidth",
        "imageHeight",
        "originalWidth",
        "originalHeight",
        "dimensions",
        // VidÃ©o (slo-mo = frameRate > 60)
        "videoFrameRate",
        "frameRate",
        "playbackVariation",
        "playbackStyle"
    ]);

    let mut results: Vec<ICloudAsset> = Vec::new();
    let batch: u32 = PAGE_SIZE;

    // â”€â”€ Pagination parallÃ¨le â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    //
    // Sans total connu : on retombe en sÃ©quentiel pour ne pas tirer dans le
    // vide sur des pages au-delÃ  de la fin (l'API CloudKit n'est pas idempotente
    // sur les startRank invalides â€” elle renvoie parfois quand mÃªme 1 record).
    //
    // Avec total : on prÃ©pare la liste complÃ¨te des offsets et on tire par
    // groupes de PARALLEL_PAGES (4) via tokio::join!. Chaque rÃ©sultat est
    // streamÃ© vers l'UI dÃ¨s qu'il arrive (event `icloud-assets-batch`) â†’ la
    // grille se peuple progressivement sans attendre la fin.
    // Fallback : on ne connaÃ®t pas le total. On pagine en parallÃ¨le (4 pages
    // Ã— ~100 assets/page â‰ˆ 400 photos par vague). Apple a un comportement
    // bizarre quand startRank dÃ©passe le nombre rÃ©el de photos : au lieu de
    // renvoyer une page vide, il rejoue les ~100 derniÃ¨res en boucle. On
    // dÃ©tecte Ã§a via **dÃ©duplication** sur le `record_name` : dÃ¨s qu'une vague
    // n'ajoute aucun NOUVEAU record, on arrÃªte. C'est aussi un filet de
    // sÃ©curitÃ© gÃ©nÃ©ral qui Ã©vite tout import en doublon.
    if total == 0 {
        let probe_step: i64 = 100; // cap pratique d'Apple par page
        let mut start_rank: i64 = 0;
        let mut dumped = false;
        let mut seen: std::collections::HashSet<String> =
            std::collections::HashSet::with_capacity(20_000);
        loop {
            let s0 = start_rank;
            let s1 = start_rank + probe_step;
            let s2 = start_rank + 2 * probe_step;
            let s3 = start_rank + 3 * probe_step;
            let b0 = build_query_body(&desired_keys, s0, batch);
            let b1 = build_query_body(&desired_keys, s1, batch);
            let b2 = build_query_body(&desired_keys, s2, batch);
            let b3 = build_query_body(&desired_keys, s3, batch);
            let dump0 = !dumped;
            dumped = true;
            let f0 = fetch_and_parse_page(window, bridge, &url, &b0, s0, dump0);
            let f1 = fetch_and_parse_page(window, bridge, &url, &b1, s1, false);
            let f2 = fetch_and_parse_page(window, bridge, &url, &b2, s2, false);
            let f3 = fetch_and_parse_page(window, bridge, &url, &b3, s3, false);
            let (r0, r1, r2, r3) = tokio::join!(f0, f1, f2, f3);
            let pages = [r0?, r1?, r2?, r3?];

            let mut new_in_wave = 0usize;
            let mut wave_empty = true;
            for (parsed, page_assets) in pages {
                if page_assets > 0 {
                    wave_empty = false;
                }
                // DÃ©dup : on ne garde que les record_name jamais vus.
                let fresh: Vec<ICloudAsset> = parsed
                    .into_iter()
                    .filter(|a| seen.insert(a.record_name.clone()))
                    .collect();
                if !fresh.is_empty() {
                    let _ = app.emit("icloud-assets-batch", &fresh);
                    new_in_wave += fresh.len();
                    results.extend(fresh);
                }
            }
            on_progress(results.len());

            if wave_empty {
                break;
            }
            if new_in_wave == 0 {
                break;
            }
            start_rank += 4 * probe_step;
            if results.len() > 100_000 {
                break;
            }
        }
        // Diagnostic lÃ©ger : juste la rÃ©partition par dossier (= ce que voit
        // l'utilisateur). On laisse, c'est utile pour comprendre les retours
        // utilisateurs. Les distributions debug `assetSubtype` /
        // `videoFrameRate` ont confirmÃ© que Apple n'expose que Panorama
        // (subtype=1) et fps pour Ralentis â€” pas de Portrait/Time-lapse â€”
        // donc on n'a plus besoin d'en logger la distribution complÃ¨te.
        {
            let mut folders: std::collections::BTreeMap<&str, usize> =
                std::collections::BTreeMap::new();
            for a in &results {
                *folders.entry(a.folder.as_str()).or_insert(0) += 1;
            }
            let f_str: Vec<String> = folders.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
        }
        return Ok(results);
    }

    let mut offsets: Vec<i64> = Vec::new();
    let mut s: i64 = 0;
    while s < total as i64 {
        offsets.push(s);
        s += batch as i64;
    }

    // â”€â”€ DÃ©duplication â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    // MÃªme avec `total` connu, on doit dÃ©dupliquer car :
    //   1. `resultsLimit = batch * 2` peut faire renvoyer des records qui
    //      chevauchent la page suivante.
    //   2. Quand on demande `startRank` proche de la fin, Apple renvoie
    //      parfois la derniÃ¨re page complÃ¨te au lieu d'une page vide
    //      (= 500 doublons).
    // Sans cette dÃ©dup, l'UI voit le compteur monter au-dessus de `total`.
    let mut seen: std::collections::HashSet<String> =
        std::collections::HashSet::with_capacity(total as usize + 1024);

    let mut dump_done = false;
    for chunk in offsets.chunks(PARALLEL_PAGES) {
        // PrÃ©-construit les bodies pour pouvoir les `&` dans les futures.
        let bodies: Vec<String> = chunk
            .iter()
            .map(|&sr| build_query_body(&desired_keys, sr, batch))
            .collect();

        // Lance jusqu'Ã  4 fetches en parallÃ¨le. tokio::join! attend tout le
        // groupe, mais Ã  l'intÃ©rieur du WebView les fetches du navigateur sont
        // vraiment concurrents (chaque `eval()` poste un IIFE indÃ©pendant qui
        // tape /records/query en parallÃ¨le, et le pont JS supporte les markers
        // simultanÃ©s).
        let mut futures: Vec<_> = Vec::with_capacity(chunk.len());
        for (i, &start_rank) in chunk.iter().enumerate() {
            let dump_this = !dump_done && start_rank == 0;
            let body = &bodies[i];
            futures.push(fetch_and_parse_page(
                window, bridge, &url, body, start_rank, dump_this,
            ));
        }

        // Ã‰value toutes les futures en parallÃ¨le. On utilise une boucle de
        // join_all maison via tokio::join! Ã©tendu (sans dÃ©pendance externe).
        let mut page_results: Vec<Result<(Vec<ICloudAsset>, usize), String>> =
            Vec::with_capacity(chunk.len());
        match chunk.len() {
            1 => {
                let r0 = futures.remove(0).await;
                page_results.push(r0);
            }
            2 => {
                let f1 = futures.remove(1);
                let f0 = futures.remove(0);
                let (r0, r1) = tokio::join!(f0, f1);
                page_results.push(r0);
                page_results.push(r1);
            }
            3 => {
                let f2 = futures.remove(2);
                let f1 = futures.remove(1);
                let f0 = futures.remove(0);
                let (r0, r1, r2) = tokio::join!(f0, f1, f2);
                page_results.push(r0);
                page_results.push(r1);
                page_results.push(r2);
            }
            _ => {
                let f3 = futures.remove(3);
                let f2 = futures.remove(2);
                let f1 = futures.remove(1);
                let f0 = futures.remove(0);
                let (r0, r1, r2, r3) = tokio::join!(f0, f1, f2, f3);
                page_results.push(r0);
                page_results.push(r1);
                page_results.push(r2);
                page_results.push(r3);
            }
        }
        dump_done = true;

        for r in page_results {
            let (parsed, _page_assets) = r?;
            // DÃ©dup : ne garde que les record_name jamais vus.
            let fresh: Vec<ICloudAsset> = parsed
                .into_iter()
                .filter(|a| seen.insert(a.record_name.clone()))
                .collect();
            if !fresh.is_empty() {
                let _ = app.emit("icloud-assets-batch", &fresh);
                results.extend(fresh);
            }
        }
        on_progress(results.len());

        // Stop dÃ¨s qu'on a atteint le total annoncÃ© : inutile de continuer
        // Ã  interroger CloudKit pour rien.
        if results.len() >= total as usize {
            break;
        }

        if results.len() > 200_000 {
            break;
        }
    }
    Ok(results)
}

fn parse_asset_pair(asset: &Value, master: Option<&Value>) -> Option<ICloudAsset> {
    let record_name = asset.get("recordName")?.as_str()?.to_string();
    let asset_fields = asset.get("fields").and_then(|f| f.as_object());
    let master_fields = master
        .and_then(|m| m.get("fields"))
        .and_then(|f| f.as_object());

    // helper : cherche un champ d'abord dans master, sinon dans asset
    let field = |k: &str| -> Option<Value> {
        master_fields
            .and_then(|f| f.get(k))
            .or_else(|| asset_fields.and_then(|f| f.get(k)))
            .cloned()
    };
    let asset_field = |k: &str| -> Option<Value> {
        asset_fields.and_then(|f| f.get(k)).cloned()
    };

    let filename = field("filenameEnc")
        .and_then(|v| v.get("value").cloned())
        .and_then(|v| v.as_str().map(String::from))
        .and_then(|s| BASE64_STD.decode(s.as_bytes()).ok())
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| {
            let snip = record_name.get(..8).unwrap_or(&record_name);
            format!("ICLD_{snip}.unknown")
        });

    let extension = filename
        .rsplit('.')
        .next()
        .filter(|e| e.len() <= 5 && !e.is_empty())
        .unwrap_or("")
        .to_lowercase();

    let date_created_ms = asset_field("assetDate")
        .or_else(|| asset_field("addedDate"))
        .or_else(|| field("originalCreationDate"))
        .and_then(|v| v.get("value").cloned())
        .and_then(|v| v.as_i64())
        .unwrap_or(0);

    let master_ref = asset_field("masterRef")
        .and_then(|v| v.get("value").cloned())
        .and_then(|v| v.get("recordName").cloned())
        .and_then(|v| v.as_str().map(String::from));

    // itemType chez Apple : "public.heic"/"public.jpeg" pour photo,
    // "com.apple.quicktime-movie" / "public.mpeg-4" pour video.
    let item_type = field("itemType")
        .and_then(|v| v.get("value").cloned())
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_default();

    let video_exts = ["mov", "mp4", "m4v", "avi", "hevc"];
    let has_video_res = master_fields
        .map(|f| f.contains_key("resOriginalVidComplRes"))
        .unwrap_or(false)
        || asset_fields
            .map(|f| f.contains_key("resVidFullRes"))
            .unwrap_or(false);
    let is_video = item_type.contains("movie")
        || item_type.contains("video")
        || item_type.contains("mpeg")
        || item_type.contains("quicktime")
        || has_video_res
        || video_exts.contains(&extension.as_str());

    // Thumbnail : on cherche une rendition JPEG dans cet ordre :
    // resJPEGThumbRes (idÃ©al), puis Med, puis Full. On NE retombe PAS sur
    // resOriginalRes car pour la majoritÃ© des iPhones, l'original est en
    // HEIC â€” illisible par Chromium dans un `<img>`. Mieux vaut afficher un
    // placeholder qu'un blob HEIC corrompu cÃ´tÃ© UI.
    //
    // âš ï¸ Apple renvoie des URLs de type :
    //   https://cvws-h2.icloud-content.com/B/<token>/${f}?o=...
    // oÃ¹ `${f}` est un **placeholder** Ã  substituer cÃ´tÃ© client. Le client
    // web iCloud le remplace par le `fileChecksum` (URL-safe base64) de la
    // rendition. Sans Ã§a, Apple renvoie 404. Le helper ci-dessous gÃ¨re Ã§a.
    //
    // On extrait la downloadURL champ par champ (pas en cascade globale)
    // parce qu'Apple peut envoyer un objet `resJPEGThumbRes` sans
    // `downloadURL` pour certains assets. On utilise `field` (asset OU
    // master) parce qu'Apple peut publier les renditions sur l'un ou l'autre
    // suivant les comptes / versions iCloud.
    let thumb_url = field("resJPEGThumbRes")
        .and_then(resolve_download_url)
        .or_else(|| field("resJPEGMedRes").and_then(resolve_download_url))
        .or_else(|| field("resJPEGFullRes").and_then(resolve_download_url));

    // Original : cÃ´tÃ© master pour les photos, cÃ´tÃ© master vidÃ©o pour les
    // vidÃ©os. Sinon resVidFullRes cÃ´tÃ© asset. Comme pour la miniature, on
    // rÃ©sout `${f}` via le helper `resolve_download_url`, sinon Apple
    // renverra 404 quand on cherchera Ã  tÃ©lÃ©charger.
    let size_from_field = |rendition: &serde_json::Value| -> u64 {
        rendition
            .get("value")
            .and_then(|v| v.get("size"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0)
    };
    let (original_url, size_bytes) = if is_video {
        let source = field("resOriginalVidComplRes")
            .or_else(|| asset_field("resVidFullRes"))
            .or_else(|| field("resOriginalRes"));
        match source {
            Some(s) => {
                let size = size_from_field(&s);
                (resolve_download_url(s), size)
            }
            None => (None, 0),
        }
    } else {
        match field("resOriginalRes") {
            Some(orig) => {
                let size = size_from_field(&orig);
                (resolve_download_url(orig), size)
            }
            None => (
                None,
                field("originalAssetSize")
                    .and_then(|v| v.get("value").cloned())
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0),
            ),
        }
    };

    // duration_ms est rarement fourni sÃ©parÃ©ment ; on le laisse Ã  0.
    let duration_ms: i64 = 0;

    let bool_field = |k: &str| -> bool {
        asset_field(k)
            .and_then(|v| v.get("value").cloned())
            .and_then(|v| v.as_i64())
            .map(|n| n != 0)
            .unwrap_or(false)
    };

    let is_hidden = bool_field("isHidden");
    let is_favorite = bool_field("isFavorite");
    let is_in_trash = bool_field("isDeleted") || bool_field("isInTrash");

    // RÃ©colte des mÃ©tadonnÃ©es disponibles cÃ´tÃ© CloudKit pour classification
    // smart-album. Toutes optionnelles : Apple omet les champs qui n'existent
    // pas sur le schÃ©ma de l'utilisateur. On essaie plusieurs noms candidats
    // par valeur car le naming a variÃ© entre versions de CloudKit.
    let int_field = |k: &str| -> Option<i64> {
        field(k)
            .and_then(|v| v.get("value").cloned())
            .and_then(|v| v.as_i64())
    };
    let float_field = |k: &str| -> Option<f64> {
        field(k)
            .and_then(|v| v.get("value").cloned())
            .and_then(|v| v.as_f64())
    };
    let meta = AssetClassificationMeta {
        subtype: int_field("subtype")
            .or_else(|| int_field("originalAssetSubtype"))
            .or_else(|| int_field("kindSubType"))
            .or_else(|| int_field("assetSubtype"))
            .or_else(|| int_field("mediaSubType"))
            .or_else(|| int_field("kindSubTypeEnum")),
        image_width: int_field("imageWidth").or_else(|| int_field("originalWidth")),
        image_height: int_field("imageHeight").or_else(|| int_field("originalHeight")),
        video_frame_rate: float_field("videoFrameRate")
            .or_else(|| float_field("frameRate")),
    };

    let folder = classify_folder(&filename, &extension, is_video, has_video_res, meta);

    Some(ICloudAsset {
        record_name,
        master_ref,
        filename,
        extension,
        is_video,
        date_created_ms,
        duration_ms,
        size_bytes,
        thumb_url,
        original_url,
        is_hidden,
        is_favorite,
        is_in_trash,
        folder,
    })
}

pub async fn download_url_binary(
    _window: &WebviewWindow,
    _bridge: &ICloudBridge,
    url: &str,
) -> Result<Vec<u8>, String> {
    let client = reqwest::Client::builder()
        .user_agent(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
             (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36",
        )
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| format!("http client: {e}"))?;
    let resp = client
        .get(url)
        .header("Origin", "https://www.icloud.com")
        .header("Referer", "https://www.icloud.com/")
        .header("Accept", "*/*")
        .send()
        .await
        .map_err(|e| format!("download GET: {e}"))?;
    let status = resp.status();
    if !status.is_success() {
        let body_preview = resp
            .text()
            .await
            .unwrap_or_default()
            .chars()
            .take(200)
            .collect::<String>();
        return Err(format!("download HTTP {status}: {body_preview}"));
    }
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("download body: {e}"))?;
    Ok(bytes.to_vec())
}

// â”€â”€ Export progressif iCloud â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ICloudFileExport {
    pub record_name: String,
    pub filename: String,
    pub download_url: String,
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
struct DoneEvent {
    exported: usize,
    failed: usize,
    skipped_cloud: usize,
    dest_dir: String,
}

pub async fn export_assets_progressive(
    app: tauri::AppHandle,
    window: WebviewWindow,
    bridge_responses: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String, crate::icloud::PendingResp>>>,
    bridge_notify: std::sync::Arc<tokio::sync::Notify>,
    files: Vec<ICloudFileExport>,
    dest_dir: String,
    cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
) {
    use std::path::Path;
    use std::sync::atomic::Ordering;
    use tauri::Emitter;

    let total = files.len();
    let dest_path = Path::new(&dest_dir).to_path_buf();
    if let Err(e) = std::fs::create_dir_all(&dest_path) {
        let _ = app.emit(
            "afc-export-error",
            format!("Impossible de crÃ©er le dossier : {e}"),
        );
        return;
    }

    let bridge = crate::icloud::ICloudBridge {
        responses: bridge_responses,
        notify: bridge_notify,
    };

    let mut exported = 0usize;
    let mut failed = 0usize;

    for (i, f) in files.iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            let _ = app.emit(
                "afc-export-done",
                DoneEvent {
                    exported,
                    failed,
                    skipped_cloud: 0,
                    dest_dir: dest_dir.clone(),
                },
            );
            return;
        }

        let _ = app.emit(
            "afc-export-progress",
            ProgressEvent {
                current: i + 1,
                total,
                filename: f.filename.clone(),
                exported,
                failed,
            },
        );

        match download_url_binary(&window, &bridge, &f.download_url).await {
            Ok(bytes) => {
                let safe_name = std::path::Path::new(&f.filename)
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| format!("file_{exported}"));
                let target = dest_path.join(&safe_name);
                match std::fs::write(&target, &bytes) {
                    Ok(_) => {
                        exported += 1;
                    }
                    Err(e) => {
                        failed += 1;
                    }
                }
            }
            Err(e) => {
                failed += 1;
            }
        }
    }

    let _ = app.emit(
        "afc-export-done",
        DoneEvent {
            exported,
            failed,
            skipped_cloud: 0,
            dest_dir,
        },
    );
}
