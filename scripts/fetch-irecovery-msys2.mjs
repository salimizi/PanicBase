/**
 * Télécharge irecovery.exe + DLL depuis les paquets MSYS2 mingw64 (miroir officiel),
 * avec SHA256 épinglés. Fusionne dans src-tauri/resources/libimobiledevice/ (ne supprime pas idevice_*).
 * Windows uniquement. Licences : voir chaque paquet (LGPL/GPL) sur https://packages.msys2.org .
 */
import crypto from 'crypto';
import fs from 'fs';
import path from 'path';
import { spawnSync } from 'child_process';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const root = path.join(__dirname, '..');
const dest = path.join(root, 'src-tauri', 'resources', 'libimobiledevice');
const cacheRoot = path.join(root, 'node_modules', '.cache', 'panicbase-msys2-irecovery');

const MIRROR = 'https://mirror.msys2.org/mingw/mingw64/';

/** Paquets + empreintes (packages.msys2.org, 2026-05). */
const PACKAGES = [
  {
    file: 'mingw-w64-x86_64-libirecovery-1.2.1-1-any.pkg.tar.zst',
    sha256: '7999dbfa4dcc2c202daa96f6061169db1caee04186f5af5020b638c66d5ed551',
  },
  {
    file: 'mingw-w64-x86_64-libimobiledevice-glue-1.3.2-1-any.pkg.tar.zst',
    sha256: 'c9d399acc69aa4d2fbff68d676d248127d4599e3a39f866719e3714c84ebe920',
  },
  {
    file: 'mingw-w64-x86_64-libplist-2.7.0-4-any.pkg.tar.zst',
    sha256: 'c3b19655c25506ce3afaa091ca31f3d7c2e115055d516c96666ad7cd701662f1',
  },
  {
    file: 'mingw-w64-x86_64-readline-8.3.003-1-any.pkg.tar.zst',
    sha256: 'b943d7e2a61ac6e0304eac11cdcee15370a8c2ccee5b84487f8ad6c1e103d66c',
  },
  {
    file: 'mingw-w64-x86_64-termcap-1.3.1-7-any.pkg.tar.zst',
    sha256: '04ca275febc3ef461d55f8468fd05c6355ca0fda3be4a77bfdb906cf9d88a1e8',
  },
];

/** Binaires à copier depuis mingw64/bin/ (arbre d’imports irecovery MSYS2). */
const BIN_NAMES = new Set([
  'irecovery.exe',
  'libirecovery-1.0.dll',
  'libimobiledevice-glue-1.0.dll',
  'libplist-2.0.dll',
  'libreadline8.dll',
  'libtermcap-0.dll',
]);

function sha256File(filePath) {
  const h = crypto.createHash('sha256');
  h.update(fs.readFileSync(filePath));
  return h.digest('hex');
}

async function download(url, outPath) {
  const res = await fetch(url);
  if (!res.ok) {
    throw new Error(`Téléchargement HTTP ${res.status} : ${url}`);
  }
  const buf = Buffer.from(await res.arrayBuffer());
  fs.mkdirSync(path.dirname(outPath), { recursive: true });
  fs.writeFileSync(outPath, buf);
}

function extract(archivePath, outDir) {
  fs.mkdirSync(outDir, { recursive: true });
  const r = spawnSync('tar', ['-xf', archivePath, '-C', outDir], {
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  if (r.status !== 0) {
    throw new Error(`tar a échoué (${r.status}) : ${r.stderr || r.stdout || archivePath}`);
  }
}

async function main() {
  if (process.platform !== 'win32') {
    console.log('fetch-irecovery-msys2 : ignoré (non-Windows).');
    return;
  }

  fs.mkdirSync(dest, { recursive: true });
  fs.mkdirSync(cacheRoot, { recursive: true });

  const binDirSeen = new Map();

  for (const pkg of PACKAGES) {
    const url = MIRROR + pkg.file;
    const archivePath = path.join(cacheRoot, pkg.file);
    const extractDir = path.join(cacheRoot, 'extract', pkg.file.replace(/\.pkg\.tar\.zst$/, ''));

    if (!fs.existsSync(archivePath) || sha256File(archivePath) !== pkg.sha256) {
      console.log('Téléchargement', pkg.file, '…');
      await download(url, archivePath);
      const got = sha256File(archivePath);
      if (got !== pkg.sha256) {
        throw new Error(`SHA256 incorrect pour ${pkg.file} :\n  attendu ${pkg.sha256}\n  reçu   ${got}`);
      }
    }

    if (!fs.existsSync(path.join(extractDir, 'mingw64', 'bin'))) {
      fs.rmSync(extractDir, { recursive: true, force: true });
      extract(archivePath, extractDir);
    }

    const mingwBin = path.join(extractDir, 'mingw64', 'bin');
    if (!fs.existsSync(mingwBin)) {
      throw new Error(`Structure d’archive inattendue (pas mingw64/bin) : ${pkg.file}`);
    }

    for (const name of fs.readdirSync(mingwBin)) {
      if (!BIN_NAMES.has(name)) continue;
      const full = path.join(mingwBin, name);
      if (!fs.statSync(full).isFile()) continue;
      binDirSeen.set(name, full);
    }
  }

  const missing = [...BIN_NAMES].filter((n) => !binDirSeen.has(n));
  if (missing.length) {
    throw new Error(`Fichiers manquants après extraction : ${missing.join(', ')}`);
  }

  for (const name of BIN_NAMES) {
    const from = binDirSeen.get(name);
    const to = path.join(dest, name);
    fs.copyFileSync(from, to);
    console.log('→', to);
  }

  console.log('irecovery + DLL copiés dans resources/libimobiledevice/.');
}

main().catch((e) => {
  console.error(e.message || e);
  process.exit(1);
});
