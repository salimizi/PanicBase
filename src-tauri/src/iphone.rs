use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use serde::Serialize;

use regex::Regex;
use crate::database;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IphoneUsbStatus {
    pub phase: String,
    pub detail: String,
    pub udids: Vec<String>,
    pub marketing_name: Option<String>,
    pub product_type: Option<String>,
    pub ios_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recovery_serial: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recovery_imei: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recovery_ecid: Option<String>,
}

pub(crate) fn usb_status(
    phase: impl Into<String>,
    detail: impl Into<String>,
    udids: Vec<String>,
    marketing_name: Option<String>,
    product_type: Option<String>,
    ios_version: Option<String>,
) -> IphoneUsbStatus {
    IphoneUsbStatus {
        phase: phase.into(),
        detail: detail.into(),
        udids,
        marketing_name,
        product_type,
        ios_version,
        recovery_serial: None,
        recovery_imei: None,
        recovery_ecid: None,
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInfoField {
    pub id: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IphoneDeviceDetails {
    pub fields: Vec<DeviceInfoField>,
    pub udid: Option<String>,
    pub hint: Option<String>,
}

pub fn marketing_lookup(product_type: &str) -> &'static str {
    match product_type {
        "iPhone8,1" => "iPhone 6s",
        "iPhone8,2" => "iPhone 6s Plus",
        "iPhone8,4" => "iPhone SE (1st gen)",
        "iPhone9,1" => "iPhone 7",
        "iPhone9,2" => "iPhone 7 Plus",
        "iPhone9,3" => "iPhone 7",
        "iPhone9,4" => "iPhone 7 Plus",
        "iPhone10,1" => "iPhone 8",
        "iPhone10,2" => "iPhone 8 Plus",
        "iPhone10,3" => "iPhone X",
        "iPhone10,4" => "iPhone 8",
        "iPhone10,5" => "iPhone 8 Plus",
        "iPhone10,6" => "iPhone X",
        "iPhone11,2" => "iPhone XS",
        "iPhone11,4" => "iPhone XS Max",
        "iPhone11,6" => "iPhone XS Max",
        "iPhone11,8" => "iPhone XR",
        "iPhone12,8" => "iPhone SE (2nd gen)",
        "iPhone12,1" => "iPhone 11",
        "iPhone12,3" => "iPhone 11 Pro",
        "iPhone12,5" => "iPhone 11 Pro Max",
        "iPhone13,1" => "iPhone 12 mini",
        "iPhone13,2" => "iPhone 12",
        "iPhone13,3" => "iPhone 12 Pro",
        "iPhone13,4" => "iPhone 12 Pro Max",
        "iPhone14,4" => "iPhone 13 mini",
        "iPhone14,5" => "iPhone 13",
        "iPhone14,2" => "iPhone 13 Pro",
        "iPhone14,3" => "iPhone 13 Pro Max",
        "iPhone14,7" => "iPhone 14",
        "iPhone14,6" => "iPhone SE (3rd gen)",
        "iPhone14,8" => "iPhone 14 Plus",
        "iPhone15,2" => "iPhone 14 Pro",
        "iPhone15,3" => "iPhone 14 Pro Max",
        "iPhone15,4" => "iPhone 15",
        "iPhone15,5" => "iPhone 15 Plus",
        "iPhone16,1" => "iPhone 15 Pro",
        "iPhone16,2" => "iPhone 15 Pro Max",
        "iPhone17,1" => "iPhone 16 Pro",
        "iPhone17,2" => "iPhone 16 Pro Max",
        "iPhone17,3" => "iPhone 16",
        "iPhone17,4" => "iPhone 16 Plus",
        "iPhone18,1" => "iPhone 17 Pro",
        "iPhone18,2" => "iPhone 17 Pro Max",
        "iPhone18,3" => "iPhone 17",
        "iPhone18,4" => "iPhone 17 Plus",
        _ => "",
    }
}

pub fn marketing_display_for_hints(hint: Option<&str>) -> Option<String> {
    let t = hint?.trim();
    if t.is_empty() {
        return None;
    }
    if t.contains(',') && t.to_lowercase().starts_with("iphone") {
        let m = marketing_lookup(t);
        if m.is_empty() {
            Some(t.to_string())
        } else {
            Some(m.to_string())
        }
    } else {
        Some(t.to_string())
    }
}

static BUNDLED_LIBIMOBILEDEVICE_DIR: OnceLock<PathBuf> = OnceLock::new();

pub(crate) fn bundled_libimobiledevice_dir() -> Option<PathBuf> {
    BUNDLED_LIBIMOBILEDEVICE_DIR.get().cloned()
}

pub(crate) fn set_bundled_libimobiledevice_dir(dir: PathBuf) {
    #[cfg(windows)]
    let probe = dir.join("idevice_id.exe");
    #[cfg(not(windows))]
    let probe = dir.join("idevice_id");
    if probe.is_file() {
        let _ = BUNDLED_LIBIMOBILEDEVICE_DIR.set(dir);
    }
}

fn bundled_libimobile_tool(tool_stem: &str) -> Option<PathBuf> {
    let base = BUNDLED_LIBIMOBILEDEVICE_DIR.get()?;
    #[cfg(windows)]
    let p = base.join(format!("{tool_stem}.exe"));
    #[cfg(not(windows))]
    let p = base.join(tool_stem);
    p.is_file().then_some(p)
}

#[cfg(windows)]
fn neighbor_resource_libimobiledevice_dir() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?.join("resources").join("libimobiledevice");
    let probe = dir.join("idevice_id.exe");
    probe.is_file().then_some(dir)
}

#[cfg(not(windows))]
fn neighbor_resource_libimobiledevice_dir() -> Option<PathBuf> {
    None
}

#[cfg(all(windows, debug_assertions))]
fn cargo_manifest_resources_libimobiledevice_dir() -> Option<PathBuf> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("resources")
        .join("libimobiledevice");
    let probe = dir.join("idevice_id.exe");
    probe.is_file().then_some(dir)
}

#[cfg(not(all(windows, debug_assertions)))]
fn cargo_manifest_resources_libimobiledevice_dir() -> Option<PathBuf> {
    None
}

pub(crate) fn try_init_bundled_neighbor_if_unset() {
    if BUNDLED_LIBIMOBILEDEVICE_DIR.get().is_some() {
        return;
    }
    if let Some(dir) = cargo_manifest_resources_libimobiledevice_dir() {
        set_bundled_libimobiledevice_dir(dir);
        return;
    }
    if let Some(dir) = neighbor_resource_libimobiledevice_dir() {
        set_bundled_libimobiledevice_dir(dir);
    }
}

#[cfg(windows)]
fn find_bundled_idevice_tool(tool_stem: &str) -> Option<PathBuf> {
    let local = std::env::var_os("LOCALAPPDATA")?;
    let root = PathBuf::from(local).join("PanicBaseTools");
    let rd = std::fs::read_dir(&root).ok()?;
    let exe_name = format!("{tool_stem}.exe");
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    for ent in rd.flatten() {
        let Ok(ft) = ent.file_type() else {
            continue;
        };
        if !ft.is_dir() {
            continue;
        }
        let fname = ent.file_name();
        let name = fname.to_string_lossy();
        if !name.starts_with("app-") {
            continue;
        }
        let candidate = ent.path().join("win-x64").join(&exe_name);
        if !candidate.is_file() {
            continue;
        }
        let ts = std::fs::metadata(ent.path())
            .and_then(|m| m.modified())
            .unwrap_or(std::time::UNIX_EPOCH);
        if best.as_ref().map(|(t, _)| ts > *t).unwrap_or(true) {
            best = Some((ts, candidate));
        }
    }
    best.map(|(_, p)| p)
}

#[cfg(not(windows))]
fn find_bundled_idevice_tool(_tool_stem: &str) -> Option<PathBuf> {
    None
}

pub(crate) fn resolved_libimobile_tool(tool_stem: &str) -> PathBuf {
    #[cfg(windows)]
    let filename = format!("{tool_stem}.exe");
    #[cfg(not(windows))]
    let filename = tool_stem.to_string();

    #[cfg(windows)]
    if let Ok(dir) = std::env::var("PANICBASE_IDEVICE_DIR") {
        let p = PathBuf::from(dir.trim().trim_matches('"')).join(&filename);
        if p.exists() {
            return p;
        }
    }

    #[cfg(windows)]
    if let Ok(home) = std::env::var("LIBIMOBILEDEVICE_HOME") {
        let p = PathBuf::from(home.trim().trim_matches('"')).join(&filename);
        if p.exists() {
            return p;
        }
    }

    #[cfg(windows)]
    if let Some(dir) = neighbor_resource_libimobiledevice_dir() {
        let p = dir.join(&filename);
        if p.is_file() {
            return p;
        }
    }

    #[cfg(windows)]
    if let Some(p) = bundled_libimobile_tool(tool_stem) {
        return p;
    }

    #[cfg(windows)]
    if let Some(p) = find_bundled_idevice_tool(tool_stem) {
        return p;
    }

    #[cfg(windows)]
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        let p = PathBuf::from(local).join("libimobiledevice").join(&filename);
        if p.exists() {
            return p;
        }
    }

    #[cfg(windows)]
    for dir in [
        r"C:\Program Files\libimobiledevice",
        r"C:\Program Files (x86)\libimobiledevice",
    ] {
        let p = PathBuf::from(dir).join(&filename);
        if p.exists() {
            return p;
        }
    }

    #[cfg(not(windows))]
    if let Some(p) = bundled_libimobile_tool(tool_stem) {
        return p;
    }

    #[cfg(windows)]
    {
        PathBuf::from(&filename)
    }
    #[cfg(not(windows))]
    {
        PathBuf::from(tool_stem)
    }
}

const TIMEOUT_ID_LIST: Duration = Duration::from_secs(5);
const TIMEOUT_IDE_KEY: Duration = Duration::from_secs(6);
const TIMEOUT_IDE_PROBE: Duration = Duration::from_millis(2200);
const TIMEOUT_IDE_KEY_BULK: Duration = Duration::from_millis(2200);
const TIMEOUT_IDE_KEY_IDENT: Duration = Duration::from_millis(1200);
const TIMEOUT_IDE_FULL_XML: Duration = Duration::from_secs(55);
const TIMEOUT_INFO_RAW: Duration = Duration::from_secs(3);

pub(crate) fn command_for_tool(tool: &Path) -> Command {
    let mut c = Command::new(tool);
    if let Some(dir) = tool.parent() {
        if !dir.as_os_str().is_empty() {
            let bundle = dir.join("imobiledevice.dll").is_file()
                || dir.join("usbmuxd.dll").is_file()
                || dir.join("libimobiledevice-1.0.dll").is_file();
            if bundle {
                c.current_dir(dir);
                #[cfg(windows)]
                {
                    use std::ffi::OsString;
                    use std::os::windows::process::CommandExt;
                    let mut path_prefixed = OsString::from(dir.as_os_str());
                    path_prefixed.push(";");
                    path_prefixed.push(std::env::var_os("PATH").unwrap_or_default());
                    c.env("PATH", path_prefixed);
                    c.creation_flags(0x08000000); // CREATE_NO_WINDOW
                }
            }
        }
    }
    c
}

pub(crate) fn command_output_with_timeout(mut cmd: Command, timeout: Duration) -> std::io::Result<std::process::Output> {
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW â€” supprime la fenÃªtre CMD visible
    }
    let mut child = cmd.spawn()?;
    let child_id = child.id();
    let start = Instant::now();
    loop {
        if child.try_wait()?.is_some() {
            return child.wait_with_output();
        }
        if start.elapsed() >= timeout {
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                let _ = Command::new("taskkill")
                    .creation_flags(0x08000000) // CREATE_NO_WINDOW â€” Ã©vite le flash CMD visible
                    .arg("/PID")
                    .arg(child_id.to_string())
                    .arg("/T")
                    .arg("/F")
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .output();
            }
            let _ = child.kill();
            let _ = child.wait();
            return Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "command timed out",
            ));
        }
        std::thread::sleep(Duration::from_millis(40));
    }
}

fn idevice_read_key_timeout(tool: &Path, udid: Option<&str>, key: &str, timeout: Duration) -> Option<String> {
    let mut c = command_for_tool(tool);
    if let Some(u) = udid {
        if !u.trim().is_empty() {
            c.arg("-u").arg(u.trim());
        }
    }
    c.arg("-k").arg(key);
    let ok = command_output_with_timeout(c, timeout)
        .ok()
        .filter(|o| o.status.success())?;
    let v = String::from_utf8_lossy(&ok.stdout).trim().to_string();
    if v.is_empty() {
        None
    } else {
        Some(v)
    }
}

fn idevice_read_key(tool: &Path, udid: Option<&str>, key: &str) -> Option<String> {
    idevice_read_key_timeout(tool, udid, key, TIMEOUT_IDE_KEY)
}

fn looks_like_strict_unplugged(msg: &str) -> bool {
    let m = msg.to_lowercase();
    m.contains("no device found")
        || m.contains("no device detected")
        || m.contains("device not found")
        || m.contains("no devices found")
        || m.contains("unable to retrieve device list")
        || m.contains("could not connect to device")
        || m.contains("connection refused")
        || m.contains("no device")
}

fn looks_like_trust_or_lockdown(msg: &str) -> bool {
    let m = msg.to_lowercase();
    m.contains("please unlock")
        || m.contains("unlock your")
        || m.contains("trust this computer")
        || m.contains("tap trust")
        || m.contains("could not connect to lockdownd")
        || m.contains("device is not paired")
        || m.contains("not paired with this host")
        || m.contains("pairing is required")
        || m.contains("pairing required")
        || m.contains("invalid host id")
        || m.contains("host is not trusted")
        || m.contains("user denied")
        || m.contains("password protected")
}

fn clamp_usb_detail(s: &str, max_chars: usize) -> String {
    let t = s.trim();
    if t.is_empty() {
        return String::new();
    }
    let n = t.chars().count();
    if n <= max_chars {
        t.to_string()
    } else {
        format!("{}â€¦", t.chars().take(max_chars.saturating_sub(1)).collect::<String>())
    }
}

fn idevice_probe_without_udid(exe_info: &Path) -> (Option<String>, Option<String>, String) {
    let product = idevice_read_key_timeout(exe_info, None, "ProductType", TIMEOUT_IDE_PROBE);
    let ios = idevice_read_key_timeout(exe_info, None, "ProductVersion", TIMEOUT_IDE_PROBE);
    if product.is_some() {
        return (product, ios, String::new());
    }

    let raw = command_for_tool(exe_info);
    let detail = match command_output_with_timeout(raw, TIMEOUT_INFO_RAW) {
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr).trim().to_string();
            let stdout = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if !stderr.is_empty() {
                stderr
            } else {
    stdout
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {
            "ideviceinfo : dÃ©lai dÃ©passÃ© (USB lent ou pilote occupÃ©).".to_string()
        }
        Err(e) => e.to_string(),
    };
    (None, None, detail)
}

fn run_idevice_id_list(exe_id: &Path) -> std::io::Result<std::process::Output> {
    let mut id_cmd = command_for_tool(exe_id);
    id_cmd.arg("-l");
    command_output_with_timeout(id_cmd, TIMEOUT_ID_LIST)
}

fn status_from_product_ios(udids: Vec<String>, product: Option<String>, ios: Option<String>) -> IphoneUsbStatus {
    let Some(product_type_val) = product else {
        return usb_status("awaiting_trust", String::new(), udids, None, None, None);
    };
    let mapped = marketing_lookup(&product_type_val);
    let mapped_opt = if mapped.is_empty() {
        None
    } else {
        Some(mapped.to_string())
    };
    usb_status(
        "connected",
        String::new(),
        udids,
        mapped_opt,
        Some(product_type_val),
        ios,
    )
}

fn token_likely_apple_udid(t: &str) -> Option<String> {
    let t = t.trim().trim_start_matches('\u{feff}');
    if t.len() < 20 || t.len() > 48 {
        return None;
    }
    if !t.chars().all(|c| c.is_ascii_hexdigit() || c == '-') {
        return None;
    }
    let hex_digits = t.chars().filter(|c| c.is_ascii_hexdigit()).count();
    if hex_digits < 16 {
        return None;
    }
    Some(t.to_string())
}

fn parse_udids(stdout: &str) -> Vec<String> {
    let text = stdout.trim().trim_start_matches('\u{feff}');
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim().trim_start_matches('\u{feff}');
        if line.is_empty() {
            continue;
        }
        let mut found_in_line = false;
        for w in line.split_whitespace() {
            if let Some(u) = token_likely_apple_udid(w) {
                found_in_line = true;
                if !out.contains(&u) {
                    out.push(u);
                }
            }
        }
        if !found_in_line {
            if let Some(u) = token_likely_apple_udid(line) {
                if !out.contains(&u) {
                    out.push(u);
                }
            }
        }
    }
    out
}

fn unplugged() -> IphoneUsbStatus {
    usb_status("unplugged", String::new(), vec![], None, None, None)
}

const TIMEOUT_IRECOVERY: Duration = Duration::from_secs(3);

#[derive(Debug, Default, Clone)]
struct IrecoveryQueryParsed {
    serial: Option<String>,
    imei: Option<String>,
    ecid: Option<String>,
    product: Option<String>,
    mode: Option<String>,
    irecovery_display_name: Option<String>,
    cpid: Option<u32>,
    bdid: Option<u32>,
}

fn parse_irecovery_hex_u32(val: &str) -> Option<u32> {
    let t = val.trim();
    let hex = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X"))?;
    u32::from_str_radix(hex, 16).ok()
}

const IRECOVERY_CPID_BDID_IPHONE: &[(u32, u32, &'static str)] = &[
    (0x8900, 0x00, "iPhone1,1"),
    (0x8900, 0x04, "iPhone1,2"),
    (0x8920, 0x00, "iPhone2,1"),
    (0x8930, 0x00, "iPhone3,1"),
    (0x8930, 0x04, "iPhone3,2"),
    (0x8930, 0x06, "iPhone3,3"),
    (0x8940, 0x08, "iPhone4,1"),
    (0x8950, 0x00, "iPhone5,1"),
    (0x8950, 0x02, "iPhone5,2"),
    (0x8950, 0x0a, "iPhone5,3"),
    (0x8950, 0x0e, "iPhone5,4"),
    (0x8960, 0x00, "iPhone6,1"),
    (0x8960, 0x02, "iPhone6,2"),
    (0x7000, 0x04, "iPhone7,1"),
    (0x7000, 0x06, "iPhone7,2"),
    (0x8000, 0x04, "iPhone8,1"),
    (0x8003, 0x04, "iPhone8,1"),
    (0x8000, 0x06, "iPhone8,2"),
    (0x8003, 0x06, "iPhone8,2"),
    (0x8003, 0x02, "iPhone8,4"),
    (0x8000, 0x02, "iPhone8,4"),
    (0x8010, 0x08, "iPhone9,1"),
    (0x8010, 0x0a, "iPhone9,2"),
    (0x8010, 0x0c, "iPhone9,3"),
    (0x8010, 0x0e, "iPhone9,4"),
    (0x8015, 0x02, "iPhone10,1"),
    (0x8015, 0x04, "iPhone10,2"),
    (0x8015, 0x06, "iPhone10,3"),
    (0x8015, 0x0a, "iPhone10,4"),
    (0x8015, 0x0c, "iPhone10,5"),
    (0x8015, 0x0e, "iPhone10,6"),
    (0x8020, 0x0e, "iPhone11,2"),
    (0x8020, 0x0a, "iPhone11,4"),
    (0x8020, 0x1a, "iPhone11,6"),
    (0x8020, 0x0c, "iPhone11,8"),
    (0x8030, 0x04, "iPhone12,1"),
    (0x8030, 0x06, "iPhone12,3"),
    (0x8030, 0x02, "iPhone12,5"),
    (0x8030, 0x10, "iPhone12,8"),
    (0x8101, 0x0a, "iPhone13,1"),
    (0x8101, 0x0c, "iPhone13,2"),
    (0x8101, 0x0e, "iPhone13,3"),
    (0x8101, 0x08, "iPhone13,4"),
    (0x8110, 0x0c, "iPhone14,2"),
    (0x8110, 0x0e, "iPhone14,3"),
    (0x8110, 0x08, "iPhone14,4"),
    (0x8110, 0x0a, "iPhone14,5"),
    (0x8110, 0x10, "iPhone14,6"),
    (0x8110, 0x18, "iPhone14,7"),
    (0x8110, 0x1a, "iPhone14,8"),
    (0x8120, 0x0c, "iPhone15,2"),
    (0x8120, 0x0e, "iPhone15,3"),
    (0x8120, 0x08, "iPhone15,4"),
    (0x8120, 0x0a, "iPhone15,5"),
    (0x8130, 0x04, "iPhone16,1"),
    (0x8130, 0x06, "iPhone16,2"),
    (0x8140, 0x0c, "iPhone17,1"),
    (0x8140, 0x0e, "iPhone17,2"),
    (0x8140, 0x08, "iPhone17,3"),
    (0x8140, 0x0a, "iPhone17,4"),
    (0x8140, 0x04, "iPhone17,5"),
    (0x8150, 0x0c, "iPhone18,1"),
    (0x8150, 0x0e, "iPhone18,2"),
    (0x8150, 0x08, "iPhone18,3"),
    (0x8150, 0x0a, "iPhone18,4"),
    (0x8150, 0x16, "iPhone18,5"),
];

fn iphone_product_type_from_cpid_bdid(cpid: u32, bdid: u32) -> Option<&'static str> {
    IRECOVERY_CPID_BDID_IPHONE
        .iter()
        .find(|(c, b, _)| *c == cpid && *b == bdid)
        .map(|(_, _, p)| *p)
}

fn parse_irecovery_query(text: &str) -> IrecoveryQueryParsed {
    let mut out = IrecoveryQueryParsed::default();
    for line in text.lines() {
        let line = line.trim();
        let Some((raw_key, mut val)) = line.split_once(':') else {
            continue;
        };
        val = val.trim();
        if val.starts_with('=') {
            val = val[1..].trim();
        }
        if val.is_empty() || val.eq_ignore_ascii_case("n/a") {
            continue;
        }
        let ku = raw_key.trim().to_uppercase();
        match ku.as_str() {
            "SRNM" | "SERIAL" => out.serial = Some(val.to_string()),
            "IMEI" => out.imei = Some(val.to_string()),
            "ECID" => out.ecid = Some(val.to_string()),
            "PRODUCT" => out.product = Some(val.to_string()),
            "NAME" => out.irecovery_display_name = Some(val.to_string()),
            "CPID" => out.cpid = parse_irecovery_hex_u32(val),
            "BDID" => out.bdid = parse_irecovery_hex_u32(val),
            "MODEL" => {
                if out.product.is_none() {
                    let v = val.trim();
                    if let Some(p) = infer_iphone_product_from_blob(v) {
                        out.product = Some(p);
                    }
                }
            }
            "MODE" => out.mode = Some(val.to_string()),
            _ => {}
        }
    }
    out
}

fn infer_iphone_product_from_blob(blob: &str) -> Option<String> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"(?i)iPhone(\d+)\s*,\s*(\d+)").expect("iphone product regex"));
    let caps = re.captures(blob)?;
    Some(format!(
        "iPhone{},{}",
        caps.get(1)?.as_str(),
        caps.get(2)?.as_str()
    ))
}

fn try_recovery_status() -> Option<IphoneUsbStatus> {
    try_init_bundled_neighbor_if_unset();
    let exe = resolved_libimobile_tool("irecovery");
    if !exe.is_file() {
        return None;
    }
    let mut c = command_for_tool(&exe);
    c.arg("-q");
    let out = command_output_with_timeout(c, TIMEOUT_IRECOVERY).ok()?;
    if !out.status.success() {
        return None;
    }
    let text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    let mut parsed = parse_irecovery_query(&text);
    if parsed.product.is_none() {
        parsed.product = infer_iphone_product_from_blob(&text);
    }
    if parsed.product.is_none() {
        if let (Some(cpid), Some(bdid)) = (parsed.cpid, parsed.bdid) {
            parsed.product = iphone_product_type_from_cpid_bdid(cpid, bdid).map(str::to_string);
        }
    }
    let has_identity = parsed.serial.as_ref().map(|s| !s.is_empty()).unwrap_or(false)
        || parsed
            .product
            .as_ref()
            .map(|p| !p.trim().is_empty())
            .unwrap_or(false)
        || parsed.mode.as_ref().map(|m| !m.trim().is_empty()).unwrap_or(false)
        || parsed.ecid.as_ref().map(|e| !e.trim().is_empty()).unwrap_or(false)
        || parsed.imei.as_ref().map(|i| !i.trim().is_empty()).unwrap_or(false)
        || (parsed.cpid.is_some() && parsed.bdid.is_some());
    if !has_identity {
        return None;
    }
    let product = parsed.product.clone();
    let marketing_name = product
        .as_deref()
        .and_then(|p| {
            let m = marketing_lookup(p);
            if !m.is_empty() {
                Some(m.to_string())
            } else {
                None
            }
        })
        .or_else(|| {
            parsed
                .irecovery_display_name
                .as_ref()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        })
        .or_else(|| {
            product
                .as_ref()
                .map(|p| p.trim().to_string())
                .filter(|s| !s.is_empty())
        });
    Some(IphoneUsbStatus {
        phase: "recovery".into(),
        detail: String::new(),
        udids: vec![],
        marketing_name,
        product_type: product,
        ios_version: parsed.mode,
        recovery_serial: parsed.serial,
        recovery_imei: parsed.imei,
        recovery_ecid: parsed.ecid,
    })
}

fn concat_cmd_output(out: &std::process::Output) -> String {
    format!(
        "{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
    .trim()
    .to_string()
}

fn irecovery_output_looks_failed(text: &str) -> bool {
    let t = text.to_lowercase();
    t.contains("error:")
        || t.contains("error ")
        || t.contains("could not")
        || t.contains("unsupported")
        || t.contains("failed")
}

pub fn exit_recovery_boot() -> Result<String, String> {
    try_init_bundled_neighbor_if_unset();
    let exe = resolved_libimobile_tool("irecovery");
    if !exe.is_file() {
        return Err("irecovery introuvable (mÃªme dossier que idevice_id).".into());
    }

    let timeout_normal = Duration::from_secs(12);
    let mut last_err = String::new();

    // 1) ChaÃ®ne explicite (souvent plus fiable quâ€™un seul `-n` si le processus quitte avant la fin du reboot USB)
    let env_chain = ["setenv auto-boot true", "saveenv", "reboot"];
    let mut chain_ok = true;
    for (i, cmd) in env_chain.iter().enumerate() {
        if i > 0 {
            std::thread::sleep(Duration::from_millis(280));
        }
        let mut c = command_for_tool(&exe);
        c.arg("-c").arg(*cmd);
        match command_output_with_timeout(c, timeout_normal) {
            Ok(out) => {
                let merged = concat_cmd_output(&out);
                if !out.status.success() || irecovery_output_looks_failed(&merged) {
                    chain_ok = false;
                    last_err = if merged.is_empty() {
                        format!("irecovery -c {cmd} : code {}", out.status)
                    } else {
                        clamp_usb_detail(&merged, 400)
                    };
                    break;
                }
            }
            Err(e) => {
                chain_ok = false;
                last_err = e.to_string();
                break;
            }
        }
    }
    if chain_ok {
        return Ok("setenv auto-boot true; saveenv; reboot".into());
    }

    // 2) Option intÃ©grÃ©e upstream
    std::thread::sleep(Duration::from_millis(200));
    {
        let mut c = command_for_tool(&exe);
        c.arg("-n");
        match command_output_with_timeout(c, timeout_normal) {
            Ok(out) => {
                let merged = concat_cmd_output(&out);
                if out.status.success() && !irecovery_output_looks_failed(&merged) {
                    return Ok("irecovery -n".into());
                }
                if !merged.is_empty() {
                    last_err = clamp_usb_detail(&merged, 400);
                }
            }
            Err(e) => last_err = e.to_string(),
        }
    }

    Err(if last_err.is_empty() {
        "Impossible de quitter le mode recovery (irecovery). RÃ©essaie, change de port USB ou redÃ©marre lâ€™iPhone Ã  la main."
            .into()
    } else {
        last_err
    })
}

pub fn detect_iphone_usb() -> IphoneUsbStatus {
    try_init_bundled_neighbor_if_unset();

    let exe_id = resolved_libimobile_tool("idevice_id");
    let exe_info = resolved_libimobile_tool("ideviceinfo");

    let out = match run_idevice_id_list(&exe_id) {
        Ok(o) => o,
        Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {
            let (product_any, ios_any, detail_any) = idevice_probe_without_udid(&exe_info);
            if let Some(p) = product_any {
                return status_from_product_ios(vec![], Some(p), ios_any);
            }
            if looks_like_strict_unplugged(&detail_any) {
                if let Some(st) = try_recovery_status() {
                    return st;
                }
                return unplugged();
            }
            if looks_like_trust_or_lockdown(&detail_any) {
                return usb_status("awaiting_trust", String::new(), vec![], None, None, None);
            }
            let msg = if detail_any.trim().is_empty() {
                "idevice_id -l : dÃ©lai dÃ©passÃ© (USB lent, port occupÃ© ou mux silencieux). RÃ©essaie ou change de cÃ¢ble/port.".to_string()
            } else {
                format!("idevice_id -l : dÃ©lai. {}", clamp_usb_detail(&detail_any, 380))
            };
            return usb_status("error", msg, vec![], None, None, None);
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            #[cfg(windows)]
            let hint = format!(
                "Impossible de lancer idevice_id (chemin rÃ©solu : {}). Installe les outils, ou npm run sync:libimobiledevice puis rebuild, ou PANICBASE_IDEVICE_DIR.",
                exe_id.display()
            );
            #[cfg(not(windows))]
            let hint = format!("Impossible de lancer idevice_id ({}).", exe_id.display());
            return usb_status("no_tools", hint, vec![], None, None, None);
        }
        Err(e) => {
            return usb_status(
                "error",
                format!("Impossible de lancer idevice_id : {e}"),
                vec![],
                None,
                None,
                None,
            );
        }
    };

    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        if looks_like_trust_or_lockdown(&stderr) {
            return usb_status("awaiting_trust", String::new(), vec![], None, None, None);
        }
        let err_detail = if stderr.is_empty() {
            format!("idevice_id a Ã©chouÃ© ({})", out.status)
        } else {
            stderr
        };
        return usb_status("error", err_detail, vec![], None, None, None);
    }

    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr_id = String::from_utf8_lossy(&out.stderr);
    // Sous Windows certains builds Ã©crivent la liste des UDID sur stderr (stdout vide â†’ faux Â« rien Â»).
    let udids = parse_udids(&format!("{}\n{}", stdout.as_ref(), stderr_id.as_ref()));

    if udids.is_empty() {
        // Sortie vide + succÃ¨s : souvent Â« rien sur lâ€™USB Â» sans message explicite â€” Ã©vite ideviceinfo (plus lent).
        let clean_empty_success = out.status.success()
            && stdout.trim().is_empty()
            && stderr_id.trim().is_empty();
        if looks_like_strict_unplugged(&stderr_id) || clean_empty_success {
            if let Some(st) = try_recovery_status() {
                return st;
            }
            return unplugged();
        }
        let (product_any, ios_any, detail_any) = idevice_probe_without_udid(&exe_info);
        if let Some(product_type_val) = product_any {
            let mapped = marketing_lookup(&product_type_val);
            let mapped_opt = if mapped.is_empty() {
                None
            } else {
                Some(mapped.to_string())
            };
            return usb_status(
                "connected",
                String::new(),
                vec![],
                mapped_opt,
                Some(product_type_val),
                ios_any,
            );
        }
        let combined = format!("{detail_any} {stderr_id}");
        // DÃ©branchement : prioritÃ© au Â« rien sur le bus Â» avant les messages qui ressemblent Ã  lockdown.
        if looks_like_strict_unplugged(&combined) {
            if let Some(st) = try_recovery_status() {
                return st;
            }
            return unplugged();
        }
        if looks_like_trust_or_lockdown(&detail_any) || looks_like_trust_or_lockdown(&stderr_id) {
            return usb_status("awaiting_trust", String::new(), vec![], None, None, None);
        }
        let trimmed_detail = detail_any.trim();
        let trimmed_err = stderr_id.trim();
        if !trimmed_detail.is_empty() || !trimmed_err.is_empty() {
            let merged = format!("{trimmed_detail} {trimmed_err}");
            if let Some(st) = try_recovery_status() {
                return st;
            }
            return usb_status(
                "error",
                format!(
                    "idevice_id -l nâ€™a renvoyÃ© aucun UDID. {}",
                    clamp_usb_detail(&merged, 400)
                ),
                vec![],
                None,
                None,
                None,
            );
        }
        // idevice_id OK mais liste vide + aucun message : souvent mux / pilote Apple, pas Â« cÃ¢ble absent Â».
        let exe_abs = exe_id.is_absolute();
        let detail = if exe_abs {
            format!(
                "idevice_id returned no UDID from {}. Check USB connection, iPhone trust prompt, and PANICBASE_IDEVICE_DIR.",
                exe_id.parent().map(|p| p.display().to_string()).unwrap_or_default()
            )
        } else {
            "idevice_id ne renvoie aucun UDID. PanicBase utilise un binaire du PATH : installe les outils libimobiledevice ou dÃ©finis PANICBASE_IDEVICE_DIR / npm run sync:libimobiledevice puis rebuild."
                .into()
        };
        if let Some(st) = try_recovery_status() {
            return st;
        }
        return usb_status("unplugged", detail, vec![], None, None, None);
    }

    finish_with_udid_list(&exe_info, udids)
}

fn finish_with_udid_list(exe_info: &Path, udids: Vec<String>) -> IphoneUsbStatus {
    let first_udid = udids[0].as_str();
    let product = idevice_read_key(exe_info, Some(first_udid), "ProductType");
    let ios = idevice_read_key(exe_info, Some(first_udid), "ProductVersion");

    if product.is_none() {
        let (p2, i2, detail) = idevice_probe_without_udid(exe_info);
        if p2.is_some() {
            return status_from_product_ios(udids, p2, i2);
        }
        if looks_like_strict_unplugged(&detail) {
            if let Some(st) = try_recovery_status() {
                return st;
            }
            return unplugged();
        }
        // UDID fantÃ´me : le bus a parfois encore lâ€™ancienne entrÃ©e une fraction de seconde aprÃ¨s dÃ©branchement.
        let exe_id = resolved_libimobile_tool("idevice_id");
        if let Ok(out2) = run_idevice_id_list(&exe_id) {
            let stdout2 = String::from_utf8_lossy(&out2.stdout);
            let stderr2 = String::from_utf8_lossy(&out2.stderr);
            let udids2 = parse_udids(&format!("{stdout2}\n{stderr2}"));
            if udids2.is_empty() {
                if let Some(st) = try_recovery_status() {
                    return st;
                }
                return unplugged();
            }
            let tail = format!("{detail} {}", stderr2.trim());
            if looks_like_strict_unplugged(&tail) {
                if let Some(st) = try_recovery_status() {
                    return st;
                }
                return unplugged();
            }
        }
        return usb_status("awaiting_trust", String::new(), udids, None, None, None);
    }

    status_from_product_ios(udids, product, ios)
}

fn value_after_plist_key(xml: &str, key: &str) -> Option<String> {
    let marker = format!("<key>{key}</key>");
    let tail = xml.split(&marker).nth(1)?.trim_start();
    if let Some(r) = tail.strip_prefix("<integer>") {
        return r
            .split("</integer>")
            .next()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
    }
    if let Some(r) = tail.strip_prefix("<string>") {
        return r
            .split("</string>")
            .next()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
    }
    if tail.starts_with("<true/>") || tail.starts_with("<true />") {
        return Some("true".into());
    }
    if tail.starts_with("<false/>") || tail.starts_with("<false />") {
        return Some("false".into());
    }
    None
}

fn battery_domain_summary(tool: &Path, udid: Option<&str>) -> Option<String> {
    let mut c = command_for_tool(tool);
    if let Some(u) = udid {
        if !u.trim().is_empty() {
            c.arg("-u").arg(u.trim());
        }
    }
    c.arg("-q").arg("com.apple.mobile.battery");
    let out = command_output_with_timeout(c, Duration::from_secs(12)).ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout);
    if s.trim().is_empty() {
        return None;
    }
    let mut parts: Vec<String> = Vec::new();
    for k in [
        "BatteryCurrentCapacity",
        "BatteryIsCharging",
        "FullyCharged",
        "ExternalConnected",
        "ExternalChargeCapable",
        "NominalChargeCapacity",
        "AppleRawMaxCapacity",
        "MaxCapacity",
        "DesignCapacity",
        "CycleCount",
        "MaximumCapacityPercent",
    ] {
        if let Some(v) = value_after_plist_key(&s, k) {
            parts.push(format!("{k}={v}"));
        }
    }
    if !parts.is_empty() {
        return Some(parts.join(" Â· "));
    }
    let t: String = s.chars().filter(|c| !c.is_control()).take(600).collect();
    if t.len() > 20 {
        Some(format!("(extrait domaine) {}", t.trim()))
    } else {
        None
    }
}

fn ideviceinfo_xml_full(tool: &Path, udid: Option<&str>) -> Option<String> {
    let mut c = command_for_tool(tool);
    if let Some(u) = udid {
        if !u.trim().is_empty() {
            c.arg("-u").arg(u.trim());
        }
    }
    c.arg("-x");
    let out = command_output_with_timeout(c, TIMEOUT_IDE_FULL_XML).ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).into_owned();
    if s.contains("<plist") {
        Some(s)
    } else {
        None
    }
}

fn ideviceinfo_domain_xml(tool: &Path, udid: Option<&str>, domain: &str, timeout: Duration) -> Option<String> {
    let mut c = command_for_tool(tool);
    if let Some(u) = udid {
        if !u.trim().is_empty() {
            c.arg("-u").arg(u.trim());
        }
    }
    c.arg("-q").arg(domain);
    c.arg("-x");
    let out = command_output_with_timeout(c, timeout).ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).into_owned();
    if s.contains("<plist") {
        Some(s)
    } else {
        None
    }
}

fn merge_plist_domain_filling_empty(map: &mut std::collections::BTreeMap<String, String>, xml: &str) {
    for f in parse_lockdown_root_plist_xml(xml) {
        let v_trim = f.value.trim();
        if v_trim.is_empty() || v_trim == "(objet)" || v_trim == "(liste)" {
            continue;
        }
        let insert = match map.get(&f.id) {
            None => true,
            Some(cur) => cur.trim().is_empty(),
        };
        if insert {
            map.insert(f.id, f.value);
        }
    }
}

const TIMEOUT_DOMAIN_XML: Duration = Duration::from_secs(14);
const TIMEOUT_IDEVICE_DIAG: Duration = Duration::from_secs(35);

pub fn idevice_diagnostics_action(udid: Option<&str>, action: &str) -> Result<(), String> {
    try_init_bundled_neighbor_if_unset();
    let exe = resolved_libimobile_tool("idevicediagnostics");
    if !exe.is_file() {
        return Err(
            "idevicediagnostics introuvable : copie le dossier win-x64 complet (ideviceinfo + idevicediagnostics + DLL) ou npm run sync:libimobiledevice."
                .into(),
        );
    }
    let mut c = command_for_tool(&exe);
    if let Some(u) = udid {
        let u = u.trim();
        if !u.is_empty() {
            c.arg("-u").arg(u);
        }
    }
    c.arg(action);
    let out = command_output_with_timeout(c, TIMEOUT_IDEVICE_DIAG).map_err(|e| e.to_string())?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        let stdout = String::from_utf8_lossy(&out.stdout);
        return Err(format!(
            "idevicediagnostics {} : {} {}",
            action,
            stderr.trim(),
            stdout.trim()
        ));
    }
    Ok(())
}

fn consume_xml_tag_block(s: &str, open: &str, close: &str) -> Option<usize> {
    if !s.starts_with(open) {
        return None;
    }
    let mut depth = 1usize;
    let mut i = open.len();
    while i < s.len() && depth > 0 {
        if s[i..].starts_with(open) {
            depth += 1;
            i += open.len();
        } else if s[i..].starts_with(close) {
            depth -= 1;
            if depth == 0 {
                return Some(i + close.len());
            }
            i += close.len();
        } else {
            i += 1;
        }
    }
    None
}

fn read_plist_value_payload(tail: &str) -> Option<(String, usize)> {
    let t = tail.trim_start();
    let pad = tail.len() - t.len();
    if t.starts_with("<string>") {
        let rest = &t[8..];
        let e = rest.find("</string>")?;
        return Some((rest[..e].to_string(), pad + 8 + e + 9));
    }
    if t.starts_with("<integer>") {
        let rest = &t[9..];
        let e = rest.find("</integer>")?;
        return Some((rest[..e].to_string(), pad + 9 + e + 10));
    }
    if t.starts_with("<real>") {
        let rest = &t[6..];
        let e = rest.find("</real>")?;
        return Some((rest[..e].to_string(), pad + 6 + e + 7));
    }
    if t.starts_with("<date>") {
        let rest = &t[6..];
        let e = rest.find("</date>")?;
        return Some((rest[..e].to_string(), pad + 6 + e + 7));
    }
    if t.starts_with("<true/>") || t.starts_with("<true />") {
        let end = t.find('>')? + 1;
        return Some(("true".into(), pad + end));
    }
    if t.starts_with("<false/>") || t.starts_with("<false />") {
        let end = t.find('>')? + 1;
        return Some(("false".into(), pad + end));
    }
    if t.starts_with("<data>") {
        let rest = &t[6..];
        let e = rest.find("</data>")?;
        let raw = rest[..e].chars().filter(|c| !c.is_whitespace()).take(96).collect::<String>();
        let val = if rest[..e].chars().filter(|c| !c.is_whitespace()).count() > raw.len() {
            format!("{raw}â€¦")
    } else {
            raw
        };
        return Some((val, pad + 6 + e + 7));
    }
    if t.starts_with("<dict>") {
        let n = consume_xml_tag_block(t, "<dict>", "</dict>")?;
        return Some(("(objet)".into(), pad + n));
    }
    if t.starts_with("<array>") {
        let n = consume_xml_tag_block(t, "<array>", "</array>")?;
        return Some(("(liste)".into(), pad + n));
    }
    None
}

fn parse_lockdown_root_plist_xml(xml: &str) -> Vec<DeviceInfoField> {
    let Some(dict_start) = xml.find("<dict>") else {
        return Vec::new();
    };
    let mut i = dict_start + 6;
    let mut out: Vec<DeviceInfoField> = Vec::new();
    while i < xml.len() {
        while i < xml.len() && xml.as_bytes()[i].is_ascii_whitespace() {
            i += 1;
        }
        if xml[i..].starts_with("</dict>") {
            break;
        }
        if !xml[i..].starts_with("<key>") {
            // Avance au prochain marqueur connu pour Ã©viter de rester bloquÃ©.
            let rest = &xml[i..];
            let skip = rest
                .find("<key>")
                .or_else(|| rest.find("</dict>"))
                .unwrap_or(rest.len());
            i += skip;
            continue;
        }
        i += 5;
        let Some(endk) = xml[i..].find("</key>") else {
            break;
        };
        let key = xml[i..i + endk].trim().to_string();
        i += endk + 6;
        let tail = &xml[i..];
        let Some((value, consumed)) = read_plist_value_payload(tail) else {
            break;
        };
        i += consumed;
        if !key.is_empty() {
            out.push(DeviceInfoField { id: key, value });
        }
    }
    out
}

fn fetch_fields_by_keys(exe: &Path, udid_ref: Option<&str>, keys: &[&str]) -> Vec<DeviceInfoField> {
    let mut fields: Vec<DeviceInfoField> = Vec::new();
    for key in keys {
        if let Some(v) = idevice_read_key_timeout(exe, udid_ref, key, TIMEOUT_IDE_KEY_BULK) {
            let val = v.trim();
            if val.is_empty() {
                continue;
            }
            fields.push(DeviceInfoField {
                id: (*key).to_string(),
                value: val.to_string(),
            });
        }
    }
    fields
}

fn fetch_fields_by_keys_timeout(
    exe: &Path,
    udid_ref: Option<&str>,
    keys: &[&str],
    per_key: Duration,
) -> Vec<DeviceInfoField> {
    let mut fields: Vec<DeviceInfoField> = Vec::new();
    for key in keys {
        if let Some(v) = idevice_read_key_timeout(exe, udid_ref, key, per_key) {
            let val = v.trim();
            if val.is_empty() {
                continue;
            }
            fields.push(DeviceInfoField {
                id: (*key).to_string(),
                value: val.to_string(),
            });
        }
    }
    fields
}

pub fn fetch_iphone_device_identifiers(udid: Option<String>) -> Result<IphoneDeviceDetails, String> {
    try_init_bundled_neighbor_if_unset();
    let exe = resolved_libimobile_tool("ideviceinfo");
    let udid_ref = udid.as_deref().map(str::trim).filter(|s| !s.is_empty());

    const KEYS: &[&str] = &[
        "SerialNumber",
        "UniqueDeviceID",
        "InternationalMobileEquipmentIdentity",
        "InternationalMobileEquipmentIdentity2",
        "IMEI",
        "MobileEquipmentIdentifier",
    ];

    let fields = fetch_fields_by_keys_timeout(&exe, udid_ref, KEYS, TIMEOUT_IDE_KEY_IDENT);

    if fields.is_empty() {
        return Err(
            "Aucune donnÃ©e lockdown (ideviceinfo). DÃ©verrouille lâ€™iPhone, accepte Â« Faire confiance Â», ou vÃ©rifie les outils USB / PANICBASE_IDEVICE_DIR."
                .into(),
        );
    }

    let has_substantial_imei = fields.iter().any(|f| {
        (f.id == "InternationalMobileEquipmentIdentity"
            || f.id == "IMEI"
            || f.id == "MobileEquipmentIdentifier")
            && f.value.chars().filter(|c| c.is_ascii_digit()).count() >= 8
            && f.value != "000000000000000"
    });
    let hint = if has_substantial_imei {
        None
    } else {
        Some(
            "Lâ€™IMEI peut Ãªtre absent selon iOS / confiance lockdown. SN + UDID restent les repÃ¨res atelier les plus fiables ici."
                .into(),
        )
    };

    Ok(IphoneDeviceDetails {
        fields,
        udid: udid_ref.map(|s| s.to_string()),
        hint,
    })
}

pub fn fetch_iphone_device_details(udid: Option<String>) -> Result<IphoneDeviceDetails, String> {
    try_init_bundled_neighbor_if_unset();
    let exe = resolved_libimobile_tool("ideviceinfo");
    let udid_ref = udid.as_deref().map(str::trim).filter(|s| !s.is_empty());

    const KEYS: &[&str] = &[
        "DeviceName",
        "SerialNumber",
        "UniqueDeviceID",
        "InternationalMobileEquipmentIdentity",
        "MobileEquipmentIdentifier",
        "IMEI",
        "ProductType",
        "ProductVersion",
        "BuildVersion",
        "HardwareModel",
        "ModelNumber",
        "CPUArchitecture",
        "ActivationState",
        "PasswordProtected",
        "BasebandVersion",
        "BasebandStatus",
        "SIMStatus",
        "WiFiAddress",
        "BluetoothAddress",
        "EthernetAddress",
        "MLBSerialNumber",
        "RegionInfo",
        "TimeZone",
        "TotalDiskCapacity",
        "AmountDataAvailable",
        "TotalDataCapacity",
        "TotalDataAvailable",
        "TotalSystemCapacity",
        "TotalSystemAvailable",
        "BatteryCurrentCapacity",
        "BatteryIsCharging",
        "ExternalConnected",
        "FullyCharged",
        "GasGaugeCapability",
    ];

    let mut map: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();

    if let Some(xml) = ideviceinfo_xml_full(&exe, udid_ref) {
        for f in parse_lockdown_root_plist_xml(&xml) {
            map.insert(f.id, f.value);
        }
    }

    if map.is_empty() {
        for f in fetch_fields_by_keys(&exe, udid_ref, KEYS) {
            map.insert(f.id, f.value);
        }
    }

    if let Some(x) = ideviceinfo_domain_xml(&exe, udid_ref, "com.apple.disk_usage", TIMEOUT_DOMAIN_XML) {
        merge_plist_domain_filling_empty(&mut map, &x);
    }
    if let Some(x) = ideviceinfo_domain_xml(&exe, udid_ref, "com.apple.mobile.battery", TIMEOUT_DOMAIN_XML) {
        merge_plist_domain_filling_empty(&mut map, &x);
    }

    if let Some(b) = battery_domain_summary(&exe, udid_ref) {
        map.entry("BatteryDomainSummary".into()).or_insert(b);
    }

    let fields: Vec<DeviceInfoField> = map
        .into_iter()
        .map(|(id, value)| DeviceInfoField { id, value })
        .collect();

    if fields.is_empty() {
        return Err(
            "Aucune donnÃ©e lockdown (ideviceinfo). DÃ©verrouille lâ€™iPhone, accepte Â« Faire confiance Â», ou vÃ©rifie les outils USB / PANICBASE_IDEVICE_DIR."
                .into(),
        );
    }

    let has_substantial_imei = fields.iter().any(|f| {
        (f.id == "InternationalMobileEquipmentIdentity"
            || f.id == "IMEI"
            || f.id == "MobileEquipmentIdentifier")
            && f.value.chars().filter(|c| c.is_ascii_digit()).count() >= 8
            && f.value != "000000000000000"
    });
    let hint = if has_substantial_imei {
        None
    } else {
        Some(
            "Lâ€™IMEI peut Ãªtre absent selon iOS / confiance lockdown. SN + UDID restent les repÃ¨res atelier les plus fiables ici."
                .into(),
        )
    };

    Ok(IphoneDeviceDetails {
        fields,
        udid: udid_ref.map(|s| s.to_string()),
        hint,
    })
}

// â”€â”€ Disk usage â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskUsage {
    pub total_bytes: u64,
    pub used_bytes:  u64,
    pub free_bytes:  u64,
    pub data_total_bytes: u64,
    pub data_free_bytes:  u64,
}

pub fn get_disk_usage(udid: Option<&str>) -> Result<DiskUsage, String> {
    let exe = resolved_libimobile_tool("ideviceinfo");
    if !exe.is_file() {
        return Err("ideviceinfo introuvable â€” vÃ©rifie le bundle libimobiledevice".into());
    }
    let xml = ideviceinfo_domain_xml(&exe, udid, "com.apple.disk_usage", TIMEOUT_DOMAIN_XML)
        .ok_or_else(|| "Domaine com.apple.disk_usage indisponible (dÃ©verrouille l'iPhone et accepte Â« Faire confiance Â»)".to_string())?;

    let total = value_after_plist_key(&xml, "TotalDiskCapacity")
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);
    let sys_total = value_after_plist_key(&xml, "TotalSystemCapacity")
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);
    let sys_free = value_after_plist_key(&xml, "TotalSystemAvailable")
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);
    let data_total = value_after_plist_key(&xml, "TotalDataCapacity")
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);
    let data_free = value_after_plist_key(&xml, "TotalDataAvailable")
        .or_else(|| value_after_plist_key(&xml, "AmountDataAvailable"))
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);

    // Le "free" pertinent cÃ´tÃ© utilisateur, c'est celui de la partition Data.
    let total_eff = if total > 0 { total } else { sys_total + data_total };
    let free_eff  = data_free;
    let used_eff  = total_eff.saturating_sub(free_eff + sys_free.min(total_eff.saturating_sub(free_eff)));

    Ok(DiskUsage {
        total_bytes: total_eff,
        used_bytes:  used_eff,
        free_bytes:  free_eff,
        data_total_bytes: data_total,
        data_free_bytes:  data_free,
    })
}

fn count_crash_like_files(root: &Path) -> usize {
    let mut n = 0;
    let Ok(rd) = std::fs::read_dir(root) else {
        return 0;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            n += count_crash_like_files(&p);
            continue;
        }
        if let Some(ext) = p.extension() {
            if ext.eq_ignore_ascii_case("ips") || ext.eq_ignore_ascii_case("crash") {
                n += 1;
            }
        }
    }
    n
}

const IDEVICE_CRASHREPORT_TIMEOUT: Duration = Duration::from_secs(120);

pub fn extract_panic_logs() -> Result<String, String> {
    let exe = resolved_libimobile_tool("idevicecrashreport");
    let out = database::ensure_crash_reports_dir()?;
    let out_str = out.to_string_lossy().to_string();

    let mut crash_cmd = command_for_tool(&exe);
    crash_cmd.arg(&out_str);
    let output = command_output_with_timeout(crash_cmd, IDEVICE_CRASHREPORT_TIMEOUT).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            return "idevicecrashreport introuvable â€” mÃªme dossier quâ€™idevice_id.".to_string();
        }
        if e.kind() == std::io::ErrorKind::TimedOut {
            return "idevicecrashreport : dÃ©lai dÃ©passÃ© (USB lent ou appareil occupÃ©). RÃ©essaie.".to_string();
        }
        e.to_string()
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!(
            "idevicecrashreport ({}) stdout: {} stderr: {}",
            output.status,
            stdout.trim(),
            stderr.trim()
        ));
    }

    let n = count_crash_like_files(&out);
    Ok(format!(
        "Extraction terminÃ©e dans Â« {} Â» Â· {} fichier(s) .ips / .crash visibles.",
        out_str, n
    ))
}

#[cfg(test)]
mod usb_phase_detection_tests {
    use super::looks_like_strict_unplugged;
    use super::looks_like_trust_or_lockdown;

    #[test]
    fn trust_detection_not_fired_on_bare_lockdownd_word() {
        assert!(!looks_like_trust_or_lockdown(
            "some random lockdownd noise on disconnect"
        ));
        assert!(looks_like_trust_or_lockdown(
            "Could not connect to lockdownd, error -2"
        ));
    }

    #[test]
    fn unplug_messages_take_strict_path() {
        assert!(looks_like_strict_unplugged("No device found"));
    }
}

#[cfg(test)]
mod device_info_plist_tests {
    use super::*;

    #[test]
    fn parse_flat_lockdown_dict() {
        let xml = "<?xml version=\"1.0\"?><plist version=\"1.0\"><dict><key>ActivationState</key><string>Activated</string><key>ChipID</key><integer>33104</integer><key>BrickState</key><false/></dict></plist>";
        let v = parse_lockdown_root_plist_xml(xml);
        assert!(v.iter().any(|f| f.id == "ActivationState" && f.value == "Activated"));
        assert!(v.iter().any(|f| f.id == "ChipID" && f.value == "33104"));
        assert!(v.iter().any(|f| f.id == "BrickState" && f.value == "false"));
    }

    #[test]
    fn parse_nested_dict_placeholder() {
        let xml = "<plist><dict><key>Outer</key><dict><key>Inner</key><string>x</string></dict></dict></plist>";
        let v = parse_lockdown_root_plist_xml(xml);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].id, "Outer");
        assert_eq!(v[0].value, "(objet)");
    }
}

#[cfg(test)]
mod irecovery_parse_tests {
    use super::*;

    #[test]
    fn cpid_bdid_resolves_iphone_17_pro() {
        assert_eq!(
            iphone_product_type_from_cpid_bdid(0x8150, 0x0c),
            Some("iPhone18,1")
        );
    }

    #[test]
    fn parse_irecovery_cpid_bdid_case_insensitive_keys() {
        let blob = "CPID: 0x8150\nbdid: 0x0c\nMODE: Recovery\n";
        let p = parse_irecovery_query(blob);
        assert_eq!(p.cpid, Some(0x8150));
        assert_eq!(p.bdid, Some(0x0c));
        assert_eq!(p.mode.as_deref(), Some("Recovery"));
    }

    #[test]
    fn parse_irecovery_product_name_when_present() {
        let blob = "PRODUCT: iPhone18,1\nNAME: iPhone 17 Pro\nCPID: 0x8150\n";
        let p = parse_irecovery_query(blob);
        assert_eq!(p.product.as_deref(), Some("iPhone18,1"));
        assert_eq!(p.irecovery_display_name.as_deref(), Some("iPhone 17 Pro"));
    }
}
