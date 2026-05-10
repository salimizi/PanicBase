/**
 * Synthèses courtes issue de communautés repair (majoritairement iFixit wiki / Answers).
 * Non officiel Apple — aide à contextualiser panic-full vs modèle ; toujours vérifier le log brut.
 */

export type RepairReference = {
  id: string;
  titleFr: string;
  url: string;
  keywords: string[];
  summaryFr: string;
  /** Vide = tout modèle */
  modelsHint?: string[];
};

export const REPAIR_REFERENCE_INDEX: RepairReference[] = [
  {
    id: 'repair-wiki-panic-restarts',
    titleFr: 'Repair Wiki — panic-full & redémarrages (par modèle)',
    url: 'https://repair.wiki/w/How_to_Troubleshoot_And_Fix_iPhone_Random_Restarts_Using_Panic_Logs',
    keywords: [
      'panic',
      'restart',
      'sensor',
      'mic1',
      'mic2',
      'prs0',
      'tg0',
      'tg0b',
      'ans2',
      'outbox',
      'smc',
      'thermalmonitord',
    ],
    summaryFr:
      'Tableaux mic/prs/TG (anciennes générations) et codes hex sensor array (13 → 15 Pro Max) ; stratégie pièces connues bonnes puis lecture panic-full.',
  },
  {
    id: 'ifixit-kernel-panics',
    titleFr: 'iFixit Wiki — Paniques kernel iPhone',
    url: 'https://www.ifixit.com/Wiki/iPhone_Kernel_Panics',
    keywords: ['panic', 'watchdog', 'kernel', 'thermalmonitord', 'userspace'],
    summaryFr:
      'Vue d’ensemble : thermalmonitord surveille les capteurs ; les reboots peuvent venir de capteurs manquants, nappes ou bus, pas seulement d’une « vraie » surchauffe.',
  },
  {
    id: 'ifixit-smc-panic',
    titleFr: 'iFixit — SMC panic / BSC failure',
    url: 'https://www.ifixit.com/Wiki/iPhone_SMC_Panic_Assertion_Failed',
    keywords: ['smc', 'bsc', 'outbox1', 'assertion'],
    summaryFr:
      'SMC PANIC + BSC : souvent liaison charge / capteurs / rails ; valeurs sensor array peuvent orienter pièce port ou nappe.',
    modelsHint: ['13', '14', '15', '16'],
  },
  {
    id: 'smc-taop-wireless-hint',
    titleFr: 'PanicBase — motif TAOP / TAOJ + OUTBOX1 (iPhone15,4)',
    url: 'https://www.ifixit.com/Wiki/iPhone_SMC_Panic_Assertion_Failed',
    keywords: ['taop', 'taoj', 'outbox1', 'bsc'],
    summaryFr:
      'Sur iPhone15,4 avec OUTBOX1 après remplacement MagSafe/bobine wireless : prioriser nappe wireless/charge sans fil et clipage I2C avant batterie ou « carte mère ».',
    modelsHint: ['15,4', 'iphone15,4'],
  },
  {
    id: 'ifixit-mic2-thermal',
    titleFr: 'iFixit — thermalmonitord + Missing sensor Mic2',
    url: 'https://www.ifixit.com/Wiki/iPhone_Kernel_Panics',
    keywords: ['mic2', 'missing sensor'],
    summaryFr:
      'Cas fréquents après vitre arrière / choc liquide : nappe/connecteur ou ligne capteur ; thermalmonitord peut être satellite (watchdog sans check-ins). Communautés repair cite souvent aussi la nappe ou connecteur bouton/flash.',
    modelsHint: ['11', '12,1'],
  },
  {
    id: 'ifixit-mic2-11-loop',
    titleFr: 'iFixit Answers — reboot loop thermalmonitord',
    url: 'https://www.ifixit.com/Answers/View/645543/iPhone+11+REBOOT+LOOP+-+THERMALMONITORD',
    keywords: ['thermalmonitord', 'thermal'],
    summaryFr:
      'Cyclés courts : vérifier nappes & connecteurs reliés aux capteurs attendus avant de conclure batterie.',
    modelsHint: ['11', '12,1'],
  },
  {
    id: 'ifixit-aop-faceid',
    titleFr: 'iFixit — AOP PANIC / TrueDepth',
    url: 'https://www.ifixit.com/Wiki/iPhone_Kernel_Panics',
    keywords: ['aop panic', 'aop'],
    summaryFr:
      'Chaîne Face ID / haut-parleur / flood illuminator / nappe avant ; corrélation souvent géométrique (nappe coupée ou court).',
  },
  {
    id: 'ifixit-ans2',
    titleFr: 'iFixit — ANS2 / stockage',
    url: 'https://www.ifixit.com/Wiki/iPhone_Kernel_Panics',
    keywords: ['ans2'],
    summaryFr:
      'ANS2 Recoverable Panic oriente généralement contrôleur stockage / NAND ; stress tests stockage peuvent être utiles.',
  },
];

/** Correspondance motifs texte brut + indications modèle pour afficher références pertinentes max 4 */
export function matchedRepairReferences(
  keywords: readonly string[],
  panicType: string,
  deviceModel: string,
  panicTextSnippet: string
): RepairReference[] {
  const hay = `${keywords.join(' ')} ${panicType} ${deviceModel} ${panicTextSnippet.slice(0, 12000)}`.toLowerCase();
  const modelLow = deviceModel.toLowerCase();
  let scored = REPAIR_REFERENCE_INDEX.map((ref) => {
    const kwHit = ref.keywords.some((k) => hay.includes(k.toLowerCase())) ? 1 : 0;
    const modelOk =
      !ref.modelsHint?.length ||
      ref.modelsHint.some((h) => modelLow.includes(`iphone ${h}`) || modelLow.includes(h));
    let score = kwHit + (modelOk ? 0.5 : -0.2);
    if (ref.id === 'ifixit-kernel-panics' && kwHit) score *= 0.35;
    return { ref, score };
  }).filter(({ score }) => score > 0.5);

  scored.sort((a, b) => b.score - a.score);
  return scored.slice(0, 5).map((x) => x.ref);
}
