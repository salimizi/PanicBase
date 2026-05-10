import { useCallback, useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { UsbCenterStage } from './components/UsbCenterStage';
import { PanicAnalyzeWorkbench } from './components/PanicAnalyzeWorkbench';
import type { AnalysisResult } from './types/analysis';

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
};

type PanicPullListResponse = {
  count: number;
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

type CommunityStats = {
  available: boolean;
  similar_count: number;
  message: string;
  buckets: Array<{ label: string; count: number; percent: number }>;
};

const POLL_MS = 2800;

function displayModelShort(marketingName: string | null, productType: string | null): string | null {
  if (marketingName) {
    const t = marketingName.replace(/^iPhone\s+/i, '').trim();
    return t || marketingName;
  }
  return productType;
}

const emptyPull: PanicPullListResponse = { count: 0, message: '', logs: [] };

function fmtPct01(x: number) {
  return Math.round(Math.max(0, Math.min(1, x)) * 100);
}

function structuredExportTail(sd: AnalysisResult['structured_diagnostic']): string {
  return [
    '',
    '--- PanicBase (compact) ---',
    `score_global: ~${fmtPct01(sd.confidence_global)}%`,
    ...(sd.possible_causes?.length
      ? sd.possible_causes.map((c) => `- ${c.name} · ${fmtPct01(c.confidence)}%`)
      : ['- (aucune cause)']),
  ].join('\n');
}

function App() {
  const fileInputRef = useRef<HTMLInputElement>(null);
  const didAutoPullRef = useRef(false);

  const [usb, setUsb] = useState<IphoneUsbStatus | null>(null);
  const [deviceHintFromUsb, setDeviceHintFromUsb] = useState<string | null>(null);
  const [log, setLog] = useState('');
  const [analysis, setAnalysis] = useState<AnalysisResult | null>(null);
  const [error, setError] = useState('');
  const [ipsFileName, setIpsFileName] = useState<string | null>(null);
  const [deviceHintFromIps, setDeviceHintFromIps] = useState<string | null>(null);
  const [extractStatus, setExtractStatus] = useState<string | null>(null);
  const [community, setCommunity] = useState<CommunityStats | null>(null);

  const [ipsImportActive, setIpsImportActive] = useState(false);
  const [ipsEnrichBusy, setIpsEnrichBusy] = useState(false);
  const [ipsSavedLine, setIpsSavedLine] = useState<string | null>(null);

  const [pullWorkspace, setPullWorkspace] = useState<'idle' | 'loading' | 'ready'>('idle');
  const [pullResult, setPullResult] = useState<PanicPullListResponse | null>(null);
  const [pullSelected, setPullSelected] = useState<number | null>(null);
  const [pullDetail, setPullDetail] = useState<PulledPanicDetailResponse | null>(null);
  const [pullAnalyzing, setPullAnalyzing] = useState(false);

  const pollUsb = useCallback(async () => {
    try {
      const status = await invoke<IphoneUsbStatus>('detect_iphone');
      setUsb(status);
      if (status.phase === 'connected' && status.productType) {
        setDeviceHintFromUsb(status.productType);
      } else if (status.phase !== 'connected') {
        setDeviceHintFromUsb(null);
      }
    } catch {
      /* silencieux */
    }
  }, []);

  useEffect(() => {
    void pollUsb();
    const id = window.setInterval(() => void pollUsb(), POLL_MS);
    return () => window.clearInterval(id);
  }, [pollUsb]);

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
    invoke<PanicPullListResponse>('pull_device_recent_panic_logs')
      .then((r) => setPullResult(r))
      .catch((err) => {
        setError(String(err));
        setPullResult(emptyPull);
      })
      .finally(() => setPullWorkspace('ready'));
  }, [usb?.phase]);

  useEffect(() => {
    if (!analysis) {
      setCommunity(null);
      return;
    }
    const modelUsb = deviceHintFromUsb ?? undefined;
    const modelHint = deviceHintFromIps ?? (analysis.device_model !== 'Non renseigné' ? analysis.device_model : null);
    const modelParam = modelUsb ?? modelHint ?? undefined;

    void invoke<CommunityStats>('get_community_stats', {
      signature_hash: analysis.signature_hash,
      model: modelParam,
    })
      .then(setCommunity)
      .catch(() => setCommunity(null));
  }, [analysis, deviceHintFromIps, deviceHintFromUsb]);

  async function selectPulledPanic(index: number) {
    setError('');
    setIpsImportActive(false);
    setIpsSavedLine(null);
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
    setIpsSavedLine(null);
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

  function onPickIpsFile() {
    fileInputRef.current?.click();
  }

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

  async function enrichLocalDbFromIps() {
    if (!log.trim() || !analysis || !ipsFileName) return;
    setError('');
    setIpsEnrichBusy(true);
    setIpsSavedLine(null);
    try {
      const id = await invoke<number>('save_imported_panic_to_db', {
        panic_text: log,
        device_hint: deviceHintFromIps ?? deviceHintFromUsb ?? undefined,
        source_filename: ipsFileName,
      });
      setIpsSavedLine(`Ajouté à la base locale · #${id} (texte anonymisé).`);
    } catch (err) {
      setError(String(err));
    } finally {
      setIpsEnrichBusy(false);
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
      `score : ${fmtPct01(a.structured_diagnostic.confidence_global)}%`,
      ...(a.structured_diagnostic.possible_causes?.map(
        (c) => `${c.name} · ${fmtPct01(c.confidence)}%`,
      ) ?? []),
      structuredExportTail(a.structured_diagnostic),
    ].join('\n');

    try {
      const saved = await invoke<string | null>('export_text_file', {
        content: body,
        default_filename: `${stem}.txt`,
      });
      if (saved) {
        setExtractStatus(`Export : ${saved}`);
      }
    } catch (err) {
      setError(String(err));
    }
  }

  const centerPhase = (usb?.phase as string) || 'unplugged';
  const modelShort =
    usb?.phase === 'connected' ? displayModelShort(usb.marketingName, usb.productType) : null;

  return (
    <main className="relative z-10 mx-auto flex h-full min-h-0 w-full max-w-[520px] flex-col gap-0 overflow-hidden px-2.5 pb-1.5 pt-2">
      <header className="navbar mb-1.5 min-h-0 shrink-0 rounded-xl border border-base-content/10 bg-base-300/45 px-2 py-2 shadow-lg backdrop-blur-md">
        <div className="navbar-start min-w-0 gap-2">
          <span className="truncate text-lg font-extrabold tracking-tight text-base-content">PanicBase</span>
          <span className="badge badge-ghost badge-sm shrink-0 opacity-60">0.1</span>
        </div>
        <div className="navbar-end">
          <nav className="flex items-center gap-2" aria-label="Actions">
            <button type="button" className="btn btn-outline btn-sm shrink-0" onClick={onPickIpsFile}>
              Importer IPS
            </button>
            <input
              ref={fileInputRef}
              type="file"
              accept=".ips,.IPS,.txt,.crash,text/plain,application/json"
              className="sr-only"
              onChange={onIpsFileChange}
            />
          </nav>
        </div>
      </header>

      <div className="relative z-[2] flex min-h-[248px] shrink-0 items-center justify-center overflow-visible px-1 py-2">
        <UsbCenterStage
          phase={centerPhase}
          modelShort={modelShort}
          iosVersion={usb?.phase === 'connected' ? usb.iosVersion : null}
          backendDetail={usb?.detail || undefined}
        />
      </div>

      {ipsImportActive && ipsFileName && analysis ? (
        <section
          className="card card-bordered mt-3 flex min-h-0 flex-1 flex-col overflow-hidden border-success/25 bg-[linear-gradient(165deg,rgba(22,40,58,0.55)_0%,rgba(8,12,22,0.72)_100%)] shadow-2xl"
          aria-label="Import IPS"
        >
          <div className="card-body min-h-0 flex-1 gap-0 overflow-hidden p-0">
            <header className="shrink-0 border-b border-base-content/10 bg-base-300/35 px-3 py-2">
              <p className="mb-0 min-w-0 truncate font-mono text-xs font-semibold text-base-content/75">
                {ipsFileName}
              </p>
            </header>
            <div className="flex min-h-0 flex-1 flex-col px-2 pb-2 pt-1">
              <PanicAnalyzeWorkbench
                panicText={log}
                analysis={analysis}
                fileLabel={null}
                leftTitle="Panic (IPS)"
                hideSourceLog={false}
                compactSummary
                rightActions={
                  <>
                    {community?.message ? (
                      <p className="comm-toned m-0 max-w-full truncate border-t border-base-content/10 pt-2 text-[11px] text-base-content/60">
                        {community.message}
                      </p>
                    ) : null}
                    <button
                      type="button"
                      className="btn btn-primary btn-sm btn-block"
                      disabled={ipsEnrichBusy}
                      onClick={() => void enrichLocalDbFromIps()}
                    >
                      {ipsEnrichBusy ? 'Enregistrement…' : 'Enrichir la base locale'}
                    </button>
                    {ipsSavedLine ? (
                      <p className="ips-saved m-0 max-w-full truncate text-xs font-semibold text-success">
                        {ipsSavedLine}
                      </p>
                    ) : null}
                  </>
                }
              />
            </div>
          </div>
        </section>
      ) : null}

      {usb?.phase === 'connected' ? (
        <section
          className="relative z-10 mt-3 flex min-h-0 flex-1 flex-col overflow-hidden border-t border-base-content/10 pt-2.5"
          aria-label="Panic logs téléphone"
        >
          {pullWorkspace === 'loading' ? (
            <div className="flex flex-col items-center gap-2.5 px-2 py-2 pb-5">
              <div className="orbit-wrap orbit-inline" aria-busy="true">
                <div className="orbit-ring" />
                <div className="orbit-core" />
              </div>
              <p className="m-0 text-sm font-semibold text-base-content/80">Récupération des 5 derniers panic-full…</p>
            </div>
          ) : null}

          {pullWorkspace === 'ready' && pullResult ? (
            <>
              <p className="mb-1.5 mt-0 text-[10px] font-extrabold uppercase tracking-widest text-base-content/45">
                Panic logs
              </p>
              {pullResult.count === 0 ? (
                <p className="m-0 mb-3 text-sm font-semibold text-base-content/55">Aucun fichier panic-full sur l’appareil.</p>
              ) : (
                <ul className="mb-2 flex max-h-[76px] shrink-0 list-none flex-col gap-1.5 overflow-hidden p-0">
                  {pullResult.logs.map((row) => (
                    <li key={`${row.filename}-${row.index}`}>
                      <button
                        type="button"
                        className={`btn btn-block h-auto min-h-0 justify-start gap-0.5 border py-2.5 text-left font-normal ${
                          pullSelected === row.index
                            ? 'border-primary bg-primary/15 text-base-content'
                            : 'btn-ghost border-base-content/10 bg-base-300/30 hover:border-primary/40'
                        }`}
                        onClick={() => void selectPulledPanic(row.index)}
                        disabled={pullAnalyzing}
                      >
                        <span className="font-mono text-xs font-extrabold break-all text-base-content">{row.filename}</span>
                        <span className="text-[10px] font-bold text-base-content/55">{row.modifiedLabel}</span>
                      </button>
                    </li>
                  ))}
                </ul>
              )}

              {pullAnalyzing ? (
                <p className="mb-2.5 flex items-center gap-2 text-xs font-bold text-info">
                  <span className="dot-pulse inline-block shrink-0" /> Chargement…
                </p>
              ) : null}

              {pullDetail ? (
                <div className="mt-1 flex min-h-0 flex-1 flex-col overflow-hidden">
                  <PanicAnalyzeWorkbench
                    panicText={pullDetail.panicText}
                    analysis={pullDetail.analysis}
                    leftTitle="Panic téléphone"
                    hideSourceLog={false}
                    compactSummary
                    rightActions={
                      <>
                        <button
                          type="button"
                          className="btn btn-outline btn-primary btn-sm w-fit"
                          onClick={() => void exportCurrentPanicOnPc()}
                        >
                          Exporter sur le PC
                        </button>
                        {community?.message ? (
                          <p className="comm-toned m-0 max-w-full truncate text-[11px] text-base-content/60">
                            {community.message}
                          </p>
                        ) : null}
                      </>
                    }
                  />
                </div>
              ) : null}
            </>
          ) : null}
        </section>
      ) : null}

      {extractStatus ? (
        <p className="mt-1 shrink-0 text-center text-xs font-semibold text-success">{extractStatus}</p>
      ) : null}

      <p className="shrink-0 py-1 text-center text-[10px] text-base-content/35">100 % local</p>

      {error ? (
        <div className="alert alert-error mt-1 shrink-0 py-3 text-sm" role="alert">
          <span>{error}</span>
        </div>
      ) : null}
    </main>
  );
}

export default App;
