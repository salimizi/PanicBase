import { useMemo } from 'react';
import type { ReferenceWorkshopCard } from '../lib/workshopTypes';
import { useI18n } from '../i18n/context';
import { localizeWorkshopCard, workshopSeverityLabel } from '../lib/workshopCardLocale';
import '../styles/panic-reference-enriched.css';

type Props = {
  card: ReferenceWorkshopCard;
  modelLabel?: string;
  open: boolean;
  onToggle: () => void;
};

const SEV_STYLE: Record<
  ReferenceWorkshopCard['severity'],
  { bg: string; text: string; border: string; dot: string }
> = {
  HARDWARE: { bg: 'bg-error/10', text: 'text-error', border: 'border-error/25', dot: 'bg-error' },
  'BOARD-LEVEL': { bg: 'bg-error/20', text: 'text-error', border: 'border-error/40', dot: 'bg-error' },
  SOFTWARE: { bg: 'bg-info/10', text: 'text-info', border: 'border-info/25', dot: 'bg-info' },
  COMBINÉ: { bg: 'bg-warning/10', text: 'text-warning', border: 'border-warning/25', dot: 'bg-warning' },
};

const SCORE_CONFIG = (score: number) =>
  score >= 90 ? { bar: 'bg-success', text: 'text-success' } :
  score >= 70 ? { bar: 'bg-warning', text: 'text-warning' } :
               { bar: 'bg-base-content/30', text: 'text-base-content/40' };

export function ReferenceWorkshopCardView({ card, modelLabel, open, onToggle }: Props) {
  const { t, locale } = useI18n();
  const uiCard = useMemo(() => localizeWorkshopCard(card, locale), [card, locale]);
  const sev = SEV_STYLE[uiCard.severity];
  const sevLabel = workshopSeverityLabel(uiCard.severity, t);
  const sc = SCORE_CONFIG(uiCard.matchScore);

  return (
    <div className={`rounded-xl border transition-all duration-200 overflow-hidden ${open ? 'border-primary/30 bg-base-100/80 shadow-lg shadow-primary/5' : 'border-base-300/50 bg-base-200/30 hover:border-base-300 hover:bg-base-200/50'}`}>
      {/* Collapsed / Header */}
      <button
        type="button"
        className="w-full text-left"
        aria-expanded={open}
        onClick={onToggle}
      >
        <div className="flex items-center gap-3 px-4 py-3.5">
          {/* Severity dot */}
          <div className={`h-2 w-2 shrink-0 rounded-full ${sev.dot} ${open ? 'ring-2 ring-offset-2 ring-offset-base-100' : ''}`} style={open ? { ['--tw-ring-color' as string]: 'currentColor' } : {}} />
          
          <div className="min-w-0 flex-1">
            <p className={`font-sora text-sm font-bold leading-snug text-base-content ${!open ? 'truncate' : ''}`}>
              {uiCard.title}
            </p>
            {!open && (
              <p className="mt-0.5 font-sora text-[11px] text-base-content/45 truncate">
                {uiCard.component}
              </p>
            )}
          </div>

          <div className="flex shrink-0 items-center gap-2.5">
            {/* Score mini */}
            <div className="flex flex-col items-end gap-1">
              <span className={`font-mono text-xs font-bold ${sc.text}`}>{uiCard.matchScore}%</span>
              <div className="h-0.5 w-10 overflow-hidden rounded-full bg-base-300">
                <div className={`h-full rounded-full ${sc.bar}`} style={{ width: `${uiCard.matchScore}%` }} />
              </div>
            </div>
            {/* Chevron */}
            <svg
              width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5"
              strokeLinecap="round" strokeLinejoin="round"
              className={`shrink-0 text-base-content/35 transition-transform duration-200 ${open ? 'rotate-90' : ''}`}
            >
              <polyline points="9 18 15 12 9 6"/>
            </svg>
          </div>
        </div>

        {/* Badges collapsed preview */}
        {!open && card.codeBadges.length > 0 && (
          <div className="flex flex-wrap gap-1.5 px-4 pb-3 pt-0">
            {card.codeBadges.slice(0, 3).map((b) => (
              <span key={b} className="rounded-md bg-base-300/60 px-2 py-0.5 font-mono text-[10px] font-medium text-base-content/55">
                {b}
              </span>
            ))}
            {card.codeBadges.length > 3 && (
              <span className="rounded-md bg-base-300/40 px-2 py-0.5 font-mono text-[10px] text-base-content/35">
                +{card.codeBadges.length - 3}
              </span>
            )}
          </div>
        )}
      </button>

      {/* Expanded body */}
      {open && (
        <div className="border-t border-base-300/40 bg-base-100/40 px-4 pb-5 pt-4">
          {/* Header open */}
          <div className="mb-4 flex flex-wrap items-center gap-2">
            {card.codeBadges.map((b) => (
              <span key={b} className="rounded-md border border-primary/20 bg-primary/8 px-2.5 py-1 font-mono text-[11px] font-semibold text-primary">
                {b}
              </span>
            ))}
            <span className={`ml-auto rounded-md border px-2.5 py-1 font-sora text-[10px] font-bold uppercase tracking-wider ${sev.bg} ${sev.text} ${sev.border}`}>
              {sevLabel}
            </span>
          </div>

          {/* Subtitle */}
          {uiCard.subtitle && (
            <p className="mb-4 font-sora text-xs text-base-content/55 leading-relaxed">{uiCard.subtitle}</p>
          )}

          {/* Info grid */}
          <div className="mb-4 grid grid-cols-1 gap-3 sm:grid-cols-2">
            <div className="rounded-xl border border-base-300/40 bg-base-200/40 p-3">
              <p className="mb-1.5 font-sora text-[10px] font-semibold uppercase tracking-widest text-base-content/40">
                🔧 {t('summary.sheetComponent')}
              </p>
              <p className="font-sora text-sm font-semibold text-base-content leading-snug">{uiCard.component}</p>
            </div>
            <div className="rounded-xl border border-base-300/40 bg-base-200/40 p-3">
              <p className="mb-1.5 font-sora text-[10px] font-semibold uppercase tracking-widest text-base-content/40">
                ⚡ {t('summary.sheetLikelyCause')}
              </p>
              <p className="font-sora text-sm text-base-content/80 leading-snug">{uiCard.likelyCause}</p>
            </div>
            {uiCard.quickTest && (
              <div className="rounded-xl border border-success/20 bg-success/5 p-3 sm:col-span-2">
                <p className="mb-1.5 font-sora text-[10px] font-semibold uppercase tracking-widest text-success/60">
                  ✓ {t('summary.sheetQuickTest')}
                </p>
                <p className="font-sora text-sm text-success leading-snug">{uiCard.quickTest}</p>
              </div>
            )}
          </div>

          {/* Steps */}
          <div className="mb-4">
            <p className="mb-2.5 font-sora text-[10px] font-semibold uppercase tracking-widest text-base-content/40">
              📋 {t('summary.sheetSteps')}
            </p>
            <div className="flex flex-col gap-2">
              {uiCard.steps.map((step, i) => (
                <div key={i} className="flex gap-3">
                  <div className="flex h-6 w-6 shrink-0 items-center justify-center rounded-lg bg-primary/15 font-mono text-[11px] font-bold text-primary">
                    {i + 1}
                  </div>
                  <p className="flex-1 font-sora text-[13px] leading-relaxed text-base-content/80 pt-0.5">{step}</p>
                </div>
              ))}
            </div>
          </div>

          {/* Keywords */}
          {uiCard.keywords.length > 0 && (
            <div className="mb-4">
              <p className="mb-2 font-sora text-[10px] font-semibold uppercase tracking-widest text-base-content/40">
                🔑 {t('summary.sheetKeywords')}
              </p>
              <div className="flex flex-wrap gap-1.5">
                {uiCard.keywords.map((k) => (
                  <span key={k} className="rounded-md bg-base-300/50 px-2 py-0.5 font-mono text-[10px] text-base-content/55">
                    {k}
                  </span>
                ))}
              </div>
            </div>
          )}

          {/* Note */}
          {uiCard.note && (
            <div className="rounded-xl border-l-4 border-warning/60 bg-warning/5 pl-4 pr-3 py-3">
              <p className="mb-1 font-sora text-[11px] font-semibold text-warning/80">⚠ {t('summary.sheetNotes')}</p>
              <p className="font-sora text-[12px] leading-relaxed text-base-content/70">{uiCard.note}</p>
            </div>
          )}

          {/* Model label footer */}
          {modelLabel && (
            <p className="mt-3 text-right font-sora text-[10px] text-base-content/30">{modelLabel}</p>
          )}
        </div>
      )}
    </div>
  );
}
