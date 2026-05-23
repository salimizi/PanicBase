import type { Locale } from '../i18n/translations';
import { localizeDiagnosticText } from './diagnosticLocale';
import type { ReferenceWorkshopCard } from './workshopTypes';

/** Libellés UI de la fiche atelier (pas le contenu métier du catalogue). */
export function workshopSeverityLabel(
  severity: ReferenceWorkshopCard['severity'],
  t: (key: string) => string,
): string {
  switch (severity) {
    case 'SOFTWARE':
      return t('summary.sheetSeveritySw');
    case 'BOARD-LEVEL':
      return t('summary.sheetSeverityBoard');
    case 'COMBINÉ':
      return t('summary.sheetSeverityCombined');
    default:
      return t('summary.sheetSeverityHw');
  }
}

/** Traduit le texte métier FR des fiches atelier pour l’UI non-FR. */
export function localizeWorkshopCard(card: ReferenceWorkshopCard, locale: Locale): ReferenceWorkshopCard {
  if (locale === 'fr') return card;
  const loc = (s: string) => localizeDiagnosticText(s, locale);
  return {
    ...card,
    title: loc(card.title),
    subtitle: loc(card.subtitle),
    component: loc(card.component),
    likelyCause: loc(card.likelyCause),
    quickTest: card.quickTest ? loc(card.quickTest) : card.quickTest,
    steps: card.steps.map(loc),
    note: card.note ? loc(card.note) : card.note,
    keywords: card.keywords.map((k) => loc(k)),
    codeBadges: card.codeBadges.map((b) => loc(b)),
  };
}
