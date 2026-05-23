/**
 * Catalogue atelier : capteurs / masques SMC → pièce + causes intelligentes + étapes.
 * Utilisé par `referenceWorkshopCards.ts` pour tous les modèles.
 */
import type { AnalysisResult } from '../types/analysis';
import type { ReferenceWorkshopCard, WorkshopCardDraft, WorkshopCardSeverity } from './workshopTypes';
import { stripDiagnosticCodesFragments } from './repairPartsSpeak';
import { MIC2_11_CAUSE } from './workshopTechnicianCopy';

export { MIC2_11_CAUSE };

export type CatalogCtx = {
  blobLower: string;
  section: string;
  missing: Set<string>;
  hasThermal: boolean;
  hasMicTempSens2: boolean;
};

type FicheOverride = Partial<
  Pick<ReferenceWorkshopCard, 'title' | 'subtitle' | 'component' | 'likelyCause' | 'steps' | 'note' | 'quickTest'>
>;

type SensorFiche = {
  id: string;
  triggers: string[];
  matchScore: number;
  severity: WorkshopCardSeverity;
  codeBadge: string;
  title: string;
  subtitle: string;
  component: string;
  likelyCause: string;
  steps: string[];
  note?: string;
  quickTest?: string;
  bySection?: Record<string, FicheOverride>;
};

/** Masques SMC (valeur exacte ou bit unique) → fiche pièce sans hex dans le titre. */
type MaskFiche = {
  ficheId: string;
  masks: number[];
  matchScore: number;
  title: string;
  component: string;
  likelyCause: string;
  steps: string[];
  note?: string;
  sections?: string[];
};

const SENSOR_FICHES: SensorFiche[] = [
  {
    id: 'sensor-mic1',
    triggers: ['mic1'],
    matchScore: 98,
    severity: 'HARDWARE',
    codeBadge: 'Capteur manquant : mic1',
    title: 'Nappe port Lightning · Micro bas-gauche',
    subtitle: 'mic1 = microphone bas-gauche sur la nappe port',
    component: 'Nappe port Lightning (micro bas-gauche)',
    likelyCause: 'Eau, choc, nappe aftermarket sans capteur, FPC dock oxydé',
    steps: [
      'Vérifier nappe port Lightning ET nappe power branchées',
      'Nettoyer connecteur carte mère (oxydation)',
      'Remplacer nappe port Lightning OEM',
      'Si persiste → diode mode FPC dock + inspection carte mère',
    ],
    bySection: {
      'iphone-x': {
        title: 'Baromètre · Câble système bas-gauche',
        component: 'Câble système bas-gauche (mic1 / baromètre)',
        likelyCause: 'Eau, choc sur le câble système, connecteur mal enfoncé',
      },
    },
  },
  {
    id: 'sensor-mic2',
    triggers: ['mic2'],
    matchScore: 99,
    severity: 'HARDWARE',
    codeBadge: 'Capteur manquant : mic2',
    title: 'Nappe bouton power · Micro côté flash',
    subtitle: 'mic2 = micro près du flash · souvent sur nappe power/volume',
    component: 'Nappe bouton power (micro côté flash / caméra arrière)',
    likelyCause: MIC2_11_CAUSE,
    steps: [
      'Connecteur carte mère (J6400) : oxydation, broches, clip',
      'Historique : chute, eau, vitre arrière ou flash',
      'Resiérer FPC power · test boot > 3 min',
      'Nappe bouton power OEM',
      'Ligne mic2 board si FPC OK',
    ],
    note: 'Sur iPhone 11 : l’écouteur n’est en général pas la première pièce à changer.',
    bySection: {
      'iphone-11': {
        likelyCause: MIC2_11_CAUSE,
        steps: [
          'Connecteur carte mère (J6400) : oxydation, broches, clip',
          'Historique : vitre arrière / flash / chute / eau',
          'Resiérer FPC power · test boot > 3 min',
          'Nappe bouton power OEM',
        ],
        note: 'MIC2 sur 11 = nappe power côté flash. Ne pas partir sur l’écouteur en premier.',
      },
      'iphone-12': {
        title: 'Écouteur interne · Pré-ensemble avant',
        component: 'Écouteur / pré-ensemble capteurs avant',
        likelyCause: 'Grille micro bouchée, liquide, nappe avant aftermarket, mauvais contact FPC',
        steps: [
          'Tester pré-ensemble avant OEM connu bon',
          'Nettoyer grilles micro et connecteur',
          'Vérifier nappe capteurs avant bien clipée',
        ],
      },
      'iphone-13': {
        title: 'Écouteur interne · Nappe capteurs avant',
        component: 'Pré-ensemble avant / écouteur',
        likelyCause: 'Liquide, FPC avant, pré-ensemble aftermarket',
      },
      'iphone-x': {
        title: 'Écouteur interne · Nappe capteurs avant',
        component: 'Écouteur / nappe capteurs avant',
        likelyCause: 'Eau, nappe avant absente ou aftermarket',
      },
    },
  },
  {
    id: 'sensor-prs0',
    triggers: ['prs0', 'prs1'],
    matchScore: 96,
    severity: 'HARDWARE',
    codeBadge: 'Capteur manquant : Prs0',
    title: 'Baromètre · Nappe port Lightning',
    subtitle: 'prs0 = capteur pression atmosphérique (souvent sur nappe port)',
    component: 'Nappe port Lightning (baromètre intégré)',
    likelyCause: 'Nappe absente, aftermarket, FPC dock, oxydation',
    steps: [
      'Brancher nappe port Lightning OEM + nappe power si requise sur le modèle',
      'Nettoyer connecteur dock',
      'Remplacer nappe port Lightning',
      'Si persiste → board-level baromètre / bus I²C dock',
    ],
    quickTest: 'Nappe Lightning OEM connue bonne → si le téléphone tient >3 min, nappe confirmée',
  },
  {
    id: 'sensor-tg0',
    triggers: ['tg0b', 'tg0v', 'tb0v'],
    matchScore: 95,
    severity: 'HARDWARE',
    codeBadge: 'Capteur manquant : TG0B',
    title: 'Batterie non détectée · TIGRIS',
    subtitle: 'TG0B / TG0V = jauge batterie / lignes batterie',
    component: 'Batterie · connecteur batterie · IC TIGRIS',
    likelyCause: 'FPC batterie déconnecté, cellule HS, oxydation, TIGRIS ou lignes BMS',
    steps: [
      'Retirer/rebrancher connecteur batterie (nettoyer oxydation)',
      'Tester batterie OEM connue bonne',
      'Diode mode sur pins données batterie (OL = ligne ouverte)',
      'Inspecter TIGRIS et alimentation BMS au microscope',
    ],
  },
  {
    id: 'sensor-mic3',
    triggers: ['mic3'],
    matchScore: 90,
    severity: 'HARDWARE',
    codeBadge: 'Capteur manquant : mic3',
    title: 'Micro arrière · Nappe caméra',
    subtitle: 'mic3 = micro vidéo / caméra arrière',
    component: 'Nappe caméra arrière / micro vidéo',
    likelyCause: 'Caméra ou nappe arrière mal remontée, liquide, aftermarket',
    steps: [
      'Vérifier nappe caméra arrière et connecteurs',
      'Remplacer nappe caméra OEM si doute',
    ],
  },
  {
    id: 'sensor-mic4',
    triggers: ['mic4'],
    matchScore: 90,
    severity: 'HARDWARE',
    codeBadge: 'Capteur manquant : mic4',
    title: 'Micro supplémentaire · Nappe associée',
    subtitle: 'mic4 = micro auxiliaire selon génération',
    component: 'Nappe selon modèle (souvent avant ou dock)',
    likelyCause: 'Nappe mal connectée après réparation, liquide',
    steps: ['Identifier la nappe sur schéma modèle', 'Swap nappe OEM du bus concerné'],
  },
  {
    id: 'sensor-prox',
    triggers: ['prox', 'scmto'],
    matchScore: 88,
    severity: 'HARDWARE',
    codeBadge: 'Proximité / SCMto',
    title: 'Capteur de proximité · Nappe avant',
    subtitle: 'Proximité / SCMto — capteurs avant ou AOP',
    component: 'Nappe capteurs avant (proximité / Face ID selon modèle)',
    likelyCause: 'Liquide, nappe prox mal branchée sur l’écran, pré-ensemble aftermarket',
    steps: [
      'Sécher / nettoyer zone prox si liquide',
      'Vérifier nappe prox bien branchée sur l’écran (FPC clic net)',
      'Remplacer pré-ensemble avant OEM',
    ],
  },
];

const MASK_FICHES: MaskFiche[] = [
  {
    ficheId: 'charge-port',
    masks: [0x800, 2048],
    matchScore: 92,
    title: 'Nappe connecteur de charge',
    component: 'Nappe port de charge (Lightning ou USB‑C)',
    likelyCause: 'FPC dock oxydé, nappe charge aftermarket, connecteur carte mère, liquide',
    steps: ['Swap nappe charge OEM', 'Nettoyer connecteur carte mère', 'Mesure diode FPC dock'],
  },
  {
    ficheId: 'proximity',
    masks: [0x1000, 4096],
    matchScore: 91,
    title: 'Nappe capteur de proximité',
    component: 'Nappe proximité / capteurs avant',
    likelyCause: 'Liquide, nappe prox mal branchée sur l’écran, pré-ensemble absent',
    steps: ['Vérifier nappe prox sur l’écran', 'Remplacer pré-ensemble avant OEM'],
  },
  {
    ficheId: 'prox-charge-combo',
    masks: [0x1800, 6144],
    matchScore: 93,
    title: 'Proximité + connecteur de charge',
    component: 'Nappes avant + nappe charge',
    likelyCause: 'Double défaut nappes : souvent après chute d’eau ou réparation incomplète',
    steps: ['Tester nappe charge seule', 'Puis nappe avant', 'Isoler la nappe qui fait tenir le boot'],
  },
  {
    ficheId: 'battery-sensor',
    masks: [0x4000, 16384],
    matchScore: 90,
    title: 'Données / capteur batterie',
    component: 'Batterie · FPC batterie · BMS',
    likelyCause: 'Connecteur batterie, cellule, lignes jauge batterie',
    steps: ['Batterie OEM connue bonne', 'FPC batterie propre', 'Diode mode BMS'],
  },
  {
    ficheId: 'gyro-bottom',
    masks: [0x400, 1024],
    matchScore: 89,
    title: 'Gyroscope · Platine basse',
    component: 'Bottom board / gyro (souvent 13 mini)',
    likelyCause: 'Chute, oxydation bottom board, nappe mal branchée',
    steps: ['Inspecter bottom board', 'Vérifier nappe port si combo 0xC00'],
    sections: ['iphone-13'],
  },
  {
    ficheId: 'power-button',
    masks: [0x100000, 1_048_576],
    matchScore: 91,
    title: 'Nappe bouton power',
    component: 'Nappe bouton power / volume',
    likelyCause:
      'FPC power mal clipé, oxydation connecteur, vitre arrière / flash mal remontés sur modèles où la nappe est côté châssis (ex. iPhone 11 mic2)',
    steps: [
      'Connecteur power : oxydation, broches, clip',
      'Historique : vitre arrière, flash, chute, eau',
      'Remplacer nappe power OEM',
    ],
  },
  {
    ficheId: 'front-qi',
    masks: [0x200000, 2_097_152],
    matchScore: 90,
    title: 'Nappe capteurs avant ou recharge sans fil',
    component: 'Nappe proximité / Qi selon modèle',
    likelyCause: 'Vitre arrière mal remontée, nappe Qi ou capteurs avant débranchés',
    steps: ['Identifier modèle sur fiche référence', 'Swap nappe avant ou bobine Qi OEM'],
  },
  {
    ficheId: 'usbc-flex',
    masks: [0x300000, 3_145_728],
    matchScore: 91,
    title: 'Nappe connecteur USB‑C',
    component: 'Nappe port USB‑C (série 15 Pro)',
    likelyCause: 'FPC USB‑C, liquide, module charge bas, flex aftermarket',
    steps: ['Nappe USB‑C OEM', 'Inspecter connecteur carte mère', 'Module charge si court sur VBUS'],
  },
  {
    ficheId: 'qi-coil',
    masks: [0x400000, 4_194_304],
    matchScore: 90,
    title: 'Bobine recharge sans fil · Vitre arrière',
    component: 'Nappe Qi / vitre arrière',
    likelyCause: 'Vitre arrière aftermarket, bobine Qi débranchée ou HS',
    steps: ['Vérifier nappe Qi', 'Tester vitre arrière OEM', 'Contrôler alignement bobine'],
  },
  {
    ficheId: 'battery-comms',
    masks: [0x500000, 5_242_880],
    matchScore: 88,
    title: 'Communication batterie · Taptic ou charge',
    component: 'Batterie · nappe charge · Taptic selon contexte',
    likelyCause: 'BMS, nappe charge, ou Taptic sur le même bus selon modèle',
    steps: ['Batterie OEM', 'Isoler nappe charge puis Taptic'],
  },
  {
    ficheId: 'prox-charge-14',
    masks: [0x80000, 524_288],
    matchScore: 90,
    title: 'Nappe proximité / charge (série 14–15)',
    component: 'Nappe proximité ou connecteur charge',
    likelyCause: 'Liquide, FPC prox sur l’écran ou FPC dock',
    steps: ['Swap nappe concernée OEM', 'Nettoyer connecteurs'],
  },
  {
    ficheId: 'charge-pro',
    masks: [0x40000, 262_144],
    matchScore: 90,
    title: 'Nappe connecteur de charge (Pro)',
    component: 'Nappe USB‑C / Lightning Pro',
    likelyCause: 'Dock Pro, liquide, connecteur carte mère',
    steps: ['Nappe charge OEM Pro', 'Inspection FPC'],
  },
  {
    ficheId: 'battery-data',
    masks: [0x41, 65, 0xa1, 161, 0xa9],
    matchScore: 89,
    title: 'Capteur ou données batterie',
    component: 'Batterie · BMS · FPC batterie',
    likelyCause: 'FPC batterie, cellule, jauge batterie',
    steps: ['Batterie connue bonne', 'Nettoyer FPC', 'Diode mode lignes BMS'],
  },
];

export function extractSensorMaskValues(blob: string): number[] {
  const vals = new Set<number>();
  for (const m of blob.matchAll(/\b0x([a-f0-9]{2,12})\b/gi)) {
    const n = parseInt(m[1], 16);
    if (Number.isFinite(n) && n > 0) vals.add(n);
  }
  for (const m of blob.matchAll(/s\.sensor array[^0-9\n]*?(\d{2,12})/gi)) {
    const n = Number(m[1]);
    if (n > 0) vals.add(n);
  }
  for (const m of blob.matchAll(/\((\d{3,12})\)/g)) {
    const n = Number(m[1]);
    if (n >= 256) vals.add(n);
  }
  return [...vals];
}

function applyOverride(base: WorkshopCardDraft, ov?: FicheOverride): WorkshopCardDraft {
  if (!ov) return base;
  return {
    ...base,
    ...ov,
    steps: ov.steps?.length ? ov.steps : base.steps,
  };
}

function sensorToCard(fiche: SensorFiche, ctx: CatalogCtx, trigger: string): WorkshopCardDraft {
  const ov = fiche.bySection?.[ctx.section];
  const badges = [fiche.codeBadge];
  if (trigger === 'mic2' && ctx.hasMicTempSens2) badges.push('Mic-temp-sens2');
  const keywords = [
    ...(ctx.hasThermal ? ['thermalmonitord'] : []),
    fiche.codeBadge,
    ...(ctx.hasMicTempSens2 && trigger === 'mic2' ? ['Mic-temp-sens2'] : []),
  ];
  const base: WorkshopCardDraft = {
    id: `${fiche.id}-${ctx.section}`,
    matchScore: fiche.matchScore,
    codeBadges: badges,
    severity: fiche.severity,
    title: fiche.title,
    subtitle: fiche.subtitle,
    component: fiche.component,
    likelyCause: fiche.likelyCause,
    keywords,
    quickTest: fiche.quickTest,
    steps: [...fiche.steps],
    note: fiche.note,
  };
  return applyOverride(base, ov);
}

function maskToCard(mf: MaskFiche, mask: number, ctx: CatalogCtx): WorkshopCardDraft {
  const hex = `0x${mask.toString(16).toUpperCase()}`;
  return {
    id: `mask-${mf.ficheId}-${ctx.section}`,
    matchScore: mf.matchScore,
    codeBadges: [hex, `Masque SMC ${mask}`],
    severity: 'HARDWARE',
    title: mf.title,
    subtitle: `Capteur(s) signalés par le masque SMC · ${hex}`,
    component: mf.component,
    likelyCause: mf.likelyCause,
    keywords: [hex, 'S.sensor array', 'SMC'],
    steps: [...mf.steps],
    note: mf.note,
  };
}

export function buildCatalogWorkshopCards(ctx: CatalogCtx, analysis: AnalysisResult): WorkshopCardDraft[] {
  const cards: WorkshopCardDraft[] = [];
  const usedIds = new Set<string>();

  const push = (c: WorkshopCardDraft) => {
    if (usedIds.has(c.id)) return;
    usedIds.add(c.id);
    cards.push(c);
  };

  for (const fiche of SENSOR_FICHES) {
    for (const trig of fiche.triggers) {
      if (ctx.missing.has(trig)) {
        push(sensorToCard(fiche, ctx, trig));
        break;
      }
    }
  }

  const masks = extractSensorMaskValues(
    `${ctx.blobLower}\n${(analysis.structured_diagnostic.critical_lines ?? []).join('\n')}`,
  );
  const usedMaskFiches = new Set<string>();
  const maskFichesByScore = [...MASK_FICHES].sort((a, b) => b.matchScore - a.matchScore);
  for (const val of masks) {
    for (const mf of maskFichesByScore) {
      if (mf.sections?.length && !mf.sections.includes(ctx.section)) continue;
      if (!mf.masks.some((m) => m === val)) continue;
      if (usedMaskFiches.has(mf.ficheId)) break;
      usedMaskFiches.add(mf.ficheId);
      push(maskToCard(mf, val, ctx));
      break;
    }
  }

  if (!cards.length && ctx.hasThermal) {
    push({
      id: `thermal-${ctx.section}`,
      matchScore: 55,
      codeBadges: ['thermalmonitord'],
      severity: 'HARDWARE',
      title: 'Watchdog thermalmonitord · Capteur absent',
      subtitle: 'Vérifier "Missing sensor(s):" ou masque SMC dans le log — pas forcément une surchauffe CPU',
      component:
        ctx.section === 'iphone-11'
          ? 'Nappe port Lightning + nappe bouton power'
          : 'Nappe charge + nappe capteurs selon modèle',
      likelyCause:
        'Capteur non vu par le SMC (nappe débranchée, oxydation, aftermarket) — le téléphone reboot en ~3 min',
      keywords: ['thermalmonitord', 'Capteur manquant'],
      steps: [
        'Ouvrir le panic-full et noter mic1 / mic2 / prs0 ou le masque hex',
        'Brancher les nappes OEM une par une',
        'Remplacer la nappe du capteur cité avant carte mère',
      ],
      note:
        ctx.section === 'iphone-11'
          ? 'iPhone 11 : souvent dock + power ensemble.'
          : undefined,
    });
  }

  cards.sort((a, b) => b.matchScore - a.matchScore);
  return cards.length ? [cards[0]] : [];
}

/** Titre atelier sans codes : extrait la partie « pièce » d’une ligne diagnostic Rust/KB. */
export function partTitleFromDiagnosticLine(line: string): string {
  const cleaned = stripDiagnosticCodesFragments(
    line.replace(/^\s*KB\s*·\s*/i, '').replace(/\s*\[Repair Wiki\]\s*$/i, '').trim(),
  );
  if (!cleaned) return '';

  const segments = cleaned
    .split('·')
    .map((s) => s.trim())
    .filter(Boolean);

  const isModelSeg = (s: string) =>
    /^iPhone\s/i.test(s) || /^iphone\d+,\d+$/i.test(s) || /^série\s/i.test(s);
  const isCodeSeg = (s: string) =>
    /^(mic|prs|tg|ans|tp)\d/i.test(s) || /^missing sensor/i.test(s) || /^0x/i.test(s);

  const parts = segments.filter((s) => !isModelSeg(s) && !isCodeSeg(s));
  if (parts.length) return parts.join(' · ').slice(0, 140);

  if (segments.length >= 3) return segments.slice(2).join(' · ').slice(0, 140);
  if (segments.length === 2) return segments[1].slice(0, 140);
  return cleaned.slice(0, 140);
}

export function intelligentLikelyCause(analysis: AnalysisResult, ctx: CatalogCtx): string {
  const pack = ctx.blobLower;
  if (ctx.missing.has('mic2') || /\bmic2\b/.test(pack)) {
    if (ctx.section === 'iphone-11') return MIC2_11_CAUSE;
    return 'Liquide, nappe avant/power mal connectée, grille micro, pièce aftermarket';
  }
  if (ctx.missing.has('prs0') || /\bprs0\b/.test(pack))
    return 'Nappe port absente ou aftermarket, oxydation FPC dock, eau';
  if (ctx.missing.has('tg0b') || ctx.missing.has('tg0v') || /\btg0/.test(pack))
    return 'Connecteur batterie, cellule, TIGRIS, lignes jauge batterie';
  if (/\b0x800\b|\(2048\)|connecteur de charge/i.test(pack))
    return 'Nappe charge : oxydation, aftermarket, FPC dock, liquide';
  if (/\b0x100000\b|\(1048576\)|bouton power/i.test(pack)) return MIC2_11_CAUSE;

  const second = analysis.structured_diagnostic.possible_causes?.[1]?.name;
  if (second) {
    const t = partTitleFromDiagnosticLine(second);
    if (t && !/^0x/i.test(t)) return t.slice(0, 200);
  }
  return 'Oxydation connecteur, nappe aftermarket, chute ou liquide récent';
}
