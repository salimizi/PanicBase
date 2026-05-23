/**
 * Installeurs Windows pour la communauté : outils USB + frontend obfusqué + binaire release Rust.
 */
import { execSync } from 'node:child_process';

const env = { ...process.env, PANICBASE_COMMUNITY_BUILD: '1' };

function run(command) {
  execSync(command, { stdio: 'inherit', env });
}

console.log('[PanicBase] Build communauté Windows (obfuscation + LTO)…');
run('npm run sync:libimobiledevice');
run('npm run fetch:irecovery');
run('npx tauri build');
