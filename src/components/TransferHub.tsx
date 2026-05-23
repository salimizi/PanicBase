import React, { startTransition, useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { useI18n } from '../i18n/context';
import type { Locale } from '../i18n/translations';
import { PhoneUsbDock } from './PhoneUsbDock';
import '../styles/transfer-hub.css';

// ── Types ─────────────────────────────────────────────────────────────────────

type MediaSource = 'afc' | 'icloud';

type AfcMediaItem = {
  objectId: string;
  filename: string;
  extension: string;
  isVideo: boolean;
  folder: string;
  sizeBytes: number;
  mtimeNs: number;
  source?: MediaSource;        // default: 'afc'
  thumbUrl?: string | null;    // iCloud only
  originalUrl?: string | null; // iCloud only
  recordName?: string | null;  // iCloud only
};

type ICloudAssetRaw = {
  recordName: string;
  masterRef: string | null;
  filename: string;
  extension: string;
  isVideo: boolean;
  dateCreatedMs: number;
  durationMs: number;
  sizeBytes: number;
  thumbUrl: string | null;
  originalUrl: string | null;
  isHidden: boolean;
  isFavorite: boolean;
  isInTrash: boolean;
  folder: string; // calculé côté Rust depuis filename + extension (WhatsApp, Captures d'écran, etc.)
};

type AfcFileExport = { objectId: string; filename: string };

type GalleryType = 'mixed' | 'photos' | 'videos';
type SortMode = 'newest' | 'oldest';
type ThumbSize = 'large' | 'medium' | 'small';
type ExportPhase = 'idle' | 'picking' | 'running' | 'paused' | 'done';

type ProgressInfo = {
  current: number; total: number; filename: string; exported: number; failed: number;
};
type PausedInfo = {
  current: number; total: number; exported: number; failed: number; skippedCloud: number;
  pendingFiles: AfcFileExport[]; destDir: string;
};
type DoneInfo = { exported: number; failed: number; skippedCloud: number; destDir: string };
type DeleteConfirm = { ids: string[]; label: string };
type DiskUsage = {
  totalBytes: number; usedBytes: number; freeBytes: number;
  dataTotalBytes: number; dataFreeBytes: number;
};

type ICloudSessionInfo = {
  appleId?: string | null;
  fullName?: string | null;
  photosUrl?: string | null;
  authenticatedAtMs: number;
};

type ICloudStatus = 'idle' | 'connecting' | 'ready' | 'error';

type ViewItem = AfcMediaItem & { id: string; createdTs: number };

const TRANSFER_COPY = {
  fr: {
    photos: 'Photos',
    videos: 'Vidéos',
    mixed: 'Photos + Vidéos',
    video: 'VIDÉO',
    albums: 'Albums',
    all: 'Tous',
    storage: 'Stockage',
    readingSizes: 'Lecture des tailles',
    capacity: 'Capacité',
    free: 'Libre',
    used: 'utilisé',
    usedOn: '{{used}} utilisés sur {{total}}',
    mediaTotal: 'Total médias',
    home: 'Accueil',
    homeTitle: 'Retour accueil PanicBase',
    library: 'Photothèque',
    directRead: 'Lecture directe iPhone · sans backup',
    dateAll: 'Tout',
    dateToday: "Aujourd'hui",
    dateRange7: '7 jours',
    dateRange30: '30 jours',
    dateRange90: '90 jours',
    dateFrom: 'Du',
    dateTo: 'au',
    readingDates: 'Lecture des dates',
    icloud: 'iCloud',
    icloudOpen: 'Ouvrir iCloud Photos',
    icloudTitle: 'Récupérer les photos iCloud',
    icloudOpenError: "Impossible d'ouvrir iCloud Photos",
    icloudCloudOnlyCta: 'Récupérer mes {{count}} photo{{plural}} iCloud',
    icloudConnecting: 'Connexion iCloud en cours…',
    icloudConnected: 'iCloud connecté',
    icloudConnectedAs: 'iCloud · {{user}}',
    icloudSignOut: 'Déconnexion iCloud',
    icloudCancelled: 'Connexion iCloud annulée.',
    icloudTimeout: "La connexion iCloud a expiré, réessayez.",
    icloudLoadingMedias: 'Chargement de la photothèque iCloud…',
    icloudLoadingInline: '{{count}} médias iCloud chargés, le scan continue…',
    icloudScanTitle: 'Import iCloud en cours',
    icloudScanSub: '{{count}} médias chargés en direct',
    icloudPreflightTitle: 'Avant de te connecter à iCloud',
    icloudPreflightIntro: "iCloud bloque l'accès aux photos depuis le web tant que tu ne l'as pas autorisé sur ton iPhone. Sans ça, tu verras une erreur « 421 / Invalid global session ».",
    icloudPreflightStepsTitle: 'Sur ton iPhone :',
    icloudPreflightStep1: 'Ouvre Réglages',
    icloudPreflightStep2: 'Touche ton nom (en haut) puis iCloud',
    icloudPreflightStep3: 'Descends jusqu’à « Accéder à iCloud sur le Web » et active-le',
    icloudPreflightStep4: 'Reviens ici et clique sur « Continuer »',
    icloudPreflightContinue: 'J’ai activé, continuer',
    icloudPreflightCancel: 'Annuler',
    icloudPreflightSkip: 'Ne plus afficher',
    icloudFinishLogin: 'J’ai fini de me connecter, importer mes photos',
    icloudFinishHint: 'Une fois connecté à iCloud.com dans la fenêtre, clique ici pour importer ta photothèque.',
    icloudCompleteError: 'Impossible de finaliser la connexion iCloud',
    newest: 'Plus récent',
    oldest: 'Plus ancien',
    large: 'Grand',
    medium: 'Moyen',
    small: 'Petit',
    export: 'Exporter',
    delete: 'Supprimer',
    refresh: 'Rafraîchir',
    displayed: 'affichés',
    selected: 'sélectionné',
    selectedPlural: 'sélectionnés',
    selectAll: 'Tout sélectionner',
    deselectAll: 'Tout désélectionner',
    loadingLibrary: 'Chargement de la photothèque…',
    waitingTitle: "En attente de l'iPhone",
    waitingSub: "Branchez votre iPhone et déverrouillez-le",
    waitingHint: 'Si « Faire confiance à cet ordinateur ? » apparaît, acceptez-le',
    retry: 'Réessayer',
    loadMore: 'Afficher 700 de plus',
    chooseFolder: 'Choisir le dossier…',
    exportRunning: 'Export en cours',
    on: 'sur',
    file: 'fichier',
    files: 'fichiers',
    copied: 'copié',
    copiedPlural: 'copiés',
    failed: 'échec',
    pickingDest: 'Sélectionner un dossier de destination…',
    cancel: 'Annuler',
    disconnected: 'iPhone déconnecté',
    reconnected: 'iPhone reconnecté',
    exportInterrupted: 'Export interrompu après {{done}} fichier{{plural}} sur {{total}}.',
    connectionDetected: 'Connexion détectée — reprise automatique…',
    waitingConnection: "En attente d'une nouvelle connexion…",
    reconnectHint: "Rebranchez votre iPhone. {{remaining}} fichier{{plural}} restant{{plural}} seront transférés sans dupliquer ceux déjà copiés.",
    autoResume: 'Reprendre automatiquement',
    resumeNow: 'Reprendre maintenant',
    waiting: 'En attente…',
    exportDone: 'Export terminé avec succès !',
    copiedShort: 'copie',
    copiedShortPlural: 'copies',
    failedShort: 'échec',
    cloudOnly: 'iCloud uniquement',
    openFolder: 'Ouvrir le dossier',
    close: 'Fermer',
    deleteQuestion: "Supprimer de l'iPhone ?",
    deleteBody: '{{count}} élément{{plural}} sera{{verbPlural}} supprimé{{plural}} {{label}}.',
    deleteWarning: 'Cette action est irréversible.',
    deleting: 'Suppression…',
    albumDeleteTitle: "Supprimer l'album {{album}}",
    selectedDeleteLabel: '{{count}} élément{{plural}} sélectionné{{plural}}',
    albumDeleteLabel: "de l'album « {{album}} »",
    exportInternalError: "Erreur interne pendant l'export",
    previewError: "Impossible d'ouvrir la prévisualisation",
    galleryAlbum: 'Galerie',
    screenshots: "Captures d'écran",
  },
  en: {
    photos: 'Photos',
    videos: 'Videos',
    mixed: 'Photos + Videos',
    video: 'VIDEO',
    albums: 'Albums',
    all: 'All',
    storage: 'Storage',
    readingSizes: 'Reading sizes',
    capacity: 'Capacity',
    free: 'Free',
    used: 'used',
    usedOn: '{{used}} used of {{total}}',
    mediaTotal: 'Media total',
    home: 'Home',
    homeTitle: 'Back to PanicBase home',
    library: 'Photo Library',
    directRead: 'Direct iPhone access · no backup',
    dateAll: 'All',
    dateToday: 'Today',
    dateRange7: '7 days',
    dateRange30: '30 days',
    dateRange90: '90 days',
    dateFrom: 'From',
    dateTo: 'to',
    readingDates: 'Reading dates',
    icloud: 'iCloud',
    icloudOpen: 'Open iCloud Photos',
    icloudTitle: 'Retrieve iCloud photos',
    icloudOpenError: 'Could not open iCloud Photos',
    icloudCloudOnlyCta: 'Retrieve my {{count}} iCloud photo{{plural}}',
    icloudConnecting: 'Connecting to iCloud…',
    icloudConnected: 'iCloud connected',
    icloudConnectedAs: 'iCloud · {{user}}',
    icloudSignOut: 'Sign out of iCloud',
    icloudCancelled: 'iCloud connection cancelled.',
    icloudTimeout: 'iCloud connection timed out, please retry.',
    icloudLoadingMedias: 'Loading iCloud library…',
    icloudLoadingInline: '{{count}} iCloud media loaded, scan still running…',
    icloudScanTitle: 'iCloud import running',
    icloudScanSub: '{{count}} media loaded live',
    icloudPreflightTitle: 'Before connecting to iCloud',
    icloudPreflightIntro: 'iCloud blocks web access to your photos unless you enable it on your iPhone. Without this you will see a "421 / Invalid global session" error.',
    icloudPreflightStepsTitle: 'On your iPhone:',
    icloudPreflightStep1: 'Open Settings',
    icloudPreflightStep2: 'Tap your name (top), then iCloud',
    icloudPreflightStep3: 'Scroll down to "Access iCloud Data on the Web" and turn it on',
    icloudPreflightStep4: 'Come back here and click "Continue"',
    icloudPreflightContinue: "I've enabled it, continue",
    icloudPreflightCancel: 'Cancel',
    icloudPreflightSkip: "Don't show again",
    icloudFinishLogin: "I'm done signing in, import my photos",
    icloudFinishHint: 'Once signed in to iCloud.com in the window, click here to import your library.',
    icloudCompleteError: 'Could not complete iCloud sign-in',
    newest: 'Newest first',
    oldest: 'Oldest first',
    large: 'Large',
    medium: 'Medium',
    small: 'Small',
    export: 'Export',
    delete: 'Delete',
    refresh: 'Refresh',
    displayed: 'shown',
    selected: 'selected',
    selectedPlural: 'selected',
    selectAll: 'Select all',
    deselectAll: 'Deselect all',
    loadingLibrary: 'Loading photo library…',
    waitingTitle: 'Waiting for iPhone',
    waitingSub: 'Connect your iPhone and unlock it',
    waitingHint: 'If “Trust This Computer?” appears, tap Trust',
    retry: 'Retry',
    loadMore: 'Show 700 more',
    footer: '{{photos}} photos · {{videos}} videos · direct AFC access',
    chooseFolder: 'Choose folder…',
    exportRunning: 'Export in progress',
    on: 'of',
    file: 'file',
    files: 'files',
    copied: 'copied',
    copiedPlural: 'copied',
    failed: 'failed',
    pickingDest: 'Select a destination folder…',
    cancel: 'Cancel',
    disconnected: 'iPhone disconnected',
    reconnected: 'iPhone reconnected',
    exportInterrupted: 'Export interrupted after {{done}} file{{plural}} out of {{total}}.',
    connectionDetected: 'Connection detected — resuming automatically…',
    waitingConnection: 'Waiting for the iPhone to reconnect…',
    reconnectHint: 'Reconnect your iPhone. {{remaining}} remaining file{{plural}} will transfer without duplicating files already copied.',
    autoResume: 'Resume automatically',
    resumeNow: 'Resume now',
    waiting: 'Waiting…',
    exportDone: 'Export completed successfully!',
    copiedShort: 'copied',
    copiedShortPlural: 'copied',
    failedShort: 'failed',
    cloudOnly: 'iCloud only',
    openFolder: 'Open folder',
    close: 'Close',
    deleteQuestion: 'Delete from iPhone?',
    deleteBody: '{{count}} item{{plural}} will be deleted {{label}}.',
    deleteWarning: 'This action cannot be undone.',
    deleting: 'Deleting…',
    albumDeleteTitle: 'Delete album {{album}}',
    selectedDeleteLabel: '{{count}} selected item{{plural}}',
    albumDeleteLabel: 'from album “{{album}}”',
    exportInternalError: 'Internal error during export',
    previewError: 'Could not open preview',
    galleryAlbum: 'Library',
    screenshots: 'Screenshots',
  },
} satisfies Record<Locale, Record<string, string>>;

// ── Icons ─────────────────────────────────────────────────────────────────────

function IconImage({ size = 18 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" aria-hidden="true">
      <rect x="3" y="4" width="18" height="16" rx="2.5" />
      <circle cx="8.5" cy="9" r="1.6" fill="currentColor" stroke="none" />
      <path d="M4 17l5-5 4 4 2.5-2.5L20 18" />
    </svg>
  );
}
function IconVideo({ size = 18 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" aria-hidden="true">
      <rect x="3" y="6" width="13" height="12" rx="2" />
      <path d="M16 10l5-3v10l-5-3z" />
    </svg>
  );
}
function IconDownload({ size = 18 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <path d="M12 4v11" /><path d="m7 10 5 5 5-5" /><path d="M5 20h14" />
    </svg>
  );
}
function IconRefresh({ size = 18 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <path d="M3 12a9 9 0 0 1 15-6.7L21 8" /><path d="M21 3v5h-5" />
      <path d="M21 12a9 9 0 0 1-15 6.7L3 16" /><path d="M3 21v-5h5" />
    </svg>
  );
}
function IconCloud({ size = 18 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.85" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <path d="M17.5 19a4.5 4.5 0 0 0 .77-8.94 6 6 0 0 0-11.68-1.5A4.5 4.5 0 0 0 7 19h10.5z" />
      <path d="M12 12v6" /><path d="m9 15 3-3 3 3" />
    </svg>
  );
}
function IconCalendar({ size = 18 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <rect x="3" y="5" width="18" height="16" rx="2.5" />
      <path d="M3 10h18" /><path d="M8 3v4" /><path d="M16 3v4" />
    </svg>
  );
}
function IconTrash({ size = 16 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <polyline points="3 6 5 6 21 6" /><path d="M19 6l-1 14H6L5 6" />
      <path d="M10 11v6" /><path d="M14 11v6" /><path d="M9 6V4h6v2" />
    </svg>
  );
}
function IconHome({ size = 16 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <path d="M3 9.5L12 3l9 6.5V20a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1z" />
      <path d="M9 21V12h6v9" />
    </svg>
  );
}

function IconFolder({ size = 18 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
    </svg>
  );
}
function IconWifi({ size = 40 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <line x1="1" y1="1" x2="23" y2="23" />
      <path d="M16.72 11.06A10.94 10.94 0 0 1 19 12.55" />
      <path d="M5 12.55a10.94 10.94 0 0 1 5.17-2.39" />
      <path d="M10.71 5.05A16 16 0 0 1 22.56 9" />
      <path d="M1.42 9a15.91 15.91 0 0 1 4.7-2.88" />
      <path d="M8.53 16.11a6 6 0 0 1 6.95 0" />
      <circle cx="12" cy="20" r="1" fill="currentColor" />
    </svg>
  );
}
function IconPhone({ size = 44 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <rect x="5" y="2" width="14" height="20" rx="3" />
      <circle cx="12" cy="17.5" r="1.1" fill="currentColor" stroke="none" />
      <path d="M9.5 6h5" strokeWidth="2" strokeLinecap="round" />
    </svg>
  );
}

// ── Helpers ───────────────────────────────────────────────────────────────────

function trText(copy: Record<string, string>, key: string, vars?: Record<string, string | number>) {
  let out = copy[key] ?? key;
  if (vars) {
    for (const [k, v] of Object.entries(vars)) out = out.split(`{{${k}}}`).join(String(v));
  }
  return out;
}

function formatSize(bytes: number, locale: Locale = 'fr') {
  if (!bytes) return '';
  const units = locale === 'fr' ? ['o', 'Ko', 'Mo', 'Go', 'To'] : ['B', 'KB', 'MB', 'GB', 'TB'];
  let value = bytes; let unit = 0;
  while (value >= 1024 && unit < units.length - 1) { value /= 1024; unit++; }
  return `${value.toFixed(value >= 100 ? 0 : value >= 10 ? 1 : 2)} ${units[unit]}`;
}

function marketedCapacity(bytes: number, locale: Locale = 'fr') {
  if (!bytes) return '';
  const decimalGb = bytes / 1_000_000_000;
  const capacities = [16, 32, 64, 128, 256, 512, 1024, 2048];
  const matched = capacities.find(cap => decimalGb <= cap * 1.08);
  const value = matched ?? Math.round(decimalGb);
  if (locale === 'fr') return value >= 1024 ? `${value / 1024} To` : `${value} Go`;
  return value >= 1024 ? `${value / 1024} TB` : `${value} GB`;
}

function albumName(folder: string, copy: Record<string, string>) {
  const raw = (folder || '').trim();
  const lower = raw.toLowerCase();
  if (!raw || lower === 'dcim' || lower === 'internal storage' || /^\d{3}apple$/i.test(raw)) return copy.galleryAlbum;
  if (lower.includes('screenshot') || lower.includes('capture')) return copy.screenshots;
  if (lower.includes('whatsapp')) return 'WhatsApp';
  if (lower.includes('snapchat') || lower.includes('snap')) return 'Snapchat';
  if (lower.includes('instagram')) return 'Instagram';
  if (lower.includes('telegram')) return 'Telegram';
  if (lower.includes('tiktok')) return 'TikTok';
  if (lower.includes('facebook') || lower.includes('messenger')) return 'Facebook';
  if (lower.includes('capcut')) return 'CapCut';
  if (lower.includes('aliexpress')) return 'AliExpress';
  return raw;
}

function fr(n: number) { return n.toLocaleString('fr-FR'); }

// ── DateRangeFilter ───────────────────────────────────────────────────────────

/** Convertit un timestamp seconde → string "YYYY-MM-DD" (UTC). */
function tsToDateStr(ts: number): string {
  if (!ts) return '';
  const d = new Date(ts * 1000);
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, '0');
  const day = String(d.getDate()).padStart(2, '0');
  return `${y}-${m}-${day}`;
}

/** "YYYY-MM-DD" → secondes epoch (à minuit local). 0 si invalide. */
function dateStrToTs(s: string): number {
  if (!s) return 0;
  const t = new Date(`${s}T00:00:00`).getTime();
  return Number.isFinite(t) ? Math.floor(t / 1000) : 0;
}

type DatePreset = 'all' | 'today' | 'week' | 'month' | 'quarter' | 'year' | 'custom';

type TransferCopy = (typeof TRANSFER_COPY)['fr'];

function DateRangeFilter({
  fromDate, toDate, activePreset, availableYears, onPreset, onFrom, onTo, onPickYear, hasDates, datesProgress, copy, locale,
}: {
  fromDate: string;
  toDate: string;
  activePreset: DatePreset;
  availableYears: number[];
  onPreset: (p: DatePreset) => void;
  onFrom: (s: string) => void;
  onTo:   (s: string) => void;
  onPickYear: (y: number) => void;
  hasDates: boolean;
  datesProgress: { done: number; total: number };
  copy: TransferCopy;
  locale: Locale;
}) {
  const todayStr = tsToDateStr(Math.floor(Date.now() / 1000));
  const loading = datesProgress.total > 0 && datesProgress.done < datesProgress.total;
  const fmtN = (n: number) => n.toLocaleString(locale === 'fr' ? 'fr-FR' : 'en-US');
  return (
    <div className="pbx-daterange">
      <div className="pbx-daterange-chips">
        <button className={`pbx-chip ${activePreset === 'all'     ? 'pbx-chip--on' : ''}`} onClick={() => onPreset('all')}>{copy.dateAll}</button>
        <button className={`pbx-chip ${activePreset === 'today'   ? 'pbx-chip--on' : ''}`} onClick={() => onPreset('today')}>{copy.dateToday}</button>
        <button className={`pbx-chip ${activePreset === 'week'    ? 'pbx-chip--on' : ''}`} onClick={() => onPreset('week')}>{copy.dateRange7}</button>
        <button className={`pbx-chip ${activePreset === 'month'   ? 'pbx-chip--on' : ''}`} onClick={() => onPreset('month')}>{copy.dateRange30}</button>
        <button className={`pbx-chip ${activePreset === 'quarter' ? 'pbx-chip--on' : ''}`} onClick={() => onPreset('quarter')}>{copy.dateRange90}</button>
        {availableYears.slice(0, 4).map(y => (
          <button key={y} className={`pbx-chip pbx-chip--year ${activePreset === 'year' && fromDate.startsWith(String(y)) ? 'pbx-chip--on' : ''}`}
            onClick={() => onPickYear(y)}>
            {y}
          </button>
        ))}
      </div>

      <div className="pbx-daterange-range">
        <IconCalendar size={15} />
        <span className="pbx-daterange-label">{copy.dateFrom}</span>
        <input type="date" className="pbx-date-input" value={fromDate} max={toDate || todayStr}
               onChange={e => onFrom(e.target.value)} disabled={!hasDates} />
        <span className="pbx-daterange-label">{copy.dateTo}</span>
        <input type="date" className="pbx-date-input" value={toDate} min={fromDate} max={todayStr}
               onChange={e => onTo(e.target.value)} disabled={!hasDates} />
        {loading && (
          <span className="pbx-daterange-loading" title={`${copy.readingDates} : ${fmtN(datesProgress.done)}/${fmtN(datesProgress.total)}`}>
            <span className="loading loading-spinner loading-xs" />
          </span>
        )}
      </div>
    </div>
  );
}

// ── MediaTile ─────────────────────────────────────────────────────────────────

/// Cache process-wide de thumbnails déjà résolus. Évite qu'un tile se
/// reconstruise (changement de filtre, scroll back, etc.) et redécoche un
/// nouveau invoke Tauri. Le cache est volontairement non-borné parce qu'il
/// stocke des base64 ; en pratique on n'en garde que ce qui a été affiché
/// (limite naturelle ≈ taille de la galerie). Évite ~500 invokes au moindre
/// re-render de la liste filtrée.
const thumbCache = new Map<string, string | 'failed'>();

function MediaTileImpl({ item, selected, size, onToggle, onOpenPreview, getThumbnail }:
  {
    item: ViewItem;
    selected: boolean;
    size: ThumbSize;
    onToggle: (id: string) => void;
    onOpenPreview: (item: ViewItem) => void;
    getThumbnail: (item: ViewItem) => Promise<string | null>;
  }) {
  const { locale } = useI18n();
  const copy = TRANSFER_COPY[locale];
  const ref = useRef<HTMLButtonElement>(null);
  const cached = thumbCache.get(item.id);
  const [thumb, setThumb] = useState<string | null>(
    typeof cached === 'string' ? cached : null
  );
  const [failed, setFailed] = useState(cached === 'failed');
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (thumb || failed) return;
    const observer = new IntersectionObserver((entries) => {
      if (!entries[0]?.isIntersecting || loading) return;
      setLoading(true); observer.disconnect();
      getThumbnail(item).then(b64 => {
        if (b64) {
          thumbCache.set(item.id, b64);
          setThumb(b64);
        } else {
          thumbCache.set(item.id, 'failed');
          setFailed(true);
        }
      })
        .catch(() => { thumbCache.set(item.id, 'failed'); setFailed(true); })
        .finally(() => setLoading(false));
    }, { rootMargin: '900px' });
    if (ref.current) observer.observe(ref.current);
    return () => observer.disconnect();
  }, [failed, getThumbnail, item, loading, thumb]);

  return (
    <button ref={ref} type="button" className={`pbx-tile pbx-tile--${size} ${selected ? 'pbx-tile--selected' : ''}`}
      onClick={() => onToggle(item.id)}
      onDoubleClick={(e) => { e.preventDefault(); onOpenPreview(item); }}
      title={item.filename}>
      <div className="pbx-tile-media">
        {thumb
          ? <img src={`data:image/jpeg;base64,${thumb}`} alt={item.filename} draggable={false} />
          : <div className="pbx-tile-fallback">
              {loading ? <span className="loading loading-spinner loading-sm" /> : item.isVideo ? <IconVideo size={42} /> : <IconImage size={42} />}
              <span>{item.extension.toUpperCase()}</span>
            </div>}
        {item.isVideo && <span className="pbx-video-badge">{copy.video}</span>}
        {item.source === 'icloud' && (
          <span className="pbx-icloud-badge" title={copy.icloud}>
            <IconCloud size={11} /> iCloud
          </span>
        )}
        {selected && <span className="pbx-check">✓</span>}
      </div>
      <div className="pbx-tile-name">{item.filename}</div>
      {item.sizeBytes > 0 && <div className="pbx-tile-size">{formatSize(item.sizeBytes, locale)}</div>}
    </button>
  );
}

const MediaTile = React.memo(MediaTileImpl, (prev, next) => {
  return (
    prev.item.id === next.item.id
    && prev.item.sizeBytes === next.item.sizeBytes
    && prev.selected === next.selected
    && prev.size === next.size
    && prev.onToggle === next.onToggle
    && prev.onOpenPreview === next.onOpenPreview
    && prev.getThumbnail === next.getThumbnail
  );
});

// ── ExportProgressModal ───────────────────────────────────────────────────────

function ExportProgressModal({ phase, progress, onCancel }: {
  phase: ExportPhase; progress: ProgressInfo; onCancel: () => void;
}) {
  const { locale } = useI18n();
  const copy = TRANSFER_COPY[locale];
  if (phase !== 'running' && phase !== 'picking') return null;
  const pct = progress.total > 0 ? (progress.current / progress.total) * 100 : 0;

  return (
    <div className="pbx-modal-backdrop">
      <div className="pbx-modal-box pbx-modal-export">
        <div className="pbx-modal-icon pbx-modal-icon--blue">
          <IconDownload size={26} />
        </div>
        <h3>{phase === 'picking' ? copy.chooseFolder : copy.exportRunning}</h3>

        {phase === 'running' && (
          <>
            <div className="pbx-progress-counters">
              <span className="pbx-progress-current">{fr(progress.current)}</span>
              <span className="pbx-progress-sep">{copy.on}</span>
              <span className="pbx-progress-total">{fr(progress.total)}</span>
              <span className="pbx-progress-label">{progress.total > 1 ? copy.files : copy.file}</span>
            </div>

            <div className="pbx-progress-track">
              <div className="pbx-progress-fill" style={{ width: `${pct}%` }} />
            </div>

            <div className="pbx-progress-stats">
              <span className="pbx-stat-ok">✓ {fr(progress.exported)} {progress.exported > 1 ? copy.copiedPlural : copy.copied}</span>
              {progress.failed > 0 && <span className="pbx-stat-err">✗ {fr(progress.failed)} {copy.failed}</span>}
              <span className="pbx-stat-pct">{pct.toFixed(0)} %</span>
            </div>

            {progress.filename && (
              <div className="pbx-progress-filename" title={progress.filename}>
                {progress.filename}
              </div>
            )}
          </>
        )}

        {phase === 'picking' && <p className="pbx-modal-sub">{copy.pickingDest}</p>}

        <div className="pbx-modal-actions">
          <button className="pbx-btn pbx-btn--ghost" onClick={onCancel}>{copy.cancel}</button>
        </div>
      </div>
    </div>
  );
}

// ── DisconnectModal ───────────────────────────────────────────────────────────

function DisconnectModal({ paused, udid, onResume, onCancel }: {
  paused: PausedInfo; udid: string | null; onResume: () => void; onCancel: () => void;
}) {
  const { locale } = useI18n();
  const copy = TRANSFER_COPY[locale];
  const [alive, setAlive] = useState(false);
  const [autoResume, setAutoResume] = useState(true);
  const triggeredRef = useRef(false);

  // Polling toutes les 1,5s pour détecter le rebranchement
  useEffect(() => {
    let cancelled = false;
    const tick = async () => {
      try {
        const ok = await invoke<boolean>('ping_afc', { udid: udid ?? null });
        if (cancelled) return;
        setAlive(ok);
        if (ok && autoResume && !triggeredRef.current) {
          triggeredRef.current = true;
          onResume();
        }
      } catch { /* ignore */ }
    };
    void tick();
    const id = window.setInterval(tick, 1500);
    return () => { cancelled = true; window.clearInterval(id); };
  }, [udid, autoResume, onResume]);

  const remaining = Math.max(0, paused.total - paused.current);

  return (
    <div className="pbx-modal-backdrop">
      <div className="pbx-modal-box pbx-modal-disconnect">
        <div className={`pbx-modal-icon ${alive ? 'pbx-modal-icon--ok' : 'pbx-modal-icon--warn'}`}>
          {alive ? '✓' : <IconWifi size={28} />}
        </div>
        <h3>{alive ? copy.reconnected : copy.disconnected}</h3>

        <p className="pbx-modal-sub">
          {trText(copy, 'exportInterrupted', { done: fr(paused.exported), total: fr(paused.total), plural: paused.exported > 1 ? 's' : '' })}
        </p>

        <div className={`pbx-disco-status ${alive ? 'pbx-disco-status--ok' : 'pbx-disco-status--wait'}`}>
          {alive ? (
            <>✓ {copy.connectionDetected}</>
          ) : (
            <>
              <span className="loading loading-spinner loading-xs" />
              {copy.waitingConnection}
            </>
          )}
        </div>

        <p className="pbx-modal-hint">
          {trText(copy, 'reconnectHint', { remaining: fr(remaining), plural: remaining > 1 ? 's' : '' })}
        </p>

        <label className="pbx-disco-auto">
          <input type="checkbox" checked={autoResume} onChange={e => setAutoResume(e.target.checked)} />
          {copy.autoResume}
        </label>

        <div className="pbx-modal-actions">
          <button className="pbx-btn pbx-btn--primary" onClick={onResume} disabled={!alive}>
            {alive ? copy.resumeNow : copy.waiting}
          </button>
          <button className="pbx-btn pbx-btn--ghost" onClick={onCancel}>{copy.cancel}</button>
        </div>
      </div>
    </div>
  );
}

// ── ICloudPreflightModal ──────────────────────────────────────────────────────

function ICloudPreflightModal({ onContinue, onCancel, onSkipForever }: {
  onContinue: () => void; onCancel: () => void; onSkipForever: () => void;
}) {
  const { locale } = useI18n();
  const copy = TRANSFER_COPY[locale];
  return (
    <div className="pbx-modal-backdrop">
      <div className="pbx-modal-box pbx-modal-preflight">
        <div className="pbx-modal-icon pbx-modal-icon--info">
          <IconCloud size={28} />
        </div>
        <h3>{copy.icloudPreflightTitle}</h3>
        <p className="pbx-modal-sub">{copy.icloudPreflightIntro}</p>
        <div className="pbx-preflight-steps">
          <div className="pbx-preflight-steps__title">{copy.icloudPreflightStepsTitle}</div>
          <ol>
            <li>{copy.icloudPreflightStep1}</li>
            <li>{copy.icloudPreflightStep2}</li>
            <li><strong>{copy.icloudPreflightStep3}</strong></li>
            <li>{copy.icloudPreflightStep4}</li>
          </ol>
        </div>
        <div className="pbx-modal-actions pbx-modal-actions--stack">
          <button className="pbx-btn pbx-btn--icloud" onClick={onContinue}>
            <IconCloud size={15} /> {copy.icloudPreflightContinue}
          </button>
          <button className="pbx-btn pbx-btn--ghost" onClick={onCancel}>{copy.icloudPreflightCancel}</button>
          <button className="pbx-link-btn" onClick={onSkipForever}>{copy.icloudPreflightSkip}</button>
        </div>
      </div>
    </div>
  );
}

// ── SuccessModal ──────────────────────────────────────────────────────────────

function SuccessModal({ done, onOpenFolder, onOpenICloud, onClose }: {
  done: DoneInfo; onOpenFolder: (path: string) => void; onOpenICloud: () => void; onClose: () => void;
}) {
  const { locale } = useI18n();
  const copy = TRANSFER_COPY[locale];
  const cloudCount = done.skippedCloud;
  const cloudCta = cloudCount > 0
    ? trText(copy, 'icloudCloudOnlyCta', { count: fr(cloudCount), plural: cloudCount > 1 ? 's' : '' })
    : '';
  return (
    <div className="pbx-modal-backdrop">
      <div className="pbx-modal-box pbx-modal-success">
        <div className="pbx-modal-icon pbx-modal-icon--ok">✓</div>
        <h3>{copy.exportDone}</h3>
        <div className="pbx-done-stats">
          <div className="pbx-done-stat">
            <strong>{fr(done.exported)}</strong>
            <span>{done.exported > 1 ? copy.copiedShortPlural : copy.copiedShort}</span>
          </div>
          {done.failed > 0 && (
            <div className="pbx-done-stat pbx-done-stat--err">
              <strong>{fr(done.failed)}</strong>
              <span>{copy.failedShort}</span>
            </div>
          )}
          {cloudCount > 0 && (
            <div className="pbx-done-stat pbx-done-stat--cloud">
              <strong>{fr(cloudCount)}</strong>
              <span>{copy.cloudOnly}</span>
            </div>
          )}
        </div>
        <div className="pbx-modal-actions">
          {cloudCount > 0 && (
            <button className="pbx-btn pbx-btn--icloud" onClick={onOpenICloud}>
              <IconCloud size={16} /> {cloudCta}
            </button>
          )}
          <button className="pbx-btn pbx-btn--primary" onClick={() => onOpenFolder(done.destDir)}>
            <IconFolder size={16} /> {copy.openFolder}
          </button>
          <button className="pbx-btn pbx-btn--ghost" onClick={onClose}>{copy.close}</button>
        </div>
      </div>
    </div>
  );
}

// ── DeleteModal ───────────────────────────────────────────────────────────────

function DeleteModal({ confirm, onConfirm, onCancel, deleting }: {
  confirm: DeleteConfirm; onConfirm: () => void; onCancel: () => void; deleting: boolean;
}) {
  const { locale } = useI18n();
  const copy = TRANSFER_COPY[locale];
  return (
    <div className="pbx-modal-backdrop">
      <div className="pbx-modal-box pbx-modal-delete">
        <div className="pbx-modal-icon pbx-modal-icon--err">
          <IconTrash size={24} />
        </div>
        <h3>{copy.deleteQuestion}</h3>
        <p className="pbx-modal-sub">
          {trText(copy, 'deleteBody', {
            count: fr(confirm.ids.length),
            plural: confirm.ids.length > 1 ? 's' : '',
            verbPlural: confirm.ids.length > 1 ? 'ont' : '',
            label: confirm.label,
          })}
        </p>
        <p className="pbx-modal-warn">{copy.deleteWarning}</p>
        <div className="pbx-modal-actions">
          <button className="pbx-btn pbx-btn--danger" onClick={onConfirm} disabled={deleting}>
            {deleting ? <span className="loading loading-spinner loading-xs" /> : <IconTrash size={14} />}
            {deleting ? copy.deleting : copy.delete}
          </button>
          <button className="pbx-btn pbx-btn--ghost" onClick={onCancel} disabled={deleting}>{copy.cancel}</button>
        </div>
      </div>
    </div>
  );
}

// ── TransferHub ───────────────────────────────────────────────────────────────

export function TransferHub({ udid, onGoHome }: { udid: string | null; onGoHome?: () => void }) {
  const { locale } = useI18n();
  const copy = TRANSFER_COPY[locale];
  const [items, setItems] = useState<AfcMediaItem[]>([]);
  const [mode, setMode] = useState<'loading' | 'ready' | 'error'>('loading');
  const [error, setError] = useState('');
  const [galleryType, setGalleryType] = useState<GalleryType>('mixed');
  const [sortMode, setSortMode] = useState<SortMode>('newest');
  const [datePreset, setDatePreset] = useState<DatePreset>('all');
  const [fromDate, setFromDate] = useState('');
  const [toDate, setToDate]     = useState('');
  const [thumbSize, setThumbSize] = useState<ThumbSize>('medium');
  const [activeAlbum, setActiveAlbum] = useState('__all');
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [renderLimit, setRenderLimit] = useState(500);

  // Export state
  const [exportPhase, setExportPhase] = useState<ExportPhase>('idle');
  const [progress, setProgress] = useState<ProgressInfo>({ current: 0, total: 0, filename: '', exported: 0, failed: 0 });
  const [paused, setPaused] = useState<PausedInfo | null>(null);
  const [done, setDone] = useState<DoneInfo | null>(null);
  const unlistenFns = useRef<UnlistenFn[]>([]);

  // Delete state
  const [deleteConfirm, setDeleteConfirm] = useState<DeleteConfirm | null>(null);
  const [deleting, setDeleting] = useState(false);

  // iPhone disk + size prefetch
  const [diskUsage, setDiskUsage] = useState<DiskUsage | null>(null);
  const [sizesProgress, setSizesProgress] = useState({ done: 0, total: 0 });

  // iCloud session + library
  const [icloudStatus, setICloudStatus] = useState<ICloudStatus>('idle');
  const [icloudSession, setICloudSession] = useState<ICloudSessionInfo | null>(null);
  const [icloudInfo, setICloudInfo] = useState('');
  const [icloudItems, setICloudItems] = useState<AfcMediaItem[]>([]);
  const [icloudLoading, setICloudLoading] = useState(false);
  const [icloudLoadCount, setICloudLoadCount] = useState(0);
  const [showPreflight, setShowPreflight] = useState(false);
  // Map recordName → set d'albums iCloud auxquels appartient l'asset
  // (Snapchat, Instagram, WhatsApp, …). Alimentée en arrière-plan via
  // l'événement `icloud-album-assignments`. Permet l'affichage exact de
  // l'arborescence iCloud.com (un asset peut être dans plusieurs albums).
  const [icloudAlbumMap, setICloudAlbumMap] = useState<Map<string, Set<string>>>(new Map());
  // Listeners spécifiques iCloud (album scan en arrière-plan). On les garde
  // séparés de `unlistenFns` qui sert au pipeline d'export — sinon
  // `cleanListeners()` détacherait nos listeners albums au mauvais moment.
  const icloudAlbumListeners = useRef<UnlistenFn[]>([]);

  // ── Gallery ──────────────────────────────────────────────────────────────

  const refresh = useCallback(async () => {
    setMode('loading'); setError(''); setSelected(new Set()); setItems([]);
    setSizesProgress({ done: 0, total: 0 });
    try {
      const ok = await invoke<boolean>('afc_available');
      if (!ok) throw new Error('not_available');
      const next = await invoke<AfcMediaItem[]>('list_afc_gallery', { udid: udid ?? null });
      setItems(next); setMode('ready');

      // Disque iPhone (en parallèle, non bloquant)
      invoke<DiskUsage>('get_iphone_disk_usage', { udid: udid ?? null })
        .then(setDiskUsage)
        .catch(() => setDiskUsage(null));

      // Prefetch des tailles par paquets de 500 (UI mise à jour à chaque batch)
      if (next.length > 0) {
        setSizesProgress({ done: 0, total: next.length });
        const ids = next.map(i => i.objectId);
        const BATCH = 500;
        (async () => {
          for (let off = 0; off < ids.length; off += BATCH) {
            const slice = ids.slice(off, off + BATCH);
            try {
              const entries = await invoke<Array<{ objectId: string; sizeBytes: number; mtimeNs: number }>>(
                'prefetch_afc_sizes', { udid: udid ?? null, objectIds: slice }
              );
              const map = new Map(entries.map(e => [e.objectId, { size: e.sizeBytes, mtime: e.mtimeNs }] as const));
              setItems(prev => prev.map(it => {
                const m = map.get(it.objectId);
                if (!m) return it;
                return { ...it, sizeBytes: m.size, mtimeNs: m.mtime || it.mtimeNs };
              }));
            } catch { /* batch ignoré, on continue */ }
            setSizesProgress(p => ({ done: Math.min(p.total, off + BATCH), total: p.total }));
          }
        })();
      }
    } catch { setMode('error'); setError(''); }
  }, [udid]);

  useEffect(() => { void refresh(); }, [refresh]);

  const viewItems = useMemo<ViewItem[]>(() => {
    const merged: AfcMediaItem[] = [...items, ...icloudItems];
    return merged.map(item => ({
      ...item,
      id: item.objectId,
      // iCloud : on garde le folder déjà classifié côté Rust (déjà préfixé "iCloud · …").
      // AFC : on traduit le nom technique via la table copy (`albumName`).
      folder: item.source === 'icloud' ? item.folder : albumName(item.folder, copy),
      createdTs: Math.floor((item.mtimeNs ?? 0) / 1_000_000_000),
    }));
  }, [items, icloudItems, copy]);

  const photos = useMemo(() => viewItems.filter(i => !i.isVideo), [viewItems]);
  const videos = useMemo(() => viewItems.filter(i =>  i.isVideo), [viewItems]);
  const mixed = viewItems;

  const storageStats = useMemo(() => {
    const pb = photos.reduce((s, i) => s + (i.sizeBytes || 0), 0);
    const vb = videos.reduce((s, i) => s + (i.sizeBytes || 0), 0);
    const total = pb + vb;
    return { photoBytes: pb, videoBytes: vb, totalBytes: total,
      photoPct: total > 0 ? (pb / total) * 100 : 50,
      videoPct: total > 0 ? (vb / total) * 100 : 50 };
  }, [photos, videos]);

  const albums = useMemo(() => {
    const base = galleryType === 'mixed' ? mixed : galleryType === 'photos' ? photos : videos;
    const map = new Map<string, number>();
    for (const item of base) {
      // 1. Catégorie système / heuristique (déjà préfixée "iCloud · …" si iCloud)
      map.set(item.folder, (map.get(item.folder) ?? 0) + 1);
      // 2. Albums iCloud utilisateur (Snapchat, Instagram, WhatsApp, …)
      //    Un même asset peut appartenir à plusieurs albums → chaque
      //    appartenance compte indépendamment, ce qui reproduit exactement
      //    la vue icloud.com où le total par album est correct même si les
      //    photos se recoupent.
      if (item.source === 'icloud' && item.recordName) {
        const userAlbums = icloudAlbumMap.get(item.recordName);
        if (userAlbums) {
          for (const albumName of userAlbums) {
            const key = `iCloud · ${albumName}`;
            map.set(key, (map.get(key) ?? 0) + 1);
          }
        }
      }
    }
    return [...map.entries()]
      .map(([name, count]) => ({ name, count }))
      .sort((a, b) => a.name === copy.galleryAlbum ? -1 : b.name === copy.galleryAlbum ? 1 : b.count - a.count || a.name.localeCompare(b.name));
  }, [galleryType, mixed, photos, videos, icloudAlbumMap, copy.galleryAlbum]);

  // Bornes du range en secondes (inclusif). 0 = pas de borne.
  const fromTs = useMemo(() => dateStrToTs(fromDate), [fromDate]);
  const toTs   = useMemo(() => {
    const t = dateStrToTs(toDate);
    return t ? t + 24 * 3600 - 1 : 0; // fin-de-journée
  }, [toDate]);

  const filtered = useMemo(() => {
    const base = galleryType === 'mixed' ? mixed : galleryType === 'photos' ? photos : videos;
    const ICLOUD_PREFIX = 'iCloud · ';
    const albumAsRawName = activeAlbum.startsWith(ICLOUD_PREFIX)
      ? activeAlbum.slice(ICLOUD_PREFIX.length)
      : null;
    return base
      .filter(item => {
        if (activeAlbum === '__all') return true;
        if (item.folder === activeAlbum) return true;
        if (albumAsRawName && item.source === 'icloud' && item.recordName) {
          const memberships = icloudAlbumMap.get(item.recordName);
          if (memberships && memberships.has(albumAsRawName)) return true;
        }
        return false;
      })
      .filter(item => {
        if (!fromTs && !toTs) return true;
        const ts = item.createdTs;
        if (!ts) return false; // date inconnue → exclue d'un range explicite
        if (fromTs && ts < fromTs) return false;
        if (toTs   && ts > toTs)   return false;
        return true;
      })
      .sort((a, b) => {
        const av = a.createdTs || 0, bv = b.createdTs || 0;
        return sortMode === 'newest' ? bv - av || b.filename.localeCompare(a.filename) : av - bv || a.filename.localeCompare(b.filename);
      });
  }, [activeAlbum, fromTs, toTs, galleryType, mixed, photos, sortMode, videos, icloudAlbumMap]);

  const visible = filtered.slice(0, renderLimit);

  useEffect(() => { setRenderLimit(500); setSelected(new Set()); }, [activeAlbum, fromTs, toTs, galleryType, sortMode, thumbSize]);

  // Années disponibles dans la photothèque (dérivées des dates connues)
  const availableYears = useMemo(() => {
    const all = galleryType === 'mixed' ? mixed : galleryType === 'photos' ? photos : videos;
    const set = new Set<number>();
    for (const it of all) {
      if (it.createdTs) set.add(new Date(it.createdTs * 1000).getFullYear());
    }
    return [...set].sort((a, b) => b - a);
  }, [galleryType, mixed, photos, videos]);

  // Combien d'items ont une date connue → indicateur "prefetch des dates"
  const datesProgress = useMemo(() => {
    const all = galleryType === 'mixed' ? mixed : galleryType === 'photos' ? photos : videos;
    const done = all.reduce((n, i) => n + (i.createdTs ? 1 : 0), 0);
    return { done, total: all.length };
  }, [galleryType, mixed, photos, videos]);

  const hasAnyDate = datesProgress.done > 0;

  // Handlers de raccourcis
  const applyPreset = useCallback((p: DatePreset) => {
    setDatePreset(p);
    const now = new Date();
    const todayStr = tsToDateStr(Math.floor(now.getTime() / 1000));
    const shift = (days: number) => {
      const d = new Date();
      d.setDate(d.getDate() - days);
      return tsToDateStr(Math.floor(d.getTime() / 1000));
    };
    if (p === 'all')       { setFromDate(''); setToDate(''); }
    else if (p === 'today'){ setFromDate(todayStr); setToDate(todayStr); }
    else if (p === 'week') { setFromDate(shift(6));  setToDate(todayStr); }
    else if (p === 'month'){ setFromDate(shift(29)); setToDate(todayStr); }
    else if (p === 'quarter'){ setFromDate(shift(89)); setToDate(todayStr); }
  }, []);

  const applyYear = useCallback((y: number) => {
    setDatePreset('year');
    setFromDate(`${y}-01-01`);
    setToDate(`${y}-12-31`);
  }, []);

  const onChangeFrom = useCallback((s: string) => { setFromDate(s); setDatePreset('custom'); }, []);
  const onChangeTo   = useCallback((s: string) => { setToDate(s);   setDatePreset('custom'); }, []);

  const getThumbnail = useCallback(async (item: ViewItem) => {
    if (item.source === 'icloud') {
      if (!item.thumbUrl) return null;
      try { return await invoke<string>('icloud_thumbnail_data', { url: item.thumbUrl }); }
      catch { return null; }
    }
    try { return await invoke<string>('get_afc_thumbnail', { udid: udid ?? null, objectId: item.objectId }); }
    catch { return null; }
  }, [udid]);

  const toggleSelect = useCallback((id: string) => {
    setSelected(prev => { const n = new Set(prev); n.has(id) ? n.delete(id) : n.add(id); return n; });
  }, []);

  const openPreview = useCallback(async (item: ViewItem) => {
    try {
      if (item.source === 'icloud') {
        // L'asset n'existe pas sur l'iPhone (AFC le rejette avec code 8).
        // On télécharge la URL pré-signée Apple et on l'ouvre dans le viewer
        // par défaut du système via une commande dédiée côté Rust.
        const url = item.originalUrl || item.thumbUrl;
        if (!url) {
          setError(`${copy.previewError} : aucun URL téléchargeable pour cet asset iCloud (il n'est peut-être pas encore prêt côté serveur Apple).`);
          return;
        }
        await invoke<string>('open_icloud_media_preview', {
          filename: item.filename,
          downloadUrl: url,
        });
        return;
      }
      await invoke<string>('open_afc_media_preview', {
        udid: udid ?? null,
        file: { objectId: item.objectId, filename: item.filename },
      });
    } catch (err) {
      setError(`${copy.previewError} : ${String(err)}`);
    }
  }, [udid, copy]);

  // Sélectionne TOUS les items filtrés (pas seulement les 500 rendus)
  const selectAll = useCallback(() => setSelected(new Set(filtered.map(i => i.id))), [filtered]);

  // ── Export ───────────────────────────────────────────────────────────────

  const cleanListeners = useCallback(() => {
    unlistenFns.current.forEach(fn => fn());
    unlistenFns.current = [];
  }, []);

  // Démarre l'export pour une liste de fichiers donnée (utilisé aussi pour la reprise)
  const startExportFiles = useCallback(async (
    files: AfcFileExport[],
    destDir: string,
    base: { current: number; total: number; exported: number; failed: number; skippedCloud: number } | null = null,
  ) => {
    const baseCurrent = base?.current ?? 0;
    const baseTotal = base?.total ?? files.length;
    const baseExported = base?.exported ?? 0;
    const baseFailed = base?.failed ?? 0;
    const baseSkippedCloud = base?.skippedCloud ?? 0;
    setExportPhase('running');
    setProgress({ current: baseCurrent, total: baseTotal, filename: '', exported: baseExported, failed: baseFailed });
    cleanListeners();

    const handlers = await Promise.all([
      listen<ProgressInfo>('afc-export-progress', e => {
        const next = e.payload;
        setProgress({
          ...next,
          current: Math.min(baseTotal, baseCurrent + next.current),
          total: baseTotal,
          exported: baseExported + next.exported,
          failed: baseFailed + next.failed,
        });
      }),

      listen<{ current: number; total: number; exported: number; failed?: number; skippedCloud?: number; completedIds: string[] }>(
        'afc-export-paused', e => {
          const { completedIds, exported } = e.payload;
          const failed = e.payload.failed ?? 0;
          const skippedCloud = e.payload.skippedCloud ?? 0;
          const done = new Set(completedIds);
          const pending = files.filter(f => !done.has(f.objectId));
          setPaused({
            current: Math.min(baseTotal, baseCurrent + done.size),
            total: baseTotal,
            exported: baseExported + exported,
            failed: baseFailed + failed,
            skippedCloud: baseSkippedCloud + skippedCloud,
            pendingFiles: pending,
            destDir,
          });
          setExportPhase('paused');
          cleanListeners();
        }),

      listen<DoneInfo>('afc-export-done', e => {
        setDone({
          exported: baseExported + e.payload.exported,
          failed: baseFailed + e.payload.failed,
          skippedCloud: baseSkippedCloud + e.payload.skippedCloud,
          destDir: e.payload.destDir,
        });
        setExportPhase('done');
        cleanListeners();
      }),

      listen<string>('afc-export-error', e => {
        setError(e.payload || copy.exportInternalError);
        setExportPhase('idle');
        cleanListeners();
      }),
    ]);
    unlistenFns.current = handlers;

    try {
      await invoke('start_afc_export', { udid: udid ?? null, files, destDir });
    } catch (err) {
      setError(String(err)); setExportPhase('idle'); cleanListeners();
    }
  }, [udid, cleanListeners, copy]);

  const exportSelected = useCallback(async () => {
    if (selected.size === 0) return;
    setExportPhase('picking');
    const destDir = await invoke<string | null>('pick_export_folder_cmd').catch(() => null);
    if (!destDir) { setExportPhase('idle'); return; }

    // IMPORTANT : on filtre sur `viewItems` (merged AFC + iCloud), pas `items`
    // qui ne contient que les assets AFC. Sinon les sélections iCloud sont
    // silencieusement perdues et l'export ne voit rien à exporter.
    const picked = viewItems.filter(item => selected.has(item.objectId));
    // Sépare les sources : AFC (iPhone branché en USB) vs iCloud (cloud).
    // Chaque pipeline a sa propre commande Tauri mais émet les mêmes
    // événements `afc-export-*` pour réutiliser les listeners.
    const icloudPicked = picked.filter(it => it.source === 'icloud');
    const afcFiles = picked
      .filter(it => it.source !== 'icloud')
      .map(it => ({ objectId: it.objectId, filename: it.filename }));

    // Si on a sélectionné des assets iCloud sans `originalUrl`, c'est qu'Apple
    // n'a pas exposé `resOriginalRes` dans la réponse list — on signale le
    // souci à l'utilisateur sans simuler une "sélection mixte" trompeuse.
    const icloudMissingUrl = icloudPicked.filter(it => !it.originalUrl);
    if (icloudMissingUrl.length > 0 && icloudPicked.length === icloudMissingUrl.length) {
      setError(
        `Impossible d'exporter : Apple n'a pas renvoyé d'URL de téléchargement pour ${icloudMissingUrl.length} asset(s) iCloud. ` +
        "Vérifie les logs Rust pour le dump des champs du premier record, puis dis-le-moi."
      );
      setExportPhase('idle');
      return;
    }
    const icloudFiles = icloudPicked
      .filter(it => it.originalUrl)
      .map(it => ({
        recordName: it.recordName ?? '',
        filename: it.filename,
        downloadUrl: it.originalUrl as string,
      }));

    if (afcFiles.length > 0 && icloudFiles.length === 0) {
      await startExportFiles(afcFiles, destDir);
      return;
    }
    if (icloudFiles.length > 0 && afcFiles.length === 0) {
      // Pipeline iCloud — réutilise les mêmes events, donc on attache les
      // mêmes listeners que startExportFiles.
      setExportPhase('running');
      cleanListeners();
      setProgress({ current: 0, total: icloudFiles.length, filename: '', exported: 0, failed: 0 });
      const handlers = await Promise.all([
        listen<ProgressInfo>('afc-export-progress', e => setProgress(e.payload)),
        listen<DoneInfo>('afc-export-done', e => {
          setDone(e.payload);
          setExportPhase('done');
          cleanListeners();
        }),
        listen<string>('afc-export-error', e => {
          setError(e.payload || copy.exportInternalError);
          setExportPhase('idle');
          cleanListeners();
        }),
      ]);
      unlistenFns.current = handlers;
      try {
        await invoke('start_icloud_export', { files: icloudFiles, destDir });
      } catch (err) {
        setError(`iCloud export : ${String(err)}`);
        setExportPhase('idle');
        cleanListeners();
      }
      return;
    }
    // Cas mixte : on enchaîne AFC puis iCloud manuellement. Pas attendu en
    // pratique (la galerie affiche soit l'un soit l'autre) mais on dégrade
    // proprement.
    setError("Sélection mixte AFC+iCloud non supportée — exporte d'abord les uns puis les autres.");
    setExportPhase('idle');
  }, [selected, viewItems, startExportFiles, cleanListeners, copy]);

  const cancelExport = useCallback(async () => {
    await invoke('cancel_afc_export').catch(() => {});
    await invoke('cancel_icloud_export').catch(() => {});
    cleanListeners(); setPaused(null); setExportPhase('idle');
  }, [cleanListeners]);

  const resumeExport = useCallback(async () => {
    if (!paused) return;
    const { pendingFiles, destDir, current, total, exported, failed, skippedCloud } = paused;
    setPaused(null);
    await startExportFiles(pendingFiles, destDir, { current, total, exported, failed, skippedCloud });
  }, [paused, startExportFiles]);

  const openFolder = useCallback(async (path: string) => {
    await invoke('open_folder', { path }).catch(() => {});
  }, []);

  const performICloudLogin = useCallback(async () => {
    setICloudInfo('');
    setICloudStatus('connecting');
    try {
      await invoke('open_icloud_window');
    } catch (e) {
      setICloudStatus('error');
      setError(`${copy.icloudOpenError} : ${String(e)}`);
    }
  }, [copy]);

  const completeICloudLogin = useCallback(async () => {
    setICloudInfo(copy.icloudConnecting);
    setICloudStatus('connecting');
    try {
      await invoke('icloud_complete_login');
      // The `icloud-session-ready` event listener will flip the UI to ready
      // and trigger loadICloudLibrary().
    } catch (e) {
      setICloudStatus('error');
      const msg = e instanceof Error ? e.message : String(e);
      let hint = '';
      if (/missing dsInfo/i.test(msg) || /dsid/i.test(msg)) {
        hint = ' — La session iCloud n\'est pas complète. Vérifie que tu es bien connecté à iCloud.com dans la fenêtre puis réessaie.';
      } else if (/Fenêtre iCloud introuvable/i.test(msg)) {
        hint = ' — Ré-ouvre la fenêtre iCloud puis réessaie.';
      }
      setError(`${copy.icloudCompleteError} : ${msg}${hint}`);
      setICloudInfo('');
    }
  }, [copy]);

  const openICloud = useCallback(() => {
    const skipPreflight = (() => {
      try { return localStorage.getItem('pb-icloud-preflight-skip') === '1'; }
      catch { return false; }
    })();
    if (skipPreflight) {
      void performICloudLogin();
    } else {
      setShowPreflight(true);
    }
  }, [performICloudLogin]);

  const onPreflightContinue = useCallback(() => {
    setShowPreflight(false);
    void performICloudLogin();
  }, [performICloudLogin]);

  const onPreflightCancel = useCallback(() => setShowPreflight(false), []);

  const onPreflightSkipForever = useCallback(() => {
    try { localStorage.setItem('pb-icloud-preflight-skip', '1'); } catch { /* noop */ }
    setShowPreflight(false);
    void performICloudLogin();
  }, [performICloudLogin]);

  const signOutICloud = useCallback(async () => {
    try { await invoke('icloud_sign_out'); } catch { /* noop */ }
    setICloudSession(null);
    setICloudStatus('idle');
    setICloudInfo('');
    setICloudItems([]);
    setICloudLoadCount(0);
    setICloudLoading(false);
    setICloudAlbumMap(new Map());
    icloudAlbumListeners.current.forEach(fn => { try { fn(); } catch { /* noop */ } });
    icloudAlbumListeners.current = [];
  }, []);

  const loadICloudLibrary = useCallback(async () => {
    setICloudLoading(true);
    setICloudLoadCount(0);
    // Reset à vide avant de streamer.
    setICloudItems([]);
    const mapAsset = (a: ICloudAssetRaw): AfcMediaItem => ({
      objectId: `icloud:${a.recordName}`,
      filename: a.filename,
      extension: a.extension,
      isVideo: a.isVideo,
      // Le backend Rust classifie chaque asset (WhatsApp, Captures d'écran,
      // Live Photos, Caméra, etc.) via `classify_folder`. On préfixe par
      // "iCloud · " pour distinguer visuellement des dossiers AFC qui
      // pourraient avoir le même nom (ex: "Captures d'écran" via PhotoData).
      folder: a.folder ? `iCloud · ${a.folder}` : 'iCloud',
      sizeBytes: a.sizeBytes,
      mtimeNs: a.dateCreatedMs > 0 ? a.dateCreatedMs * 1_000_000 : 0,
      source: 'icloud' as const,
      thumbUrl: a.thumbUrl,
      originalUrl: a.originalUrl,
      recordName: a.recordName,
    });
    const pendingItems: AfcMediaItem[] = [];
    let itemsFlushTimer: number | null = null;
    const flushItems = () => {
      itemsFlushTimer = null;
      if (pendingItems.length === 0) return;
      const drained = pendingItems.splice(0, pendingItems.length);
      startTransition(() => {
        setICloudItems(prev => prev.concat(drained));
      });
    };
    const batchUnlisten = await listen<ICloudAssetRaw[]>('icloud-assets-batch', e => {
      const batch = (e.payload || []).map(mapAsset);
      if (batch.length === 0) return;
      pendingItems.push(...batch);
      if (itemsFlushTimer === null) {
        itemsFlushTimer = window.setTimeout(flushItems, 450);
      }
    });
    // Phase 2 : on reset le mapping albums au début d'un nouveau scan, et
    // on l'enrichit en arrière-plan au fur et à mesure que Rust nous envoie
    // les memberships par batch.
    setICloudAlbumMap(new Map());
    type AlbumAssignment = { assetRecordName: string; albumName: string };
    // Coalescing des batches : Apple nous envoie un event par album/smart
    // album terminé (parfois 50+ events en moins d'une seconde). Chaque
    // setICloudAlbumMap déclenche un re-render + recompute du Memo
    // `filtered`/`albums` (12k items). On accumule dans une ref et on
    // flush une seule fois par fenêtre de 250 ms — passe de ~50 re-renders
    // à ~5 sans perdre en réactivité visuelle.
    const pendingAssignments: AlbumAssignment[] = [];
    let flushTimer: number | null = null;
    const flushPending = () => {
      flushTimer = null;
      if (pendingAssignments.length === 0) return;
      const drained = pendingAssignments.splice(0, pendingAssignments.length);
      startTransition(() => {
        setICloudAlbumMap(prev => {
          const next = new Map(prev);
          for (const a of drained) {
            if (!a.assetRecordName) continue;
            const set = next.get(a.assetRecordName);
            if (set) {
              set.add(a.albumName);
            } else {
              next.set(a.assetRecordName, new Set([a.albumName]));
            }
          }
          return next;
        });
      });
    };
    const albumsUnlisten = await listen<AlbumAssignment[]>('icloud-album-assignments', e => {
      const batch = e.payload || [];
      if (batch.length === 0) return;
      pendingAssignments.push(...batch);
      if (flushTimer === null) {
        flushTimer = window.setTimeout(flushPending, 700);
      }
    });
    const albumsDoneUnlisten = await listen<number>('icloud-albums-done', () => {
      // Flush immédiat des assignements en attente pour que le dropdown
      // d'albums affiche le bon état final dès la fin du scan.
      if (flushTimer !== null) { window.clearTimeout(flushTimer); flushTimer = null; }
      flushPending();
    });
    try {
      const assets = await invoke<ICloudAssetRaw[]>('icloud_list_photos');
      // Flush en attente avant de remplacer par la version autoritative.
      if (itemsFlushTimer !== null) { window.clearTimeout(itemsFlushTimer); itemsFlushTimer = null; }
      pendingItems.length = 0;
      const mapped: AfcMediaItem[] = assets.map(mapAsset);
      startTransition(() => {
        setICloudItems(mapped);
      });
    } catch (e) {
      const raw = e instanceof Error ? e.message : String(e);
      // Les messages côté Rust contiennent déjà un suffixe explicatif quand
      // ils sont actionnables (421, 401…). Sinon on en ajoute un.
      let hint = '';
      if (/Timeout/i.test(raw)) {
        hint = " — Le pont JS n'a pas répondu à temps. Reconnecte iCloud (croix puis bouton iCloud) et réessaie.";
      } else if (/ckdatabasews missing/i.test(raw)) {
        hint = ' — Apple n\'a pas renvoyé d\'URL CloudKit, ton compte n\'a peut-être pas iCloud Photos activé (Réglages → iCloud → Photos).';
      }
      setError(`iCloud : ${raw}${hint}`);
      startTransition(() => {
        setICloudItems([]);
      });
    } finally {
      setICloudLoading(false);
      batchUnlisten();
      // NB : on garde albumsUnlisten et albumsDoneUnlisten actifs car le
      // scan d'albums continue en arrière-plan après le retour de
      // icloud_list_photos. On les stocke dans une ref dédiée (jamais
      // nettoyée par le pipeline d'export) ; le mount-effect les détache à
      // l'unmount du composant et `signOutICloud` les détache aussi.
      icloudAlbumListeners.current.forEach(fn => { try { fn(); } catch { /* noop */ } });
      icloudAlbumListeners.current = [albumsUnlisten, albumsDoneUnlisten];
    }
  }, []);

  // Restore an existing iCloud session at mount + subscribe to lifecycle events.
  useEffect(() => {
    let alive = true;
    const unlisteners: UnlistenFn[] = [];

    (async () => {
      try {
        const existing = await invoke<{
          apple_id?: string | null;
          full_name?: string | null;
          photos_url?: string | null;
          authenticated_at_ms: number;
        } | null>('get_icloud_session');
        if (alive && existing) {
          setICloudSession({
            appleId: existing.apple_id ?? null,
            fullName: existing.full_name ?? null,
            photosUrl: existing.photos_url ?? null,
            authenticatedAtMs: existing.authenticated_at_ms,
          });
          setICloudStatus('ready');
        }
      } catch { /* noop */ }

      unlisteners.push(await listen('icloud-session-pending', () => {
        if (!alive) return;
        setICloudStatus('connecting');
        setICloudInfo(copy.icloudConnecting);
      }));

      unlisteners.push(await listen<{
        apple_id?: string | null;
        full_name?: string | null;
        photos_url?: string | null;
        authenticated_at_ms: number;
      }>('icloud-session-ready', e => {
        if (!alive) return;
        setICloudSession({
          appleId: e.payload.apple_id ?? null,
          fullName: e.payload.full_name ?? null,
          photosUrl: e.payload.photos_url ?? null,
          authenticatedAtMs: e.payload.authenticated_at_ms,
        });
        setICloudStatus('ready');
        setICloudInfo('');
        // Auto-load library
        loadICloudLibrary();
      }));

      unlisteners.push(await listen<number>('icloud-list-progress', e => {
        if (!alive) return;
        startTransition(() => {
          setICloudLoadCount(e.payload ?? 0);
        });
      }));

      unlisteners.push(await listen('icloud-session-cancelled', () => {
        if (!alive) return;
        setICloudStatus('idle');
        setICloudInfo(copy.icloudCancelled);
      }));

      unlisteners.push(await listen('icloud-session-timeout', () => {
        if (!alive) return;
        setICloudStatus('error');
        setICloudInfo(copy.icloudTimeout);
      }));
    })();

    return () => {
      alive = false;
      unlisteners.forEach(fn => fn());
      // Détache aussi les listeners albums iCloud (scan en arrière-plan)
      icloudAlbumListeners.current.forEach(fn => { try { fn(); } catch { /* noop */ } });
      icloudAlbumListeners.current = [];
    };
  }, [copy.icloudCancelled, copy.icloudConnecting, copy.icloudTimeout, loadICloudLibrary]);

  const closeDone = useCallback(() => { setDone(null); setExportPhase('idle'); }, []);

  // Cleanup on unmount
  useEffect(() => () => { cleanListeners(); }, [cleanListeners]);

  // ── Delete ───────────────────────────────────────────────────────────────

  const requestDeleteSelected = useCallback(() => {
    if (selected.size === 0) return;
    setDeleteConfirm({
      ids: [...selected],
      label: trText(copy, 'selectedDeleteLabel', { count: fr(selected.size), plural: selected.size > 1 ? 's' : '' }),
    });
  }, [selected, copy]);

  const requestDeleteAlbum = useCallback((albumLabel: string, base: ViewItem[]) => {
    const ids = base.filter(i => i.folder === albumLabel).map(i => i.id);
    if (ids.length === 0) return;
    setDeleteConfirm({ ids, label: trText(copy, 'albumDeleteLabel', { album: albumLabel }) });
  }, [copy]);

  const confirmDelete = useCallback(async () => {
    if (!deleteConfirm) return;
    setDeleting(true);
    try {
      await invoke<number>('delete_afc_items', { udid: udid ?? null, objectIds: deleteConfirm.ids });
      const idSet = new Set(deleteConfirm.ids);
      setItems(prev => prev.filter(item => !idSet.has(item.objectId)));
      setSelected(prev => { const n = new Set(prev); idSet.forEach(id => n.delete(id)); return n; });
      setDeleteConfirm(null);
    } catch (err) {
      setError(String(err));
      setDeleteConfirm(null);
    } finally {
      setDeleting(false);
    }
  }, [deleteConfirm, udid]);

  // ── Render ───────────────────────────────────────────────────────────────

  const base = galleryType === 'mixed' ? mixed : galleryType === 'photos' ? photos : videos;
  const galleryLabel = galleryType === 'mixed' ? copy.mixed : galleryType === 'photos' ? copy.photos : copy.videos;

  return (
    <div className="pbx-root">

      {/* ── Modales ─────────────────────────────────────────────────────── */}
      <ExportProgressModal phase={exportPhase} progress={progress} onCancel={cancelExport} />

      {exportPhase === 'paused' && paused && (
        <DisconnectModal paused={paused} udid={udid} onResume={resumeExport} onCancel={cancelExport} />
      )}

      {exportPhase === 'done' && done && (
        <SuccessModal done={done} onOpenFolder={openFolder} onOpenICloud={openICloud} onClose={closeDone} />
      )}

      {deleteConfirm && (
        <DeleteModal confirm={deleteConfirm} onConfirm={confirmDelete} onCancel={() => setDeleteConfirm(null)} deleting={deleting} />
      )}

      {showPreflight && (
        <ICloudPreflightModal
          onContinue={onPreflightContinue}
          onCancel={onPreflightCancel}
          onSkipForever={onPreflightSkipForever}
        />
      )}

      {/* ── Sidebar ─────────────────────────────────────────────────────── */}
      <aside className="pbx-sidebar">
        <button className={`pbx-nav ${galleryType === 'mixed' ? 'pbx-nav--active' : ''}`}
          onClick={() => { setGalleryType('mixed'); setActiveAlbum('__all'); }}>
          <IconFolder /> <span>{copy.mixed}</span> <strong>{fr(mixed.length)}</strong>
        </button>
        <button className={`pbx-nav ${galleryType === 'photos' ? 'pbx-nav--active' : ''}`}
          onClick={() => { setGalleryType('photos'); setActiveAlbum('__all'); }}>
          <IconImage /> <span>{copy.photos}</span> <strong>{fr(photos.length)}</strong>
        </button>
        <button className={`pbx-nav ${galleryType === 'videos' ? 'pbx-nav--active' : ''}`}
          onClick={() => { setGalleryType('videos'); setActiveAlbum('__all'); }}>
          <IconVideo /> <span>{copy.videos}</span> <strong>{fr(videos.length)}</strong>
        </button>

        <div className="pbx-side-title">{copy.albums}</div>
        <button className={`pbx-album ${activeAlbum === '__all' ? 'pbx-album--active' : ''}`}
          onClick={() => setActiveAlbum('__all')}>
          <span>{copy.all}</span><strong>{fr(base.length)}</strong>
        </button>
        {albums.map(album => (
          <div key={album.name} className="pbx-album-row">
            <button className={`pbx-album pbx-album--flex ${activeAlbum === album.name ? 'pbx-album--active' : ''}`}
              onClick={() => setActiveAlbum(album.name)}>
              <span>{album.name}</span><strong>{fr(album.count)}</strong>
            </button>
            <button className="pbx-album-delete" title={trText(copy, 'albumDeleteTitle', { album: album.name })}
              onClick={e => { e.stopPropagation(); requestDeleteAlbum(album.name, base); }}>
              <IconTrash size={12} />
            </button>
          </div>
        ))}

        {/* ── Stockage ──────────────────────────────────────────────────── */}
        {(mode === 'ready' || icloudItems.length > 0 || icloudLoading) && (
          <div className="pbx-storage">
            <div className="pbx-storage-header">
              <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
                <ellipse cx="12" cy="5" rx="9" ry="3"/><path d="M3 5v14c0 1.66 4.03 3 9 3s9-1.34 9-3V5"/>
                <path d="M3 12c0 1.66 4.03 3 9 3s9-1.34 9-3"/>
              </svg>
              {copy.storage}
              {sizesProgress.total > 0 && sizesProgress.done < sizesProgress.total && (
                <span className="pbx-storage-loading" title={`${copy.readingSizes} : ${fr(sizesProgress.done)}/${fr(sizesProgress.total)}`}>
                  <span className="loading loading-spinner loading-xs" />
                </span>
              )}
            </div>

            {/* iPhone disk usage */}
            {diskUsage && diskUsage.totalBytes > 0 && (() => {
              const usedPct = ((diskUsage.totalBytes - diskUsage.freeBytes) / diskUsage.totalBytes) * 100;
              const usedBytes = diskUsage.totalBytes - diskUsage.freeBytes;
              return (
                <>
                  <div className="pbx-iphone-disk">
                    <div className="pbx-iphone-capacity">
                      <div>
                        <span>{copy.capacity}</span>
                        <strong>{marketedCapacity(diskUsage.totalBytes, locale)}</strong>
                      </div>
                      <div>
                        <span>{copy.free}</span>
                        <strong>{formatSize(diskUsage.freeBytes, locale)}</strong>
                      </div>
                    </div>
                    <div className="pbx-iphone-disk-label">
                      <span>iPhone</span>
                      <strong>{usedPct.toFixed(0)}% {copy.used}</strong>
                    </div>
                    <div className="pbx-iphone-disk-bar" title={`${usedPct.toFixed(0)}% utilisé`}>
                      <div className="pbx-iphone-disk-fill" style={{ width: `${usedPct}%` }} />
                    </div>
                    <div className="pbx-iphone-disk-total">
                      {trText(copy, 'usedOn', { used: formatSize(usedBytes, locale), total: formatSize(diskUsage.totalBytes, locale) })}
                    </div>
                  </div>
                  <div className="pbx-storage-divider" />
                </>
              );
            })()}

            {/* Photo/vidéo breakdown (Go) */}
            <div className="pbx-storage-row">
              <span className="pbx-storage-dot pbx-storage-dot--photo" />
              <span className="pbx-storage-name">{copy.photos}</span>
              <span className="pbx-storage-val">
                {storageStats.photoBytes > 0 ? formatSize(storageStats.photoBytes, locale) : '—'}
              </span>
            </div>
            <div className="pbx-storage-row">
              <span className="pbx-storage-dot pbx-storage-dot--video" />
              <span className="pbx-storage-name">{copy.videos}</span>
              <span className="pbx-storage-val">
                {storageStats.videoBytes > 0 ? formatSize(storageStats.videoBytes, locale) : '—'}
              </span>
            </div>

            {storageStats.totalBytes > 0 && (
              <>
                <div className="pbx-storage-bar" title={`${copy.photos} ${storageStats.photoPct.toFixed(0)}% · ${copy.videos} ${storageStats.videoPct.toFixed(0)}%`}>
                  <div className="pbx-storage-bar-photo" style={{ width: `${storageStats.photoPct}%` }} />
                  <div className="pbx-storage-bar-video" style={{ width: `${storageStats.videoPct}%` }} />
                </div>
                <div className="pbx-storage-total">
                  <span>{copy.mediaTotal}</span>
                  <strong>{formatSize(storageStats.totalBytes, locale)}</strong>
                </div>
              </>
            )}
          </div>
        )}
      </aside>

      {/* ── Main ────────────────────────────────────────────────────────── */}
      <main className="pbx-main">
        <header className="pbx-header">
          <div className="pbx-header-left">
            {onGoHome && (
              <button className="pbx-btn pbx-btn--home" onClick={onGoHome} title={copy.homeTitle}>
                <IconHome size={15} /> {copy.home}
              </button>
            )}
            <div>
              <h1>{galleryType === 'mixed' ? copy.library : galleryLabel}</h1>
              <p>{copy.directRead}</p>
            </div>
          </div>
          <DateRangeFilter
            fromDate={fromDate}
            toDate={toDate}
            activePreset={datePreset}
            availableYears={availableYears}
            onPreset={applyPreset}
            onFrom={onChangeFrom}
            onTo={onChangeTo}
            onPickYear={applyYear}
            hasDates={hasAnyDate}
            datesProgress={datesProgress}
            copy={copy}
            locale={locale}
          />
        </header>

        <div className="pbx-toolbar">
          <div className="pbx-seg">
            <button className={sortMode === 'newest' ? 'on' : ''} onClick={() => setSortMode('newest')}>{copy.newest}</button>
            <button className={sortMode === 'oldest' ? 'on' : ''} onClick={() => setSortMode('oldest')}>{copy.oldest}</button>
          </div>
          <div className="pbx-seg">
            <button className={thumbSize === 'large'  ? 'on' : ''} onClick={() => setThumbSize('large')}>{copy.large}</button>
            <button className={thumbSize === 'medium' ? 'on' : ''} onClick={() => setThumbSize('medium')}>{copy.medium}</button>
            <button className={thumbSize === 'small'  ? 'on' : ''} onClick={() => setThumbSize('small')}>{copy.small}</button>
          </div>
          <button className="pbx-btn pbx-btn--primary" onClick={exportSelected}
            disabled={selected.size === 0 || exportPhase !== 'idle'}>
            <IconDownload /> {copy.export}{selected.size ? ` (${fr(selected.size)})` : ''}
          </button>
          {selected.size > 0 && (
            <button className="pbx-btn pbx-btn--danger-outline" onClick={requestDeleteSelected}>
              <IconTrash /> {copy.delete} ({fr(selected.size)})
            </button>
          )}
          {icloudStatus === 'ready' && icloudSession ? (
            <div className="pbx-icloud-chip" title={icloudSession.appleId ?? ''}>
              <IconCloud size={14} />
              <span>{trText(copy, 'icloudConnectedAs', {
                user: icloudSession.fullName || icloudSession.appleId || copy.icloudConnected,
              })}</span>
              <button className="pbx-icloud-chip__close" onClick={signOutICloud} title={copy.icloudSignOut}>×</button>
            </div>
          ) : icloudStatus === 'connecting' ? (
            <div className="pbx-icloud-chip pbx-icloud-chip--loading" title={copy.icloudConnecting}>
              <span className="loading loading-spinner loading-xs" />
              <span>{icloudInfo || copy.icloudConnecting}</span>
            </div>
          ) : (
            <button
              className="pbx-btn pbx-btn--icloud"
              onClick={openICloud}
              title={copy.icloudTitle}
            >
              <IconCloud size={16} /> {copy.icloud}
            </button>
          )}
          <button className="pbx-btn" onClick={refresh}><IconRefresh /> {copy.refresh}</button>
        </div>

        <div className="pbx-countbar">
          <span>
            <strong>{fr(filtered.length)}</strong> {galleryLabel.toLowerCase()}
            {visible.length < filtered.length ? ` · ${fr(visible.length)} ${copy.displayed}` : ''}
            {selected.size > 0 && <span className="pbx-sel-badge"> · {fr(selected.size)} {selected.size > 1 ? copy.selectedPlural : copy.selected}</span>}
          </span>
          <button onClick={selected.size === filtered.length && filtered.length > 0 ? () => setSelected(new Set()) : selectAll}>
            {selected.size === filtered.length && filtered.length > 0
              ? copy.deselectAll
              : `${copy.selectAll}${filtered.length > 0 ? ` (${fr(filtered.length)})` : ''}`}
          </button>
        </div>

        {/* Erreurs transitoires uniquement (export, suppression) — pas les erreurs AFC de démarrage */}
        {error && mode !== 'error' && (
          <div className="pbx-message pbx-message--err">{error}<button onClick={() => setError('')}>×</button></div>
        )}
        {icloudInfo && (
          <div className="pbx-message pbx-message--info">
            {icloudInfo}<button onClick={() => setICloudInfo('')}>×</button>
          </div>
        )}
        {icloudLoading && (
          <div className="pbx-icloud-stream" role="status" aria-live="polite">
            <div className="pbx-icloud-stream__orb" aria-hidden="true">
              <span />
            </div>
            <div className="pbx-icloud-stream__body">
              <div className="pbx-icloud-stream__top">
                <strong>{copy.icloudScanTitle}</strong>
                <span>
                  {Math.max(icloudItems.length, icloudLoadCount) > 0
                    ? trText(copy, 'icloudScanSub', { count: fr(Math.max(icloudItems.length, icloudLoadCount)) })
                    : copy.icloudLoadingMedias}
                </span>
              </div>
              <div className="pbx-icloud-stream__track">
                <span className="pbx-icloud-stream__fill" />
              </div>
            </div>
          </div>
        )}

        {mode === 'loading' && icloudItems.length === 0 && !icloudLoading && (
          <div className="pbx-state"><span className="loading loading-spinner loading-lg" /> {copy.loadingLibrary}</div>
        )}
        {/* L'écran "iPhone non détecté" ne s'affiche que si on n'a PAS d'items
            iCloud à montrer ET pas de scan iCloud en cours. iCloud étant un
            service cloud, il doit fonctionner sans téléphone branché. */}
        {mode === 'error' && icloudItems.length === 0 && !icloudLoading && (
          <div className="pbx-waiting">
            <div className="pbx-waiting-icon-wrap">
              <span className="pbx-waiting-ring pbx-waiting-ring--1" />
              <span className="pbx-waiting-ring pbx-waiting-ring--2" />
              <span className="pbx-waiting-ring pbx-waiting-ring--3" />
              <div className="pbx-waiting-phone">
                <PhoneUsbDock phase="unplugged" />
              </div>
            </div>
            <h2 className="pbx-waiting-title">{copy.waitingTitle}</h2>
            <p className="pbx-waiting-sub">{copy.waitingSub}</p>
            <p className="pbx-waiting-hint">{copy.waitingHint}</p>
            <button className="pbx-btn pbx-btn--primary" onClick={refresh}>
              <IconRefresh size={15} /> {copy.retry}
            </button>
          </div>
        )}
        {/* La grille apparaît dès qu'il y a des items à montrer (AFC ou
            iCloud). Important : on n'attend plus `mode === 'ready'` car ça
            bloquait l'affichage des photos iCloud quand l'iPhone est
            débranché. */}
        {(mode === 'ready' || icloudItems.length > 0) && (
          <div className={`pbx-grid pbx-grid--${thumbSize}`}>
            {visible.map(item => (
              <MediaTile key={item.id} item={item} selected={selected.has(item.id)}
                size={thumbSize} onToggle={toggleSelect} onOpenPreview={openPreview} getThumbnail={getThumbnail} />
            ))}
            {visible.length < filtered.length && (
              <button className="pbx-more" onClick={() => setRenderLimit(n => n + 700)}>
                {copy.loadMore}
                <span>{fr(visible.length)} / {fr(filtered.length)}</span>
              </button>
            )}
          </div>
        )}

      </main>
    </div>
  );
}
