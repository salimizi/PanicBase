/**
 * Masques SMC composites — libellés atelier cohérents (alignés repair_wiki.rs).
 * Priorité sur les entrées JSON ambiguës (« Prox + Bouton power » alors que 0x40000 = Lightning).
 */
import type { CatalogCtx } from './workshopCatalog';
import { extractSensorMaskValues } from './workshopCatalog';
import type { WorkshopCardDraft, WorkshopCardSeverity } from './workshopTypes';

type MaskFicheDef = {
  id: string;
  sections: string[];
  maskValues: number[];
  severity: WorkshopCardSeverity;
  title: string;
  subtitle: string;
  component: string;
  likelyCause: string;
  codeBadges: string[];
  keywords: string[];
  steps: string[];
  note?: string;
};

/** Plus le masque est spécifique (triple > double), plus le score de tri est haut. */
const MASK_FICHES: MaskFicheDef[] = [
  // ── iPhone 14 Pro / Pro Max ──
  {
    id: '14p-mask-1c0000',
    sections: ['iphone-14pro'],
    maskValues: [0x1c0000, 1_835_008],
    severity: 'COMBINÉ',
    title: 'Proximité + Bouton power + Port Lightning',
    subtitle: 'Triple défaut SMC · masque 0x1C0000',
    component: 'Nappe capteurs avant · Nappe bouton power · Nappe port Lightning',
    likelyCause: 'Choc, liquide, ou plusieurs FPC débranchés (prox, power, dock)',
    codeBadges: ['0x1C0000', 'triple défaut'],
    keywords: ['proximité', 'bouton power', 'port Lightning'],
    steps: [
      'Rebrancher les trois nappes (avant, power, dock) une par une',
      'Tester housing OEM complet connu bon',
      'Remplacer la nappe suspecte en premier selon historique client (chute, eau, liquide)',
      'Si code identique → séparation sandwich / carte mère',
    ],
    note: '0x80000 prox · 0x100000 power · 0x40000 Lightning.',
  },
  {
    id: '14p-mask-180000',
    sections: ['iphone-14pro'],
    maskValues: [0x180000, 1_572_864],
    severity: 'COMBINÉ',
    title: 'Proximité + Bouton power',
    subtitle: 'Double défaut SMC · masque 0x180000',
    component: 'Nappe capteurs avant (proximité) · Nappe bouton power',
    likelyCause:
      'FPC prox (sur l’écran) ou power mal clipé, liquide zone Face ID, oxydation connecteurs',
    codeBadges: ['0x180000', 'prox + power'],
    keywords: ['proximité', 'bouton power'],
    steps: [
      'Vérifier nappe capteurs avant + nappe bouton power',
      'Remplacer la nappe la plus suspecte en OEM',
      'Si un seul sous-code reste → isoler prox (0x80000) ou power (0x100000)',
    ],
    note: '0x80000 = proximité · 0x100000 = bouton power (pas le port Lightning).',
  },
  {
    id: '14p-mask-140000',
    sections: ['iphone-14pro'],
    maskValues: [0x140000, 1_310_720],
    severity: 'COMBINÉ',
    title: 'Bouton power + Port Lightning',
    subtitle: 'Double défaut SMC · masque 0x140000',
    component: 'Nappe bouton power · Nappe port Lightning',
    likelyCause: 'FPC power ou dock mal clipé, oxydation connecteurs, liquide bas de châssis',
    codeBadges: ['0x140000', 'power + dock'],
    keywords: ['bouton power', 'port Lightning'],
    steps: [
      'Rebrancher nappe power puis nappe port Lightning',
      'Remplacer en OEM la nappe qui ne rétablit pas le boot',
    ],
    note: '0x100000 = power · 0x40000 = port Lightning.',
  },
  {
    id: '14p-mask-c0000',
    sections: ['iphone-14pro'],
    maskValues: [0xc0000, 786_432],
    severity: 'COMBINÉ',
    title: 'Proximité + Port Lightning',
    subtitle: 'Double défaut SMC · masque 0xC0000',
    component: 'Nappe capteurs avant (proximité) · Nappe port Lightning',
    likelyCause:
      'Nappe prox mal branchée sur l’écran, ou port Lightning débranché — ce n’est pas le bouton power (voir 0x100000).',
    codeBadges: ['0xC0000', 'prox + Lightning'],
    keywords: ['proximité', 'nappe port Lightning', '0xC0000'],
    steps: [
      'Remplacer ou rebrancher la nappe capteurs avant (proximité)',
      'Remplacer ou rebrancher la nappe port Lightning OEM',
      'Si un seul sous-code persiste → traiter prox (0x80000) ou dock (0x40000) seul',
    ],
    note: '0x80000 = proximité · 0x40000 = port Lightning. Pas confondre avec bouton power.',
  },
  {
    id: '14p-mask-80000',
    sections: ['iphone-14pro'],
    maskValues: [0x80000, 524_288],
    severity: 'HARDWARE',
    title: 'Nappe capteurs avant (proximité)',
    subtitle: 'Masque 0x80000',
    component: 'Nappe capteurs avant / proximité',
    likelyCause: 'Liquide, pré-ensemble aftermarket sur l’écran, connecteur face avant',
    codeBadges: ['0x80000'],
    keywords: ['proximité', 'capteurs avant'],
    steps: [
      'Vérifier connecteur face avant sur carte mère',
      'Remplacer nappe capteurs avant OEM',
    ],
  },
  {
    id: '14p-mask-40000',
    sections: ['iphone-14pro'],
    maskValues: [0x40000, 262_144],
    severity: 'HARDWARE',
    title: 'Nappe port Lightning',
    subtitle: 'Masque 0x40000',
    component: 'Nappe port Lightning (connecteur de charge)',
    likelyCause: 'FPC dock, liquide, nappe aftermarket',
    codeBadges: ['0x40000'],
    keywords: ['port Lightning', 'connecteur de charge'],
    steps: ['Remplacer nappe port Lightning OEM', 'Microscope + diode mode si persiste'],
  },
  {
    id: '14p-mask-100000',
    sections: ['iphone-14pro'],
    maskValues: [0x100000, 1_048_576],
    severity: 'HARDWARE',
    title: 'Nappe bouton power',
    subtitle: 'Masque 0x100000',
    component: 'Nappe bouton power / volume',
    likelyCause: 'FPC power mal clipé ou oxydation connecteur',
    codeBadges: ['0x100000'],
    keywords: ['bouton power'],
    steps: ['Reconnecter nappe power', 'Remplacer nappe power OEM'],
  },
  // ── iPhone 13 (ex. 0x1800) ──
  {
    id: '13-mask-1800',
    sections: ['iphone-13'],
    maskValues: [0x1800, 6144],
    severity: 'COMBINÉ',
    title: 'Port Lightning + Proximité',
    subtitle: 'Double défaut · masque 0x1800',
    component: 'Nappe port Lightning · Nappe proximité',
    likelyCause: 'Dock et prox KO : liquide, FPC dock ou nappe prox mal branchée sur l’écran',
    codeBadges: ['0x1800', '0x800 + 0x1000'],
    keywords: ['connecteur de charge', 'proximité'],
    steps: [
      'Nappe prox bien branchée sur écran OEM connu bon',
      'Tester nappe charge OEM',
      'Puis pré-ensemble avant si code persiste',
    ],
    note: 'Composition 0x800 (charge) + 0x1000 (prox).',
  },
  {
    id: '13-mask-800',
    sections: ['iphone-13'],
    maskValues: [0x800, 2048],
    severity: 'HARDWARE',
    title: 'Nappe connecteur de charge',
    subtitle: 'Masque 0x800',
    component: 'Nappe port Lightning',
    likelyCause: 'FPC dock oxydé, aftermarket, liquide',
    codeBadges: ['0x800'],
    keywords: ['connecteur de charge'],
    steps: ['Swap nappe charge OEM', 'Nettoyer connecteur carte mère'],
  },
  {
    id: '13-mask-1000',
    sections: ['iphone-13'],
    maskValues: [0x1000, 4096],
    severity: 'HARDWARE',
    title: 'Nappe proximité',
    component: 'Nappe capteurs avant (proximité)',
    subtitle: 'Masque 0x1000',
    likelyCause: 'Liquide, nappe prox mal branchée sur l’écran, pré-ensemble absent',
    codeBadges: ['0x1000'],
    keywords: ['proximité'],
    steps: ['Vérifier nappe avant', 'Remplacer pré-ensemble avant OEM'],
  },
  {
    id: '13-mask-4000',
    sections: ['iphone-13'],
    maskValues: [0x4000, 16384],
    severity: 'HARDWARE',
    title: 'Données batterie',
    component: 'Batterie · FPC batterie · BMS',
    subtitle: 'Masque 0x4000',
    likelyCause: 'Connecteur batterie, cellule, gas gauge',
    codeBadges: ['0x4000'],
    keywords: ['batterie', 'BMS'],
    steps: ['Batterie OEM', 'FPC propre', 'Diode mode BMS'],
  },
  // ── iPhone 14 / 14 Plus ──
  {
    id: '14-mask-400000',
    sections: ['iphone-14'],
    maskValues: [0x400000, 4_194_304],
    severity: 'HARDWARE',
    title: 'Bobine recharge sans fil · Vitre arrière',
    component: 'Nappe Qi · vitre arrière',
    subtitle: 'Masque 0x400000',
    likelyCause: 'Vitre arrière aftermarket, bobine Qi débranchée',
    codeBadges: ['0x400000'],
    keywords: ['recharge sans fil', 'Qi'],
    steps: ['Vérifier nappe Qi', 'Vitre arrière OEM / SSP'],
  },
  {
    id: '14-mask-100000',
    sections: ['iphone-14'],
    maskValues: [0x100000, 1_048_576],
    severity: 'HARDWARE',
    title: 'Nappe port Lightning',
    component: 'Nappe connecteur de charge (Lightning)',
    subtitle: 'Masque 0x100000',
    likelyCause: 'FPC dock, liquide, aftermarket',
    codeBadges: ['0x100000'],
    keywords: ['port Lightning'],
    steps: ['Nappe Lightning OEM', 'Connecteur carte mère propre'],
  },
  {
    id: '14-mask-200000',
    sections: ['iphone-14'],
    maskValues: [0x200000, 2_097_152],
    severity: 'HARDWARE',
    title: 'Nappe capteurs avant',
    component: 'Nappe proximité / capteurs avant',
    subtitle: 'Masque 0x200000',
    likelyCause: 'Nappe prox mal branchée sur l’écran, liquide',
    codeBadges: ['0x200000'],
    keywords: ['proximité', 'capteurs avant'],
    steps: ['Nappe avant OEM', 'Housing test'],
  },
  {
    id: '14-mask-600000',
    sections: ['iphone-14'],
    maskValues: [0x600000, 6_291_456],
    severity: 'COMBINÉ',
    title: 'Qi + Proximité',
    component: 'Bobine Qi · nappe capteurs avant',
    subtitle: 'Masque 0x600000',
    likelyCause: 'Vitre arrière et nappe prox : deux nappes à contrôler',
    codeBadges: ['0x600000'],
    keywords: ['Qi', 'proximité'],
    steps: ['Tester Qi seul', 'Puis nappe avant', 'Isoler la pièce qui fait tenir le boot'],
    note: '0x400000 Qi + 0x200000 prox.',
  },
  // ── iPhone 15 / 15 Plus ──
  {
    id: '15-mask-380000',
    sections: ['iphone-15'],
    maskValues: [0x380000, 3_670_016],
    severity: 'COMBINÉ',
    title: 'Qi + USB‑C + Proximité',
    component: 'Bobine Qi · nappe USB‑C · nappe capteurs avant',
    subtitle: 'Masque 0x380000',
    likelyCause: 'Plusieurs nappes débranchées (Qi, USB‑C, prox sur l’écran)',
    codeBadges: ['0x380000'],
    keywords: ['Qi', 'USB-C', 'proximité'],
    steps: ['Rebrancher Qi, USB‑C et avant une par une', 'Remplacer la nappe fautive en OEM'],
  },
  {
    id: '15-mask-280000',
    sections: ['iphone-15'],
    maskValues: [0x280000, 2_621_440],
    severity: 'COMBINÉ',
    title: 'Qi + Port USB‑C',
    component: 'Bobine Qi · nappe port USB‑C',
    subtitle: 'Masque 0x280000',
    likelyCause: 'Vitre arrière ou port bas mal remontés',
    codeBadges: ['0x280000'],
    keywords: ['Qi', 'USB-C'],
    steps: ['Vérifier nappe Qi', 'Puis nappe USB‑C OEM'],
  },
  {
    id: '15-mask-80000',
    sections: ['iphone-15'],
    maskValues: [0x80000, 524_288],
    severity: 'HARDWARE',
    title: 'USB‑C + module micro du bas (MIC1)',
    component: 'Module micro bas MEMS + nappe USB‑C',
    subtitle: 'Masque 0x80000 — oxydation MEMS fréquente',
    likelyCause: 'Module micro bas oxydé, clip MIC1, liquide, flex aftermarket',
    codeBadges: ['0x80000', 'MIC1'],
    keywords: ['USB-C', 'MIC1', 'MEMS', 'oxydation'],
    steps: [
      'Inspecter connecteur MEMS + joint mousse',
      'Reseat clip MIC1 sur flex USB‑C',
      'Nappe USB‑C OEM',
    ],
  },
  {
    id: '15-mask-100000-prox',
    sections: ['iphone-15'],
    maskValues: [0x100000, 1_048_576],
    severity: 'HARDWARE',
    title: 'Nappe capteurs avant',
    component: 'Nappe proximité / capteurs avant',
    subtitle: 'Masque 0x100000 (série 15)',
    likelyCause: 'Nappe prox mal branchée sur l’écran, liquide zone Face ID',
    codeBadges: ['0x100000'],
    keywords: ['proximité', 'capteurs avant'],
    steps: ['Pré-ensemble avant OEM', 'Connecteur face avant propre'],
    note: 'Sur 15/15 Plus : 0x100000 = avant (pas power comme sur 14 Pro).',
  },
  // ── iPhone 15 Pro / Pro Max ──
  {
    id: '15p-mask-300000',
    sections: ['iphone-15pro'],
    maskValues: [0x300000, 3_145_728],
    severity: 'HARDWARE',
    title: 'USB‑C + module micro du bas (MIC1)',
    component: 'Module micro bas MEMS + nappe USB‑C Pro',
    subtitle: 'Masque 0x300000 — oxydation MEMS fréquente',
    likelyCause: 'Module micro bas oxydé, clip MIC1, liquide',
    codeBadges: ['0x300000', 'MIC1'],
    keywords: ['USB-C', 'MIC1', 'MEMS'],
    steps: [
      'Inspecter module MEMS + clip sur flex USB‑C',
      'Nappe USB‑C Pro OEM',
      'Ultrason si oxydé',
    ],
  },
  {
    id: '15p-mask-400000',
    sections: ['iphone-15pro'],
    maskValues: [0x400000, 4_194_304],
    severity: 'HARDWARE',
    title: 'Bobine recharge sans fil',
    component: 'Nappe Qi arrière',
    subtitle: 'Masque 0x400000',
    likelyCause: 'Vitre arrière, bobine Qi',
    codeBadges: ['0x400000'],
    keywords: ['Qi', 'vitre arrière'],
    steps: ['Nappe Qi', 'Vitre SSP si besoin'],
  },
  {
    id: '15p-mask-700000',
    sections: ['iphone-15pro'],
    maskValues: [0x700000, 7_340_032],
    severity: 'COMBINÉ',
    title: 'USB‑C + Qi',
    component: 'Nappe USB‑C · bobine Qi',
    subtitle: 'Masque 0x700000',
    likelyCause: 'Module bas / vitre arrière après réparation',
    codeBadges: ['0x700000'],
    keywords: ['USB-C', 'Qi'],
    steps: ['Isoler USB‑C puis Qi', 'Remplacer la nappe qui rétablit le boot'],
  },
  {
    id: '15p-mask-200000',
    sections: ['iphone-15pro'],
    maskValues: [0x200000, 2_097_152],
    severity: 'HARDWARE',
    title: 'Nappe proximité',
    component: 'Nappe capteurs avant',
    subtitle: 'Masque 0x200000',
    likelyCause: 'Écran, liquide, FPC avant',
    codeBadges: ['0x200000'],
    keywords: ['proximité'],
    steps: ['Nappe avant Pro OEM'],
  },
];

function sectionOk(fiche: MaskFicheDef, ctx: CatalogCtx): boolean {
  return fiche.sections.includes(ctx.section);
}

function maskFicheToCard(f: MaskFicheDef, mask: number): WorkshopCardDraft {
  const hex = `0x${mask.toString(16).toUpperCase()}`;
  return {
    id: f.id,
    matchScore: 98,
    codeBadges: f.codeBadges.length ? f.codeBadges : [hex],
    severity: f.severity,
    title: f.title,
    subtitle: f.subtitle,
    component: f.component,
    likelyCause: f.likelyCause,
    keywords: f.keywords,
    steps: f.steps,
    note: f.note,
  };
}

/** Meilleure fiche pour un masque SMC exact dans le log (source de vérité atelier). */
export function matchExactMaskWorkshopCard(ctx: CatalogCtx, blobExtra = ''): WorkshopCardDraft | null {
  const blob = `${ctx.blobLower}\n${blobExtra}`.toLowerCase();
  const masks = extractSensorMaskValues(blob);

  const ranked = [...MASK_FICHES].sort((a, b) => b.maskValues[0] - a.maskValues[0]);

  for (const mask of masks) {
    for (const f of ranked) {
      if (!sectionOk(f, ctx)) continue;
      if (!f.maskValues.some((m) => m === mask)) continue;
      return maskFicheToCard(f, mask);
    }
  }

  return null;
}

/** Nettoie titres JSON : pas de hex dans le titre, cause depuis notes si vide. */
export function sanitizeWorkshopDraft(draft: WorkshopCardDraft, recNotes?: string[]): WorkshopCardDraft {
  let title = draft.title
    .replace(/\s*\([^)]*0x[^)]*\)/gi, '')
    .replace(/\s*0x[a-f0-9]{2,12}(\s*\|\s*0x[a-f0-9]{2,12})*/gi, '')
    .replace(/\s+/g, ' ')
    .trim();

  if (/bouton\s*power.*0x40000|prox\s*\+\s*bouton\s*power.*lightning/i.test(draft.title)) {
    title = 'Proximité + Port Lightning';
  }

  let likelyCause = draft.likelyCause;
  if (!likelyCause || /^voir étapes/i.test(likelyCause)) {
    const note = recNotes?.find((n) => n.length > 20)?.trim();
    if (note && /nappe|lightning|prox|power|combiné/i.test(note)) {
      likelyCause = note.replace(/^Code combiné\s*=\s*/i, '').trim();
    }
  }

  const keywords = draft.keywords.filter((k) => !/^0x[a-f0-9]+$/i.test(k) || k === draft.codeBadges[0]);
  if (!keywords.length && draft.codeBadges[0]) keywords.push(draft.codeBadges[0]);

  return {
    ...draft,
    title: title || draft.component,
    likelyCause: likelyCause || draft.likelyCause,
    keywords: keywords.slice(0, 6),
    component: draft.component || title,
  };
}
