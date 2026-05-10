# PanicBase V0.1

Analyseur de panic logs iPhone — Windows-first — Tauri + Rust + React.

## Lancer le projet

```bash
npm install
npm run tauri:dev
```

## Build Windows

```bash
npm run tauri:build
```

## Fonctionnalités V0.1

- Interface React propre
- Commande Rust `detect_iphone()` via `idevice_id -l`
- Analyse locale basique des panic logs
- Détection de mots-clés : mic1, mic2, prs0, TG0B, TG0V, ANS2, baseband, thermalmonitord, AppleBCMWLAN
- Signature + hash SHA256
- Anonymisation locale basique
- Aucun upload serveur

## Pré-requis Windows

- Node.js
- Rust
- Visual Studio Build Tools C++
- WebView2 Runtime
- libimobiledevice dans le PATH Windows pour la détection iPhone

## Roadmap prochaine étape

1. Ajouter récupération réelle des panic logs avec `idevicecrashreport`
2. Détecter modèle iPhone + iOS
3. Ajouter SQLite local
4. Ajouter API communautaire opt-in
5. Ajouter confirmation réparation
6. Ajouter scoring communautaire

## Principe sécurité

Le logiciel est gratuit mais pas open-source. Le moteur avancé et la base communautaire doivent rester côté serveur.
