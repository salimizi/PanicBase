import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react';

import referenceBundle from '../generated/panicReferenceSections.json';
import { inferPanicReferenceFocus } from '../lib/inferPanicReferenceFocus';
import type { AnalysisResult } from '../types/analysis';
import { useOptionalI18n } from '../i18n/context';
import { readStoredLocale, translateKey } from '../i18n/translations';

import '../styles/panic-reference-enriched.css';

type ReferenceSectionRow = {
  htmlId: string;
  key: string;
  innerHtml: string;
};

const REFERENCE_SECTIONS: ReferenceSectionRow[] = referenceBundle.sections;

function stripInlineHandlers(html: string): string {
  return html.replace(/\s+on\w+="[^"]*"/gi, '').replace(/\s+on\w+='[^']*'/gi, '');
}

function useRefNavGroups(t: (key: string) => string) {
  return useMemo(
    () => [
      {
        label: t('ref.nav.preSmc'),
        items: [
          { key: 'iphone-x', label: t('ref.nav.iphoneX') },
          { key: 'iphone-11', label: t('ref.nav.iphone11') },
          { key: 'iphone-12', label: t('ref.nav.iphone12') },
        ],
      },
      {
        label: t('ref.nav.smcEra'),
        items: [
          { key: 'iphone-13', label: t('ref.nav.iphone13') },
          { key: 'iphone-14', label: t('ref.nav.iphone14') },
          { key: 'iphone-14pro', label: t('ref.nav.iphone14pro') },
          { key: 'iphone-15', label: t('ref.nav.iphone15') },
          { key: 'iphone-15pro', label: t('ref.nav.iphone15pro') },
          { key: 'iphone-16', label: t('ref.nav.iphone16') },
        ],
      },
      {
        label: t('ref.nav.reference'),
        items: [
          { key: 'universal', label: t('ref.nav.universal') },
          { key: 'product-ids', label: t('ref.nav.productIds') },
          { key: 'enriched', label: t('ref.nav.enriched') },
        ],
      },
    ],
    [t],
  );
}

type Props = {
  immersive?: boolean;
  panicText?: string;
  analysis?: AnalysisResult | null;
  productType?: string | null;
};

/**
 * Référence enrichie intégrée en React (même logique que le HTML : nav, sections,
 * recherche « toutes sections + cartes ouvertes », cartes repliables).
 */
export function PanicReferenceEnrichedPanel({
  immersive = false,
  panicText = '',
  analysis = null,
  productType = null,
}: Props) {
  const i18n = useOptionalI18n();
  const locale = i18n?.locale ?? readStoredLocale();
  const t = i18n?.t ?? ((key: string) => translateKey(locale, key));
  const navGroups = useRefNavGroups(t);

  const sectionsHtml = useMemo(
    () =>
      REFERENCE_SECTIONS.map((s) => ({
        key: s.key,
        htmlId: s.htmlId,
        innerHtml: stripInlineHandlers(s.innerHtml),
      })),
    [],
  );

  const focusKey = useMemo(
    () =>
      [
        productType ?? '',
        String(panicText.length),
        panicText.slice(0, 128),
        analysis?.signature_hash ?? '',
      ].join('|'),
    [productType, panicText, analysis?.signature_hash],
  );

  const focus = useMemo(() => {
    if (!analysis) {
      return { navSection: 'iphone-x', confidence: 0, initialSearch: '' };
    }
    return inferPanicReferenceFocus({ panicText, analysis, productType });
  }, [panicText, analysis, productType]);

  const [navSection, setNavSection] = useState(focus.navSection);
  const [search, setSearch] = useState(focus.initialSearch);
  const [noResults, setNoResults] = useState(false);
  const mainRef = useRef<HTMLElement>(null);
  const prevFocusKey = useRef(focusKey);

  useEffect(() => {
    if (prevFocusKey.current === focusKey) return;
    prevFocusKey.current = focusKey;
    setNavSection(focus.navSection);
    setSearch(focus.initialSearch);
  }, [focusKey, focus.navSection, focus.initialSearch]);

  const searching = search.trim().length > 0;

  const applySearchToMain = useCallback(() => {
    const root = mainRef.current;
    if (!root) return;
    const q = search.trim().toLowerCase();
    const cards = root.querySelectorAll<HTMLElement>('.card, .univ-card');
    if (!q) {
      cards.forEach((c) => c.classList.remove('hidden'));
      setNoResults(false);
      return;
    }
    let found = 0;
    cards.forEach((card) => {
      const text = (card.textContent ?? '').toLowerCase();
      if (text.includes(q)) {
        card.classList.remove('hidden');
        card.classList.add('open');
        found++;
      } else {
        card.classList.add('hidden');
      }
    });
    setNoResults(found === 0);
  }, [search]);

  useLayoutEffect(() => {
    applySearchToMain();
  }, [applySearchToMain]);

  function showSection(id: string) {
    setNavSection(id);
    setSearch('');
    setNoResults(false);
  }

  function onMainClick(e: React.MouseEvent<HTMLElement>) {
    const target = e.target as HTMLElement;
    const cardHeader = target.closest('.card-header');
    if (cardHeader?.parentElement?.classList.contains('card')) {
      cardHeader.parentElement.classList.toggle('open');
      return;
    }
    const univHeader = target.closest('.univ-header');
    if (univHeader?.parentElement?.classList.contains('univ-card')) {
      univHeader.parentElement.classList.toggle('open');
    }
  }

  const rootClass = 'panic-ref-root panic-ref-root--embedded';

  const core = (
    <>
      <div className="topbar">
        <div className="topbar-dot" />
        <div>
          <div className="topbar-title">{t('ref.topbarTitle')}</div>
          <div className="topbar-sub">{t('ref.topbarSub')}</div>
        </div>
      </div>

      <div className="layout">
        <nav className="sidebar">
          <div className="sidebar-title">{t('ref.sidebarModels')}</div>
          {navGroups.map((g) => (
            <div key={g.label}>
              <div className="nav-group-label">{g.label}</div>
              {g.items.map((item) => (
                <div
                  key={item.key}
                  role="button"
                  tabIndex={0}
                  className={`nav-item ${!searching && navSection === item.key ? 'active' : ''}`}
                  onClick={() => showSection(item.key)}
                  onKeyDown={(ev) => {
                    if (ev.key === 'Enter' || ev.key === ' ') {
                      ev.preventDefault();
                      showSection(item.key);
                    }
                  }}
                >
                  <span className="nav-dot" />
                  {item.label}
                </div>
              ))}
            </div>
          ))}
        </nav>

        <main
          ref={mainRef}
          className="main"
          onClick={onMainClick}
          role="presentation"
        >
          <div className="search-wrap">
            <span className="search-icon">⌕</span>
            <input
              type="text"
              placeholder={t('ref.searchPlaceholder')}
              value={search}
              onChange={(ev) => setSearch(ev.target.value)}
              aria-label={t('ref.searchAria')}
            />
          </div>
          <div className={`no-results${noResults ? ' is-visible' : ''}`}>{t('ref.noResults')}</div>

          {sectionsHtml.map((s) => (
            <div
              key={s.key}
              id={s.htmlId}
              className={`section ${searching || s.key === navSection ? 'active' : ''}`}
              dangerouslySetInnerHTML={{ __html: s.innerHtml }}
            />
          ))}
        </main>
      </div>

      <div
        style={{
          padding: '20px 32px',
          color: '#777',
          fontFamily: '"IBM Plex Mono", monospace',
          fontSize: 11,
          borderTop: '1px solid #222',
          marginTop: 40,
        }}
      >
        {t('ref.footerSources')}
      </div>
    </>
  );

  if (immersive) {
    return (
      <div className="flex min-h-0 flex-1 flex-col bg-[#0a0a0b]">
        <div className={rootClass}>{core}</div>
      </div>
    );
  }

  return (
    <section
      className="flex min-h-0 flex-col gap-2 rounded-2xl border border-primary/20 bg-base-200/30 p-3 shadow-sm backdrop-blur-sm sm:p-4"
      aria-labelledby="panic-ref-enriched-title"
    >
      <div className="min-w-0">
        <h3 id="panic-ref-enriched-title" className="font-sora m-0 text-sm font-semibold text-base-content">
          {t('detail.referenceTitle')}
        </h3>
        <p className="font-sora m-0 mt-1 text-[11px] leading-relaxed text-base-content/55">{t('detail.referenceHint')}</p>
      </div>
      <div className={`relative ${rootClass} min-h-[min(70vh,720px)] w-full flex-1 overflow-hidden rounded-xl border border-base-content/10 shadow-inner`}>
        <div className="max-h-[min(70vh,720px)] overflow-y-auto">{core}</div>
      </div>
    </section>
  );
}
