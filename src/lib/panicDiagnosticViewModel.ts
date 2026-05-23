/**
 * Modèle de présentation unique pour l’UI atelier (fiche type HTML enrichi).
 * Toute la dérivation depuis `AnalysisResult` est ici — les composants ne font qu’afficher.
 */
import type { Locale } from '../i18n/translations';
import type { AnalysisResult } from '../types/analysis';
import { localizeDiagnosticText } from './diagnosticLocale';
import { primaryPartHeadline, stripDiagnosticCodesFragments } from './repairPartsSpeak';
import { localizeWorkshopCard } from './workshopCardLocale';
import { resolveCoherentWorkshopCard } from './workshopResolver';
import type { ReferenceWorkshopCard } from './workshopTypes';

export type PanicDiagnosticSeverity = 'hw' | 'sw';

export type PanicDiagnosticSecondary = { name: string; pct: number };

export type PanicDiagnosticViewModel = {
  unclear: boolean;
  model: string;
  title: string;
  subtitle: string;
  severity: PanicDiagnosticSeverity;
  badges: string[];
  component: string;
  likelyCauseLine: string;
  priorityValue: string;
  scoreLine: string;
  keywords: string[];
  steps: string[];
  notes: string[];
  signals: string[];
  secondary: PanicDiagnosticSecondary[];
};

type TFn = (key: string, vars?: Record<string, string | number>) => string;

export function shortenDiagnosticLine(s: string, max: number): string {
  const t = s.replace(/\s+/g, ' ').trim();
  if (t.length <= max) return t;
  return `${t.slice(0, max - 1)}…`;
}

function cleanCauseName(s: string): string {
  return s.replace(/\s*\[Repair Wiki\]\s*$/i, '').trim();
}

function formatPanicFamily(pt: string): string {
  return pt.replace(/_/g, ' ').trim();
}

/** English fallbacks for panic_type slugs not covered by i18n keys. */
const PANIC_TYPE_LABELS_EN: Record<string, string> = {
  applesochot_soc_thermal: 'AppleSocHot · thermal SoC',
  smc_bsc_outbox_chain: 'SMC / BSC / OUTBOX',
  aop_nmi_power: 'AOP NMI · power',
  no_valid_cfg_nand: 'NAND · invalid configuration',
  ans2_storage: 'ANS2 · storage',
  baseband_panic: 'Baseband panic',
  sep_panic: 'SEP panic',
  aop_panic: 'AOP panic',
  undefined_kernel_instruction: 'Invalid kernel instruction',
  unclassified_panicstring: '',
  unknown: '',
};

function subtitleFromPanicType(pt: string, t: TFn): string {
  const slug = pt.trim().toLowerCase();
  if (!slug) return '';
  const i18nKey = `workbench.panicFamily.${slug}`;
  const i18nVal = t(i18nKey);
  if (i18nVal !== i18nKey) return i18nVal.trim();
  const mapped = PANIC_TYPE_LABELS_EN[slug];
  if (mapped !== undefined) return mapped.trim();
  if (/^bug_/.test(slug)) return formatPanicFamily(slug.replace(/^bug_\d+_/, ''));
  return formatPanicFamily(pt);
}

function uniqSteps(items: string[], max: number): string[] {
  const seen = new Set<string>();
  const out: string[] = [];
  for (const raw of items) {
    const s = raw.trim().replace(/^\d+[\].)\s]+/i, '').trim();
    if (!s) continue;
    const k = s.toLowerCase().replace(/\s+/g, ' ');
    if (seen.has(k)) continue;
    seen.add(k);
    out.push(raw.trim());
    if (out.length >= max) break;
  }
  return out;
}

function keywordChips(analysis: AnalysisResult, max: number): string[] {
  const seen = new Set<string>();
  const out: string[] = [];
  const push = (s: string) => {
    const t = s.trim();
    if (!t || t.length > 48) return;
    const k = t.toLowerCase();
    if (seen.has(k)) return;
    seen.add(k);
    out.push(t);
  };
  for (const k of analysis.keywords) push(k);
  for (const k of analysis.structured_diagnostic.normalized_signatures) push(k);
  return out.slice(0, max);
}

function badgeTexts(analysis: AnalysisResult, max: number): string[] {
  const seen = new Set<string>();
  const out: string[] = [];
  const push = (s: string) => {
    const t = stripDiagnosticCodesFragments(cleanCauseName(s)).trim();
    if (!t || t.length < 4 || t.length > 56) return;
    const k = t.toLowerCase();
    if (seen.has(k)) return;
    seen.add(k);
    out.push(t.slice(0, 56));
  };
  const primary = cleanCauseName(analysis.probable_cause);
  const head = primary.split('·')[0]?.trim();
  if (head) push(head);
  for (const c of analysis.structured_diagnostic.possible_causes) {
    push(c.name.split('·')[0]?.trim() ?? c.name);
    if (out.length >= max) break;
  }
  for (const sig of analysis.structured_diagnostic.normalized_signatures) {
    push(sig);
    if (out.length >= max) break;
  }
  return out.slice(0, max);
}

function severityKind(analysis: AnalysisResult): PanicDiagnosticSeverity {
  const blob = `${analysis.panic_type}\n${analysis.probable_cause}\n${analysis.keywords.join('\n')}`.toLowerCase();
  if (/\bfirmware fatal\b|software issue|dfu restore|itunes restore|ios corrupt/.test(blob)) return 'sw';
  return 'hw';
}

function repairPriorityLabel(code: string, t: TFn): string {
  switch (code.trim().toLowerCase()) {
    case 'high':
      return t('workshop.priority.high');
    case 'medium':
      return t('workshop.priority.mid');
    case 'low':
      return t('workshop.priority.cautious');
    default:
      return code || '—';
  }
}

export function analysisIsDiagnosticUnclear(analysis: AnalysisResult): boolean {
  const sd = analysis.structured_diagnostic;
  if ((sd.possible_causes?.length ?? 0) > 0) return false;
  const p = analysis.probable_cause?.trim() ?? '';
  return /^Non classifié|Unclassified/i.test(p) || analysis.confidence < 8;
}

function viewModelFromWorkshopCard(
  card: ReferenceWorkshopCard,
  analysis: AnalysisResult,
  locale: Locale,
  t: TFn,
): PanicDiagnosticViewModel {
  const ui = localizeWorkshopCard(card, locale);
  const sd = analysis.structured_diagnostic;
  const pct = Math.round(Math.max(0, Math.min(1, sd.confidence_global)) * 100);
  const sev: PanicDiagnosticSeverity = ui.severity === 'SOFTWARE' ? 'sw' : 'hw';

  return {
    unclear: false,
    model: analysis.device_model,
    title: ui.title,
    subtitle: ui.subtitle,
    severity: sev,
    badges: ui.codeBadges.slice(0, 4),
    component: ui.component,
    likelyCauseLine: ui.likelyCause,
    priorityValue: repairPriorityLabel(sd.repair_priority, t),
    scoreLine: t('summary.sheetScoreLine', { pct: String(pct) }),
    keywords: ui.keywords.filter((k) => !/^0x[a-f0-9]+$/i.test(k)).slice(0, 8),
    steps: ui.steps,
    notes: ui.note ? [ui.note] : [],
    signals: [],
    secondary: [],
  };
}

export type BuildPanicDiagnosticOpts = {
  panicText?: string;
  productType?: string | null;
};

export function buildPanicDiagnosticViewModel(
  analysis: AnalysisResult,
  locale: Locale,
  t: TFn,
  opts?: BuildPanicDiagnosticOpts,
): PanicDiagnosticViewModel {
  const sd = analysis.structured_diagnostic;

  if (opts?.panicText !== undefined) {
    const card = resolveCoherentWorkshopCard({
      panicText: opts.panicText,
      analysis,
      productType: opts.productType ?? null,
    });
    if (card) return viewModelFromWorkshopCard(card, analysis, locale, t);
  }

  if (analysisIsDiagnosticUnclear(analysis)) {
    return {
      unclear: true,
      model: analysis.device_model,
      title: '',
      subtitle: '',
      severity: 'hw',
      badges: [],
      component: '',
      likelyCauseLine: '',
      priorityValue: '',
      scoreLine: '',
      keywords: keywordChips(analysis, 12),
      steps: [],
      notes: [...(sd.danger_flags ?? []), ...(sd.wiki_hints ?? [])]
        .slice(0, 8)
        .map((n) => localizeDiagnosticText(n, locale)),
      signals: (sd.critical_lines ?? [])
        .slice(0, 4)
        .map((l) => shortenDiagnosticLine(localizeDiagnosticText(l, locale), 240)),
      secondary: [],
    };
  }

  const title = localizeDiagnosticText(
    stripDiagnosticCodesFragments(cleanCauseName(analysis.probable_cause)),
    locale,
  );
  const subtitle = subtitleFromPanicType(analysis.panic_type, t).trim();

  const partsHead = (sd.likely_parts ?? []).filter(Boolean).join(' · ');
  const spoke = primaryPartHeadline(analysis, locale, t);
  const component = partsHead || spoke || title;

  let likelyCauseLine = sd.possible_causes[0]?.name
    ? localizeDiagnosticText(stripDiagnosticCodesFragments(cleanCauseName(sd.possible_causes[0].name)), locale)
    : title;
  if (likelyCauseLine.trim().toLowerCase() === title.trim().toLowerCase() && sd.possible_causes[1]?.name) {
    likelyCauseLine = localizeDiagnosticText(
      stripDiagnosticCodesFragments(cleanCauseName(sd.possible_causes[1].name)),
      locale,
    );
  }
  const duplicateLikely =
    !likelyCauseLine.trim() || likelyCauseLine.trim().toLowerCase() === title.trim().toLowerCase();

  const steps = uniqSteps(
    [...(sd.action_plan ?? []), ...(sd.isolation_sequence ?? []), ...(sd.recommended_checks ?? [])].map((s) =>
      localizeDiagnosticText(s, locale),
    ),
    14,
  );

  const pct = Math.round(Math.max(0, Math.min(1, sd.confidence_global)) * 100);
  const likelyNorm = duplicateLikely ? '' : likelyCauseLine.trim().toLowerCase();
  const titleNorm = title.trim().toLowerCase();
  const secondary = (sd.possible_causes ?? [])
    .map((c) => ({
      name: localizeDiagnosticText(stripDiagnosticCodesFragments(cleanCauseName(c.name)), locale),
      pct: Math.round(Math.max(0, Math.min(1, c.confidence)) * 100),
    }))
    .filter((c) => {
      const n = c.name.trim().toLowerCase();
      if (n === titleNorm) return false;
      if (likelyNorm && n === likelyNorm) return false;
      return true;
    })
    .slice(0, 5);

  const notes = uniqSteps(
    [...(sd.danger_flags ?? []), ...(sd.wiki_hints ?? [])].map((n) => localizeDiagnosticText(n, locale)),
    10,
  );

  const signals = (sd.critical_lines ?? [])
    .slice(0, 5)
    .map((l) => shortenDiagnosticLine(localizeDiagnosticText(l, locale), 220));

  return {
    unclear: false,
    model: analysis.device_model,
    title,
    subtitle,
    severity: severityKind(analysis),
    badges: badgeTexts(analysis, 6),
    component,
    likelyCauseLine: duplicateLikely ? '' : likelyCauseLine,
    priorityValue: repairPriorityLabel(sd.repair_priority, t),
    scoreLine: t('summary.sheetScoreLine', { pct: String(pct) }),
    keywords: keywordChips(analysis, 16),
    steps,
    notes,
    signals,
    secondary,
  };
}
