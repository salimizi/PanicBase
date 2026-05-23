/**
 * Extrait `<style>` de public/iphone_panic_reference_enriched.html →
 * styles scopées sous `.panic-ref-root`.
 */
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const root = path.join(__dirname, '..');
const srcHtml = path.join(root, 'public', 'iphone_panic_reference_enriched.html');
const outCss = path.join(root, 'src', 'styles', 'panic-reference-enriched.css');

const html = fs.readFileSync(srcHtml, 'utf8');
const a = html.indexOf('<style>') + 7;
const b = html.indexOf('</style>');
if (a < 7 || b === -1) throw new Error('No <style> block found');

let css = html.slice(a, b).trimEnd();

css = css.replace(/\r\n/g, '\n');
css = css.replace(/^\s*:root\s*\{/m, '.panic-ref-root {');
css = css.replace(/^\s*\*\s*\{[^}]*\}\s*\n/gm, '');
css = css.replace(/^\s*body\s*\{/m, '.panic-ref-root {');

const lines = css.split('\n');
const prefixed = [];
for (const line of lines) {
  const m = line.match(/^(\s+)(.+)$/);
  if (!m) {
    prefixed.push(line);
    continue;
  }
  const ind = m[1];
  const body = m[2];
  const skip =
    /^@import\b/.test(body) ||
    /^\/\//.test(body) ||
    /^\{|^\}/.test(body) ||
    /^\*$/.test(body);
  if (skip) {
    prefixed.push(line);
    continue;
  }
  if (/^@media\b/.test(body)) {
    prefixed.push(ind + body.replace(/\{\s*\./g, '{ .panic-ref-root .'));
    continue;
  }
  if (/^\.panic-ref-root(\s|,|\{|\.)/.test(body)) {
    prefixed.push(line);
    continue;
  }
  if (/^\.([\w_-]+)/.test(body)) {
    prefixed.push(ind + `.panic-ref-root ${body}`);
    continue;
  }
  prefixed.push(line);
}

const finalCss = [...prefixed, ''].join('\n');
fs.mkdirSync(path.dirname(outCss), { recursive: true });
fs.writeFileSync(outCss, finalCss + '\n', 'utf8');
console.log('Wrote', path.relative(root, outCss));
