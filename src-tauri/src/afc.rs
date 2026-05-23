#![allow(non_camel_case_types)]
#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::path::Path;
use std::sync::OnceLock;

use libloading::{Library, Symbol};

// â”€â”€ Types opaques â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

pub type idevice_t = *mut c_void;
pub type lockdownd_client_t = *mut c_void;
pub type lockdownd_service_descriptor_t = *mut c_void;
pub type afc_client_t = *mut c_void;
pub type np_client_t = *mut c_void;
pub type diagnostics_relay_client_t = *mut c_void;

pub type idevice_error_t = c_int;
pub type lockdownd_error_t = c_int;
pub type afc_error_t = c_int;
pub type np_error_t = c_int;
pub type diagnostics_relay_error_t = c_int;

pub const AFC_E_SUCCESS: afc_error_t = 0;

// Modes d'ouverture AFC (cf. afc.h libimobiledevice)
pub const AFC_FOPEN_RDONLY: u64 = 0x00000001;
pub const AFC_FOPEN_WRONLY: u64 = 0x00000003;
pub const AFC_FOPEN_WR: u64 = 0x00000004;

// â”€â”€ Symboles dynamiques â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

type FnIdeviceNew              = unsafe extern "C" fn(*mut idevice_t, *const c_char) -> idevice_error_t;
type FnIdeviceFree             = unsafe extern "C" fn(idevice_t) -> idevice_error_t;
type FnLockdownNewHandshake    = unsafe extern "C" fn(idevice_t, *mut lockdownd_client_t, *const c_char) -> lockdownd_error_t;
type FnLockdownFree            = unsafe extern "C" fn(lockdownd_client_t) -> lockdownd_error_t;
type FnLockdownStartService    = unsafe extern "C" fn(lockdownd_client_t, *const c_char, *mut lockdownd_service_descriptor_t) -> lockdownd_error_t;
type FnLockdownServiceFree     = unsafe extern "C" fn(lockdownd_service_descriptor_t) -> lockdownd_error_t;
type FnAfcClientNew            = unsafe extern "C" fn(idevice_t, lockdownd_service_descriptor_t, *mut afc_client_t) -> afc_error_t;
type FnAfcClientFree           = unsafe extern "C" fn(afc_client_t) -> afc_error_t;
type FnAfcReadDir              = unsafe extern "C" fn(afc_client_t, *const c_char, *mut *mut *mut c_char) -> afc_error_t;
type FnAfcGetFileInfo          = unsafe extern "C" fn(afc_client_t, *const c_char, *mut *mut *mut c_char) -> afc_error_t;
type FnAfcFileOpen             = unsafe extern "C" fn(afc_client_t, *const c_char, u64, *mut u64) -> afc_error_t;
type FnAfcFileClose            = unsafe extern "C" fn(afc_client_t, u64) -> afc_error_t;
type FnAfcFileRead             = unsafe extern "C" fn(afc_client_t, u64, *mut c_char, u32, *mut u32) -> afc_error_t;
type FnAfcFileWrite            = unsafe extern "C" fn(afc_client_t, u64, *const c_char, u32, *mut u32) -> afc_error_t;
type FnAfcMakeDirectory        = unsafe extern "C" fn(afc_client_t, *const c_char) -> afc_error_t;
type FnAfcRemovePath           = unsafe extern "C" fn(afc_client_t, *const c_char) -> afc_error_t;

struct AfcLib {
    _lib: Library, // garder vivante toute la durÃ©e du programme
    idevice_new:                FnIdeviceNew,
    idevice_free:               FnIdeviceFree,
    lockdownd_client_new_with_handshake: FnLockdownNewHandshake,
    lockdownd_client_free:      FnLockdownFree,
    lockdownd_start_service:    FnLockdownStartService,
    lockdownd_service_descriptor_free: FnLockdownServiceFree,
    afc_client_new:             FnAfcClientNew,
    afc_client_free:            FnAfcClientFree,
    afc_read_directory:         FnAfcReadDir,
    afc_get_file_info:          FnAfcGetFileInfo,
    afc_file_open:              FnAfcFileOpen,
    afc_file_close:             FnAfcFileClose,
    afc_file_read:              FnAfcFileRead,
    afc_file_write:             FnAfcFileWrite,
    afc_make_directory:         FnAfcMakeDirectory,
    afc_remove_path:            FnAfcRemovePath,
}

static LIB: OnceLock<Result<AfcLib, String>> = OnceLock::new();

unsafe fn load_lib_from_dir(dir: &Path) -> Result<AfcLib, String> {
    // Les DLL dÃ©pendantes de imobiledevice.dll (plist, usbmuxd, ssl, etc.) sont
    // dans le mÃªme dossier. En dev Windows, ce dossier n'est pas forcÃ©ment dans
    // la recherche DLL du process Tauri, donc on l'ajoute explicitement.
    if let Some(dir_s) = dir.to_str() {
        let old = std::env::var("PATH").unwrap_or_default();
        let needle = dir_s.to_ascii_lowercase();
        let already = old
            .split(';')
            .any(|p| p.trim_matches('"').eq_ignore_ascii_case(&needle));
        if !already {
            std::env::set_var("PATH", format!("{dir_s};{old}"));
        }
    }

    let candidates = [
        "imobiledevice.dll",
        "libimobiledevice-1.0.dll",
        "imobiledevice-1.0.dll",
    ];
    let mut last_err = String::from("aucune candidate testÃ©e");
    let mut lib_opt: Option<Library> = None;
    for name in &candidates {
        let p = dir.join(name);
        if !p.is_file() { continue; }
        match Library::new(&p) {
            Ok(lib) => { lib_opt = Some(lib); break; }
            Err(e)  => { last_err = format!("{}: {e}", p.display()); }
        }
    }
    let lib = lib_opt.ok_or_else(|| format!("libimobiledevice introuvable dans {} : {}", dir.display(), last_err))?;

    macro_rules! sym {
        ($name:expr) => {{
            let s: Symbol<*const ()> = lib.get($name).map_err(|e| format!("Symbole `{}` introuvable : {e}", std::str::from_utf8($name).unwrap_or("?")))?;
            std::mem::transmute::<*const (), _>(*s)
        }};
    }

    Ok(AfcLib {
        idevice_new:                sym!(b"idevice_new\0"),
        idevice_free:               sym!(b"idevice_free\0"),
        lockdownd_client_new_with_handshake: sym!(b"lockdownd_client_new_with_handshake\0"),
        lockdownd_client_free:      sym!(b"lockdownd_client_free\0"),
        lockdownd_start_service:    sym!(b"lockdownd_start_service\0"),
        lockdownd_service_descriptor_free: sym!(b"lockdownd_service_descriptor_free\0"),
        afc_client_new:             sym!(b"afc_client_new\0"),
        afc_client_free:            sym!(b"afc_client_free\0"),
        afc_read_directory:         sym!(b"afc_read_directory\0"),
        afc_get_file_info:          sym!(b"afc_get_file_info\0"),
        afc_file_open:              sym!(b"afc_file_open\0"),
        afc_file_close:             sym!(b"afc_file_close\0"),
        afc_file_read:              sym!(b"afc_file_read\0"),
        afc_file_write:             sym!(b"afc_file_write\0"),
        afc_make_directory:         sym!(b"afc_make_directory\0"),
        afc_remove_path:            sym!(b"afc_remove_path\0"),
        _lib: lib,
    })
}

fn get_lib() -> Result<&'static AfcLib, String> {
    let r = LIB.get_or_init(|| {
        let dir = crate::iphone::bundled_libimobiledevice_dir()
            .ok_or_else(|| "Dossier libimobiledevice non initialisÃ©".to_string())?;
        unsafe { load_lib_from_dir(&dir) }
    });
    r.as_ref().map_err(|e| e.clone())
}

pub fn afc_available() -> bool {
    get_lib().is_ok()
}

// â”€â”€ Wrappers RAII â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

pub struct AfcSession {
    lib: &'static AfcLib,
    device: idevice_t,
    client: lockdownd_client_t,
    service: lockdownd_service_descriptor_t,
    afc: afc_client_t,
}

// AFC client n'est pas Send/Sync ; on l'exÃ©cute sur le thread Tauri spawn_blocking.
// Le pointeur est valide tant que la session est vivante.

impl AfcSession {
    pub fn open(udid: Option<&str>) -> Result<Self, String> {
        Self::open_service(udid, "com.apple.afc")
    }

    pub fn open_service(udid: Option<&str>, service_name: &str) -> Result<Self, String> {
        let lib = get_lib()?;
        unsafe {
            let mut device: idevice_t = std::ptr::null_mut();
            let udid_c = udid.map(|u| CString::new(u).unwrap());
            let udid_ptr = udid_c.as_ref().map(|c| c.as_ptr()).unwrap_or(std::ptr::null());
            let r = (lib.idevice_new)(&mut device, udid_ptr);
            if r != 0 || device.is_null() {
                return Err(format!("idevice_new Ã©chouÃ© (code {r}). iPhone branchÃ© et dÃ©verrouillÃ© ?"));
            }

            let mut client: lockdownd_client_t = std::ptr::null_mut();
            let label = CString::new("PanicBase").unwrap();
            let r = (lib.lockdownd_client_new_with_handshake)(device, &mut client, label.as_ptr());
            if r != 0 || client.is_null() {
                (lib.idevice_free)(device);
                return Err(format!(
                    "lockdownd handshake Ã©chouÃ© (code {r}). \
                     Avez-vous approuvÃ© Â« Faire confiance Ã  cet ordinateur Â» sur l'iPhone ?"
                ));
            }

            let mut service: lockdownd_service_descriptor_t = std::ptr::null_mut();
            let svc_id = CString::new(service_name)
                .map_err(|e| format!("Nom de service invalide : {e}"))?;
            let r = (lib.lockdownd_start_service)(client, svc_id.as_ptr(), &mut service);
            if r != 0 || service.is_null() {
                (lib.lockdownd_client_free)(client);
                (lib.idevice_free)(device);
                return Err(format!("DÃ©marrage du service Â« {service_name} Â» Ã©chouÃ© (code {r})"));
            }

            let mut afc: afc_client_t = std::ptr::null_mut();
            let r = (lib.afc_client_new)(device, service, &mut afc);
            if r != AFC_E_SUCCESS || afc.is_null() {
                (lib.lockdownd_service_descriptor_free)(service);
                (lib.lockdownd_client_free)(client);
                (lib.idevice_free)(device);
                return Err(format!("afc_client_new Ã©chouÃ© pour Â« {service_name} Â» (code {r})"));
            }

            Ok(AfcSession { lib, device, client, service, afc })
        }
    }

    pub fn read_directory(&self, path: &str) -> Result<Vec<String>, String> {
        unsafe {
            let p = CString::new(path).unwrap();
            let mut list: *mut *mut c_char = std::ptr::null_mut();
            let r = (self.lib.afc_read_directory)(self.afc, p.as_ptr(), &mut list);
            if r != AFC_E_SUCCESS || list.is_null() {
                return Err(format!("afc_read_directory({}) Ã©chouÃ© (code {r})", path));
            }
            let mut out = Vec::new();
            let mut i = 0isize;
            loop {
                let s = *list.offset(i);
                if s.is_null() { break; }
                let name = CStr::from_ptr(s).to_string_lossy().into_owned();
                libc_free(s as *mut c_void);
                if name != "." && name != ".." {
                    out.push(name);
                }
                i += 1;
            }
            libc_free(list as *mut c_void);
            Ok(out)
        }
    }

    pub fn file_info(&self, path: &str) -> Result<FileInfo, String> {
        unsafe {
            let p = CString::new(path).unwrap();
            let mut list: *mut *mut c_char = std::ptr::null_mut();
            let r = (self.lib.afc_get_file_info)(self.afc, p.as_ptr(), &mut list);
            if r != AFC_E_SUCCESS || list.is_null() {
                return Err(format!("afc_get_file_info({}) code {r}", path));
            }
            let mut info = FileInfo::default();
            let mut i = 0isize;
            loop {
                let key_ptr = *list.offset(i);
                if key_ptr.is_null() { break; }
                let val_ptr = *list.offset(i + 1);
                if val_ptr.is_null() {
                    libc_free(key_ptr as *mut c_void);
                    break;
                }
                let key = CStr::from_ptr(key_ptr).to_string_lossy().into_owned();
                let val = CStr::from_ptr(val_ptr).to_string_lossy().into_owned();
                libc_free(key_ptr as *mut c_void);
                libc_free(val_ptr as *mut c_void);
                match key.as_str() {
                    "st_size"      => info.size       = val.parse().unwrap_or(0),
                    "st_ifmt"      => info.ifmt       = val,
                    "st_mtime"     => info.mtime_ns   = val.parse().unwrap_or(0),
                    "st_birthtime" => info.birth_ns   = val.parse().unwrap_or(0),
                    _ => {}
                }
                i += 2;
            }
            libc_free(list as *mut c_void);
            Ok(info)
        }
    }

    pub fn read_file(&self, path: &str, max_bytes: usize) -> Result<Vec<u8>, String> {
        unsafe {
            let p = CString::new(path).unwrap();
            let mut handle: u64 = 0;
            let r = (self.lib.afc_file_open)(self.afc, p.as_ptr(), AFC_FOPEN_RDONLY, &mut handle);
            if r != AFC_E_SUCCESS || handle == 0 {
                return Err(format!("afc_file_open({}) code {r}", path));
            }
            // 512KB de buffer : divise par 8 le nombre d'aller-retours vs 64KB
            let mut out: Vec<u8> = Vec::with_capacity(512 * 1024);
            let mut buf = vec![0i8; 512 * 1024];
            loop {
                let mut got: u32 = 0;
                let r = (self.lib.afc_file_read)(self.afc, handle, buf.as_mut_ptr(), buf.len() as u32, &mut got);
                if r != AFC_E_SUCCESS { break; }
                if got == 0 { break; }
                out.extend_from_slice(std::slice::from_raw_parts(buf.as_ptr() as *const u8, got as usize));
                if max_bytes > 0 && out.len() >= max_bytes {
                    out.truncate(max_bytes);
                    break;
                }
            }
            (self.lib.afc_file_close)(self.afc, handle);
            Ok(out)
        }
    }

    pub fn copy_file_to<W: std::io::Write>(&self, path: &str, out: &mut W) -> Result<usize, String> {
        unsafe {
            let p = CString::new(path).unwrap();
            let mut handle: u64 = 0;
            let r = (self.lib.afc_file_open)(self.afc, p.as_ptr(), AFC_FOPEN_RDONLY, &mut handle);
            if r != AFC_E_SUCCESS || handle == 0 {
                return Err(format!("afc_file_open({}) code {r}", path));
            }
            let mut buf = vec![0i8; 512 * 1024]; // 512KB chunks
            let mut total = 0usize;
            loop {
                let mut got: u32 = 0;
                let r = (self.lib.afc_file_read)(self.afc, handle, buf.as_mut_ptr(), buf.len() as u32, &mut got);
                if r != AFC_E_SUCCESS {
                    (self.lib.afc_file_close)(self.afc, handle);
                    return Err(format!("afc_file_read({}) code {r}", path));
                }
                if got == 0 { break; }
                let slice = std::slice::from_raw_parts(buf.as_ptr() as *const u8, got as usize);
                out.write_all(slice).map_err(|e| format!("write local: {e}"))?;
                total += got as usize;
            }
            (self.lib.afc_file_close)(self.afc, handle);
            Ok(total)
        }
    }

    pub fn make_directory(&self, path: &str) -> Result<(), String> {
        unsafe {
            let p = CString::new(path).unwrap();
            let r = (self.lib.afc_make_directory)(self.afc, p.as_ptr());
            if r == AFC_E_SUCCESS || self.file_info(path).map(|i| i.is_dir()).unwrap_or(false) {
                return Ok(());
            }
            Err(format!("afc_make_directory({}) Ã©chouÃ© (code {r})", path))
        }
    }

    pub fn remove_path(&self, path: &str) -> Result<(), String> {
        unsafe {
            let p = CString::new(path).map_err(|e| e.to_string())?;
            let r = (self.lib.afc_remove_path)(self.afc, p.as_ptr());
            if r == AFC_E_SUCCESS { Ok(()) }
            else { Err(format!("afc_remove_path({}) code {r}", path)) }
        }
    }

    pub fn remove_path_recursive(&self, path: &str) -> Result<(), String> {
        // Essai direct (fichier ou dossier vide)
        if self.remove_path(path).is_ok() { return Ok(()); }
        // C'est un dossier non vide : vider d'abord
        if let Ok(entries) = self.read_directory(path) {
            for entry in entries {
                let child = format!("{path}/{entry}");
                let _ = self.remove_path_recursive(&child);
            }
        }
        self.remove_path(path)
    }

    pub fn write_file(&self, path: &str, data: &[u8]) -> Result<(), String> {
        unsafe {
            let p = CString::new(path).unwrap();
            let mut handle: u64 = 0;
            let mut r = (self.lib.afc_file_open)(self.afc, p.as_ptr(), AFC_FOPEN_WR, &mut handle);
            if r != AFC_E_SUCCESS || handle == 0 {
                r = (self.lib.afc_file_open)(self.afc, p.as_ptr(), AFC_FOPEN_WRONLY, &mut handle);
            }
            if r != AFC_E_SUCCESS || handle == 0 {
                return Err(format!("afc_file_open({}) Ã©criture code {r}", path));
            }

            let mut offset = 0usize;
            while offset < data.len() {
                let chunk_len = (data.len() - offset).min(64 * 1024);
                let mut written: u32 = 0;
                let r = (self.lib.afc_file_write)(
                    self.afc,
                    handle,
                    data[offset..offset + chunk_len].as_ptr() as *const c_char,
                    chunk_len as u32,
                    &mut written,
                );
                if r != AFC_E_SUCCESS {
                    (self.lib.afc_file_close)(self.afc, handle);
                    return Err(format!("afc_file_write({}) code {r}", path));
                }
                if written == 0 {
                    (self.lib.afc_file_close)(self.afc, handle);
                    return Err(format!("afc_file_write({}) n'a rien Ã©crit", path));
                }
                offset += written as usize;
            }
            (self.lib.afc_file_close)(self.afc, handle);
            Ok(())
        }
    }
}

impl Drop for AfcSession {
    fn drop(&mut self) {
        unsafe {
            if !self.afc.is_null()     { (self.lib.afc_client_free)(self.afc); }
            if !self.service.is_null() { (self.lib.lockdownd_service_descriptor_free)(self.service); }
            if !self.client.is_null()  { (self.lib.lockdownd_client_free)(self.client); }
            if !self.device.is_null()  { (self.lib.idevice_free)(self.device); }
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct FileInfo {
    pub size: u64,
    pub ifmt: String,    // "S_IFREG" pour fichier, "S_IFDIR" pour dossier
    pub mtime_ns: i64,
    pub birth_ns: i64,
}

impl FileInfo {
    pub fn is_dir(&self)  -> bool { self.ifmt == "S_IFDIR" }
    pub fn is_file(&self) -> bool { self.ifmt == "S_IFREG" }
}

// â”€â”€ Helpers â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

unsafe fn libc_free(ptr: *mut c_void) {
    if ptr.is_null() { return; }
    // libimobiledevice utilise malloc/free du CRT C â€” sur Windows on appelle le mÃªme free.
    extern "C" { fn free(ptr: *mut c_void); }
    free(ptr);
}
