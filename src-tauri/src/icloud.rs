use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{Manager, WebviewWindow};
use uuid::Uuid;

// â”€â”€ Public types â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ICloudSession {
    pub cookie_header: String,
    pub dsid: String,
    pub apple_id: Option<String>,
    pub full_name: Option<String>,
    pub webservices: HashMap<String, String>,
    pub client_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CookiePair {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct ICloudSessionPublic {
    pub apple_id: Option<String>,
    pub full_name: Option<String>,
    pub photos_url: Option<String>,
    pub authenticated_at_ms: i64,
}

impl ICloudSession {
    pub fn to_public(&self) -> ICloudSessionPublic {
        ICloudSessionPublic {
            apple_id: self.apple_id.clone(),
            full_name: self.full_name.clone(),
            photos_url: self
                .webservices
                .get("ckdatabasews")
                .cloned()
                .or_else(|| self.webservices.get("photos").cloned()),
            authenticated_at_ms: chrono::Utc::now().timestamp_millis(),
        }
    }
}

// â”€â”€ Shared state â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

pub type SessionState = Arc<Mutex<Option<ICloudSession>>>;

pub fn new_session_state() -> SessionState {
    Arc::new(Mutex::new(None))
}

pub const BRIDGE_HOST: &str = "pb-bridge.invalid";

#[derive(Default)]
pub struct PendingResp {
    pub status: u16,
    pub total: usize,
    pub chunks: HashMap<usize, String>,
}

impl PendingResp {
    pub fn is_complete(&self) -> bool {
        self.total > 0 && self.chunks.len() == self.total
    }
    pub fn assemble(&self) -> String {
        let mut out = String::new();
        for i in 0..self.total {
            if let Some(c) = self.chunks.get(&i) {
                out.push_str(c);
            }
        }
        out
    }
}

pub struct ICloudBridge {
    pub responses: Arc<Mutex<HashMap<String, PendingResp>>>,
    pub notify: Arc<tokio::sync::Notify>,
}

impl Default for ICloudBridge {
    fn default() -> Self {
        Self {
            responses: Arc::new(Mutex::new(HashMap::new())),
            notify: Arc::new(tokio::sync::Notify::new()),
        }
    }
}

// â”€â”€ Cookie detection â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

pub fn is_authenticated(cookies: &[CookiePair]) -> bool {
    const TOKEN_NAMES: &[&str] = &[
        "X-APPLE-WEBAUTH-TOKEN",
        "X-APPLE-DS-WEB-SESSION-TOKEN",
        "X-APPLE-WEBAUTH-HSA-TRUST",
        "X-APPLE-WEBAUTH-PCS-Cloudkit",
        "X-APPLE-WEBAUTH-PCS-Photos",
    ];
    cookies.iter().any(|c| {
        !c.value.is_empty()
            && TOKEN_NAMES
                .iter()
                .any(|n| c.name.eq_ignore_ascii_case(n))
    })
}

// â”€â”€ WebView-proxied fetch â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

pub async fn webview_fetch(
    window: &WebviewWindow,
    bridge: &ICloudBridge,
    url: &str,
    body: Option<&str>,
) -> Result<(u16, String), String> {
    let marker = Uuid::new_v4().simple().to_string();
    let method = if body.is_some() { "POST" } else { "GET" };
    let body_b64 = body.map(|b| B64.encode(b));

    // JS partagÃ© : helper de post chunkÃ© via fausses navigations. On a besoin
    // de dÃ©couper parce qu'une URL > ~2 MB est silencieusement rejetÃ©e par
    // WebView2 et notre hook de navigation ne se dÃ©clenche jamais.
    let js = format!(
        r#"(async () => {{
            const marker = {marker_json};
            const target = {url_json};
            const BRIDGE = '{bridge_host}';
            console.log('[PB bridge] start fetch', marker, target);
            const navTo = (u) => new Promise(res => {{
                try {{
                    const a = document.createElement('a');
                    a.href = u;
                    a.style.display = 'none';
                    document.documentElement.appendChild(a);
                    a.click();
                    a.remove();
                }} catch (e1) {{
                    console.error('[PB bridge] anchor click failed', e1);
                    try {{ window.location.href = u; }} catch (e2) {{}}
                }}
                // Laisse le temps au hook de Tauri de capturer.
                setTimeout(res, 25);
            }});
            const postChunked = async (status, body) => {{
                const CHUNK = 200000; // ~200 KB sÃ©curitÃ©
                const total = Math.max(1, Math.ceil(body.length / CHUNK));
                console.log('[PB bridge] post', marker, 'status=', status, 'len=', body.length, 'chunks=', total);
                for (let i = 0; i < total; i++) {{
                    const piece = body.substr(i * CHUNK, CHUNK);
                    const u = 'https://' + BRIDGE + '/?marker=' + encodeURIComponent(marker)
                              + '&status=' + status
                              + '&seq=' + i + '&total=' + total
                              + '&body=' + encodeURIComponent(piece);
                    await navTo(u);
                }}
            }};
            try {{
                const opts = {{
                    method: {method_json},
                    headers: {{ 'Content-Type': 'text/plain', 'Accept': '*/*' }},
                    credentials: 'include',
                }};
                if ({has_body}) {{
                    opts.body = atob({body_b64_json});
                }}
                const r = await fetch(target, opts);
                console.log('[PB bridge] fetch resolved', marker, 'status=', r.status);
                const text = await r.text();
                const enc = btoa(unescape(encodeURIComponent(text)));
                await postChunked(r.status, enc);
            }} catch (e) {{
                console.error('[PB bridge] fetch threw', marker, e);
                const enc = btoa(unescape(encodeURIComponent(String(e && e.message ? e.message : e))));
                await postChunked(0, enc);
            }}
        }})();"#,
        marker_json = serde_json::to_string(&marker).unwrap(),
        bridge_host = BRIDGE_HOST,
        method_json = serde_json::to_string(method).unwrap(),
        url_json = serde_json::to_string(url).unwrap(),
        body_b64_json = serde_json::to_string(body_b64.as_deref().unwrap_or("")).unwrap(),
        has_body = if body_b64.is_some() { "true" } else { "false" },
    );
    window.eval(&js).map_err(|e| format!("eval: {e}"))?;
    let assembled = await_bridge_response(bridge, &marker, 120).await?;
    let decoded = B64
        .decode(assembled.1.as_bytes())
        .map_err(|e| format!("base64 decode: {e}"))?;
    let body = String::from_utf8(decoded).map_err(|e| format!("utf8: {e}"))?;
    Ok((assembled.0, body))
}

pub async fn await_bridge_response(
    bridge: &ICloudBridge,
    marker: &str,
    timeout_secs: u64,
) -> Result<(u16, String), String> {
    let start = std::time::Instant::now();
    let timeout = Duration::from_secs(timeout_secs);
    loop {
        if start.elapsed() > timeout {
            return Err(format!(
                "Timeout {timeout_secs}s en attente d'une rÃ©ponse iCloud."
            ));
        }
        let notified = bridge.notify.notified();
        {
            let mut guard = bridge
                .responses
                .lock()
                .map_err(|e| format!("bridge lock: {e}"))?;
            if let Some(pending) = guard.get(marker) {
                if pending.is_complete() {
                    let assembled = pending.assemble();
                    let status = pending.status;
                    guard.remove(marker);
                    drop(guard);
                    return Ok((status, assembled));
                }
            }
        }
        let _ = tokio::time::timeout(Duration::from_millis(1500), notified).await;
    }
}

#[allow(dead_code)]
pub async fn webview_fetch_binary(
    window: &WebviewWindow,
    bridge: &ICloudBridge,
    url: &str,
) -> Result<Vec<u8>, String> {
    let marker = Uuid::new_v4().simple().to_string();

    let js = format!(
        r#"(async () => {{
            const marker = {marker_json};
            const BRIDGE = '{bridge_host}';
            const navTo = (u) => new Promise(res => {{
                try {{
                    const a = document.createElement('a');
                    a.href = u;
                    a.style.display = 'none';
                    document.documentElement.appendChild(a);
                    a.click();
                    a.remove();
                }} catch (_e1) {{
                    try {{ window.location.href = u; }} catch (_e2) {{}}
                }}
                setTimeout(res, 25);
            }});
            const postChunked = async (status, body) => {{
                const CHUNK = 200000;
                const total = Math.max(1, Math.ceil(body.length / CHUNK));
                for (let i = 0; i < total; i++) {{
                    const piece = body.substr(i * CHUNK, CHUNK);
                    const u = 'https://' + BRIDGE + '/?marker=' + encodeURIComponent(marker)
                              + '&status=' + status
                              + '&seq=' + i + '&total=' + total
                              + '&body=' + encodeURIComponent(piece);
                    await navTo(u);
                }}
            }};
            try {{
                const r = await fetch({url_json}, {{ credentials: 'include' }});
                if (!r.ok) {{ await postChunked(r.status, ''); return; }}
                const buf = await r.arrayBuffer();
                const bytes = new Uint8Array(buf);
                let bin = '';
                const chunk = 0x8000;
                for (let i = 0; i < bytes.length; i += chunk) {{
                    bin += String.fromCharCode.apply(null, bytes.subarray(i, i + chunk));
                }}
                await postChunked(r.status, btoa(bin));
            }} catch (_e) {{
                await postChunked(0, '');
            }}
        }})();"#,
        marker_json = serde_json::to_string(&marker).unwrap(),
        bridge_host = BRIDGE_HOST,
        url_json = serde_json::to_string(url).unwrap(),
    );

    window.eval(&js).map_err(|e| format!("eval: {e}"))?;
    let (status, body_b64) = await_bridge_response(bridge, &marker, 60).await?;
    if status != 200 {
        return Err(format!("binary HTTP {status}"));
    }
    B64.decode(body_b64.as_bytes())
        .map_err(|e| format!("base64 decode: {e}"))
}

const CLIENT_BUILD: &str = "2421Project53";
const CLIENT_MASTER: &str = "2421B25";

pub async fn validate_session_via_webview(
    window: &WebviewWindow,
    bridge: &ICloudBridge,
) -> Result<ICloudSession, String> {
    let client_id = Uuid::new_v4().to_string().to_uppercase();
    let url = format!(
        "https://setup.icloud.com/setup/ws/1/validate\
         ?clientBuildNumber={CLIENT_BUILD}\
         &clientMasteringNumber={CLIENT_MASTER}\
         &clientId={client_id}"
    );

    let (status, body) = webview_fetch(window, bridge, &url, Some("")).await?;

    if status != 200 {
        return Err(format!(
            "validate HTTP {status}: {}",
            body.chars().take(500).collect::<String>()
        ));
    }

    let v: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| format!("validate JSON parse: {e}"))?;

    let dsid = v
        .get("dsInfo")
        .and_then(|d| d.get("dsid"))
        .and_then(|d| d.as_str())
        .map(String::from)
        .ok_or_else(|| {
            "validate response missing dsInfo.dsid â€” sign in to iCloud first."
                .to_string()
        })?;

    let apple_id = v
        .get("dsInfo")
        .and_then(|d| d.get("appleId"))
        .and_then(|d| d.as_str())
        .map(String::from);

    let full_name = v
        .get("dsInfo")
        .and_then(|d| d.get("fullName"))
        .and_then(|d| d.as_str())
        .map(String::from);

    let mut webservices = HashMap::new();
    if let Some(ws) = v.get("webservices").and_then(|w| w.as_object()) {
        for (k, info) in ws {
            if let Some(url) = info.get("url").and_then(|u| u.as_str()) {
                webservices.insert(k.clone(), url.to_string());
            }
        }
    }

    // Cookie header retained as fallback for non-WebView callers (download URLs).
    let cookie_header = window
        .cookies()
        .ok()
        .map(|list| {
            list.into_iter()
                .map(|c| format!("{}={}", c.name(), c.value()))
                .collect::<Vec<_>>()
                .join("; ")
        })
        .unwrap_or_default();

    Ok(ICloudSession {
        cookie_header,
        dsid,
        apple_id,
        full_name,
        webservices,
        client_id,
    })
}

pub async fn validate_session(_cookies: Vec<CookiePair>) -> Result<ICloudSession, String> {
    Err("validate_session: deprecated â€” use validate_session_via_webview".to_string())
}

// Silence the otherwise-unused `Manager` import on platforms where we don't
// reach into AppHandle here. Kept because future commands will use it.
#[allow(dead_code)]
fn _force_use_manager(_app: &tauri::AppHandle) {
    let _ = <tauri::AppHandle as Manager<tauri::Wry>>::package_info;
}
