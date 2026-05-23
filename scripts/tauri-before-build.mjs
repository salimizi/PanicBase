/**
 * Hook Tauri `beforeBuildCommand` : build communauté obfusqué par défaut.
 * Désactiver (dev CI uniquement) : PANICBASE_DEV_PLAIN_BUILD=1
 */
import { execSync } from 'node:child_process';

const plain = process.env.PANICBASE_COMMUNITY_BUILD === '0' || process.env.PANICBASE_DEV_PLAIN_BUILD === '1';
const cmd = plain ? 'npm run build' : 'npm run build:community';

if (!plain && process.env.PANICBASE_COMMUNITY_BUILD !== '1') {
  process.env.PANICBASE_COMMUNITY_BUILD = '1';
}

console.log(
  plain
    ? '[PanicBase] Build production (JS non obfusqué — PANICBASE_DEV_PLAIN_BUILD)'
    : '[PanicBase] Build communauté (obfuscation JS)…',
);
execSync(cmd, { stdio: 'inherit', env: process.env });
