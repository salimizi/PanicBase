# Politique de sécurité — PanicBase

## Modèle de sécurité

PanicBase est un outil de diagnostic local. Tout le traitement des panic
logs et des données de l'iPhone se fait **uniquement sur la machine de
l'utilisateur**. Aucune donnée n'est envoyée à un serveur tiers ni à
PanicBase.

### Données traitées localement

| Donnée                          | Stockage              | Chiffrement                  |
| ------------------------------- | --------------------- | ---------------------------- |
| Panic logs anonymisés           | SQLite local          | ChaCha20-Poly1305            |
| Cause probable & explication    | SQLite local          | ChaCha20-Poly1305            |
| Notes techniciens               | SQLite local          | ChaCha20-Poly1305            |
| Modèle / version iOS / date     | SQLite local          | clair (non identifiant)      |
| Signature SHA-256 du panic      | SQLite local          | clair (hash uniquement)      |
| Cookies de session iCloud       | WebView2 store du sys.| protégé par DPAPI Windows    |

### Clé de chiffrement

La clé de chiffrement de la base SQLite est **générée aléatoirement à la
première exécution** (`OsRng`, 32 octets) et stockée dans le **gestionnaire
d'identifiants Windows** via le service `com.panicbase.app`. La clé n'est
**jamais écrite en clair sur disque** ni transmise.

### Anonymisation avant stockage

Les marqueurs suivants sont systématiquement retirés du texte d'un panic
log avant chiffrement et insertion en base :

- `Serial Number: ...`
- `IMEI: ...`
- `UDID: ...`
- `device name: ...`
- Chemins utilisateur Windows (`/users/...`)

### Communication réseau

PanicBase ne contacte **qu'un seul ensemble de domaines**, et **uniquement
sur initiative explicite de l'utilisateur** :

- `*.icloud.com`, `*.apple.com` — pour la fonctionnalité iCloud Photos
  (parcourir et exporter sa propre photothèque)
- `*.icloud-content.com` — CDN Apple pour les fichiers média iCloud

Aucun autre serveur n'est joint. Aucune télémétrie, aucun crash report
distant, aucun phone-home.

### Communication USB

PanicBase utilise la bibliothèque open-source [libimobiledevice]
(LGPL-2.1+) pour parler à l'iPhone via le service AFC officiel d'Apple.
Aucune opération destructive n'est tentée. Les opérations de lecture
(panic logs, photos, contacts) sont initiées explicitement par
l'utilisateur depuis l'interface.

[libimobiledevice]: https://libimobiledevice.org

## Surface d'attaque

PanicBase est compilée avec :

- **Tauri 2** avec capabilities minimales (`core:default`, `shell:default`,
  `core:webview:allow-create-webview-window`, `core:window:allow-set-focus`,
  `core:window:allow-unminimize`)
- **CSP stricte** : `default-src 'self'`, `connect-src ipc: http://ipc.localhost`,
  pas d'origines distantes pour les requêtes JS
- **Drag & drop natif désactivé** (`dragDropEnabled: false`)
- **DevTools désactivés en build release** (`devtools(cfg!(debug_assertions))`)
- **Navigation guard** sur la fenêtre iCloud : seules les URL sous
  `*.icloud.com`, `*.apple.com` ou le host bridge `pb-bridge.invalid`
  (RFC-2606) sont autorisées
- **Profile release Rust** : `opt-level = "z"`, `lto = true`, `strip =
  "symbols"`, `panic = "abort"` — symboles strippés, exposition minimale
- **Obfuscation JavaScript** via `javascript-obfuscator` au build

## Signaler une vulnérabilité

Pour signaler une faille de sécurité, **n'ouvrez pas d'issue publique**.
Contactez directement le mainteneur via :

- Une issue privée sur ce dépôt avec le tag `security`
- Ou par message direct à `@salimizi` sur GitHub

Délai de réponse cible : **72 heures**. Un correctif est généralement
publié dans la release suivante. Les divulgateurs sont crédités dans le
changelog (sauf demande contraire).

## Périmètre

Sont couverts par cette politique :

- L'exécutable PanicBase pour Windows distribué via les Releases GitHub
  de ce dépôt
- Les sous-modules officiels distribués dans le même installeur

Ne sont **pas** couverts :

- Les versions modifiées, recompilées ou redistribuées par des tiers
- Les builds de développement (`tauri:dev`)
- Les vulnérabilités dans Apple Mobile Device Support, WebView2, Windows
  ou tout autre composant tiers — qui doivent être signalées à leur
  éditeur respectif

## Mise à jour

Vérifiez régulièrement la page **[Releases](../../releases)** pour les
mises à jour de sécurité. Chaque release publie un fichier `SHA256SUMS`
pour vérifier l'intégrité du binaire téléchargé.
