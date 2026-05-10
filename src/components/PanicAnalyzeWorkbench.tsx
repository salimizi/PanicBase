import type { ReactNode } from 'react';
import type { AnalysisResult } from '../types/analysis';

function fmtPct01(x: number) {
  return Math.round(Math.max(0, Math.min(1, x)) * 100);
}

/** Affichage liste : même libellé que le moteur, sans suffixe encyclopédique. */
function stripWikiTag(name: string) {
  return name.replace(/\s*\[Repair Wiki\]\s*$/i, '').trim();
}

/** Type de panic moteur (contexte SMC / watchdog…). */
function panicFamilleCourt(panicType: string): string {
  const labels: Record<string, string> = {
    smc_bsc_outbox_chain: 'SMC · BSC · OUTBOX',
    ans2_storage: 'ANS2 / stockage',
    baseband_panic: 'Baseband',
    sep_panic: 'SEP',
    aop_panic: 'AOP',
    unknown: '—',
  };
  const known = labels[panicType];
  if (known) return known;

  const m = /^bug_(\d+)_(.+)$/.exec(panicType);
  if (m) return `${m[2].replace(/_/g, ' ')} · ${m[1]}`;
  return panicType.replace(/_/g, ' ');
}

const UNCLASSIFIED_RE = /^Non classifié/i;

const MAX_UI_CRITICAL_LINES = 10;
const MAX_UI_CRITICAL_CHARS = 88;

/** Ce que le backend a retenu dans le log (SMC, sensor array, missing sensor, etc.) — lisible, sans dump complet. */
function formatCriticalLinesForUi(lines: string[] | undefined): string {
  if (!lines?.length) {
    return '— (aucun signal SMC / capteur isolé dans l’extrait analysé)';
  }
  return lines
    .slice(0, MAX_UI_CRITICAL_LINES)
    .map((l) => {
      const t = l.trim();
      if (t.length <= MAX_UI_CRITICAL_CHARS) return t;
      return `${t.slice(0, MAX_UI_CRITICAL_CHARS)}…`;
    })
    .join('\n');
}

/** Ex. « 0x280000 · 0x100000 » — aligné sur les lignes `Mask 0x…` du moteur (même IPS peut en avoir plusieurs). */
function sensorMasksFromWikiHints(wikiHints: string[]): string | null {
  const seen = new Set<string>();
  const order: string[] = [];
  for (const line of wikiHints) {
    const m = /Mask\s+(0x[0-9a-f]+)\s*\(\d+\)/i.exec(line);
    if (!m) continue;
    const hex = m[1].toLowerCase();
    if (seen.has(hex)) continue;
    seen.add(hex);
    order.push(hex);
  }
  return order.length ? order.join(' · ') : null;
}

/** Libellé atelier : nappe / assemblage (cause n°1), pas seulement la famille SMC. */
function technicianHardwareLine(analysis: AnalysisResult): {
  piece: string;
  famillePanic: string;
  conf01: number;
  hasOrientedGuess: boolean;
} {
  const sd = analysis.structured_diagnostic;
  const famillePanic = panicFamilleCourt(analysis.panic_type);
  const rawProb = analysis.probable_cause?.trim() ?? '';
  const top = sd.possible_causes?.[0];
  const conf01 =
    typeof top?.confidence === 'number' ? top.confidence : Math.min(1, analysis.confidence / 100);

  if (rawProb && !UNCLASSIFIED_RE.test(rawProb)) {
    return {
      piece: stripWikiTag(rawProb),
      famillePanic,
      conf01,
      hasOrientedGuess: true,
    };
  }

  return {
    piece: famillePanic,
    famillePanic,
    conf01,
    hasOrientedGuess: false,
  };
}

type Props = {
  panicText: string;
  analysis: AnalysisResult;
  fileLabel?: string | null;
  leftTitle?: string;
  rightActions?: ReactNode;
  hideSourceLog?: boolean;
  compactSummary?: boolean;
};

export function PanicAnalyzeWorkbench({
  panicText,
  analysis,
  fileLabel,
  leftTitle = 'Panic',
  rightActions,
  hideSourceLog = false,
  compactSummary = false,
}: Props) {
  const sd = analysis.structured_diagnostic;
  const tech = technicianHardwareLine(analysis);
  const masksInLog = sensorMasksFromWikiHints(sd.wiki_hints ?? []);
  const maxC =
    sd.possible_causes?.reduce((m, c) => Math.max(m, c.confidence), 0.01) ?? 1;

  const signalsBlock = compactSummary
    ? formatCriticalLinesForUi(sd.critical_lines)
    : panicText.trim() || '—';

  const leftHeading = compactSummary ? 'Signaux retenus' : leftTitle;

  const showLeftPane = compactSummary ? true : !hideSourceLog;

  const gridCols = !showLeftPane
    ? 'grid-cols-1'
    : compactSummary
      ? 'grid-cols-2'
      : hideSourceLog
        ? 'grid-cols-1'
        : 'grid-cols-2';

  return (
    <div
      className={`grid min-h-0 flex-1 gap-0 overflow-hidden rounded-lg border border-base-content/10 bg-base-300/15 ${gridCols}`}
    >
      {showLeftPane ? (
        <aside
          className="flex min-h-0 min-w-0 flex-col overflow-hidden border-r border-base-content/10 bg-base-300/50"
          aria-labelledby="panic-code-heading"
        >
          <div className="shrink-0 border-b border-base-content/5 px-2.5 py-1.5">
            <h2 id="panic-code-heading" className="mb-0 text-[10px] font-black uppercase tracking-widest text-base-content/45">
              {leftHeading}
            </h2>
            {fileLabel ? (
              <span className="mt-1 block font-mono text-xs font-bold break-all text-info">{fileLabel}</span>
            ) : null}
          </div>
          <div className="flex min-h-0 flex-1 flex-col overflow-hidden p-2">
            <pre
              className="font-mono m-0 min-h-0 flex-1 overflow-hidden text-[10px] leading-snug whitespace-pre-wrap break-words text-info"
              tabIndex={0}
            >
              {signalsBlock}
            </pre>
          </div>
        </aside>
      ) : null}

      <section
        className={`flex min-h-0 min-w-0 flex-col overflow-hidden ${
          compactSummary ? '' : 'bg-base-200/20'
        }`}
        aria-label="Diagnostic"
      >
        <div
          className={
            compactSummary
              ? 'flex min-h-0 flex-1 flex-col gap-1 overflow-hidden px-3.5 py-2'
              : 'flex flex-col gap-3.5 overflow-hidden p-4'
          }
        >
          {compactSummary ? (
            <>
              <p className="m-0 text-[9px] font-extrabold uppercase tracking-widest text-base-content/60">Modèle</p>
              <p className="m-0 text-lg font-black leading-tight tracking-tight text-base-content">
                {analysis.device_model}
              </p>
              <p className="m-0 mt-3 text-[9px] font-extrabold uppercase tracking-widest text-base-content/60">
                Pièce probable
              </p>
              <p className="m-0 line-clamp-4 text-base font-extrabold leading-snug text-info">{tech.piece}</p>
              {masksInLog ? (
                <p
                  className="m-0 mt-2 line-clamp-3 rounded-lg border border-primary/20 bg-base-300/80 px-2 py-1.5 font-mono text-[11px] leading-snug text-base-content/90"
                  title="Valeurs exactes lues dans S./F.sensor array de ce log"
                >
                  Masques (log) · {masksInLog}
                </p>
              ) : null}
              {tech.hasOrientedGuess ? (
                <p className="m-0 mt-1 text-[11px] font-semibold text-base-content/55" title={tech.famillePanic}>
                  {tech.famillePanic}
                  {' · ~'}
                  {fmtPct01(tech.conf01)}%
                </p>
              ) : (
                tech.famillePanic !== tech.piece && (
                  <p className="m-0 mt-1 text-[11px] text-base-content/55">{tech.famillePanic}</p>
                )
              )}
            </>
          ) : (
            <>
              <p className="m-0 grid grid-cols-[minmax(106px,34%)_1fr] items-baseline gap-x-2.5 gap-y-0 text-sm">
                <span className="text-[10px] font-extrabold uppercase tracking-widest text-base-content/70">Modèle</span>
                <span className="font-bold tracking-tight text-base-content">{analysis.device_model}</span>
              </p>
              <p className="m-0 grid grid-cols-[minmax(106px,34%)_1fr] items-start gap-x-2.5 text-sm">
                <span className="text-[10px] font-extrabold uppercase tracking-widest text-base-content/70">
                  Pièce probable
                </span>
                <span className="flex flex-col items-start gap-1 font-bold text-base-content">
                  <span>{tech.piece}</span>
                  {tech.hasOrientedGuess ? (
                    <span className="text-[11px] font-semibold text-base-content/55">
                      {tech.famillePanic} · ~{fmtPct01(tech.conf01)}%
                    </span>
                  ) : null}
                </span>
              </p>
              <p className="m-0 grid grid-cols-[minmax(106px,34%)_1fr] items-baseline gap-x-2.5 text-sm">
                <span className="text-[10px] font-extrabold uppercase tracking-widest text-base-content/70">Score</span>
                <span className="text-2xl font-black tracking-tight text-success">{fmtPct01(sd.confidence_global)}%</span>
              </p>
            </>
          )}

          {!compactSummary &&
            (sd.possible_causes?.length ? (
              <ul className="m-1 mt-0 flex list-none flex-col gap-2.5 overflow-hidden p-0" aria-label="Probabilités par cause">
                {sd.possible_causes.map((c, i) => (
                  <li key={`${i}-${c.name.slice(0, 48)}`} className="grid grid-cols-[1fr_auto] gap-x-3 gap-y-1">
                    <span className="col-span-2 text-xs font-semibold leading-snug text-base-content">
                      {stripWikiTag(c.name)}
                    </span>
                    <span className="h-1 self-center rounded-full bg-base-content/10" aria-hidden>
                      <span
                        className="block h-full rounded-full bg-gradient-to-r from-success/55 to-info/85"
                        style={{ width: `${Math.round((c.confidence / maxC) * 100)}%` }}
                      />
                    </span>
                    <span className="min-w-[2.25rem] text-right text-xs font-extrabold text-success">
                      {fmtPct01(c.confidence)}%
                    </span>
                  </li>
                ))}
              </ul>
            ) : (
              <p className="m-0 text-xs text-base-content/55">Pas assez de signal — log plus complet.</p>
            ))}

          {rightActions ? (
            <div
              className={
                compactSummary
                  ? 'mt-3 flex min-h-0 shrink-0 flex-col gap-2 overflow-hidden [&_.comm-toned]:mt-0 [&_.comm-toned]:truncate [&_.comm-toned]:border-t-0 [&_.comm-toned]:pt-0 [&_.ips-saved]:truncate'
                  : 'mt-2 flex shrink-0 flex-col gap-2'
              }
            >
              {rightActions}
            </div>
          ) : null}
        </div>
      </section>
    </div>
  );
}
