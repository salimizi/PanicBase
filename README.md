<p align="center">
  <img src="logo.png" width="120" alt="PanicBase" />
</p>

# PanicBase

**iPhone panic log analyzer for repair technicians — Windows-first — Tauri 2 + Rust + React**

PanicBase reads iPhone panic logs (via USB or `.ips` / `.panic` file import), diagnoses hardware faults, and generates repair-oriented reports. Designed for independent repair shops and advanced technicians.

---

## Features

- **Panic log analysis** — pattern-matched diagnosis with confidence score, probable cause, and recommended checks
- **USB iPhone detection** — live plug/unplug detection via libimobiledevice, no iTunes required
- **Direct log pull** — extracts panic logs from connected iPhone over USB
- **`.ips` / `.panic` import** — drag-and-drop or file picker for offline analysis
- **Gallery & media transfer** — browse, preview and export photos/videos from iPhone over USB
- **Contacts export** — export contacts to `.vcf` from iPhone backup
- **iCloud Photos** — browse and download iCloud photo library (requires iCloud sign-in via WebView)
- **Encrypted local database** — all stored data encrypted at rest (ChaCha20-Poly1305, key in OS keychain)
- **Anonymization** — IMEI, UDID, Serial stripped before storage
- **Multi-language UI** — English, French, and more

---

## Requirements

| Dependency | Notes |
|---|---|
| Node.js 20+ | |
| Rust stable | `rustup update stable` |
| Visual Studio Build Tools (C++) | Required by Tauri on Windows |
| WebView2 Runtime | Pre-installed on Windows 11 |
| `libimobiledevice` | `idevice_id`, `idevicecrashreport` in PATH or `PANICBASE_IDEVICE_DIR` |

### libimobiledevice setup

PanicBase does **not** bundle libimobiledevice binaries. At least one of these must be available:

- `idevice_id.exe` + DLLs in PATH
- `PANICBASE_IDEVICE_DIR` env var pointing to a folder containing `idevice_id.exe` and its DLLs
- `LIBIMOBILEDEVICE_HOME` env var

Verify: `idevice_id -l` should list a connected device's UDID.

---

## Development

```powershell
npm install
npm run tauri:dev
```

---

## Build

A 32-byte build key is required to seal the embedded knowledge base at compile time.

```powershell
# Generate once and store securely
$env:PANICBASE_KB_KEY = (openssl rand -hex 32)

# Or load from .env.local (gitignored)
Get-Content .env.local | ForEach-Object { $k,$v = $_ -split '=',2; [System.Environment]::SetEnvironmentVariable($k,$v) }

npm run tauri:build
```

> **Keep `PANICBASE_KB_KEY` secret.** It is required to rebuild and to decrypt any previously stored data.

Output installers:
- `src-tauri/target/release/bundle/msi/PanicBase_x64_en-US.msi`
- `src-tauri/target/release/bundle/nsis/PanicBase_x64-setup.exe`

---

## Architecture

| Layer | Technology |
|---|---|
| Frontend | React 18 + TypeScript + Tailwind CSS + DaisyUI (Vite) |
| Backend | Rust (Tauri 2) |
| Storage | SQLite — sensitive fields encrypted (ChaCha20-Poly1305) |
| iPhone USB | libimobiledevice (no network, no iTunes) |
| Knowledge base | Embedded, sealed at build time |

---

## Security

- All sensitive DB fields encrypted at rest (ChaCha20-Poly1305)
- Encryption key stored in Windows Credential Manager / macOS Keychain
- Panic log data anonymized before storage (IMEI, UDID, Serial, paths stripped)
- Knowledge base sealed at build time — key never in source or binary plain text
- CSP enforced on WebView
- No network telemetry

---

## License

All rights reserved — © 2025 PanicBase. Source published for transparency; redistribution and commercial use require written permission.
