/**
 * Fiches atelier depuis `iphone_panic_database.json` (référence HTML fusionnée).
 * Règle produit : **1 seule fiche** par panic log (meilleur match).
 */
import db from '../data/iphone_panic_database.json';
import type { WorkshopCardDraft, WorkshopCardSeverity } from './workshopTypes';
import type { CatalogCtx } from './workshopCatalog';
import { MIC2_11_CAUSE } from './workshopCatalog';
import { matchExactMaskWorkshopCard, sanitizeWorkshopDraft } from './workshopMaskSemantics';

export type PanicDbRecord = {
  id: string;
  section_id: string;
  model_group?: string;
  code_or_pattern: string;
  component: string;
  panic_signature: string;
  severity: string;
  keywords: string[];
  info: Record<string, unknown>;
  diagnostic_steps: string[];
  notes: string[];
  search_text: string;
};

type PanicDatabase = {
  records: PanicDbRecord[];
};

const RECORDS = (db as unknown as PanicDatabase).records;

const SKIP_SECTIONS = new Set(['product-ids', 'diagnostic', 'dev-reference']);

function normSeverity(s: string): WorkshopCardSeverity {
  const u = s.toUpperCase();
  if (u === 'SOFTWARE') return 'SOFTWARE';
  if (u === 'COMBINÉ' || u === 'COMBINE') return 'COMBINÉ';
  if (u === 'BOARD-LEVEL') return 'BOARD-LEVEL';
  return 'HARDWARE';
}

function tokensFromPattern(pattern: string): string[] {
  return pattern
    .toLowerCase()
    .split(/[^a-z0-9]+/)
    .filter((t) => t.length >= 2);
}

function scoreRecord(rec: PanicDbRecord, ctx: CatalogCtx): number {
  if (SKIP_SECTIONS.has(rec.section_id)) return 0;

  const blob = ctx.blobLower;
  let score = 0;

  const sectionMatch = rec.section_id === ctx.section;
  const universal = rec.section_id === 'universal' || rec.section_id === 'enriched';
  if (sectionMatch) score += 35;
  else if (universal) score += 8;
  else if (ctx.section && rec.section_id !== ctx.section) return 0;

  const codeLc = rec.code_or_pattern.toLowerCase();
  if (codeLc && blob.includes(codeLc)) score += 55;

  for (const kw of rec.keywords) {
    const k = kw.toLowerCase();
    if (k.length >= 3 && blob.includes(k)) score += 18;
  }

  for (const m of ctx.missing) {
    if (codeLc.includes(m) || rec.search_text.toLowerCase().includes(m)) score += 42;
    for (const kw of rec.keywords) {
      if (kw.toLowerCase().includes(m)) score += 12;
    }
  }

  const codeTokens = tokensFromPattern(rec.code_or_pattern);
  for (const t of codeTokens) {
    if (t.length >= 4 && blob.includes(t)) score += 8;
  }

  if (rec.panic_signature) {
    const sigBits = rec.panic_signature
      .toLowerCase()
      .split(/[^a-z0-9]+/)
      .filter((t) => t.length >= 5);
    for (const t of sigBits) {
      if (blob.includes(t)) score += 6;
    }
  }

  const maskMatch = blob.match(/\b0x[a-f0-9]{2,12}\b/g);
  if (maskMatch) {
    for (const hx of maskMatch) {
      if (rec.search_text.toLowerCase().includes(hx)) score += 25;
    }
  }

  if (ctx.hasThermal && rec.keywords.some((k) => /thermal/i.test(k))) score += 10;
  if (ctx.hasMicTempSens2 && rec.keywords.some((k) => /mic-temp/i.test(k))) score += 12;

  return score;
}

function recordToCard(rec: PanicDbRecord, matchScore: number): WorkshopCardDraft {
  const info = rec.info ?? {};
  const str = (k: string) => {
    const v = info[k];
    return typeof v === 'string' ? v : '';
  };
  let cause = str('cause_fréquente') || str('cause_frequente') || 'Voir étapes atelier ci-dessous';
  if (rec.id === 'card-11-mic2') cause = MIC2_11_CAUSE;

  const badges = [rec.code_or_pattern, ...rec.keywords.filter((k) => k.length <= 32)].slice(0, 4);

  const draft: WorkshopCardDraft = {
    id: rec.id,
    matchScore,
    codeBadges: badges,
    severity: normSeverity(rec.severity),
    title: String(rec.component ?? ''),
    subtitle: String(rec.panic_signature || `Code log : ${rec.code_or_pattern}`),
    component: str('composant') || String(rec.component ?? ''),
    likelyCause: cause,
    keywords: rec.keywords.length ? [...rec.keywords] : [rec.code_or_pattern],
    quickTest: str('test_rapide') || undefined,
    steps: rec.diagnostic_steps?.length
      ? rec.diagnostic_steps
      : ['Recouper avec la fiche modèle dans la référence atelier.'],
    note: rec.notes?.filter(Boolean).join(' ') || undefined,
  };

  return sanitizeWorkshopDraft(draft, rec.notes);
}

/** Meilleure fiche DB pour ce panic, ou `null` si score trop faible. */
export function matchBestPanicDatabaseCard(
  ctx: CatalogCtx,
  opts?: { skipMask?: boolean },
): WorkshopCardDraft | null {
  if (!opts?.skipMask) {
    const maskCard = matchExactMaskWorkshopCard(ctx);
    if (maskCard) return maskCard;
  }

  let best: { rec: PanicDbRecord; score: number } | null = null;

  for (const rec of RECORDS) {
    const score = scoreRecord(rec, ctx);
    if (score < 45) continue;
    if (!best || score > best.score) best = { rec, score };
  }

  if (!best) return null;
  return recordToCard(best.rec, Math.min(100, best.score));
}

export function panicDatabaseRecordCount(): number {
  return RECORDS.length;
}
