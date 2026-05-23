import { useCallback, useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { BrandHeader } from './components/BrandHeader';
import { CreatorNavActions } from './components/CreatorNavActions';
import { LanguageSelect } from './components/LanguageSelect';
import { PanicLogSourceModal } from './components/PanicLogSourceModal';
import { PanicLogsTable } from './components/PanicLogsTable';
import {
  ConnectedDeviceInfoModal,
  type DeviceInfoField,
  type IphoneDeviceDetailsPayload,
} from './components/ConnectedDeviceInfoModal';
import { UsbCenterStage } from './components/UsbCenterStage';
import { InsightDetailBar } from './components/InsightDetailBar';
import { PanicOverviewCard } from './components/PanicOverviewCard';
import { IpsIngestHero, IPS_FILE_ACCEPT } from './components/IpsIngestHero';
import { PanicReferenceEnrichedPanel } from './components/PanicReferenceEnrichedPanel';
import { ReferenceActionButton } from './components/ReferenceActionButton';
import type { AnalysisResult } from './types/analysis';
import { useI18n } from './i18n/context';
import { playConnectChime } from './lib/connectChime';
import { TransferHub } from './components/TransferHub';

type IpsInterpretOutcome = {
  analysis: AnalysisResult;
  panicText: string;
  extractionMethod: string;
  deviceHint: string | null;
};

type IphoneUsbStatus = {
  phase: string;
  detail: string;
  udids: string[];
  marketingName: string | null;
  productType: string | null;
  iosVersion: string | null;
  recoverySerial: string | null;
  recoveryImei: string | null;
  recoveryEcid: string | null;
};

const INITIAL_USB: IphoneUsbStatus = {
  phase: 'unplugged',
  detail: '',
  udids: [],
  marketingName: null,
  productType: null,
  iosVersion: null,
  recoverySerial: null,
  recoveryImei: null,
  recoveryEcid: null,
};

type PanicPullListResponse = {
  count: number;
  totalDownloaded: number;
  message: string;
  logs: Array<{
    index: number;
    filename: string;
    modifiedLabel: string;
    snippet: string;
  }>;
};

type PulledPanicDetailResponse = {
  panicText: string;
  analysis: AnalysisResult;
};

/** Détection branchement / débranchement USB : court pour rester réactif (voir timeouts Rust côté idevice_id). */
const POLL_MS = 450;
const DEVICE_PULL_MAX = 5;

function displayModelShort(marketingName: string | null, productType: string | null): string | null {
  if (marketingName) {
    const t = marketingName.replace(/^iPhone\s+/i, '').trim();
    return t || marketingName;
  }
  return productType;
}

/** Titre recovery : nom commercial complet ou identifiant matériel (ex. iPhone 17 Pro, iPhone18,1). */
function displayRecoveryModelFull(marketingName: string | null, productType: string | null): string {
  const m = marketingName?.trim();
  if (m) return m;
  const p = productType?.trim();
  if (p) {
    if (/^iphone/i.test(p)) return p;
    return `iPhone (${p})`;
  }
  return 'iPhone';
}

/** Ligne technique sous le mockup (ex. DFU) — masque les libellés redondants avec « recovery ». */
function recoveryTechCaption(mode: string | null | undefined): string | null {
  const raw = mode?.trim();
  if (!raw) return null;
  const up = raw.toUpperCase();
  if (up.includes('DFU')) return raw.length <= 18 ? raw : 'DFU';
  if (up.includes('RESTORE') || up.includes('UPDATE')) return raw.length <= 24 ? raw : null;
  if (/\bRECOVERY\b/i.test(raw)) return null;
  return raw.length <= 16 ? raw : null;
}

const emptyPull: PanicPullListResponse = { count: 0, totalDownloaded: 0, message: '', logs: [] };

function fmtPct01(x: number) {
  return Math.round(Math.max(0, Math.min(1, x)) * 100);
}

function structuredExportTail(sd: AnalysisResult['structured_diagnostic']): string {
  return [
    '',
    '--- PanicBase (compact) ---',
    `global_score: ~${fmtPct01(sd.confidence_global)}%`,
    ...(sd.possible_causes?.length
      ? sd.possible_causes.map((c) => `- ${c.name} · ${fmtPct01(c.confidence)}%`)
      : ['- (no cause)']),
  ].join('\n');
}

// ── NavViewToggle ─────────────────────────────────────────────────────────────

function NavViewToggle({ active, onToggle }: { active: boolean; onToggle: () => void }) {
  const { t } = useI18n();
  return (
    <button
      type="button"
      onClick={onToggle}
      aria-pressed={active}
      title={active ? t('nav.transfer.backTitle') : t('nav.transfer.openTitle')}
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        gap: '7px',
        padding: '0 14px',
        height: '32px',
        borderRadius: '9999px',
        border: active
          ? '1px solid rgba(234, 88, 12, 0.55)'
          : '1px solid rgba(15, 23, 42, 0.16)',
        background: active
          ? 'linear-gradient(135deg, rgba(249, 115, 22, 0.16) 0%, rgba(234, 88, 12, 0.10) 100%)'
          : 'linear-gradient(135deg, #ffffff 0%, rgba(248, 250, 252, 0.85) 100%)',
        color: active ? '#c2410c' : 'rgba(31, 41, 55, 0.85)',
        cursor: 'pointer',
        fontFamily: 'inherit',
        fontSize: '12.5px',
        fontWeight: 600,
        letterSpacing: '0.005em',
        whiteSpace: 'nowrap',
        transition: 'background 0.2s, border-color 0.2s, color 0.2s, box-shadow 0.2s',
        boxShadow: active
          ? '0 0 0 1px rgba(249, 115, 22, 0.18) inset, 0 6px 18px -10px rgba(249, 115, 22, 0.35)'
          : '0 1px 2px rgba(15, 23, 42, 0.04), 0 0 0 1px rgba(15, 23, 42, 0.02) inset',
        flexShrink: 0,
      }}
    >
      {active ? (
        <>
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
            <path d="M19 12H5" /><path d="m12 19-7-7 7-7" />
          </svg>
          {t('nav.transfer.back')}
        </>
      ) : (
        <>
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
            <rect x="3" y="4" width="18" height="16" rx="2.5" />
            <circle cx="8.5" cy="9.5" r="1.6" fill="currentColor" stroke="none" />
            <path d="M4 18l5-5.5 4 4 2.5-2.5L20 18" />
          </svg>
          {t('nav.transfer.open')}
        </>
      )}
    </button>
  );
}

function App() {
  const { t, locale } = useI18n();
  const fileInputRef = useRef<HTMLInputElement>(null);
  const didAutoPullRef = useRef(false);
  const prevUsbPhaseRef = useRef<string | undefined>(undefined);
  const usbPollInFlightRef = useRef(false);
  /** Préchargement identifiants lockdown (même UDID → ouverture modal instantanée). */
  const identifiersCacheRef = useRef<{ udid: string; data: IphoneDeviceDetailsPayload } | null>(null);

  const [usb, setUsb] = useState<IphoneUsbStatus>(() => ({ ...INITIAL_USB }));
  const [connectCelebrationCount, setConnectCelebrationCount] = useState(0);
  const [deviceHintFromUsb, setDeviceHintFromUsb] = useState<string | null>(null);
  const [log, setLog] = useState('');
  const [analysis, setAnalysis] = useState<AnalysisResult | null>(null);
  const [error, setError] = useState('');
  const [extractStatus, setExtractStatus] = useState<string | null>(null);
  const extractStatusTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const showExtractStatus = useCallback((msg: string) => {
    if (extractStatusTimerRef.current !== null) clearTimeout(extractStatusTimerRef.current);
    setExtractStatus(msg);
    extractStatusTimerRef.current = setTimeout(() => setExtractStatus(null), 4000);
  }, []);
  const [ipsImportActive, setIpsImportActive] = useState(false);
  const [ipsFileName, setIpsFileName] = useState<string | null>(null);
  const [deviceHintFromIps, setDeviceHintFromIps] = useState<string | null>(null);
  const [ipsLogViewerOpen, setIpsLogViewerOpen] = useState(false);
  const [ipsUiDepth, setIpsUiDepth] = useState<'overview' | 'detail'>('overview');
  const [pullWorkspace, setPullWorkspace] = useState<'idle' | 'loading' | 'ready'>('idle');
  const [pullResult, setPullResult] = useState<PanicPullListResponse | null>(null);
  const [pullSelected, setPullSelected] = useState<number | null>(null);
  const [pullDetail, setPullDetail] = useState<PulledPanicDetailResponse | null>(null);
  const [pullAnalyzing, setPullAnalyzing] = useState(false);
  const [logViewerOpen, setLogViewerOpen] = useState(false);
  const [deviceInfoOpen, setDeviceInfoOpen] = useState(false);
  const [deviceInfoLoading, setDeviceInfoLoading] = useState(false);
  const [deviceInfoError, setDeviceInfoError] = useState('');
  const [deviceInfoData, setDeviceInfoData] = useState<IphoneDeviceDetailsPayload | null>(null);
  const [exitRecoveryBootBusy, setExitRecoveryBootBusy] = useState(false);
  const [activeView, setActiveView] = useState<'analysis' | 'transfer'>('analysis');

  useEffect(() => {
    document.title = t('app.htmlTitle');
  }, [t, locale]);

  useEffect(() => {
    if (usb?.phase !== 'connected' && usb?.phase !== 'recovery') {
      setLogViewerOpen(false);
      setDeviceInfoOpen(false);
      setDeviceInfoLoading(false);
      setDeviceInfoError('');
      setDeviceInfoData(null);
    }
    if (usb?.phase !== 'connected') {
      identifiersCacheRef.current = null;
    }
  }, [usb?.phase]);

  const udidForIdentifiers = usb.phase === 'connected' ? usb.udids[0] ?? null : null;

  useEffect(() => {
    if (!udidForIdentifiers) {
      identifiersCacheRef.current = null;
      return;
    }
    let cancelled = false;
    void invoke<IphoneDeviceDetailsPayload>('get_iphone_device_identifiers', { udid: udidForIdentifiers })
      .then((data) => {
        if (!cancelled) identifiersCacheRef.current = { udid: udidForIdentifiers, data };
      })
      .catch(() => {
        if (!cancelled) identifiersCacheRef.current = null;
      });
    return () => {
      cancelled = true;
    };
  }, [udidForIdentifiers]);

  const fetchIphoneDeviceIdentifiers = useCallback(async () => {
    setDeviceInfoLoading(true);
    setDeviceInfoError('');
    try {
      const udid = usb.udids[0] ?? null;
      const data = await invoke<IphoneDeviceDetailsPayload>('get_iphone_device_identifiers', { udid });
      setDeviceInfoData(data);
      if (udid) identifiersCacheRef.current = { udid, data };
    } catch (err) {
      setDeviceInfoError(String(err));
      setDeviceInfoData(null);
    } finally {
      setDeviceInfoLoading(false);
    }
  }, [usb.udids]);

  const refreshDevicePanel = useCallback(async () => {
    if (usb.phase === 'recovery') {
      setDeviceInfoLoading(true);
      setDeviceInfoError('');
      try {
        const status = await invoke<IphoneUsbStatus>('detect_iphone');
        setUsb({ ...INITIAL_USB, ...status });
        if (status.phase === 'recovery') {
          const fields: DeviceInfoField[] = [];
          const imeiR = status.recoveryImei?.trim();
          if (imeiR) fields.push({ id: 'InternationalMobileEquipmentIdentity', value: imeiR });
          const sn = status.recoverySerial?.trim();
          if (sn) fields.push({ id: 'SerialNumber', value: sn });
          const ecid = status.recoveryEcid?.trim();
          if (ecid) fields.push({ id: 'ECID', value: ecid });
          setDeviceInfoData({
            fields,
            udid: null,
            hint: fields.length ? t('deviceInfo.recoveryReadHint') : t('deviceInfo.recoveryNoSn'),
          });
        }
      } catch (err) {
        setDeviceInfoError(String(err));
      } finally {
        setDeviceInfoLoading(false);
      }
      return;
    }
    void fetchIphoneDeviceIdentifiers();
  }, [usb.phase, fetchIphoneDeviceIdentifiers, t]);

  const openConnectedDeviceInfo = useCallback(() => {
    setDeviceInfoOpen(true);
    if (usb.phase === 'recovery') {
      setDeviceInfoError('');
      const fields: DeviceInfoField[] = [];
      const imeiR = usb.recoveryImei?.trim();
      if (imeiR) fields.push({ id: 'InternationalMobileEquipmentIdentity', value: imeiR });
      const sn = usb.recoverySerial?.trim();
      if (sn) fields.push({ id: 'SerialNumber', value: sn });
      const ecid = usb.recoveryEcid?.trim();
      if (ecid) fields.push({ id: 'ECID', value: ecid });
      setDeviceInfoData({
        fields,
        udid: null,
        hint: fields.length ? t('deviceInfo.recoveryReadHint') : t('deviceInfo.recoveryNoSn'),
      });
      setDeviceInfoLoading(false);
      return;
    }
    const udid = usb.udids[0] ?? null;
    const cached = identifiersCacheRef.current;
    if (cached && udid && cached.udid === udid) {
      setDeviceInfoData(cached.data);
      setDeviceInfoError('');
      setDeviceInfoLoading(false);
      return;
    }
    setDeviceInfoData(null);
    void fetchIphoneDeviceIdentifiers();
  }, [usb.phase, usb.recoveryEcid, usb.recoveryImei, usb.recoverySerial, usb.udids, fetchIphoneDeviceIdentifiers, t]);


  useEffect(() => {
    setIpsUiDepth('overview');
  }, [ipsFileName]);

  useEffect(() => {
    if (!ipsImportActive) {
      setIpsLogViewerOpen(false);
      setIpsUiDepth('overview');
    }
  }, [ipsImportActive]);

  const pollUsb = useCallback(() => {
    if (usbPollInFlightRef.current) return;
    usbPollInFlightRef.current = true;
    invoke<IphoneUsbStatus>('detect_iphone')
      .then((status) => {
        setUsb({ ...INITIAL_USB, ...status });
        if (status.phase === 'connected' && status.productType) {
          setDeviceHintFromUsb(status.productType);
        } else if (status.phase === 'recovery' && status.productType) {
          setDeviceHintFromUsb(status.productType);
        } else {
          setDeviceHintFromUsb(null);
        }
      })
      .catch(() => {
        setUsb({
          phase: 'error',
          detail: t('usb.detectBridgeError'),
          udids: [],
          marketingName: null,
          productType: null,
          iosVersion: null,
          recoverySerial: null,
          recoveryImei: null,
          recoveryEcid: null,
        });
        setDeviceHintFromUsb(null);
      })
      .finally(() => {
        usbPollInFlightRef.current = false;
      });
  }, [t]);

  const handleExitRecoveryBoot = useCallback(async () => {
    if (usb.phase !== 'recovery') return;
    setExitRecoveryBootBusy(true);
    setError('');
    try {
      await invoke<string>('exit_iphone_recovery_boot');
      await new Promise<void>((resolve) => {
        window.setTimeout(resolve, 1400);
      });
      pollUsb();
    } catch {
      setError(t('usb.exitRecoveryBootFailed'));
    } finally {
      setExitRecoveryBootBusy(false);
    }
  }, [usb.phase, pollUsb, t]);

  useEffect(() => {
    void pollUsb();
    const id = window.setInterval(() => void pollUsb(), POLL_MS);
    return () => window.clearInterval(id);
  }, [pollUsb]);

  useEffect(() => {
    const onVis = () => {
      if (document.visibilityState === 'visible') void pollUsb();
    };
    const onFocus = () => void pollUsb();
    document.addEventListener('visibilitychange', onVis);
    window.addEventListener('focus', onFocus);
    return () => {
      document.removeEventListener('visibilitychange', onVis);
      window.removeEventListener('focus', onFocus);
    };
  }, [pollUsb]);

  useEffect(() => {
    const phase = usb?.phase;
    const prev = prevUsbPhaseRef.current;
    if (phase === 'connected' && prev !== 'connected') {
      void playConnectChime();
      setConnectCelebrationCount((n) => n + 1);
    }
    prevUsbPhaseRef.current = phase;
  }, [usb?.phase]);

  useEffect(() => {
    if (usb?.phase !== 'connected') {
      didAutoPullRef.current = false;
      setPullWorkspace('idle');
      setPullResult(null);
      setPullSelected(null);
      setPullDetail(null);
      setPullAnalyzing(false);
      return;
    }

    if (didAutoPullRef.current) return;
    didAutoPullRef.current = true;
      setPullWorkspace('loading');
      setPullSelected(null);
      setPullDetail(null);
      setError('');
      let completed = false;
      let canceled = false;
      const guard = window.setTimeout(() => {
        if (completed || canceled) return;
        completed = true;
        setError(
          "La récupération des panic-full dépasse 10 secondes. PanicBase arrête l’attente pour éviter le blocage : débranche/rebranche l’iPhone, déverrouille-le, puis relance.",
        );
        setPullResult(emptyPull);
        setPullWorkspace('ready');
      }, 12000);

      invoke<PanicPullListResponse>('pull_device_recent_panic_logs', { udid: usb?.udids?.[0] ?? null })
        .then((r) => {
          if (completed || canceled) return;
          completed = true;
          window.clearTimeout(guard);
          setPullResult(r);
        })
        .catch((err) => {
          if (completed || canceled) return;
          completed = true;
          window.clearTimeout(guard);
          setError(String(err));
          setPullResult(emptyPull);
        })
        .finally(() => {
          if (completed && !canceled) setPullWorkspace('ready');
        });
      return () => {
        canceled = true;
        window.clearTimeout(guard);
      };
    }, [usb?.phase]);

  async function selectPulledPanic(index: number) {
    setError('');
    setIpsImportActive(false);
    setIpsFileName(null);
    setDeviceHintFromIps(null);
    setPullSelected(index);
    setPullAnalyzing(true);
    setPullDetail(null);
    try {
      const dh = deviceHintFromUsb ?? undefined;
      const detail = await invoke<PulledPanicDetailResponse>('analyze_pulled_device_panic', {
        index,
        device_hint: dh,
      });
      setPullDetail(detail);
      setLog(detail.panicText);
      setAnalysis(detail.analysis);
    } catch (err) {
      setError(String(err));
    } finally {
      setPullAnalyzing(false);
    }
  }

  async function interpretIpsContent(raw: string, fileName?: string) {
    setError('');
    try {
      const outcome = await invoke<IpsInterpretOutcome>('interpret_ips_file', { content: raw });
      setLog(outcome.panicText);
      setAnalysis(outcome.analysis);
      setDeviceHintFromIps(outcome.deviceHint ?? null);
      setIpsFileName(fileName ?? null);
      setIpsImportActive(true);
    } catch (err) {
      setError(String(err));
      setIpsFileName(null);
      setIpsImportActive(false);
    }
  }

  const clearImportedIps = useCallback(() => {
    setIpsImportActive(false);
    setIpsFileName(null);
    setDeviceHintFromIps(null);
    setIpsUiDepth('overview');
    setIpsLogViewerOpen(false);
    setLogViewerOpen(false);
    setError('');
    if (pullDetail != null && pullSelected !== null) {
      setLog(pullDetail.panicText);
      setAnalysis(pullDetail.analysis);
    } else {
      setLog('');
      setAnalysis(null);
    }
  }, [pullDetail, pullSelected]);

  function onIpsFileChange(e: React.ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0];
    e.target.value = '';
    if (!file) return;
    const reader = new FileReader();
    reader.onload = () => {
      const text = typeof reader.result === 'string' ? reader.result : '';
      void interpretIpsContent(text, file.name);
    };
    reader.readAsText(file, 'UTF-8');
  }

  async function exportPulledDeviceRawFile() {
    if (pullSelected === null || !pullResult) return;
    setError('');
    const row = pullResult.logs.find((l) => l.index === pullSelected);
    const defaultFilename = row?.filename ?? `panic-export-${pullSelected}.ips`;

    try {
      const saved = await invoke<string | null>('export_pulled_device_panic_file', {
        index: pullSelected,
        defaultFilename,
      });
      if (saved) {
        showExtractStatus(t('export.savedRaw', { path: saved }));
      }
    } catch (err) {
      setError(String(err));
    }
  }

  async function exportCurrentPanicOnPc() {
    if (!pullDetail || pullSelected === null) return;
    setError('');
    const row = pullResult?.logs.find((l) => l.index === pullSelected);
    const stem = row?.filename?.replace(/\.[^.]+$/i, '') ?? 'panic';
    const a = pullDetail.analysis;
    const body = [
      pullDetail.panicText,
      '',
      '--- PanicBase ---',
      a.device_model,
      a.panic_type,
      `score: ${fmtPct01(a.structured_diagnostic.confidence_global)}%`,
      ...(a.structured_diagnostic.possible_causes?.map(
        (c) => `${c.name} · ${fmtPct01(c.confidence)}%`,
      ) ?? []),
      structuredExportTail(a.structured_diagnostic),
    ].join('\n');

    try {
      const saved = await invoke<string | null>('export_text_file', {
        content: body,
        defaultFilename: `${stem}.txt`,
      });
      if (saved) {
        showExtractStatus(t('export.savedTxt', { path: saved }));
      }
    } catch (err) {
      setError(String(err));
    }
  }



  const centerPhase = usb.phase || 'unplugged';
  const modelShort =
    usb?.phase === 'connected' || usb?.phase === 'recovery'
      ? displayModelShort(usb.marketingName, usb.productType)
      : null;
  const recoveryModelFull =
    usb?.phase === 'recovery' ? displayRecoveryModelFull(usb.marketingName, usb.productType) : null;
  const recoveryTechLine =
    usb?.phase === 'recovery' ? recoveryTechCaption(usb.iosVersion) : null;

  const selectedPullFilename =
    pullSelected !== null ? pullResult?.logs.find((l) => l.index === pullSelected)?.filename ?? null : null;

  const hidePhoneDock = Boolean(ipsImportActive) && ipsUiDepth === 'detail';

  const hideIngestHero = Boolean(ipsImportActive) && ipsUiDepth === 'detail';

  /** iPhone utile en direct (connecté ou confiance) : bandeau centré en haut. Problèmes USB/outils → accueil classique (dock à droite). */
  const phoneFrontAndCenter =
    !hidePhoneDock &&
    (usb?.phase === 'connected' || usb?.phase === 'awaiting_trust' || usb?.phase === 'recovery');

  /** Connecté : moitié gauche (panic / IPS), moitié droite (animation iPhone). Débranché → accueil actuel. */
  const connectedSplitLayout = usb?.phase === 'connected' && !hidePhoneDock;

  /** Masque drop + collage tant qu'un rapport importé / collé est ouvert. */
  const showIngestHero = !hideIngestHero && !phoneFrontAndCenter && !ipsImportActive;

  const importedIpsDisplayName = ipsFileName ?? t('ips.pastedLogLabel');

  const usbStage = (
    <UsbCenterStage
      phase={centerPhase}
      modelShort={modelShort}
      recoveryModelFull={recoveryModelFull}
      recoveryTechLine={recoveryTechLine}
      iosVersion={usb?.phase === 'connected' || usb?.phase === 'recovery' ? usb.iosVersion : null}
      backendDetail={usb?.detail || undefined}
      connectCelebrationKey={centerPhase === 'connected' || centerPhase === 'recovery' ? connectCelebrationCount : 0}
      onConnectedPhoneClick={
        usb?.phase === 'connected' || usb?.phase === 'recovery' ? openConnectedDeviceInfo : undefined
      }
      onExitRecoveryBoot={usb?.phase === 'recovery' ? handleExitRecoveryBoot : undefined}
      exitRecoveryBootBusy={exitRecoveryBootBusy}
    />
  );

  const phoneDockAside = !hidePhoneDock ? (
    <aside
      className="relative z-[2] flex min-h-[220px] w-full shrink-0 flex-col items-stretch justify-center overflow-visible border-t border-base-content/10 py-4 lg:min-h-0 lg:max-w-none lg:flex-[0_0_clamp(400px,38vw,540px)] lg:justify-start lg:border-l lg:border-t-0 lg:py-3 lg:pl-4"
      aria-label={t('aria.phoneDock')}
    >
      <div className="flex w-full min-w-0 flex-1 items-center justify-center px-3 sm:px-4 lg:sticky lg:top-3 lg:max-h-[min(85vh,720px)]">
        {usbStage}
      </div>
    </aside>
  ) : null;

  function renderDeviceConnectedSection(splitMode: boolean) {
    if (usb?.phase !== 'connected') return null;
    return (
      <section
        className={`relative z-0 flex min-h-0 flex-col overflow-x-hidden ${
          splitMode ? 'min-h-0 shrink-0 flex-1 pt-0' : 'shrink-0 border-t border-base-content/10 pt-1'
        }`}
        aria-label={t('device.title')}
      >
        {pullWorkspace === 'loading' ? (
          <div className="flex flex-col items-center gap-2.5 px-2 py-2 pb-5">
            <div className="orbit-wrap orbit-inline" aria-busy="true">
              <div className="orbit-ring" />
              <div className="orbit-core" />
            </div>
            <p className="m-0 text-center text-sm font-semibold text-base-content/80">{t('device.loading')}</p>
          </div>
        ) : null}

        {pullWorkspace === 'ready' && pullResult ? (
          <div
            className={`card card-bordered border-base-300 bg-base-100/40 shadow-sm pb-workspace-card ${
              splitMode ? 'flex min-h-0 flex-1 flex-col' : ''
            }`}
          >
            <div
              className={`card-body min-h-0 overflow-x-hidden gap-3 overflow-y-auto p-3 sm:p-4 ${
                splitMode ? 'flex min-h-0 flex-1 flex-col' : ''
              }`}
            >
              <div
                className={`flex min-w-0 gap-2 ${
                  splitMode
                    ? 'flex-col'
                    : 'flex-col sm:flex-row sm:items-start sm:justify-between sm:gap-3'
                }`}
              >
                <div className="min-w-0 flex-1">
                  <h2 className="font-sora m-0 text-[0.9375rem] font-semibold normal-case tracking-normal text-base-content/80">
                    {t('device.title')}
                  </h2>
                  <p className="m-0 max-w-full font-sora text-[11px] leading-snug text-base-content/50 sm:text-[12px]">
                    {t('device.subtitle')}
                  </p>
                </div>
                {pullResult.count > 0 ? (
                  <p className="m-0 max-w-full shrink-0 break-words font-mono text-[10px] leading-relaxed text-base-content/45 sm:max-w-[min(100%,22rem)] sm:text-right">
                    {t('device.pullSummary', { count: String(pullResult.count), max: String(DEVICE_PULL_MAX), total: String(pullResult.totalDownloaded) })}
                  </p>
                ) : (
                  <p className="m-0 max-w-full shrink-0 break-words font-sora text-[10px] leading-relaxed text-base-content/45 sm:max-w-none">
                    {t('device.pullNoneLong')}
                  </p>
                )}
              </div>

              {pullResult.count > 0 ? (
                <PanicLogsTable
                  expanded={splitMode}
                  logs={pullResult.logs}
                  selectedIndex={pullSelected}
                  onSelect={(idx) => void selectPulledPanic(idx)}
                  disabled={pullAnalyzing}
                />
              ) : null}

              {pullResult.count > 0 && pullSelected === null && !pullAnalyzing ? (
                <p className="m-0 font-sora text-[12px] leading-relaxed text-base-content/55">{t('summary.pickRow')}</p>
              ) : null}

              {pullAnalyzing ? (
                <p className="m-0 flex items-center gap-2 border-t border-base-300 pt-3 text-xs font-bold text-info">
                  <span className="dot-pulse inline-block shrink-0" /> {t('loading')}
                </p>
              ) : null}
            </div>
          </div>
        ) : null}
      </section>
    );
  }

  function renderIpsImportSection() {
    if (!ipsImportActive || !analysis) return null;
    return (
      <section
        className={`ips-active-file relative z-0 flex min-h-0 flex-col overflow-hidden pb-workspace-card backdrop-blur-md ${
          ipsUiDepth === 'detail' ? 'max-h-[min(82dvh,860px)] min-h-[16rem] flex-1' : 'shrink-0'
        }`}
        aria-label={t('aria.ipsImportSection')}
      >
        <div
          className={`flex min-h-0 flex-1 flex-col gap-0 p-0 ${
            ipsUiDepth === 'detail' ? 'overflow-hidden' : 'overflow-x-hidden overflow-y-auto'
          }`}
        >
          {ipsUiDepth === 'overview' ? (
            <header className="pb-workspace-header shrink-0 border-b border-[#2a2a30] px-4 py-3.5">
              <div className="flex items-start justify-between gap-3">
                <div className="min-w-0 flex-1">
                  <p className="mb-0 flex flex-wrap items-center gap-2 font-sora text-[11px] font-medium text-base-content/55">
                    <span>{t('detail.fileContext')}</span>
                    <span className="rounded-md bg-base-300/40 px-1.5 py-0.5 text-[10px] text-base-content/50 dark:bg-white/[0.06]">
                      {t('ips.activeBadge')}
                    </span>
                  </p>
                  <p
                    className="mb-0 mt-2 min-w-0 break-all font-mono text-[13px] font-semibold leading-snug text-base-content sm:text-[14px]"
                    title={importedIpsDisplayName}
                  >
                    {importedIpsDisplayName}
                  </p>
                  <p className="font-sora mb-0 mt-1 text-[10px] font-normal tracking-normal text-base-content/42">
                    {t('ips.analyzedFile')}
                  </p>
                </div>
                <button
                  type="button"
                  className="btn btn-circle btn-sm btn-ghost shrink-0 border border-error/45 text-error hover:border-error hover:bg-error/15"
                  aria-label={t('ips.closeImport')}
                  title={t('ips.closeImport')}
                  onClick={clearImportedIps}
                >
                  ×
                </button>
              </div>
            </header>
          ) : null}
          <div
            className={`flex min-h-0 flex-1 flex-col ${
              ipsUiDepth === 'overview'
                ? 'overflow-x-hidden overflow-y-auto bg-base-200/25 px-3 pb-4 pt-3 sm:px-5'
                : 'min-h-[12rem] overflow-hidden bg-[#0a0a0b]'
            }`}
          >
            {ipsUiDepth === 'overview' ? (
              <div className="flex min-h-0 flex-1 flex-col gap-4">
                <PanicOverviewCard
                  analysis={analysis}
                  panicText={log}
                  productType={analysis.structured_diagnostic.device ?? null}
                />
                <div className="flex flex-wrap gap-2">
                  <button
                    type="button"
                    className="btn btn-outline btn-sm font-sora"
                    onClick={() => {
                      setLogViewerOpen(false);
                      setIpsLogViewerOpen(true);
                    }}
                    disabled={!log.trim()}
                  >
                    {t('device.viewFullLog')}
                  </button>
                </div>
              </div>
            ) : (
              <div className="flex min-h-0 flex-1 flex-col overflow-hidden">
                <InsightDetailBar
                  compact
                  referenceChrome
                  backLabel={t('ips.backOverview')}
                  onBack={() => setIpsUiDepth('overview')}
                  title={t('ips.detailTitle')}
                  fileContextLabel={t('detail.fileContext')}
                  fileName={importedIpsDisplayName}
                  onDismiss={clearImportedIps}
                  dismissAriaLabel={t('ips.closeImport')}
                >
                  <ReferenceActionButton
                    disabled={!log.trim()}
                    onClick={() => {
                      setLogViewerOpen(false);
                      setIpsLogViewerOpen(true);
                    }}
                  >
                    {t('device.viewFullLog')}
                  </ReferenceActionButton>
                </InsightDetailBar>
                <PanicReferenceEnrichedPanel immersive panicText={log} analysis={analysis} productType={null} />
              </div>
            )}
          </div>
        </div>
      </section>
    );
  }

  return (
    <main className="pb-app-in pb-shell relative z-10 mx-auto flex h-full min-h-0 w-full flex-col gap-0 overflow-hidden">
      <header className="navbar pb-navbar-clean relative z-50 mb-2 min-h-0 shrink-0 px-3 py-2.5 backdrop-blur-md sm:px-4">
        <div className="navbar-start min-w-0 flex-1">
          <BrandHeader />
        </div>
        <div className="navbar-end flex shrink-0 items-center gap-1.5 sm:gap-2">
          <NavViewToggle active={activeView === 'transfer'} onToggle={() => setActiveView(v => v === 'transfer' ? 'analysis' : 'transfer')} />
          <CreatorNavActions />
          <LanguageSelect />
        </div>
      </header>

      <input
        id="ips-ingest-file"
        ref={fileInputRef}
        type="file"
        accept={IPS_FILE_ACCEPT}
        className="sr-only"
        onChange={onIpsFileChange}
        aria-hidden="true"
        tabIndex={-1}
      />

      <div className={`min-h-0 flex-1 overflow-hidden ${activeView === 'transfer' ? 'flex' : 'hidden'}`}>
        <TransferHub udid={usb.phase === 'connected' ? (usb.udids[0] ?? null) : null} />
      </div>

      <div
        className={`flex min-h-0 flex-1 gap-0 overflow-hidden ${activeView === 'transfer' ? 'hidden' : ''} ${
          connectedSplitLayout
            ? 'flex-col lg:flex-row lg:items-stretch'
            : phoneFrontAndCenter
              ? 'flex-col'
              : 'flex-col lg:flex-row lg:items-stretch lg:gap-8'
        }`}
      >
        {connectedSplitLayout ? (
          <>
            <div className="flex min-h-0 min-w-0 flex-1 flex-col border-b border-base-content/10 lg:min-h-0 lg:basis-0 lg:border-b-0 lg:border-e lg:border-base-content/10 lg:pe-4">
              <div className="flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto subtle-scrollbar px-0 pb-2 pt-1 sm:px-1 lg:pt-2">
                {renderIpsImportSection()}
                {renderDeviceConnectedSection(true)}
                <footer className="shrink-0 px-2 py-3 text-center" aria-label={t('aria.footerCredit')}>
                  <p className="font-sora m-0 text-[10px] font-medium italic leading-relaxed tracking-[0.06em] text-base-content/42">
                    {t('footer')}
                  </p>
                </footer>
              </div>
            </div>
            <aside
              className="relative z-[2] flex min-h-[min(42vh,300px)] w-full shrink-0 flex-col items-center justify-center border-t border-base-content/10 bg-base-100/15 py-6 sm:py-8 lg:min-h-0 lg:min-w-0 lg:shrink lg:basis-0 lg:flex-1 lg:border-t-0 lg:py-4"
              aria-label={t('aria.phoneDock')}
            >
              <div className="flex w-full max-w-sm flex-1 items-center justify-center px-4 lg:max-h-[min(90vh,820px)]">
                {usbStage}
              </div>
            </aside>
          </>
        ) : (
          <>
            {phoneFrontAndCenter ? (
              <div className="relative z-[2] shrink-0 border-b border-base-content/5 bg-transparent">
                <aside className="mx-auto w-full max-w-sm px-6 pb-10 pt-9 sm:max-w-xs sm:pb-12 sm:pt-10" aria-label={t('aria.phoneHeroAside')}>
                  <div className="flex w-full justify-center">{usbStage}</div>
                </aside>
              </div>
            ) : null}

            <div className="flex min-h-0 min-w-0 flex-1 flex-col gap-4 overflow-x-hidden overflow-y-auto subtle-scrollbar lg:max-w-none">
              {showIngestHero ? (
                <IpsIngestHero
                  onBrowseClick={() => fileInputRef.current?.click()}
                  onAnalyze={(raw, name) => void interpretIpsContent(raw, name)}
                  onError={setError}
                />
              ) : null}

              {renderIpsImportSection()}

              {renderDeviceConnectedSection(false)}

              <footer className="shrink-0 px-2 py-3 text-center" aria-label={t('aria.footerCredit')}>
                <p className="font-sora m-0 text-[10px] font-medium italic leading-relaxed tracking-[0.06em] text-base-content/42">
                  {t('footer')}
                </p>
              </footer>
            </div>

            {!phoneFrontAndCenter ? phoneDockAside : null}
          </>
        )}
      </div>

      <PanicLogSourceModal
        open={logViewerOpen || ipsLogViewerOpen}
        onClose={() => {
          setLogViewerOpen(false);
          setIpsLogViewerOpen(false);
        }}
        logIndex={ipsLogViewerOpen ? null : pullSelected}
        fileName={ipsLogViewerOpen ? importedIpsDisplayName : selectedPullFilename}
        inlineText={ipsLogViewerOpen ? log : null}
      />

      <ConnectedDeviceInfoModal
        open={deviceInfoOpen}
        onClose={() => setDeviceInfoOpen(false)}
        loading={deviceInfoLoading}
        error={deviceInfoError}
        data={deviceInfoData}
        onRetry={() => void refreshDevicePanel()}
      />

      {pullDetail !== null && usb?.phase === 'connected' ? (
        <div
          className="fixed inset-0 z-[200] flex flex-col bg-base-200 overflow-hidden"
          role="dialog"
          aria-modal="true"
          aria-label={selectedPullFilename ?? t('device.title')}
        >
          {/* Bandeau accent en haut */}
          <div className="h-0.5 shrink-0 bg-gradient-to-r from-primary/70 via-secondary/50 to-primary/20" />

          {/* Header */}
          <div className="shrink-0 flex items-center gap-2 border-b border-base-300 bg-base-100 px-4 py-2.5">
            <button
              type="button"
              className="btn btn-ghost btn-sm gap-1.5 font-sora text-base-content/60 hover:text-base-content"
              onClick={() => setPullDetail(null)}
            >
              <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden>
                <polyline points="15 18 9 12 15 6"/>
              </svg>
              {t('device.backOverview')}
            </button>

            {selectedPullFilename ? (
              <>
                <div className="h-4 w-px shrink-0 bg-base-300" />
                <span className="min-w-0 flex-1 truncate rounded-md border border-base-300 bg-base-200 px-2 py-0.5 font-mono text-[10px] text-base-content/55">
                  {selectedPullFilename}
                </span>
              </>
            ) : null}

            <div className="ml-auto flex shrink-0 flex-wrap items-center gap-1.5">
              <button
                type="button"
                className="btn btn-ghost btn-xs font-sora text-base-content/50 hover:text-base-content"
                onClick={() => { setIpsLogViewerOpen(false); setLogViewerOpen(true); }}
              >
                {t('device.viewFullLog')}
              </button>
              <button
                type="button"
                className="btn btn-primary btn-xs font-sora"
                onClick={() => void exportPulledDeviceRawFile()}
              >
                {t('device.exportRaw')}
              </button>
            </div>
          </div>

          {/* Corps */}
          <div className="subtle-scrollbar flex-1 overflow-y-auto px-4 py-6 sm:px-6">
            <div className="mx-auto max-w-3xl">
              <PanicOverviewCard
                analysis={pullDetail.analysis}
                panicText={pullDetail.panicText}
                productType={usb?.productType ?? null}
              />
            </div>
          </div>
        </div>
      ) : null}

      {extractStatus ? (
        <p className="mt-1 shrink-0 text-center text-xs font-semibold text-success">{extractStatus}</p>
      ) : null}

      {error ? (
        <div className="alert alert-error mt-1 shrink-0 py-3 text-sm" role="alert">
          <span>{error}</span>
        </div>
      ) : null}
    </main>
  );
}

export default App;
