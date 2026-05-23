import type { ReactNode } from 'react';
import { useI18n } from '../i18n/context';
import { localizeDiagnosticText } from '../lib/diagnosticLocale';
import { primaryPartHeadline } from '../lib/repairPartsSpeak';
import type { AnalysisResult } from '../types/analysis';
import type { CommunityStats } from '../types/community';
import type { Locale } from '../i18n/translations';

function stripWikiTag(name: string) {
  return name.replace(/\s*\[Repair Wiki\]\s*$/i, '').trim();
}

type TFn = (key: string, vars?: Record<string, string | number>) => string;

function localizedPanicFamily(t: TFn, panicType: string): string {
  const key = `workbench.panicFamily.${panicType}`;
  const s = t(key);
  if (s !== key) return s;
  return panicFamilleCourt(panicType);
}

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

const UNCLASSIFIED_RE = /^(Non classifié|Unclassified)/i;

function fmtPct01(x: number) {
  return Math.round(Math.max(0, Math.min(1, x)) * 100);
}

const MAX_UI_CRITICAL_LINES = 14;
const MAX_UI_CRITICAL_CHARS = 140;

function formatCriticalLinesForUi(lines: string[] | undefined, emptyHint: string, locale: Locale): string {
  if (!lines?.length) {
    return emptyHint;
  }
  return lines
    .slice(0, MAX_UI_CRITICAL_LINES)
    .map((l) => {
      const line = localizeDiagnosticText(l.trim(), locale);
      if (line.length <= MAX_UI_CRITICAL_CHARS) return line;
      return `${line.slice(0, MAX_UI_CRITICAL_CHARS)}…`;
    })
    .join('\n');
}

function technicianHardwareLine(analysis: AnalysisResult, t: TFn): {
  piece: string;
  famillePanic: string;
  conf01: number;
  hasOrientedGuess: boolean;
} {
  const sd = analysis.structured_diagnostic;
  const famillePanic = localizedPanicFamily(t, analysis.panic_type);
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

function FieldLabel({ children }: { children: ReactNode }) {
  return (
    <p className="m-0 font-sora text-[10px] font-medium uppercase tracking-wide text-base-content/45">{children}</p>
  );
}

export type WorkbenchCompactLabels = {
  signals: string;
  noSignals: string;
  model: string;
  part: string;
};

export type WorkbenchFullLabels = {
  model: string;
  part: string;
  score: string;
  notEnough: string;
  causesAria: string;
};

type Props = {
  panicText: string;
  analysis: AnalysisResult;
  fileLabel?: string | null;
  leftTitle?: string;
  rightActions?: ReactNode;
  hideSourceLog?: boolean;
  compactSummary?: boolean;
  /** Libellés UI (compact + mode détaillé). Par défaut : anglais. */
  labels?: {
    compact?: WorkbenchCompactLabels;
    full?: WorkbenchFullLabels;
  };
  /** Stats base communautaire (optionnel) — affichées avec les causes moteur en vue détail. */
  community?: CommunityStats | null;
};

export function PanicAnalyzeWorkbench({
  panicText,
  analysis,
  fileLabel,
  leftTitle = 'Panic',
  rightActions,
  hideSourceLog = false,
  compactSummary = false,
  labels,
  community = null,
}: Props) {
  const { locale, t } = useI18n();
  const sd = analysis.structured_diagnostic;
  const tech = technicianHardwareLine(analysis, t);
  const pieceUi = localizeDiagnosticText(tech.piece, locale);
  const pieceReadable = primaryPartHeadline(analysis, locale, t) || pieceUi;

  const compactL: WorkbenchCompactLabels = {
    signals: t('workbench.signals'),
    noSignals: t('workbench.noSignals'),
    model: t('workbench.model'),
    part: t('workbench.part'),
    ...labels?.compact,
  };
  const fullL: WorkbenchFullLabels = {
    model: t('workbench.model'),
    part: t('workbench.part'),
    score: t('workbench.score'),
    notEnough: t('workbench.notEnough'),
    causesAria: t('workbench.causesAria'),
    ...labels?.full,
  };

  const communityBucketsSorted = community?.buckets?.length
    ? [...community.buckets].sort((a, b) => b.count - a.count).slice(0, 3)
    : [];

  const signalsBlock = compactSummary
    ? formatCriticalLinesForUi(sd.critical_lines, compactL.noSignals, locale)
    : panicText.trim() || '—';

  const leftHeading = compactSummary ? compactL.signals : leftTitle;

  const showLeftPane = compactSummary ? true : !hideSourceLog;

  const gridCols = !showLeftPane
    ? 'grid-cols-1'
    : compactSummary
      ? 'grid-cols-1 md:grid-cols-2'
      : hideSourceLog
        ? 'grid-cols-1'
        : 'grid-cols-1 lg:grid-cols-2';

  const gridHeight =
    compactSummary ? 'min-h-0 flex-1' : 'min-h-[min(52vh,480px)] sm:min-h-0 flex-1';

  return (
    <div
      className={`grid w-full min-w-0 grid-rows-1 flex-1 gap-0 overflow-hidden rounded-box border border-base-300 bg-base-100/40 ${gridHeight} ${gridCols}`}
    >
      {showLeftPane ? (
        <aside
          className={`flex min-w-0 flex-col overflow-hidden border-base-300 bg-base-200/50 sm:border-r ${
            compactSummary ? 'min-h-0 md:border-r' : 'min-h-[12rem] sm:min-h-0'
          }`}
          aria-labelledby="panic-code-heading"
        >
          <div className={`shrink-0 border-b border-base-300 sm:px-4 ${compactSummary ? 'px-2.5 py-2' : 'px-3 py-2.5'}`}>
            <h2 id="panic-code-heading" className="m-0 font-sora text-xs font-semibold tracking-wide text-base-content/70">
              {leftHeading}
            </h2>
            {fileLabel ? (
              <p className="mb-0 mt-1 font-mono text-[10px] leading-snug text-base-content/60">{fileLabel}</p>
            ) : null}
          </div>
          <div className={`subtle-scrollbar flex min-h-0 flex-1 flex-col overflow-y-auto overflow-x-hidden ${compactSummary ? 'p-2 sm:p-3' : 'p-3 sm:p-4'}`}>
            <pre
              className={`font-mono m-0 min-h-0 w-full min-w-0 flex-1 rounded-md border border-base-300/80 bg-base-300/35 whitespace-pre-wrap break-words text-base-content/85 ${
                compactSummary
                  ? 'p-2 text-[10px] leading-snug sm:text-[11px]'
                  : 'p-3 text-[11px] leading-relaxed sm:text-[12px]'
              }`}
              tabIndex={0}
            >
              {signalsBlock}
            </pre>
          </div>
        </aside>
      ) : null}

      <section
        className={`flex min-w-0 flex-col overflow-hidden bg-base-100/25 ${compactSummary ? 'min-h-0' : 'min-h-[12rem] sm:min-h-0'}`}
        aria-label={t('aria.diagnosticSection')}
      >
        <div
          className={
            compactSummary
              ? 'subtle-scrollbar flex min-h-0 w-full min-w-0 flex-1 flex-col gap-2.5 overflow-y-auto overflow-x-hidden px-2.5 py-2.5 sm:gap-3 sm:px-3 sm:py-3'
              : 'subtle-scrollbar flex flex-col gap-3.5 overflow-y-auto overflow-x-hidden p-4'
          }
        >
          {compactSummary ? (
            <>
              <div className="flex min-w-0 flex-col gap-2.5 sm:flex-row sm:items-start sm:gap-4">
                <div className="min-w-0 sm:flex-1">
                  <FieldLabel>{compactL.model}</FieldLabel>
                  <p className="mb-0 mt-0.5 font-outfit text-sm font-semibold leading-snug tracking-tight text-base-content sm:text-base">
                    {analysis.device_model}
                  </p>
                </div>
                <div className="min-w-0 border-t border-base-300/70 pt-2.5 sm:flex-1 sm:border-l sm:border-t-0 sm:pl-4 sm:pt-0">
                  <FieldLabel>{compactL.part}</FieldLabel>
                  <p className="mb-0 mt-0.5 font-sora text-[12px] font-medium leading-snug text-base-content sm:text-[13px]">
                    {pieceReadable}
                  </p>
                  {tech.famillePanic ? (
                    <p className="mb-0 mt-1 font-sora text-[9px] leading-snug text-base-content/48">{tech.famillePanic}</p>
                  ) : null}
                </div>
              </div>

              {((typeof sd.confidence_global === 'number' && sd.confidence_global > 0) ||
                community?.message ||
                communityBucketsSorted.length > 0) && (
                <div className="flex min-w-0 flex-col gap-2.5 border-t border-base-300/70 pt-2.5">
                  {typeof sd.confidence_global === 'number' && sd.confidence_global > 0 ? (
                    <p className="m-0 font-sora text-[10px] leading-snug text-base-content/55">
                      {t('workbench.consolidatedScore', { pct: String(fmtPct01(sd.confidence_global)) })}
                    </p>
                  ) : null}

                  {community?.message || communityBucketsSorted.length > 0 ? (
                    <div className="min-w-0">
                      <FieldLabel>{t('workbench.communityBaseTitle')}</FieldLabel>
                      {communityBucketsSorted.length > 0 ? (
                        <ol className="mb-0 mt-1.5 list-decimal space-y-1.5 ps-4 font-sora text-[11px] leading-snug text-base-content/80 sm:text-[12px]">
                          {communityBucketsSorted.map((b, i) => (
                            <li key={`b-${i}-${b.label.slice(0, 48)}`} className="break-words pl-0.5 marker:text-base-content/40">
                              <span className="font-medium text-base-content/90">
                                {localizeDiagnosticText(b.label.trim(), locale)}
                              </span>
                              <span className="ms-1 font-mono text-[10px] text-base-content/50">
                                {t('workbench.communityBucketMeta', {
                                  pct: String(b.percent),
                                  count: String(b.count),
                                })}
                              </span>
                            </li>
                          ))}
                        </ol>
                      ) : community?.message ? (
                        <p className="comm-toned m-0 mt-1.5 max-w-full text-[11px] leading-relaxed text-base-content/60 break-words">
                          {localizeDiagnosticText(community.message, locale)}
                        </p>
                      ) : null}
                    </div>
                  ) : null}
                </div>
              )}
            </>
          ) : (
            <>
              <p className="m-0 grid grid-cols-[minmax(106px,34%)_1fr] items-baseline gap-x-2.5 gap-y-0 text-sm">
                <span className="font-sora text-[10px] font-medium uppercase tracking-wide text-base-content/45">
                  {fullL.model}
                </span>
                <span className="font-outfit font-semibold tracking-tight text-base-content">{analysis.device_model}</span>
              </p>
              <p className="m-0 grid grid-cols-[minmax(106px,34%)_1fr] items-start gap-x-2.5 text-sm">
                <span className="font-sora text-[10px] font-medium uppercase tracking-wide text-base-content/45">
                  {fullL.part}
                </span>
                <span className="flex min-w-0 flex-col items-start gap-1 font-sora font-medium leading-relaxed text-base-content">
                  <span className="break-words">{pieceReadable}</span>
                  {tech.famillePanic ? <span className="text-[11px] text-base-content/50">{tech.famillePanic}</span> : null}
                </span>
              </p>
            </>
          )}

          {rightActions ? (
            <div
              className={
                compactSummary
                  ? 'mt-auto flex min-h-0 shrink-0 flex-col gap-1.5 border-t border-base-300/80 pt-2.5 [&_.comm-toned]:mt-0 [&_.comm-toned]:border-t-0 [&_.comm-toned]:pt-0 [&_.comm-toned]:break-words [&_.comm-toned]:whitespace-normal'
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
