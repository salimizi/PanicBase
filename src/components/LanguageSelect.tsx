import { useEffect, useRef, useState } from 'react';
import { useI18n } from '../i18n/context';
import { SUPPORTED_LOCALES, type Locale } from '../i18n/translations';

const FLAG_ICON_URL_BY_LOCALE: Record<Locale, string> = {
  en: 'https://flagcdn.com/24x18/gb.png',
  fr: 'https://flagcdn.com/24x18/fr.png',
};

export function LanguageSelect() {
  const { locale, setLocale, t } = useI18n();
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);
  const cur = SUPPORTED_LOCALES.find((x) => x.id === locale) ?? SUPPORTED_LOCALES[0];

  useEffect(() => {
    if (!open) return;
    const onDoc = (e: MouseEvent) => {
      const el = rootRef.current;
      if (el && !el.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener('mousedown', onDoc);
    return () => document.removeEventListener('mousedown', onDoc);
  }, [open]);

  const pick = (id: Locale) => {
    setLocale(id);
    setOpen(false);
  };

  return (
    <div ref={rootRef} className="relative" title={t('nav.language')}>
      <button
        type="button"
        className="btn btn-sm h-8 min-h-8 gap-2 rounded-lg border border-base-content/15 bg-base-100/85 px-2.5 font-normal text-base-content shadow-[0_1px_2px_rgba(15,23,42,0.04)] backdrop-blur-sm transition-[border-color,box-shadow,background] hover:border-primary/45 hover:bg-base-100 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/40"
        aria-expanded={open}
        aria-haspopup="listbox"
        aria-label={t('nav.language')}
        onClick={() => setOpen((o) => !o)}
      >
        <img
          src={FLAG_ICON_URL_BY_LOCALE[cur.id]}
          alt=""
          aria-hidden
          className="h-[14px] w-[19px] rounded-[2px] border border-base-content/25 object-cover"
          loading="lazy"
          decoding="async"
        />
        <span className="font-sora hidden text-[11px] font-medium leading-none text-base-content/90 sm:inline">
          {cur.label}
        </span>
        <span className="font-mono text-[10px] font-semibold uppercase tracking-wide text-base-content/65">{locale}</span>
      </button>
      {open ? (
        <ul
          role="listbox"
          className="menu absolute right-0 top-full z-[60] mt-1 w-48 rounded-box border border-base-300 bg-base-100 p-1 shadow-lg"
        >
          {SUPPORTED_LOCALES.map((l) => (
            <li key={l.id} role="option" aria-selected={l.id === locale}>
              <button
                type="button"
                className={`font-sora flex w-full cursor-pointer items-center rounded-lg px-2 py-2 text-left text-sm transition-colors hover:bg-base-200/90 focus:bg-base-200/90 focus:outline-none ${l.id === locale ? 'bg-base-200/70 font-semibold' : ''}`}
                onMouseDown={(e) => e.preventDefault()}
                onClick={() => pick(l.id)}
              >
                <img
                  src={FLAG_ICON_URL_BY_LOCALE[l.id]}
                  alt=""
                  aria-hidden
                  className="me-2 h-[14px] w-[19px] rounded-[2px] border border-base-content/20 object-cover"
                  loading="lazy"
                  decoding="async"
                />
                {l.label}
              </button>
            </li>
          ))}
        </ul>
      ) : null}
    </div>
  );
}
