import { execSync } from 'node:child_process';

const env = { ...process.env, PANICBASE_DEV_PLAIN_BUILD: '1', PANICBASE_COMMUNITY_BUILD: '0' };
console.log('[PanicBase] Build production sans obfuscation JS…');
execSync('npx tauri build', { stdio: 'inherit', env });
