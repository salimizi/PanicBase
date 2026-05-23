/**
 * Fiches atelier — fusion résolveur cohérent + catalogue par modèle (patch Claude).
 * Règle produit : **1 seule fiche** par panic log (meilleur score).
 */
import type { AnalysisResult } from '../types/analysis';
import { primaryModelWorkshopCard, resolveModelWorkshopCards } from './workshopCardsByModel';
import {
  buildWorkshopContext,
  resolveCoherentWorkshopCard,
  resolveCoherentWorkshopCards,
  type WorkshopResolveOpts,
} from './workshopResolver';
import type { Locale } from '../i18n/translations';
import { applyTechnicianVoice } from './workshopTechnicianCopy';
import { localizeWorkshopCard } from './workshopCardLocale';
import type { ModelWorkshopCard } from './workshopCardsByModel';
import type { ReferenceWorkshopCard, WorkshopCardDraft } from './workshopTypes';

export type { ReferenceWorkshopCard, WorkshopCardSeverity } from './workshopTypes';

function toUiCard(draft: Omit<ReferenceWorkshopCard, 'uiKey'>, uiKey: string): ReferenceWorkshopCard {
  return { ...draft, uiKey };
}

function polishModelCard(
  card: ModelWorkshopCard,
  opts: WorkshopResolveOpts,
  uiKey: string,
  locale: Locale,
): ReferenceWorkshopCard {
  const ctx = buildWorkshopContext(opts);
  const polished = applyTechnicianVoice(card as WorkshopCardDraft, ctx);
  const ui = toUiCard(polished, uiKey);
  return localizeWorkshopCard(ui, locale);
}

function finalizeUiCard(card: ReferenceWorkshopCard, locale: Locale): ReferenceWorkshopCard {
  return localizeWorkshopCard(card, locale);
}

function pickBestCard(opts: WorkshopResolveOpts, locale: Locale): ReferenceWorkshopCard | null {
  const coherent = resolveCoherentWorkshopCard(opts);
  const modelRaw = primaryModelWorkshopCard(opts.panicText, opts.analysis, opts.productType);

  if (!modelRaw && !coherent) return null;
  if (!modelRaw) return coherent ? finalizeUiCard(coherent, locale) : null;
  if (!coherent) {
    return polishModelCard(modelRaw, opts, `${modelRaw.id}__model`, locale);
  }

  const model = polishModelCard(modelRaw, opts, `${modelRaw.id}__model`, locale);
  const coherentUi = finalizeUiCard(coherent, locale);
  return model.matchScore >= coherentUi.matchScore ? model : coherentUi;
}

export function resolveWorkshopReferenceCards(
  panicText: string,
  analysis: AnalysisResult,
  productType: string | null,
  locale: Locale,
): ReferenceWorkshopCard[] {
  const opts: WorkshopResolveOpts = { panicText, analysis, productType };
  const best = pickBestCard(opts, locale);
  return best ? [best] : [];
}

export function primaryWorkshopCard(
  panicText: string,
  analysis: AnalysisResult,
  productType: string | null,
  locale: Locale,
): ReferenceWorkshopCard | null {
  return resolveWorkshopReferenceCards(panicText, analysis, productType, locale)[0] ?? null;
}

/** Liste complète par modèle (panneau référence / debug) — peut retourner plusieurs fiches. */
export function resolveAllModelWorkshopCards(
  panicText: string,
  analysis: AnalysisResult,
  productType: string | null,
  locale: Locale,
): ReferenceWorkshopCard[] {
  const opts: WorkshopResolveOpts = { panicText, analysis, productType };
  return resolveModelWorkshopCards(panicText, analysis, productType).map((c, i) =>
    polishModelCard(c, opts, `${c.id}__${i}`, locale),
  );
}

export function resolveCoherentOnly(opts: WorkshopResolveOpts): ReferenceWorkshopCard[] {
  return resolveCoherentWorkshopCards(opts);
}

export type { WorkshopResolveOpts };
