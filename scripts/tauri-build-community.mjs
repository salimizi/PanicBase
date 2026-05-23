import { execSync } from 'node:child_process';

const env = { ...process.env, PANICBASE_COMMUNITY_BUILD: '1' };
console.log('[PanicBase] Build communauté (obfuscation JS + release Rust)…');
execSync('npx tauri build', { stdio: 'inherit', env });
