/**
 * Prépare 1024×1024 pour `tauri icon` à partir de `app-icon-master.png` ou `app-icon-master.jpg`
 * (icône app carrée — recadrage centré si besoin).
 */
import fs from 'fs';
import sharp from 'sharp';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = join(__dirname, '..');
const iconsDir = join(root, 'src-tauri', 'icons');
const output = join(iconsDir, 'icon-square-1024.png');

const candidates = [join(iconsDir, 'app-icon-master.png'), join(iconsDir, 'app-icon-master.jpg')];
const input = candidates.find((p) => fs.existsSync(p));
if (!input) {
  throw new Error('Place l’icône source dans src-tauri/icons/app-icon-master.png ou .jpg');
}

const meta = await sharp(input).metadata();
const w = meta.width ?? 0;
const h = meta.height ?? 0;
if (!w || !h) throw new Error('Could not read image dimensions');

await sharp(input)
  .resize(1024, 1024, { fit: 'cover', position: 'center' })
  .png()
  .toFile(output);

console.log('Wrote', output, { input, source: { w, h } });
