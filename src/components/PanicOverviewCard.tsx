import { useEffect, useMemo, useState } from 'react';
import { useI18n } from '../i18n/context';
import { analysisIsDiagnosticUnclear } from '../lib/panicDiagnosticViewModel';
import { resolveWorkshopReferenceCards } from '../lib/referenceWorkshopCards';
import type { AnalysisResult } from '../types/analysis';
import { ReferenceWorkshopCardView } from './ReferenceWorkshopCardView';
import '../styles/panic-reference-enriched.css';

type Props = {
  analysis: AnalysisResult;
  panicText?: string;
  productType?: string | null;
};

export function PanicOverviewCard({ analysis, panicText = '', productType = null }: Props) {
  const { t, locale } = useI18n();
  const cards = useMemo(
    () => resolveWorkshopReferenceCards(panicText, analysis, productType, locale),
    [panicText, analysis, productType, locale],
  );
  const modelLabel = analysis.device_model?.trim() || analysis.structured_diagnostic.marketing_name || '';
  const deviceId = analysis.structured_diagnostic.device;
  const confidence = Math.round(Math.max(0, Math.min(1, analysis.structured_diagnostic.confidence_global)) * 100);
  const cardsKey = useMemo(
    () => `${analysis.signature_hash ?? ''}|${cards.map((c) => c.uiKey ?? c.id).join(',')}`,
    [analysis.signature_hash, cards],
  );
  const [expandedId, setExpandedId] = useState<string | null>(null);
  useEffect(() => {
    setExpandedId(null);
  }, [cardsKey]);

  const confidenceColor = confidence >= 80 ? 'text-success' : confidence >= 55 ? 'text-warning' : 'text-error';
  const confidenceBar = confidence >= 80 ? 'bg-success' : confidence >= 55 ? 'bg-warning' : 'bg-error';

  if (analysisIsDiagnosticUnclear(analysis) && cards.length === 0) {
    return (
      <div className="rounded-2xl border border-warning/20 bg-warning/5 p-4">
        <div className="flex items-center gap-3">
          <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-xl bg-warning/15 text-warning">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <path d="M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z" />
              <line x1="12" y1="9" x2="12" y2="13" />
              <line x1="12" y1="17" x2="12.01" y2="17" />
            </svg>
          </div>
          <div>
            <p className="font-sora text-sm font-semibold text-base-content">{t('summary.unclearTitle')}</p>
            <p className="mt-0.5 font-sora text-xs text-base-content/55">{t('summary.unclear')}</p>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-3">
      {modelLabel ? (
        <div className="flex items-center justify-between gap-3 rounded-xl border border-primary/20 bg-primary/5 px-4 py-3">
          <div className="flex min-w-0 items-center gap-2.5">
            <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg bg-primary/15 text-primary">
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <rect x="5" y="2" width="14" height="20" rx="2" />
                <line x1="12" y1="18" x2="12.01" y2="18" />
              </svg>
            </div>
            <div className="min-w-0">
              <p className="truncate font-sora text-sm font-bold text-base-content">{modelLabel}</p>
              {deviceId && deviceId !== 'unknown' ? (
                <p className="mt-0 font-mono text-[10px] text-base-content/40">{deviceId}</p>
              ) : null}
            </div>
          </div>
          <div className="flex shrink-0 flex-col items-end gap-1.5">
            <span className={`font-mono text-lg font-bold leading-none ${confidenceColor}`}>{confidence}%</span>
            <div className="flex items-center gap-1.5">
              <div className="h-1 w-14 overflow-hidden rounded-full bg-base-300">
                <div className={`h-full rounded-full ${confidenceBar}`} style={{ width: `${confidence}%` }} />
              </div>
              <span className="font-sora text-[9px] text-base-content/40">{t('summary.confidenceLabel')}</span>
            </div>
          </div>
        </div>
      ) : null}
      <div className="flex flex-col gap-2">
        {cards.map((c) => (
          <ReferenceWorkshopCardView
            key={c.uiKey ?? c.id}
            card={c}
            modelLabel={modelLabel || undefined}
            open={expandedId === (c.uiKey ?? c.id)}
            onToggle={() =>
              setExpandedId((id) => (id === (c.uiKey ?? c.id) ? null : (c.uiKey ?? c.id)))
            }
          />
        ))}
      </div>
    </div>
  );
}
