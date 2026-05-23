/**
 * Extrait les blocs <div class="section" id="section-…"> depuis
 * public/iphone_panic_reference_enriched.html → src/generated/panicReferenceSections.json
 */
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const root = path.join(__dirname, '..');
const srcHtml = path.join(root, 'public', 'iphone_panic_reference_enriched.html');
const outJson = path.join(root, 'src', 'generated', 'panicReferenceSections.json');

function extractSections(html) {
  const marker = '<div class="section';
  const sections = [];
  let pos = 0;

  while (true) {
    const start = html.indexOf(marker, pos);
    if (start === -1) break;

    const afterOpen = start + marker.length;
    const next = html[afterOpen];
    // Exclure section-header, etc. (seul « section » ou « section … » est valide)
    if (next !== '"' && next !== ' ') {
      pos = start + marker.length;
      continue;
    }

    const idStart = html.indexOf('id="', start) + 4;
    const idEnd = html.indexOf('"', idStart);
    const fullId = html.slice(idStart, idEnd);
    const openTagEnd = html.indexOf('>', idEnd) + 1;

    let depth = 1;
    let j = openTagEnd;
    let sliceEnd = -1;

    while (depth > 0 && j < html.length) {
      const nextOpen = html.indexOf('<div', j);
      const nextClose = html.indexOf('</div>', j);

      if (nextClose === -1) throw new Error('Unclosed div in section ' + fullId);

      if (nextOpen !== -1 && nextOpen < nextClose) {
        depth += 1;
        j = nextOpen + 4;
      } else {
        depth -= 1;
        if (depth === 0) sliceEnd = nextClose;
        j = nextClose + 6;
      }
    }

    if (sliceEnd === -1) throw new Error('Could not find end for ' + fullId);

    const innerHtml = html.slice(openTagEnd, sliceEnd).trim();

    sections.push({
      htmlId: fullId,
      key: fullId.startsWith('section-') ? fullId.slice('section-'.length) : fullId,
      innerHtml,
    });

    pos = j;
  }

  return sections;
}

const html = fs.readFileSync(srcHtml, 'utf8');
const sections = extractSections(html);
fs.mkdirSync(path.dirname(outJson), { recursive: true });
fs.writeFileSync(outJson, JSON.stringify({ sections }) + '\n', 'utf8');
console.log('Wrote', sections.length, 'sections to', path.relative(root, outJson));
