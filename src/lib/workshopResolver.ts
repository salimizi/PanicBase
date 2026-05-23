/**
 * Point d’entrée unique : 1 fiche cohérente par panic log, texte centré sur les pièces concernées.
 */
import type { AnalysisResult } from '../types/analysis';
import { inferPanicReferenceFocus } from './inferPanicReferenceFocus';
import { matchBestPanicDatabaseCard } from './iphonePanicDatabase';
import {
  buildCatalogWorkshopCards,
  intelligentLikelyCause,
  partTitleFromDiagnosticLine,
  type CatalogCtx,
} from './workshopCatalog';
import { matchExactMaskWorkshopCard, sanitizeWorkshopDraft } from './workshopMaskSemantics';
import { applyTechnicianVoice } from './workshopTechnicianCopy';
import { stripDiagnosticCodesFragments } from './repairPartsSpeak';
import type { ReferenceWorkshopCard, WorkshopCardDraft } from './workshopTypes';

export type WorkshopResolveOpts = {
  panicText: string;
  analysis: AnalysisResult;
  productType: string | null;
};

function extractMissing(panicText: string, analysis: AnalysisResult): Set<string> {
  const out = new Set<string>();
  const re = /missing sensor\(s?\)?:?\s*([^\n\r]+)/gi;
  const hay = `${panicText}\n${analysis.structured_diagnostic.critical_lines?.join('\n') ?? ''}`;
  let m: RegExpExecArray | null;
  while ((m = re.exec(hay)) !== null) {
    for (const part of m[1].split(/[,;\s]+/)) {
      const t = part.replace(/[^a-z0-9-]/gi, '').toLowerCase();
      if (t.length >= 2 && t.length <= 16) out.add(t);
    }
  }
  return out;
}

/** Aligne les sections Claude (`iphone-14-pro`) avec le catalogue interne (`iphone-14pro`). */
function catalogSection(navSection: string): string {
  const map: Record<string, string> = {
    'iphone-14-pro': 'iphone-14pro',
    'iphone-15-pro': 'iphone-15pro',
    'iphone-13-pro': 'iphone-13pro',
    'iphone-12-pro': 'iphone-12pro',
    'iphone-16-pro': 'iphone-16pro',
    'iphone-17-pro': 'iphone-17pro',
  };
  return map[navSection] ?? navSection;
}

export function buildWorkshopContext(opts: WorkshopResolveOpts): CatalogCtx {
  const { panicText, analysis, productType } = opts;
  const focus = inferPanicReferenceFocus({ panicText, analysis, productType });
  const blobLower = [
    panicText,
    analysis.probable_cause,
    ...(analysis.keywords ?? []),
    ...(analysis.structured_diagnostic.normalized_signatures ?? []),
    ...(analysis.structured_diagnostic.critical_lines ?? []),
    ...(analysis.structured_diagnostic.wiki_hints ?? []),
    ...(analysis.structured_diagnostic.possible_causes?.map((c) => c.name) ?? []),
    ...(analysis.structured_diagnostic.likely_parts ?? []),
  ]
    .join('\n')
    .toLowerCase();

  return {
    blobLower,
    section: catalogSection(focus.navSection),
    missing: extractMissing(panicText, analysis),
    hasThermal: blobLower.includes('thermalmonitord') || blobLower.includes('no successful checkins'),
    hasMicTempSens2: /\bmic-temp-sens2\b/i.test(blobLower),
  };
}

/** Texte atelier : ne garde que le vocabulaire des pièces listées dans `component`. */
export function enforcePartFocusedCopy(draft: WorkshopCardDraft, ctx?: CatalogCtx): WorkshopCardDraft {
  let base = sanitizeWorkshopDraft(draft);
  if (ctx) base = applyTechnicianVoice(base, ctx);
  const parts = base.component.split('·').map((p) => p.trim()).filter(Boolean);
  const title =
    parts.length >= 2 ? parts.slice(0, 2).join(' · ') : base.title || base.component;

  const humanKeywords = base.keywords.filter(
    (k) => !/^0x[a-f0-9]+$/i.test(k) && !/^masque/i.test(k),
  );

  return {
    ...base,
    title,
    component: base.component || title,
    likelyCause: base.likelyCause,
    keywords: humanKeywords.length ? humanKeywords : base.codeBadges.slice(0, 2),
    subtitle: base.subtitle.replace(/\b0x[a-f0-9]{2,12}\b/gi, '').replace(/\s+/g, ' ').trim() || base.subtitle,
  };
}

function withUiKey(draft: WorkshopCardDraft, ctx: CatalogCtx): ReferenceWorkshopCard {
  return { ...enforcePartFocusedCopy(draft, ctx), uiKey: `${draft.id}__0` };
}

function fallbackFromAnalysis(opts: WorkshopResolveOpts, ctx: CatalogCtx): WorkshopCardDraft | null {
  const { analysis } = opts;
  const sd = analysis.structured_diagnostic;
  const raw = analysis.probable_cause?.replace(/\s*\[Repair Wiki\]\s*$/i, '').trim();
  if (!raw || /^Non classifié|Unclassified/i.test(raw)) return null;

  const partFromList = (sd.likely_parts ?? []).map((p) => stripDiagnosticCodesFragments(p)).filter(Boolean)[0];
  const title =
    partFromList || partTitleFromDiagnosticLine(raw) || stripDiagnosticCodesFragments(raw.split('·').pop()?.trim() ?? raw);
  if (!title || /^0x[\da-f]+$/i.test(title)) return null;

  const steps = [...(sd.action_plan ?? []), ...(sd.isolation_sequence ?? []), ...(sd.recommended_checks ?? [])]
    .map((s) => s.trim())
    .filter(Boolean)
    .slice(0, 6);

  return {
    id: 'fallback-structured',
    matchScore: 40,
    codeBadges: [analysis.panic_type.replace(/_/g, ' ')].filter((b) => b.length > 2),
    severity: /firmware fatal|software|dfu/i.test(raw) ? 'SOFTWARE' : 'HARDWARE',
    title: title.slice(0, 120),
    subtitle: sd.marketing_name || analysis.device_model || '',
    component: (sd.likely_parts ?? []).filter(Boolean).join(' · ').slice(0, 120) || title,
    likelyCause: intelligentLikelyCause(analysis, ctx),
    keywords: (analysis.keywords ?? []).slice(0, 6),
    steps: steps.length ? steps : ['Tester la nappe OEM liée au capteur cité dans le log.'],
    note: sd.danger_flags?.[0],
  };
}

/**
 * Résout la fiche atelier unique pour un panic log.
 * Priorité : masques SMC par modèle → capteurs nommés → base JSON → repli structuré.
 */
export function resolveCoherentWorkshopCard(opts: WorkshopResolveOpts): ReferenceWorkshopCard | null {
  const ctx = buildWorkshopContext(opts);

  const mask = matchExactMaskWorkshopCard(ctx);
  if (mask) return withUiKey(mask, ctx);

  const catalog = buildCatalogWorkshopCards(ctx, opts.analysis)
    .sort((a, b) => b.matchScore - a.matchScore);
  if (catalog[0]) return withUiKey(catalog[0], ctx);

  const db = matchBestPanicDatabaseCard(ctx, { skipMask: true });
  if (db) return withUiKey(db, ctx);

  const fb = fallbackFromAnalysis(opts, ctx);
  if (fb) return withUiKey(fb, ctx);

  return null;
}

export function resolveCoherentWorkshopCards(opts: WorkshopResolveOpts): ReferenceWorkshopCard[] {
  const one = resolveCoherentWorkshopCard(opts);
  return one ? [one] : [];
}

/** Résumé court pour exports / autres écrans (même logique que la fiche). */
export function coherentWorkshopHeadline(opts: WorkshopResolveOpts): string {
  const card = resolveCoherentWorkshopCard(opts);
  if (card) return card.title;
  const p = opts.analysis.probable_cause?.replace(/\s*\[Repair Wiki\]\s*$/i, '').trim();
  return p?.split('·').pop()?.trim() ?? p ?? '';
}
