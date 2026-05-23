/**
 * Voix atelier : pas de « écran » dans les résumés sauf proximité (nappe sur l’ensemble écran).
 */
import type { CatalogCtx } from './workshopCatalog';
import type { WorkshopCardDraft } from './workshopTypes';

export const MIC2_11_CAUSE =
  'Connecteur nappe power oxydé, FPC mal clipé ou nappe HS. Pistes : chute, liquide, vitre arrière / flash, pièce aftermarket.';

const IPHONE11_MIC2 = {
  likelyCause: MIC2_11_CAUSE,
  steps: [
    'Connecteur carte mère (J6400) : oxydation, broches pliées, clip mal fermé',
    'Historique : chute, eau, vitre arrière ou module flash',
    'Resiérer le FPC power (clic net) puis test boot > 3 minutes',
    'Nappe bouton power OEM si la nappe est suspecte',
    'Ligne mic2 / board si FPC et nappe sont OK',
  ],
  note: 'MIC2 sur 11 = nappe power côté flash. Ne pas partir sur l’écouteur en premier.',
};

const IPHONE11_MIC1_PRS = {
  likelyCause:
    'Nappe port Lightning absente, aftermarket ou FPC dock oxydé. Sur 11, brancher aussi la nappe power si le baromètre/mic1 est sur le bus dock.',
  steps: [
    'Nappe port Lightning + nappe power bien clipées (les deux sont souvent nécessaires)',
    'Connecteur dock : nettoyage, pas de broches pliées',
    'Nappe port OEM · test tenue > 3 min',
  ],
};

const PROX_PACK = {
  likelyCause: 'Liquide, poussière, FPC prox mal clipé ou pré-ensemble aftermarket sur l’écran.',
  steps: [
    'Sécher / nettoyer zone prox et connecteur avant',
    'Vérifier nappe prox bien branchée sur l’écran (FPC clic net)',
    'Pré-ensemble avant OEM · test appel + capteur prox',
  ],
};

/** Seule zone où le mot « écran » est pertinent. */
export function isProximityContext(draft: WorkshopCardDraft): boolean {
  const t = `${draft.id} ${draft.title} ${draft.component} ${draft.keywords?.join(' ') ?? ''}`.toLowerCase();
  return (
    /proximit|\bprox\b|capteur[s]?\s+avant|pré-ensemble/.test(t) &&
    !/bouton\s*power|mic2|flash|lightning|dock\b|port\s+lightning|usb-?c/i.test(t)
  );
}

/** Retire toute mention d’écran hors contexte prox. */
export function sanitizeEcranMentions(text: string, allowEcran: boolean): string {
  if (!text?.trim()) return text;
  if (allowEcran) {
    return text
      .replace(/écran\s+mal\s+remonté/gi, 'nappe prox mal branchée sur l’écran')
      .replace(/réparation\s+écran/gi, 'pose pré-ensemble / écran')
      .replace(/\s+/g, ' ')
      .trim();
  }

  let s = text;
  const clauses = [
    /[^.!?]*\bécran\b[^.!?]*/gi,
    /[^.!?]*\bswap\s+écran\b[^.!?]*/gi,
    /[^.!?]*\bchangement\s+d['’]?écran\b[^.!?]*/gi,
  ];
  for (const re of clauses) {
    s = s.replace(re, '');
  }
  s = s
    .replace(/\bécran\s+ou\s+/gi, '')
    .replace(/\bou\s+écran\b/gi, '')
    .replace(/\bécran\s*\/\s*/gi, '')
    .replace(/\s*[,;]\s*[,;]+/g, ',')
    .replace(/\s{2,}/g, ' ')
    .replace(/^[,;.\s—–-]+|[,;.\s—–-]+$/g, '')
    .replace(/\(\s*\)/g, '')
    .trim();

  if (s.length < 12) return text.replace(/\bécran\b/gi, '').replace(/\s+/g, ' ').trim();
  return s;
}

function isIphone11PowerFlexMic2(ctx: CatalogCtx, draft: WorkshopCardDraft): boolean {
  if (ctx.section !== 'iphone-11') return false;
  const blob = `${draft.id} ${draft.title} ${draft.component}`.toLowerCase();
  return (
    ctx.missing.has('mic2') ||
    blob.includes('mic2') ||
    /bouton\s*power|power.*flash|micro\s+côté\s+flash/.test(blob)
  );
}

function pickTechnicianPack(ctx: CatalogCtx, draft: WorkshopCardDraft): Partial<WorkshopCardDraft> | null {
  if (isIphone11PowerFlexMic2(ctx, draft)) return IPHONE11_MIC2;

  if (
    ctx.section === 'iphone-11' &&
    (ctx.missing.has('mic1') || ctx.missing.has('prs0')) &&
    /lightning|port|baromètre|dock/i.test(`${draft.title} ${draft.component}`)
  ) {
    return IPHONE11_MIC1_PRS;
  }

  if (isProximityContext(draft)) return PROX_PACK;

  if (/bouton\s*power|nappe\s+bouton\s+power/i.test(`${draft.title} ${draft.component}`)) {
    if (ctx.section === 'iphone-11') return IPHONE11_MIC2;
    return {
      likelyCause: 'FPC power mal clipé, oxydation connecteur ou nappe HS.',
      steps: ['Connecteur power : oxydation, clip', 'Nappe power OEM', 'Diode mode si persiste'],
    };
  }

  if (/port\s+lightning|connecteur\s+de\s+charge|nappe\s+port/i.test(`${draft.title} ${draft.component}`)) {
    return {
      likelyCause: 'FPC dock oxydé, nappe charge aftermarket, connecteur carte mère ou liquide bas de châssis.',
      steps: [
        'Connecteur dock propre · nappe port OEM',
        'Test charge + tenue boot > 3 min',
        'Diode mode FPC si persiste',
      ],
    };
  }

  return null;
}

function polishField(value: string | undefined, draft: WorkshopCardDraft): string | undefined {
  if (!value) return value;
  return sanitizeEcranMentions(value, isProximityContext(draft));
}

/** Réécrit cause / étapes / notes — « écran » uniquement si prox. */
export function applyTechnicianVoice(draft: WorkshopCardDraft, ctx: CatalogCtx): WorkshopCardDraft {
  const pack = pickTechnicianPack(ctx, draft);
  const prox = isProximityContext(draft) || (pack === PROX_PACK);

  let likelyCause = pack?.likelyCause ?? draft.likelyCause;
  let note = pack?.note ?? draft.note;
  let steps = pack?.steps ?? draft.steps;

  if (isIphone11PowerFlexMic2(ctx, draft)) {
    likelyCause = IPHONE11_MIC2.likelyCause;
    note = IPHONE11_MIC2.note;
    steps = IPHONE11_MIC2.steps;
  }

  likelyCause = polishField(likelyCause, draft) ?? likelyCause;
  note = polishField(note, draft);
  steps = steps.map((s) => polishField(s, draft) ?? s).filter((s) => s.length > 4);

  return {
    ...draft,
    title: polishField(pack?.title ?? draft.title, draft) ?? draft.title,
    component: pack?.component ?? draft.component,
    likelyCause,
    steps,
    note,
  };
}
