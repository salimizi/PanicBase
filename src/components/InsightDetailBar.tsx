import type { ReactNode } from 'react';
import { useI18n } from '../i18n/context';

type Props = {
  backLabel: string;
  onBack: () => void;
  title: string;
  fileContextLabel: string;
  fileName?: string | null;
  /** Fermer la session (ex. import IPS) — croix rouge */
  onDismiss?: () => void;
  dismissAriaLabel?: string;
  children?: ReactNode;
  /** Barre plus basse : moins de scroll dans la vue détail */
  compact?: boolean;
  /** Topbar sombre type fiche `iphone_panic_reference_enriched.html` */
  referenceChrome?: boolean;
};

export function InsightDetailBar({
  backLabel,
  onBack,
  title,
  fileContextLabel,
  fileName,
  onDismiss,
  dismissAriaLabel,
  children,
  compact = false,
  referenceChrome = false,
}: Props) {
  const { t } = useI18n();
  const dismissLabel = dismissAriaLabel ?? t('logViewer.close');
  if (compact && referenceChrome) {
    return (
      <div className="sticky top-0 z-[4] shrink-0 border-b border-[#2a2a30] bg-[#111113] px-3 py-2.5 sm:px-4">
        <div className="flex min-w-0 flex-col gap-2.5">
          <div className="flex min-w-0 flex-wrap items-start gap-2 sm:items-center">
            <button
              type="button"
              className="inline-flex shrink-0 items-center gap-1.5 rounded-md border border-[#2a2a30] bg-[#1a1a1e] px-3 py-1.5 font-mono text-[11px] font-semibold text-[#c8c8d8] transition-colors hover:border-[#4f8ef7]/40 hover:text-[#e8e8f0]"
              onClick={onBack}
            >
              <span className="leading-none opacity-80" aria-hidden>
                ←
              </span>
              {backLabel}
            </button>
            {fileName ? (
              <div className="min-w-0 max-w-full flex-1 rounded-md border border-[#2a2a30] bg-[#0f0f11] px-2.5 py-1.5 sm:max-w-[min(100%,22rem)]">
                <p className="m-0 font-mono text-[8px] font-semibold uppercase tracking-[0.14em] text-[#555568]">{fileContextLabel}</p>
                <p className="m-0 truncate font-mono text-[11px] font-medium leading-snug text-[#d8d8e4]" title={fileName}>
                  {fileName}
                </p>
              </div>
            ) : null}
            <div className="flex min-h-[28px] min-w-0 flex-1 items-center gap-2 sm:justify-end">
              <div className="hidden h-2 w-2 shrink-0 rounded-full bg-[#4f8ef7] opacity-90 sm:block" aria-hidden />
              <span className="truncate font-mono text-[11px] font-semibold tracking-wide text-[#8888a0]">{title}</span>
            </div>
            {onDismiss ? (
              <button
                type="button"
                className="ml-auto inline-flex size-8 shrink-0 items-center justify-center rounded-md border border-[#ef4444]/35 bg-[rgba(239,68,68,0.08)] font-mono text-[14px] leading-none text-[#f87171] transition-colors hover:bg-[rgba(239,68,68,0.14)]"
                aria-label={dismissLabel}
                title={dismissAriaLabel ?? dismissLabel}
                onClick={onDismiss}
              >
                ×
              </button>
            ) : null}
          </div>
          {children ? <div className="flex flex-wrap gap-1.5 border-t border-[#2a2a30] pt-2.5">{children}</div> : null}
        </div>
      </div>
    );
  }
  if (compact) {
    return (
      <div className="sticky top-0 z-[4] shrink-0 rounded-lg border border-primary/25 bg-base-200/90 px-2 py-2 shadow-md backdrop-blur-md sm:px-3">
        <div className="flex min-w-0 flex-col gap-2 sm:flex-row sm:flex-wrap sm:items-center sm:justify-between sm:gap-x-3 sm:gap-y-2">
          <div className="flex min-w-0 flex-1 flex-wrap items-center gap-2">
            <button
              type="button"
              className="btn btn-secondary btn-xs shrink-0 gap-1.5 px-3 font-sora font-semibold sm:btn-sm"
              onClick={onBack}
            >
              <span className="text-base leading-none opacity-90" aria-hidden>
                ←
              </span>
              {backLabel}
            </button>
            {fileName ? (
              <div className="min-w-0 max-w-full rounded-md border border-base-content/12 bg-base-100/50 px-2 py-1 sm:max-w-[min(100%,20rem)]">
                <p className="font-sora m-0 text-[8px] font-bold uppercase tracking-[0.14em] text-primary/90">
                  {fileContextLabel}
                </p>
                <p
                  className="m-0 truncate font-mono text-[11px] font-semibold leading-tight text-base-content"
                  title={fileName}
                >
                  {fileName}
                </p>
              </div>
            ) : null}
            {onDismiss ? (
              <button
                type="button"
                className="btn btn-circle btn-ghost btn-xs ml-auto shrink-0 border border-error/40 text-error hover:bg-error/10"
                aria-label={dismissLabel}
                title={dismissAriaLabel ?? dismissLabel}
                onClick={onDismiss}
              >
                ×
              </button>
            ) : null}
          </div>
          <div className="flex min-w-0 flex-wrap items-center gap-2 border-t border-base-content/10 pt-2 sm:border-t-0 sm:pt-0">
            <span className="font-syne max-w-[14rem] truncate text-[10px] font-bold uppercase tracking-[0.12em] text-base-content/70 sm:max-w-[18rem]">
              {title}
            </span>
            {children ? <div className="flex flex-wrap gap-1.5 sm:justify-end">{children}</div> : null}
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="sticky top-0 z-[4] shrink-0 rounded-xl border border-warning/35 bg-base-200/95 px-3 py-3 shadow-md backdrop-blur-md sm:px-4">
      <div className="flex min-w-0 flex-col gap-3">
        <div className="flex min-w-0 items-start justify-between gap-2">
          <div className="flex min-w-0 flex-1 flex-col gap-2 sm:flex-row sm:items-center sm:gap-3">
            <button
              type="button"
              className="btn btn-secondary btn-sm shrink-0 gap-2 px-4 font-sora font-semibold shadow-md sm:btn-md"
              onClick={onBack}
            >
              <span className="text-lg leading-none opacity-90" aria-hidden>
                ←
              </span>
              {backLabel}
            </button>
            {fileName ? (
              <div className="min-w-0 max-w-full flex-1 rounded-lg border border-warning/25 bg-warning/10 px-3 py-2 sm:min-w-[12rem] lg:max-w-lg">
                <p className="font-sora m-0 text-[9px] font-bold uppercase tracking-[0.18em] text-warning/90">
                  {fileContextLabel}
                </p>
                <p className="m-0 truncate font-mono text-[12px] font-semibold leading-snug text-base-content sm:text-[13px]" title={fileName}>
                  {fileName}
                </p>
              </div>
            ) : null}
          </div>
          {onDismiss ? (
            <button
              type="button"
              className="btn btn-circle btn-sm shrink-0 border border-error/80 bg-error/90 text-base font-bold leading-none text-error-content shadow-sm hover:bg-error"
              aria-label={dismissLabel}
              title={dismissAriaLabel ?? dismissLabel}
              onClick={onDismiss}
            >
              ×
            </button>
          ) : null}
        </div>
        <div className="flex min-w-0 flex-1 flex-wrap items-center gap-2 border-t border-warning/20 pt-3">
          <span className="font-syne min-w-0 shrink text-xs font-bold uppercase tracking-[0.14em] text-base-content/75">{title}</span>
          {children ? (
            <div className="flex w-full flex-wrap gap-2 lg:ml-auto lg:w-auto lg:justify-end">{children}</div>
          ) : null}
        </div>
      </div>
    </div>
  );
}
