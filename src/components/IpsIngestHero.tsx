import { useCallback, useState } from 'react';
import { useI18n } from '../i18n/context';

/** Hidden file input `accept` — keep in sync with supported extensions. */
export const IPS_FILE_ACCEPT = '.ips,.crash,.txt,.json,application/json,text/plain,text/xml';

const DROP_EXT = /\.(ips|crash|txt|json)$/i;

const FORMAT_BADGES = ['.ips', '.crash', '.txt', '.json'] as const;

export type IpsIngestHeroProps = {
  onBrowseClick: () => void;
  onAnalyze: (raw: string, fileName?: string) => void;
  onError: (msg: string) => void;
};

export function IpsIngestHero({ onBrowseClick, onAnalyze, onError }: IpsIngestHeroProps) {
  const { t } = useI18n();
  const [dragOver, setDragOver] = useState(false);
  const [paste, setPaste] = useState('');

  const readDroppedOrPickedFile = useCallback(
    (file: File) => {
      if (!DROP_EXT.test(file.name)) {
        onError(t('ips.dropUnsupported'));
        return;
      }
      const reader = new FileReader();
      reader.onload = () => {
        const text = typeof reader.result === 'string' ? reader.result : '';
        if (!text.trim()) {
          onError(t('ips.dropReadError'));
          return;
        }
        onAnalyze(text, file.name);
      };
      reader.onerror = () => onError(t('ips.dropReadError'));
      reader.readAsText(file, 'UTF-8');
    },
    [onAnalyze, onError, t],
  );

  const onDrop = useCallback(
    (e: React.DragEvent) => {
      e.preventDefault();
      e.stopPropagation();
      setDragOver(false);
      const f = e.dataTransfer.files?.[0];
      if (f) readDroppedOrPickedFile(f);
    },
    [readDroppedOrPickedFile],
  );

  const onPasteInterpret = () => {
    const raw = paste.trim();
    if (!raw) return;
    onAnalyze(raw, undefined);
  };

  return (
    <section
      className="relative z-0 shrink-0 overflow-hidden rounded-2xl border border-base-content/[0.07] bg-base-100/50 shadow-[0_22px_60px_-28px_rgba(0,0,0,0.35)] backdrop-blur-md dark:border-white/[0.08] dark:bg-base-100/[0.14] dark:shadow-[0_24px_70px_-30px_rgba(0,0,0,0.55)]"
      aria-label={t('aria.homeWelcome')}
    >
      {/* Fond décoratif */}
      <div
        className="pointer-events-none absolute inset-0 opacity-[0.55] dark:opacity-[0.42]"
        aria-hidden
      >
        <div className="absolute -left-1/4 top-0 h-[min(22rem,55%)] w-[min(28rem,70%)] rounded-full bg-primary/[0.11] blur-3xl dark:bg-primary/[0.14]" />
        <div className="absolute -right-1/5 bottom-0 h-[min(18rem,45%)] w-[min(24rem,65%)] rounded-full bg-secondary/[0.08] blur-3xl dark:bg-secondary/[0.12]" />
        <div className="absolute inset-0 bg-gradient-to-br from-transparent via-base-200/20 to-transparent dark:via-white/[0.03]" />
      </div>

      <div className="relative flex flex-col gap-0">
        {/* En-tête accueil */}
        <header className="border-b border-base-content/[0.06] px-5 py-5 sm:px-7 sm:py-6 dark:border-white/[0.06]">
          <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between sm:gap-4">
            <div className="min-w-0 flex-1 space-y-2">
              <div className="flex flex-wrap items-center gap-2">
                <span className="inline-flex items-center rounded-full border border-primary/25 bg-primary/10 px-2.5 py-0.5 font-sora text-[9px] font-semibold uppercase tracking-[0.12em] text-primary">
                  {t('ips.ingestBadge')}
                </span>
                <span className="font-sora text-[10px] font-medium uppercase tracking-wide text-base-content/40">
                  {t('ips.ingestLegend')}
                </span>
              </div>
              <h2 className="font-sora m-0 text-[clamp(1.25rem,3.5vw,1.65rem)] font-bold leading-tight tracking-tight text-base-content">
                {t('home.title')}
              </h2>
              <p className="font-sora m-0 max-w-[40rem] text-[13px] leading-relaxed text-base-content/65 sm:text-[14px]">
                {t('home.subtitle')}
              </p>
            </div>
          </div>

          <ul className="mt-4 flex list-none flex-wrap gap-2 p-0 sm:mt-5">
            <li>
              <span className="inline-flex items-center rounded-lg border border-base-content/10 bg-base-200/40 px-2.5 py-1.5 font-sora text-[10px] font-medium text-base-content/75 dark:border-white/[0.08] dark:bg-white/[0.05]">
                {t('home.chipLocal')}
              </span>
            </li>
            <li>
              <span className="inline-flex items-center rounded-lg border border-base-content/10 bg-base-200/40 px-2.5 py-1.5 font-sora text-[10px] font-medium text-base-content/75 dark:border-white/[0.08] dark:bg-white/[0.05]">
                {t('home.chipUsb')}
              </span>
            </li>
            <li>
              <span className="inline-flex items-center rounded-lg border border-base-content/10 bg-base-200/40 px-2.5 py-1.5 font-sora text-[10px] font-medium text-base-content/75 dark:border-white/[0.08] dark:bg-white/[0.05]">
                {t('home.chipBench')}
              </span>
            </li>
          </ul>
        </header>

        <div className="grid gap-5 p-5 sm:gap-6 sm:p-7 lg:grid-cols-2 lg:items-stretch">
          {/* Zone glisser-déposer */}
          <div className="group/drop relative flex min-h-0 min-w-0 flex-col">
            <div
              role="presentation"
              className={`relative flex min-h-[12.5rem] flex-1 flex-col items-center justify-center gap-3 rounded-xl border-2 border-dashed px-4 py-8 text-center transition-[border-color,box-shadow,background-color] duration-200 ease-out sm:min-h-[13rem] sm:px-6 sm:py-9 ${
                dragOver
                  ? 'border-primary/55 bg-primary/[0.08] ring-4 ring-primary/15 dark:border-primary/50 dark:bg-primary/[0.1]'
                  : 'border-base-content/14 bg-base-200/20 hover:border-primary/30 hover:bg-base-200/30 dark:border-white/12 dark:bg-white/[0.04] dark:hover:border-primary/35 dark:hover:bg-white/[0.07]'
              }`}
              onDragEnter={(e) => {
                e.preventDefault();
                setDragOver(true);
              }}
              onDragOver={(e) => {
                e.preventDefault();
                e.dataTransfer.dropEffect = 'copy';
              }}
              onDragLeave={(e) => {
                e.preventDefault();
                if (e.currentTarget === e.target) setDragOver(false);
              }}
              onDrop={onDrop}
            >
              <div
                className={`flex h-14 w-14 items-center justify-center rounded-2xl bg-gradient-to-br from-primary/20 to-primary/5 text-primary shadow-inner ring-1 ring-primary/15 transition-transform duration-200 dark:from-primary/25 dark:to-primary/8 dark:ring-primary/20 ${
                  dragOver ? 'scale-105' : 'group-hover/drop:scale-[1.02]'
                }`}
              >
                <svg className="h-7 w-7" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" aria-hidden>
                  <path strokeLinecap="round" strokeLinejoin="round" d="M12 16V5m0 0l4 4m-4-4l-4 4M5 19h14" />
                </svg>
              </div>

              <div className="space-y-1.5">
                <p className="font-sora m-0 text-[15px] font-semibold tracking-tight text-base-content sm:text-base">
                  {dragOver ? t('ips.dropActive') : t('ips.dropTitle')}
                </p>
                <p className="font-sora m-0 max-w-[36ch] text-[12.5px] leading-relaxed text-base-content/55 sm:text-[13px]">
                  {t('ips.dropHint')}
                </p>
                <p className="font-sora m-0 text-[10px] leading-snug text-base-content/40">{t('ips.dropFocusHint')}</p>
              </div>

              <div className="mt-1 flex flex-wrap items-center justify-center gap-1.5">
                {FORMAT_BADGES.map((ext) => (
                  <span
                    key={ext}
                    className="rounded-md border border-base-content/8 bg-base-100/60 px-2 py-0.5 font-mono text-[10px] font-medium text-base-content/60 dark:border-white/10 dark:bg-base-100/10"
                  >
                    {ext}
                  </span>
                ))}
              </div>

              <div className="mt-2 flex flex-wrap items-center justify-center gap-2">
                <button
                  type="button"
                  className="btn btn-primary btn-sm h-10 min-h-10 rounded-xl px-6 font-sora font-semibold shadow-sm"
                  onClick={onBrowseClick}
                >
                  {t('ips.dropBrowse')}
                </button>
              </div>
            </div>
          </div>

          {/* Coller */}
          <div className="flex min-h-0 min-w-0 flex-col rounded-xl border border-base-content/[0.07] bg-base-200/25 p-4 shadow-inner dark:border-white/[0.07] dark:bg-white/[0.04] sm:p-5">
            <p className="font-sora m-0 mb-2 text-[13px] font-semibold text-base-content/85">{t('ips.pasteTitle')}</p>
            <textarea
              className="textarea textarea-bordered mb-3 min-h-[6.5rem] w-full flex-1 resize-y border-base-content/12 bg-base-100/80 text-[12px] leading-relaxed dark:border-white/10 dark:bg-base-100/12"
              placeholder={t('ips.pastePlaceholder')}
              value={paste}
              onChange={(e) => setPaste(e.target.value)}
              spellCheck={false}
            />
            <div className="mt-auto flex flex-wrap items-center gap-2">
              <button
                type="button"
                className="btn btn-primary btn-sm rounded-lg font-sora"
                onClick={onPasteInterpret}
                disabled={!paste.trim()}
              >
                {t('ips.pasteInterpret')}
              </button>
              <button type="button" className="btn btn-ghost btn-sm font-sora" onClick={() => setPaste('')} disabled={!paste}>
                {t('ips.pasteClear')}
              </button>
              <span className="ms-auto font-mono text-[10px] text-base-content/35">{t('ips.pasteKbd')}</span>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}
