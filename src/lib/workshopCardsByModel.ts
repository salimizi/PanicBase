/**
 * Fiches atelier alignées sur `iphone_panic_reference_enriched.html`
 * (même structure : badges code, composant, mots-clés, étapes numérotées).
 *
 * Couverture étendue : iPhone X → iPhone 17 + toute la gamme intermédiaire.
 * Logique métier enrichie : pour chaque capteur manquant / code erreur, on
 * remonte la chaîne causale réaliste atelier (nappe → oxydation → carte mère).
 */
import type { AnalysisResult } from '../types/analysis';
import { inferPanicReferenceFocus, inferSearchQueryFromPanicBlob } from './inferPanicReferenceFocus';
import type { ReferenceWorkshopCard, WorkshopCardSeverity } from './workshopTypes';

/** Fiche atelier sans clé UI (ajoutée par `referenceWorkshopCards`). */
export type ModelWorkshopCard = Omit<ReferenceWorkshopCard, 'uiKey'>;

type MatchCtx = {
  blobLower: string;
  section: string;
  missing: Set<string>;
  hasThermal: boolean;
  hasMicTempSens2: boolean;
  sensorMasks: number[];
  productTypeLc: string;
};

function extractMissingFromAnalysis(analysis: AnalysisResult, panicText: string): Set<string> {
  const out = new Set<string>();
  const re = /missing sensor\(s?\)?:?\s*([^\n\r]+)/gi;
  let m: RegExpExecArray | null;
  const hay = `${panicText}\n${analysis.structured_diagnostic.critical_lines?.join('\n') ?? ''}`;
  while ((m = re.exec(hay)) !== null) {
    for (const part of m[1].split(/[,;\s]+/)) {
      const t = part.replace(/[^a-z0-9]/gi, '').toLowerCase();
      if (t.length >= 2 && t.length <= 12) out.add(t);
    }
  }
  for (const sig of analysis.structured_diagnostic.normalized_signatures ?? []) {
    const cap = sig.match(/Capteur absent \(([^)]+)\)/i);
    if (cap?.[1]) {
      for (const part of cap[1].split(/[,;\s]+/)) {
        const t = part.trim().toLowerCase();
        if (t) out.add(t);
      }
    }
  }
  return out;
}

function extractSensorMasks(panicText: string): number[] {
  const out: number[] = [];
  const re = /[sf]\.sensor\s+array[^\n]{0,600}/gi;
  let m: RegExpExecArray | null;
  while ((m = re.exec(panicText)) !== null) {
    const segment = m[0];
    const afterIs = segment.split(/\bis\b/i)[1] ?? segment;
    for (const tok of afterIs.split(/[\s,]+/)) {
      const t = tok.trim();
      if (!t) continue;
      let val: number | null = null;
      if (/^0x[0-9a-f]+$/i.test(t)) val = parseInt(t, 16);
      else if (/^\d+$/.test(t)) val = parseInt(t, 10);
      if (val !== null && val > 0) out.push(val);
    }
  }
  return [...new Set(out)];
}

function ctxFrom(panicText: string, analysis: AnalysisResult, productType: string | null): MatchCtx {
  const focus = inferPanicReferenceFocus({ panicText, analysis, productType });
  const blobLower = [
    panicText,
    analysis.probable_cause,
    ...(analysis.keywords ?? []),
    ...(analysis.structured_diagnostic.normalized_signatures ?? []),
    ...(analysis.structured_diagnostic.critical_lines ?? []),
  ]
    .join('\n')
    .toLowerCase();

  const ptLc = (productType ?? analysis.structured_diagnostic.device ?? '').toLowerCase().trim();

  return {
    blobLower,
    section: focus.navSection,
    missing: extractMissingFromAnalysis(analysis, panicText),
    hasThermal: blobLower.includes('thermalmonitord') || blobLower.includes('no successful checkins'),
    hasMicTempSens2: /\bmic-temp-sens2\b/i.test(blobLower),
    sensorMasks: extractSensorMasks(panicText),
    productTypeLc: ptLc,
  };
}

function card(
  partial: Omit<ModelWorkshopCard, 'matchScore'> & { matchScore?: number },
): ModelWorkshopCard {
  return { matchScore: partial.matchScore ?? 100, ...partial };
}


// ── iPhone 11 / 11 Pro / 11 Pro Max ──────────────────────────────────────────
function iphone11Cards(ctx: MatchCtx): ModelWorkshopCard[] {
  const out: ModelWorkshopCard[] = [];

  if (ctx.missing.has('mic1')) {
    out.push(card({
      id: '11-mic1', matchScore: 98,
      codeBadges: ['Capteur manquant : mic1'],
      severity: 'HARDWARE',
      title: 'Nappe port Lightning · Micro bas-gauche',
      subtitle: 'mic1 = microphone bas-gauche intégré à la nappe port',
      component: 'Nappe port Lightning (micro bas-gauche)',
      likelyCause: 'Eau, choc mécanique, nappe aftermarket sans capteur',
      keywords: ['thermalmonitord', 'Capteur manquant : mic1'],
      quickTest: 'Brancher une nappe Lightning OEM connue bonne → si tient >3 min, nappe défaillante',
      steps: [
        "S'assurer que la nappe port Lightning ET la nappe power sont connectées",
        'Inspecter visuellement la nappe port Lightning (oxydation, pliure, connecteur arraché)',
        'Remplacer la nappe port Lightning par OEM (mic1 = capteur sur cette nappe)',
        'Si persiste → inspecter connecteur carte mère au microscope + mesure diode',
        'Lignes I2C1_AP_SCL / SDA : OL = ligne ouverte → microsoudure',
      ],
      note: 'Très souvent suite à une réparation de vitre arrière ou une immersion. Tester nappe OEM avant carte.',
    }));
  }

  if (ctx.missing.has('mic2')) {
    const badges = ['Capteur manquant : mic2'];
    if (ctx.hasMicTempSens2) badges.push('Mic-temp-sens2');
    out.push(card({
      id: '11-mic2', matchScore: 99,
      codeBadges: badges,
      severity: 'HARDWARE',
      title: 'Nappe bouton power · Micro côté flash',
      subtitle: 'mic2 = micro près du flash · nappe power/volume · reboot si débranchée ou oxydée',
      component: 'Nappe bouton power (côté caméra arrière / flash)',
      likelyCause: 'Nappe débranchée lors réparation écran, vitre arrière remplacée (flash sur même nappe), oxydation',
      keywords: [...(ctx.hasThermal ? ['thermalmonitord'] : []), 'Capteur manquant : mic2', ...(ctx.hasMicTempSens2 ? ['Mic-temp-sens2'] : [])],
      quickTest: "Ouvrir le téléphone et vérifier que la nappe bouton power est branchée — souvent visible à l'œil nu",
      steps: [
        "ÉTAPE 1 : Vérifier que la nappe bouton power est bien connectée (débranche lors d'un changement d'écran ou de vitre arrière)",
        "ÉTAPE 2 : Inspecter la nappe pour oxydation, pliure ou dommage mécanique",
        "ÉTAPE 3 : ATTENTION — sur iPhone 11, le flash arrière est relié à cette même nappe. Si le flash a été endommagé lors d'un remplacement de vitre par tiers, cela provoque des reboots car la nappe flex est commune",
        'ÉTAPE 4 : Remplacer la nappe bouton power/volume par OEM (éviter aftermarket)',
        'ÉTAPE 5 : Si persiste après nappe OEM → board-level : mesure diode sur les lignes mic2',
      ],
      note: "PIÈGE FRÉQUENT : après remplacement de vitre arrière par un tiers, le flash est souvent endommagé ou la nappe pincée → reboot car mic2 partage cette nappe flex.",
    }));
  }

  if (ctx.missing.has('prs0')) {
    out.push(card({
      id: '11-prs0', matchScore: 96,
      codeBadges: ['Capteur manquant : Prs0'],
      severity: 'HARDWARE',
      title: 'Baromètre · Nappe port Lightning',
      subtitle: 'Prs0 = baromètre intégré à la nappe port Lightning',
      component: 'Nappe port Lightning (baromètre intégré)',
      likelyCause: 'Nappe absente, aftermarket sans capteur, FPC dock oxydé',
      keywords: ['Capteur manquant : Prs0', 'thermalmonitord'],
      quickTest: 'Brancher nappe OEM et voir si le panic disparaît après 3 min',
      steps: [
        'Vérifier que les deux nappes (port Lightning ET bouton power) sont branchées — souvent toutes deux requises sur série 11',
        'Remplacer la nappe port Lightning OEM (le baromètre Prs0 est intégré à cette nappe)',
        'Nettoyer le connecteur carte mère si traces de liquide',
        'Si persiste → board-level : lignes I2C baromètre',
      ],
    }));
  }

  if (ctx.missing.has('tg0b') || ctx.missing.has('tg0v')) {
    out.push(card({
      id: '11-tg0', matchScore: 95,
      codeBadges: [
        ...(ctx.missing.has('tg0b') ? ['Capteur manquant : TG0B'] : []),
        ...(ctx.missing.has('tg0v') ? ['Capteur manquant : TG0V'] : []),
      ],
      severity: 'HARDWARE',
      title: 'Batterie non détectée · IC TIGRIS',
      subtitle: 'TG0B/TG0V = jauge batterie batterie · données non lisibles',
      component: 'Batterie · IC TIGRIS · connecteur batterie · FPC batterie',
      likelyCause: 'Connecteur batterie débranché, batterie morte, FPC batterie oxydé, IC TIGRIS défaillant',
      keywords: ['TG0B', 'TG0V', 'Jauge batterie'],
      quickTest: 'Débrancher/rebrancher la batterie proprement, tester avec batterie OEM connue bonne',
      steps: [
        'Débrancher et rebrancher le connecteur batterie (nettoyer si oxydation visible)',
        'Tester avec une batterie OEM connue bonne',
        'Mesurer en mode diode sur les pins de données batterie (SCL/SDA jauge batterie)',
        'OL sur ces lignes → rupture de piste → reboulement IC TIGRIS',
        'Inspecter IC TIGRIS au microscope (soudure froide, déplacement)',
      ],
      note: 'Sur 11 Pro / Pro Max : BSC failure associé = souvent même faisceau batterie. Ne pas conclure PMIC mort sans preuve.',
    }));
  }

  if (!out.length && ctx.hasThermal) {
    out.push(card({
      id: '11-tip-thermal', matchScore: 55,
      codeBadges: ['thermalmonitord'],
      severity: 'HARDWARE',
      title: 'Watchdog thermalmonitord — capteur absent (pas une surchauffe)',
      subtitle: 'Lire "Missing sensor(s):" dans le log pour identifier le capteur',
      component: 'Nappe port Lightning + nappe bouton power (série 11)',
      likelyCause: 'Capteur mic1, mic2 ou Prs0 absent — pas forcément de la chaleur',
      keywords: ['thermalmonitord', 'Capteur manquant'],
      quickTest: 'Lire la ligne exacte "Missing sensor(s):" dans le panic',
      steps: [
        "Lire la ligne 'Missing sensor(s):' dans le panicString pour identifier le capteur exact",
        'Brancher nappe port Lightning ET nappe bouton power (les deux souvent nécessaires)',
        'mic1 → remplacer nappe port Lightning OEM',
        'mic2 → vérifier/remplacer nappe bouton power OEM (attention : liée au flash sur 11)',
        'Prs0 → remplacer nappe port Lightning OEM',
      ],
      note: 'SPÉCIFICITÉ iPhone 11 : les deux nappes (port + power) doivent être branchées simultanément pour tenir >3 minutes.',
    }));
  }

  return out;
}

// ── iPhone X ──────────────────────────────────────────────────────────────────
function iphoneXCards(ctx: MatchCtx): ModelWorkshopCard[] {
  const out: ModelWorkshopCard[] = [];

  if (ctx.missing.has('prs0')) {
    out.push(card({
      id: 'x-prs0', matchScore: 97,
      codeBadges: ['Capteur manquant : Prs0'],
      severity: 'HARDWARE',
      title: 'Baromètre · Nappe port Lightning',
      subtitle: 'thermalmonitord · capteur manquant : Prs0 (baromètre)',
      component: 'Nappe port Lightning (baromètre intégré, bas gauche)',
      likelyCause: 'Nappe absente, arrachée, eau, qualité aftermarket',
      keywords: ['thermalmonitord', 'Capteur manquant : Prs0'],
      quickTest: 'Brancher une nappe Lightning OEM → si tient >3 min, nappe défaillante confirmée',
      steps: [
        'Vérifier la nappe port Lightning (connecteur propre, sans oxydation)',
        'Remplacer par une nappe OEM ou premium',
        'Si persistant → inspecter connecteur sur la carte mère au microscope',
        'Mesurer en mode diode sur le connecteur carte mère (SCL/SDA baromètre)',
        'Si board-level confirmé → microsoudure ou remplacement carte mère',
      ],
    }));
  }

  if (ctx.missing.has('tg0b') || ctx.missing.has('tg0v')) {
    out.push(card({
      id: 'x-tg0', matchScore: 94,
      codeBadges: (['tg0b', 'tg0v'] as const)
        .filter((id) => ctx.missing.has(id))
        .map((id) => `Capteur manquant : ${id.toUpperCase()}`),
      severity: 'HARDWARE',
      title: 'Batterie non détectée · TIGRIS',
      subtitle: 'TG0B / TG0V : données batterie non lisibles',
      component: 'Batterie · IC TIGRIS · connecteur batterie',
      likelyCause: 'Connecteur batterie, cellule HS, IC TIGRIS défaillant',
      keywords: ['TG0B', 'TG0V', 'Jauge batterie'],
      steps: [
        'Vérifier le connecteur batterie (retirer, nettoyer, rebrancher)',
        'Tester avec une batterie OEM connue bonne',
        'Mesure diode sur les pins de données batterie',
        'Inspecter IC TIGRIS sous microscope',
      ],
    }));
  }

  if (ctx.missing.has('mic1') || (ctx.hasThermal && ctx.blobLower.includes('mic1'))) {
    out.push(card({
      id: 'x-mic1', matchScore: 93,
      codeBadges: ['Capteur manquant : mic1', 'thermalmonitord'],
      severity: 'HARDWARE',
      title: 'Nappe charge / dock · Micro bas (iPhone X)',
      subtitle: 'thermalmonitord + mic1 = signature atelier très fiable sur X',
      component: 'Nappe port Lightning (micro bas intégré)',
      likelyCause: 'Nappe dock absente ou aftermarket, eau, FPC dock oxydé',
      keywords: ['thermalmonitord', 'mic1'],
      steps: [
        'Remplacer la nappe port Lightning OEM',
        "Vérifier pas de trace de liquide sur le connecteur carte mère (lignes I2C dock)",
        'Si persiste → diode mode sur I2C1_AP_SCL/SDA → board-level si OL',
      ],
      note: "Signature très reconnue atelier sur iPhone X : thermalmonitord + mic1 = nappe dock à 90%.",
    }));
  }

  return out;
}

// ── iPhone XS / XS Max ───────────────────────────────────────────────────────
function iphoneXsCards(ctx: MatchCtx): ModelWorkshopCard[] {
  const out: ModelWorkshopCard[] = [];

  if (ctx.missing.has('mic2') && (ctx.blobLower.includes('boot') || ctx.blobLower.includes('loop'))) {
    out.push(card({
      id: 'xs-mic2-boot', matchScore: 96,
      codeBadges: ['Capteur manquant : mic2', 'bootloop'],
      severity: 'HARDWARE',
      title: 'Écouteur interne · Nappe capteurs avant',
      subtitle: 'Bootloop après changement écouteur + mic2 = signature XS très connue',
      component: 'Écouteur interne / ensemble avant (capteurs + micro haut)',
      likelyCause: "Écouteur aftermarket ou mal monté après réparation d'écran",
      keywords: ['mic2', 'bootloop'],
      quickTest: "Rebrancher ou remplacer l'écouteur interne par OEM",
      steps: [
        "Vérifier que le câble de l'écouteur interne est bien connecté",
        "Tester avec un écouteur OEM connu bon",
        "Si OEM résout : écouteur aftermarket sans capteur mic2",
        "Si persiste avec OEM → inspecter les lignes mic2 en mode diode",
      ],
      note: "Signature TRÈS connue sur XS / XS Max : bootloop après changement d'écran avec écouteur aftermarket.",
    }));
  }

  if (ctx.missing.has('prs0')) {
    out.push(card({
      id: 'xs-prs0', matchScore: 95,
      codeBadges: ['Capteur manquant : Prs0'],
      severity: 'HARDWARE',
      title: 'Baromètre · Nappe port Lightning',
      subtitle: 'Même composant que iPhone X',
      component: 'Nappe port Lightning (baromètre intégré)',
      likelyCause: 'Nappe absente, eau, aftermarket sans capteur',
      keywords: ['Capteur manquant : Prs0', 'thermalmonitord'],
      steps: [
        'Remplacer la nappe port Lightning OEM',
        'Vérifier les deux nappes connectées (port + power)',
        'Si persiste → board-level baromètre',
      ],
    }));
  }

  return out;
}

// ── iPhone XR ────────────────────────────────────────────────────────────────
function iphoneXrCards(ctx: MatchCtx): ModelWorkshopCard[] {
  const out: ModelWorkshopCard[] = [];
  if (ctx.missing.has('prs0')) {
    out.push(card({
      id: 'xr-prs0', matchScore: 94,
      codeBadges: ['Capteur manquant : Prs0'],
      severity: 'HARDWARE',
      title: 'Baromètre · Nappe recharge / périphérique (XR)',
      subtitle: 'XR : Prs0 peut aussi être lié à la recharge sans fil',
      component: 'Nappe port Lightning OU nappe recharge sans fil',
      likelyCause: 'Nappe dock ou flex Qi absent, aftermarket, eau',
      keywords: ['Capteur manquant : Prs0', 'thermalmonitord'],
      steps: [
        'Remplacer la nappe port Lightning OEM en premier',
        "Si XR avec recharge sans fil endommagée, vérifier aussi la nappe Qi",
        'Si persiste → board-level',
      ],
    }));
  }
  return out;
}

// ── iPhone 12 / 12 mini ──────────────────────────────────────────────────────
function iphone12Cards(ctx: MatchCtx): ModelWorkshopCard[] {
  const out: ModelWorkshopCard[] = [];

  if (ctx.missing.has('mic1') || (ctx.hasThermal && ctx.blobLower.includes('mic1'))) {
    out.push(card({
      id: '12-mic1', matchScore: 97,
      codeBadges: ['Capteur manquant : mic1'],
      severity: 'HARDWARE',
      title: 'Nappe connecteur de charge · Micro bas',
      subtitle: 'mic1 = microphone bas sur nappe dock Lightning',
      component: 'Nappe port Lightning (micro bas intégré)',
      likelyCause: 'Nappe absente ou aftermarket sans capteur, eau, FPC dock endommagé',
      keywords: ['mic1', 'thermalmonitord'],
      quickTest: 'Tester avec nappe OEM connue bonne',
      steps: [
        'Vérifier que la nappe port Lightning est connectée et en bon état',
        'Remplacer par une nappe Lightning OEM (mic1 sur cette nappe)',
        'Éviter les nappes aftermarket : fort taux sans capteur sur série 12',
        'Si persiste → diode mode I2C1_AP_SCL/SDA → board-level si OL',
      ],
      note: "Corrélation forte (>85%) sur iPhone 12 : mic1 + reboot ~3 min = nappe dock. Tester OEM avant carte mère.",
    }));
  }

  if (ctx.missing.has('mic2')) {
    out.push(card({
      id: '12-mic2', matchScore: 96,
      codeBadges: ['Capteur manquant : mic2'],
      severity: 'HARDWARE',
      title: 'Écouteur interne · Ensemble capteurs avant',
      subtitle: 'mic2 = micro sur écouteur avant / pré-ensemble avant',
      component: "Écouteur interne / pré-ensemble avant",
      likelyCause: "Écouteur aftermarket après changement d'écran, câble mal rebranché",
      keywords: ['mic2'],
      quickTest: "Rebrancher ou swapper l'écouteur interne avec OEM",
      steps: [
        "Vérifier que le câble de l'écouteur est bien connecté",
        "Tester avec un écouteur OEM",
        "Si résolution avec OEM : pièce aftermarket sans capteur mic2",
        "Si persiste → inspecter lignes mic2 avant de conclure board-level",
      ],
    }));
  }

  if (ctx.missing.has('prs0')) {
    out.push(card({
      id: '12-prs0', matchScore: 93,
      codeBadges: ['Capteur manquant : Prs0'],
      severity: 'HARDWARE',
      title: 'Baromètre · Nappe port Lightning',
      subtitle: 'Même logique que iPhone 11 / X',
      component: 'Nappe port Lightning (baromètre intégré)',
      likelyCause: 'Nappe absente, aftermarket, eau',
      keywords: ['Capteur manquant : Prs0'],
      steps: [
        'Remplacer la nappe port Lightning OEM',
        'Vérifier connecteur carte mère (pas de corrosion)',
        'Si persiste → board-level',
      ],
    }));
  }

  if (ctx.missing.has('tg0b') || ctx.missing.has('tg0v')) {
    out.push(card({
      id: '12-tg0', matchScore: 92,
      codeBadges: (['tg0b', 'tg0v'] as const)
        .filter((id) => ctx.missing.has(id))
        .map((id) => `Capteur manquant : ${id.toUpperCase()}`),
      severity: 'HARDWARE',
      title: 'Batterie non détectée · Jauge batterie',
      subtitle: 'TG0B/TG0V = données batterie non lisibles',
      component: 'Batterie · IC jauge batterie · connecteur batterie',
      likelyCause: 'Connecteur batterie, batterie HS, IC jauge batterie défaillant',
      keywords: ['TG0B', 'TG0V'],
      steps: [
        'Débrancher/rebrancher connecteur batterie proprement',
        'Tester batterie OEM',
        'Mesure diode pins données batterie',
        'IC jauge batterie si board-level confirmé',
      ],
    }));
  }

  if (!out.length && ctx.hasThermal) {
    out.push(card({
      id: '12-thermal-fallback', matchScore: 50,
      codeBadges: ['thermalmonitord'],
      severity: 'HARDWARE',
      title: 'Watchdog thermalmonitord — capteur à identifier',
      subtitle: 'Lire "Missing sensor(s):" dans le log pour identifier le capteur',
      component: 'Nappe port Lightning · écouteur · batterie selon capteur',
      likelyCause: 'Capteur mic1, mic2 ou Prs0 absent',
      keywords: ['thermalmonitord'],
      steps: [
        "Trouver la ligne 'Missing sensor(s):' dans le panicString",
        'mic1 → nappe port Lightning OEM',
        'mic2 → écouteur interne OEM',
        'Prs0 → nappe port Lightning OEM',
      ],
      note: "Coller le panic complet pour un diagnostic précis du capteur manquant.",
    }));
  }

  return out;
}

function iphone12ProCards(ctx: MatchCtx): ModelWorkshopCard[] {
  return iphone12Cards(ctx).map((c) => ({ ...c, id: c.id.replace(/^12-/, '12p-') }));
}


// ── iPhone 13 / 13 mini ──────────────────────────────────────────────────────
function iphone13Cards(ctx: MatchCtx): ModelWorkshopCard[] {
  const out: ModelWorkshopCard[] = [];
  const isMini = ctx.productTypeLc === 'iphone14,4';

  for (const mask of ctx.sensorMasks) {
    if (isMini && (mask === 0xc00 || mask === 3072)) {
      out.push(card({
        id: '13mini-0xc00', matchScore: 97,
        codeBadges: ['S.sensor 0xC00', 'Bottom board + Dock'],
        severity: 'HARDWARE',
        title: 'Nappe port de charge + Bottom board · iPhone 13 mini',
        subtitle: '0xC00 = double défaut : gyroscope + nappe de charge',
        component: 'Nappe port Lightning + bottom board (carte fille)',
        likelyCause: 'Connexion bottom board défaillante, nappe dock endommagée',
        keywords: ['0xC00', '3072', 'bottom board'],
        quickTest: 'Déconnecter/reconnecter le bottom board et la nappe dock',
        steps: [
          'Déconnecter et reconnecter le bottom board (carte fille)',
          'Vérifier/remplacer la nappe port Lightning OEM',
          "Tester sans la carte fille pour isoler (si reboot disparaît → carte fille défaillante)",
          'Si persiste → board-level lignes gyroscope',
        ],
        note: "Spécificité iPhone 13 mini : le bottom board est une source fréquente de panique capteur.",
      })); continue;
    }

    if (isMini && (mask === 0x400 || mask === 1024)) {
      out.push(card({
        id: '13mini-0x400', matchScore: 95,
        codeBadges: ['S.sensor 0x400', 'Gyroscope'],
        severity: 'HARDWARE',
        title: 'Gyroscope · iPhone 13 mini',
        subtitle: '0x400 = gyroscope non lisible',
        component: 'Gyroscope (soudé sur carte mère)',
        likelyCause: 'Chute sur angle, gyroscope décollé, soudure froide',
        keywords: ['0x400', 'gyroscope'],
        steps: [
          "Vérifier si le téléphone a subi une chute",
          'Inspecter sous microscope la zone gyroscope',
          "Reflow du gyroscope (chaleur douce) comme premier essai",
          'Si persiste → remplacement gyroscope ou carte mère',
        ],
      })); continue;
    }

    if (mask === 0x1800 || mask === 6144) {
      out.push(card({
        id: '13-0x1800', matchScore: 95,
        codeBadges: ['S.sensor 0x1800', 'Proximité + Dock'],
        severity: 'HARDWARE',
        title: 'Double défaut · Nappe charge + Proximité · iPhone 13',
        subtitle: '0x1800 = dock ET capteur de proximité simultanément',
        component: 'Nappe port Lightning + capteur proximité / écran avant',
        likelyCause: "Démontage précédent mal refait : dock aftermarket + écran aftermarket",
        keywords: ['0x1800', 'dock', 'proximité'],
        steps: [
          "Remplacer les deux composants par OEM : nappe dock ET écouteur/écran OEM",
          "Ne pas mélanger OEM et aftermarket",
          "Tester les composants un par un pour isoler",
          "Si persiste → board-level sur les deux bus",
        ],
        note: "Très souvent suite à une réparation avec pièces mixtes (dock aftermarket + écran aftermarket).",
      }));
    }

    if (mask === 0x800 || mask === 2048) {
      out.push(card({
        id: '13-0x800', matchScore: 93,
        codeBadges: ['S.sensor 0x800', 'Connecteur charge'],
        severity: 'HARDWARE',
        title: 'Nappe connecteur de charge · iPhone 13',
        subtitle: '0x800 = capteur sur la nappe dock Lightning',
        component: 'Nappe port Lightning (connecteur de charge)',
        likelyCause: 'Nappe absente ou aftermarket, eau, choc, réparation précédente',
        keywords: ['0x800', '2048', 'connecteur charge'],
        quickTest: 'Remplacer par nappe OEM et observer le reboot',
        steps: [
          "Vérifier l'état de la nappe port Lightning (connecteur propre, sans pliure)",
          'Remplacer par nappe Lightning OEM (fort taux de résolution)',
          "Si aftermarket installé précédemment : fort taux d'échec sans capteur",
          'Si persiste avec OEM → mesure diode sur connecteur carte mère',
          'Board-level si OL sur lignes sensor dock',
        ],
        note: "Corrélation forte (>85%) sur iPhone 13 : 0x800 + reboot ~3 min = nappe dock.",
      }));
    }

    if (mask === 0x1000 || mask === 4096) {
      out.push(card({
        id: '13-0x1000', matchScore: 92,
        codeBadges: ['S.sensor 0x1000', 'Proximité'],
        severity: 'HARDWARE',
        title: 'Capteur de proximité · Écran avant',
        subtitle: '0x1000 = capteur proximité non détecté',
        component: "Écran avant (capteur proximité intégré) · écouteur avant",
        likelyCause: "Écran aftermarket sans capteur, câble mal connecté après réparation",
        keywords: ['0x1000', 'proximité'],
        quickTest: "Rebrancher le connecteur écran et l'écouteur ; tester avec écran OEM",
        steps: [
          "Vérifier que le câble de l'écran est correctement connecté",
          "Vérifier l'écouteur avant (capteur proximité souvent intégré sur 13)",
          "Tester avec un écran OEM pour isoler",
          "Si persiste avec OEM → board-level sur les lignes proximité",
        ],
      }));
    }

    if (mask === 0x4000 || mask === 16384) {
      out.push(card({
        id: '13-0x4000', matchScore: 91,
        codeBadges: ['S.sensor 0x4000', 'Données batterie'],
        severity: 'HARDWARE',
        title: 'Données batterie non lisibles · Jauge batterie',
        subtitle: '0x4000 = capteur ou données batterie absents',
        component: 'Batterie · connecteur batterie · IC jauge batterie',
        likelyCause: 'Connecteur batterie mal rebranché, batterie HS, IC jauge batterie',
        keywords: ['0x4000', 'batterie'],
        steps: [
          'Débrancher/rebrancher proprement le connecteur batterie',
          'Tester avec une batterie OEM connue bonne',
          "Mesure diode sur les pins de données batterie",
          "IC jauge batterie si board-level confirmé",
        ],
      }));
    }
  }

  if (!out.length) {
    if (ctx.missing.has('mic1') || ctx.blobLower.includes('mic1')) {
      out.push(card({
        id: '13-mic1-text', matchScore: 88,
        codeBadges: ['Capteur manquant : mic1'],
        severity: 'HARDWARE',
        title: 'Nappe connecteur de charge · Micro bas',
        subtitle: 'mic1 sur nappe dock — série 13',
        component: 'Nappe port Lightning',
        likelyCause: 'Nappe absente, aftermarket, eau',
        keywords: ['mic1'],
        steps: ['Remplacer nappe port Lightning OEM', 'Vérifier connecteur carte mère', 'Si persiste → board-level'],
      }));
    }
    if (ctx.missing.has('mic2')) {
      out.push(card({
        id: '13-mic2-text', matchScore: 87,
        codeBadges: ['Capteur manquant : mic2'],
        severity: 'HARDWARE',
        title: 'Écouteur avant · Capteurs avant',
        subtitle: 'mic2 = micro sur ensemble avant',
        component: 'Écouteur interne OEM',
        likelyCause: "Écouteur aftermarket après changement d'écran",
        keywords: ['mic2'],
        steps: ["Vérifier câble écouteur rebranché", "Tester avec écouteur OEM", "Si persiste → board-level lignes mic2"],
      }));
    }
  }
  return out;
}

function iphone13ProCards(ctx: MatchCtx): ModelWorkshopCard[] {
  return iphone13Cards(ctx).map((c) => ({ ...c, id: c.id.replace(/^13/, '13p') }));
}

// ── iPhone 14 / 14 Plus ──────────────────────────────────────────────────────
function iphone14Cards(ctx: MatchCtx): ModelWorkshopCard[] {
  const out: ModelWorkshopCard[] = [];
  for (const mask of ctx.sensorMasks) {
    if (mask === 0x600000 || mask === 6291456) {
      out.push(card({
        id: '14-0x600000', matchScore: 96,
        codeBadges: ['S.sensor 0x600000', 'Qi + Proximité'],
        severity: 'HARDWARE',
        title: 'Double défaut · Recharge sans fil + Proximité · iPhone 14',
        subtitle: '0x600000 = Qi/MagSafe ET capteur de proximité',
        component: 'Nappe Qi (vitre arrière) + écouteur avant / proximité',
        likelyCause: 'Vitre arrière remplacée (nappe Qi abîmée) + écran aftermarket',
        keywords: ['0x600000', 'Qi', 'proximité'],
        steps: [
          "PRIORITÉ : vérifier si la vitre arrière a été remplacée récemment (= nappe Qi souvent abîmée)",
          "Remplacer la nappe Qi/vitre arrière OEM",
          "Vérifier l'écouteur avant / capteur de proximité",
          "Tester pièces une par une pour isoler",
        ],
        note: "Double défaut fréquent après remplacement de vitre arrière par tiers + écran aftermarket.",
      })); continue;
    }
    if (mask === 0x400000 || mask === 4194304) {
      out.push(card({
        id: '14-0x400000', matchScore: 94,
        codeBadges: ['S.sensor 0x400000', 'Recharge sans fil'],
        severity: 'HARDWARE',
        title: 'Nappe recharge sans fil (Qi/MagSafe) · Vitre arrière · iPhone 14',
        subtitle: '0x400000 = bobine / nappe Qi non lisible',
        component: 'Nappe MagSafe/Qi intégrée à la vitre arrière',
        likelyCause: "Vitre arrière remplacée par tiers (nappe Qi non reconnectée ou coupée), chute",
        keywords: ['0x400000', 'Qi', 'MagSafe'],
        quickTest: "Désactiver la recharge sans fil dans réglages : si panic disparaît, Qi confirmé",
        steps: [
          "Ouvrir le téléphone et vérifier le connecteur de la nappe Qi (vitre arrière)",
          "Si vitre arrière remplacée : vérifier que la nappe Qi a été reconnectée correctement",
          "Remplacer la nappe Qi / ensemble vitre arrière OEM",
          "Si persiste → vérifier l'IC de gestion Qi sur la carte mère",
        ],
        note: "Très fréquent sur iPhone 14 après remplacement de vitre arrière : la nappe Qi/MagSafe est coupée ou non reconnectée.",
      }));
    }
    if (mask === 0x100000 || mask === 1048576) {
      out.push(card({
        id: '14-0x100000', matchScore: 93,
        codeBadges: ['S.sensor 0x100000', 'Connecteur charge'],
        severity: 'HARDWARE',
        title: 'Nappe connecteur de charge Lightning · iPhone 14',
        subtitle: '0x100000 = capteur sur nappe dock',
        component: 'Nappe port Lightning (connecteur de charge)',
        likelyCause: 'Nappe dock absente, aftermarket, eau',
        keywords: ['0x100000', 'dock', 'Lightning'],
        steps: ['Vérifier la nappe port Lightning (état, oxydation)', 'Remplacer par OEM', 'Si persiste → board-level'],
      }));
    }
    if (mask === 0x200000 || mask === 2097152) {
      out.push(card({
        id: '14-0x200000', matchScore: 92,
        codeBadges: ['S.sensor 0x200000', 'Proximité'],
        severity: 'HARDWARE',
        title: 'Capteur de proximité · Écran avant · iPhone 14',
        subtitle: '0x200000 = proximité non détecté',
        component: 'Écran avant (capteur proximité) · écouteur avant',
        likelyCause: "Écran aftermarket, réparation précédente, capteur non rebranché",
        keywords: ['0x200000', 'proximité'],
        steps: ["Rebrancher les connecteurs écran et écouteur", "Tester avec écran OEM", "Si persiste → board-level lignes proximité"],
      }));
    }
    if (mask === 0x500000 || mask === 5242880) {
      out.push(card({
        id: '14-0x500000', matchScore: 88,
        codeBadges: ['S.sensor 0x500000', 'Batterie / Taptic'],
        severity: 'HARDWARE',
        title: 'Communication batterie · Taptic Engine · iPhone 14',
        subtitle: '0x500000 = batterie OU Taptic Engine selon contexte',
        component: 'Batterie · Taptic Engine · connecteur batterie',
        likelyCause: 'Batterie HS, Taptic Engine débranché, connecteur batterie mal rebranché',
        keywords: ['0x500000', 'batterie', 'Taptic'],
        steps: [
          'Vérifier et rebrancher le connecteur batterie',
          'Vérifier le connecteur du Taptic Engine',
          'Tester batterie OEM',
          "Si résolution sans Taptic branché → Taptic Engine en cause",
          "Si persiste → board-level",
        ],
      }));
    }
    if (mask === 0x20000 || mask === 131072) {
      out.push(card({
        id: '14-0x20000', matchScore: 78,
        codeBadges: ['S.sensor 0x20000', 'Logic board'],
        severity: 'BOARD-LEVEL',
        title: 'Problème carte mère · iPhone 14',
        subtitle: '0x20000 = défaut interne carte mère (architecture sandwich)',
        component: 'Carte mère (deux parties reliées)',
        likelyCause: 'Chute, infiltration eau, soudure froide inter-cartes',
        keywords: ['0x20000', 'board', 'sandwich'],
        steps: [
          "Inspecter visuellement la carte mère (fissure, corrosion)",
          "Nettoyage ultrasons si traces de liquide",
          "Vérifier les connecteurs inter-cartes (iPhone 14 = deux cartes)",
          "Si persiste → microsoudure ou remplacement carte mère",
        ],
        note: "0x20000 sur iPhone 14 peut indiquer un problème de séparation des deux parties de la carte mère.",
      }));
    }
  }
  if (!out.length) out.push(...genericSensorFallback(ctx, '14'));
  return out;
}

// ── iPhone 14 Pro / 14 Pro Max ───────────────────────────────────────────────
function iphone14ProCards(ctx: MatchCtx): ModelWorkshopCard[] {
  const out: ModelWorkshopCard[] = [];
  for (const mask of ctx.sensorMasks) {
    if (mask === 0x1c0000 || mask === 1835008) {
      out.push(card({
        id: '14p-0x1c0000', matchScore: 97,
        codeBadges: ['S.sensor 0x1C0000', 'Triple défaut'],
        severity: 'HARDWARE',
        title: 'Triple défaut · Proximité + Bouton Power + Dock · 14 Pro',
        subtitle: '0x1C0000 = trois capteurs défaillants simultanément',
        component: 'Nappe dock + nappe bouton power + capteur proximité',
        likelyCause: 'Démontage multiple, eau, chute sévère',
        keywords: ['0x1C0000', 'proximité', 'power', 'dock'],
        steps: [
          "Tester chaque pièce séparément : dock OEM, puis nappe power OEM, puis écouteur/proximité OEM",
          "Commencer par la nappe dock (corrélation la plus forte)",
          "Si deux pièces OEM ne résolvent pas → board-level",
        ],
        note: "Triple défaut rare en dehors d'une immersion ou chute sévère. Vérifier traces de liquide en priorité.",
      })); continue;
    }
    if (mask === 0x180000 || mask === 1572864) {
      out.push(card({
        id: '14p-0x180000', matchScore: 96,
        codeBadges: ['S.sensor 0x180000', 'Proximité + Power'],
        severity: 'HARDWARE',
        title: 'Double défaut · Proximité + Bouton Power · 14 Pro',
        subtitle: '0x180000 = proximité ET nappe bouton power',
        component: 'Nappe bouton power + capteur de proximité',
        likelyCause: "Réparation écran mal refaite, nappe power débranchée",
        keywords: ['0x180000', 'proximité', 'bouton power'],
        steps: [
          "Vérifier la nappe bouton power (souvent débranchée lors d'un remplacement d'écran)",
          "Vérifier le capteur de proximité (câble écouteur avant)",
          "Remplacer nappe power par OEM",
          "Tester avec écran/écouteur OEM pour le capteur de proximité",
        ],
      })); continue;
    }
    if (mask === 0x140000 || mask === 1310720) {
      out.push(card({
        id: '14p-0x140000', matchScore: 96,
        codeBadges: ['S.sensor 0x140000', 'Power + Dock'],
        severity: 'HARDWARE',
        title: 'Double défaut · Bouton Power + Connecteur charge · 14 Pro',
        subtitle: '0x140000 = nappe power ET nappe dock',
        component: 'Nappe bouton power + nappe dock USB-C',
        likelyCause: 'Eau, chute, deux nappes endommagées simultanément',
        keywords: ['0x140000', 'power', 'dock'],
        steps: ["Remplacer la nappe bouton power OEM", "Remplacer la nappe dock USB-C OEM", "Tester les pièces séparément"],
      })); continue;
    }
    if (mask === 0xc0000 || mask === 786432) {
      out.push(card({
        id: '14p-0xc0000', matchScore: 95,
        codeBadges: ['S.sensor 0xC0000', 'Proximité + Dock'],
        severity: 'HARDWARE',
        title: 'Double défaut · Proximité + Connecteur charge · 14 Pro',
        subtitle: '0xC0000 = proximité ET dock USB-C',
        component: 'Nappe dock USB-C + capteur de proximité / écouteur avant',
        likelyCause: 'Pièces aftermarket multiples, réparation précédente',
        keywords: ['0xC0000', 'proximité', 'dock'],
        steps: ["Remplacer la nappe dock USB-C OEM en premier", "Vérifier l'écouteur avant", "Tester séparément"],
      })); continue;
    }
    if (mask === 0x100000 || mask === 1048576) {
      out.push(card({
        id: '14p-0x100000', matchScore: 94,
        codeBadges: ['S.sensor 0x100000', 'Bouton Power'],
        severity: 'HARDWARE',
        title: 'Nappe bouton Power · iPhone 14 Pro',
        subtitle: '0x100000 = micro / capteur sur nappe bouton power',
        component: 'Nappe bouton power (côté caméra)',
        likelyCause: 'Nappe débranchée lors réparation, oxydation, chute',
        keywords: ['0x100000', 'bouton power'],
        quickTest: "Vérifier visuellement que la nappe power est branchée",
        steps: [
          "Vérifier que la nappe bouton power est connectée",
          "Inspecter l'état de la nappe (oxydation, coupure)",
          "Remplacer par OEM",
          "Si persiste → board-level sur les lignes capteur associées",
        ],
        note: "Même logique que iPhone 11 mic2 : la nappe power partage le chemin du micro/capteur flash.",
      }));
    }
    if (mask === 0x80000 || mask === 524288) {
      out.push(card({
        id: '14p-0x80000', matchScore: 93,
        codeBadges: ['S.sensor 0x80000', 'Proximité'],
        severity: 'HARDWARE',
        title: 'Capteur de proximité · Nappe capteurs avant · 14 Pro',
        subtitle: '0x80000 = proximité / capteurs avant',
        component: "Écran avant (capteur proximité) · écouteur avant",
        likelyCause: "Écran aftermarket, réparation précédente",
        keywords: ['0x80000', 'proximité'],
        steps: ["Rebrancher les câbles écran et écouteur", "Tester avec écran OEM", "Si persiste → board-level"],
      }));
    }
    if (mask === 0x40000 || mask === 262144) {
      out.push(card({
        id: '14p-0x40000', matchScore: 93,
        codeBadges: ['S.sensor 0x40000', 'Connecteur charge USB-C'],
        severity: 'HARDWARE',
        title: 'Nappe connecteur de charge USB-C · iPhone 14 Pro',
        subtitle: '0x40000 = capteur sur nappe dock USB-C',
        component: 'Nappe dock USB-C',
        likelyCause: 'Nappe dock absente, aftermarket, eau',
        keywords: ['0x40000', 'USB-C', 'dock'],
        steps: ['Vérifier la nappe dock USB-C', 'Remplacer par OEM', 'Si persiste → board-level'],
      }));
    }
    if (mask === 0x41 || mask === 65) {
      out.push(card({
        id: '14p-0x41', matchScore: 90,
        codeBadges: ['S.sensor 0x41', 'Données batterie'],
        severity: 'HARDWARE',
        title: 'Données batterie · Jauge batterie · iPhone 14 Pro',
        subtitle: '0x41 = capteur/données batterie',
        component: 'Batterie · connecteur batterie · IC jauge batterie',
        likelyCause: 'Connecteur batterie, batterie HS',
        keywords: ['0x41', 'batterie'],
        steps: ['Rebrancher le connecteur batterie proprement', 'Tester avec batterie OEM', 'Mesure diode pins données si persiste'],
      }));
    }
    if (mask === 0x20000 || mask === 131072) {
      out.push(card({
        id: '14p-0x20000', matchScore: 82,
        codeBadges: ['S.sensor 0x20000', 'Sandwich board'],
        severity: 'BOARD-LEVEL',
        title: 'Séparation carte mère (sandwich) · iPhone 14 Pro',
        subtitle: '0x20000 = problème inter-cartes (sandwich board Pro)',
        component: 'Carte mère supérieure/inférieure · connecteur inter-cartes',
        likelyCause: 'Chute, eau, séparation mal refaite, soudure froide inter-cartes',
        keywords: ['0x20000', 'sandwich board'],
        steps: [
          "Vérifier les connecteurs inter-cartes après démontage complet",
          "Nettoyer les contacts si oxydation",
          "Si réparation sandwich précédente : vérifier qualité du rebrassage",
          "Microsoudure si contact ouvert",
        ],
        note: "iPhone 14 Pro = architecture sandwich. Toujours vérifier les connecteurs inter-cartes avant board-level.",
      }));
    }
  }
  if (!out.length) out.push(...genericSensorFallback(ctx, '14p'));
  return out;
}

const IPHONE15_BOTTOM_MIC_HW =
  'Module PCB MEMS clipsé sur flex USB-C (grille métal + joint mousse) — panics fréquents si oxydé';

function iphone15BottomMicCards(ctx: MatchCtx, idPrefix: string): ModelWorkshopCard[] {
  const mic1 =
    ctx.missing.has('mic1') || ctx.blobLower.includes('mic1') || ctx.blobLower.includes('missing sensor');
  if (!mic1 && !ctx.hasThermal) return [];
  const oxy =
    /oxyd|corros|liquid|eau|humid|water/.test(ctx.blobLower);
  return [
    card({
      id: `${idPrefix}-bottom-mic-mems`,
      matchScore: ctx.hasThermal && mic1 ? 98 : oxy ? 97 : 94,
      codeBadges: ['MIC1', ctx.hasThermal ? 'thermalmonitord' : 'capteur bas'],
      severity: 'HARDWARE',
      title: 'Module micro du bas (MIC1) · PCB MEMS · iPhone 15',
      subtitle: IPHONE15_BOTTOM_MIC_HW,
      component: 'Module micro bas (MEMS) + assemblage connecteur USB-C',
      likelyCause: oxy
        ? 'Oxydation / liquide sur connecteur MEMS ou clip mal enfoncé'
        : 'Clip MIC1 sur flex USB-C, joint acoustique, flex aftermarket',
      keywords: ['MIC1', 'thermalmonitord', 'USB-C', 'MEMS', 'oxydation'],
      quickTest: 'Le téléphone peut encore charger alors que MIC1 est absent',
      steps: [
        'Loupe : grille métal, connecteur MEMS, joint mousse/caoutchouc',
        'Reseat clip micro bas sur nappe USB-C',
        oxy ? 'Nettoyage ultrason / IPA si oxydé' : 'Tester flex USB-C OEM connu bon',
        'Remplacer assemblage charge port OEM si persiste',
      ],
      note: 'thermalmonitord sur série 15 ≠ surchauffe CPU — prioriser module micro bas.',
    }),
  ];
}

// ── iPhone 15 / 15 Plus ──────────────────────────────────────────────────────
function iphone15Cards(ctx: MatchCtx): ModelWorkshopCard[] {
  const out: ModelWorkshopCard[] = [...iphone15BottomMicCards(ctx, '15')];
  for (const mask of ctx.sensorMasks) {
    if (mask === 0x380000 || mask === 3670016) {
      out.push(card({
        id: '15-0x380000', matchScore: 97,
        codeBadges: ['S.sensor 0x380000', 'Triple : Qi + Dock + Proximité'],
        severity: 'HARDWARE',
        title: 'Triple défaut · Qi + USB-C + Proximité · iPhone 15',
        subtitle: '0x380000 = recharge sans fil + dock USB-C + capteurs avant',
        component: 'Nappe Qi/MagSafe + nappe dock USB-C + capteur proximité',
        likelyCause: 'Immersion, chute sévère, vitre arrière remplacée + réparation écran',
        keywords: ['0x380000', 'Qi', 'USB-C', 'proximité'],
        steps: [
          "Vérifier traces de liquide (immersion fréquente sur masque triple)",
          "Remplacer la nappe Qi (vitre arrière) OEM",
          "Remplacer la nappe dock USB-C OEM",
          "Vérifier l'écouteur avant / capteur proximité",
          "Tester pièce par pièce pour isoler",
        ],
        note: "Triple défaut sur 15 = souvent immersion. Nettoyage ultrasons avant remplacement des nappes.",
      })); continue;
    }
    if (mask === 0x280000 || mask === 2621440) {
      out.push(card({
        id: '15-0x280000', matchScore: 96,
        codeBadges: ['S.sensor 0x280000', 'Qi + Dock USB-C'],
        severity: 'HARDWARE',
        title: 'Double défaut · Recharge sans fil + USB-C · iPhone 15',
        subtitle: '0x280000 = Qi/MagSafe ET nappe dock USB-C',
        component: 'Nappe Qi/MagSafe (vitre arrière) + nappe dock USB-C',
        likelyCause: 'Vitre arrière remplacée (nappe Qi coupée) + dock endommagé',
        keywords: ['0x280000', 'Qi', 'USB-C'],
        quickTest: "Commencer par la nappe Qi (vitre arrière) — cause la plus fréquente",
        steps: [
          "ÉTAPE 1 : Remplacer la nappe Qi/MagSafe (vitre arrière) OEM",
          "ÉTAPE 2 : Si persiste, remplacer aussi la nappe dock USB-C OEM",
          "Vérifier les deux connexions sur la carte mère",
          "Si persiste avec les deux OEM → board-level sur les bus Qi et USB-C",
        ],
        note: "Signature très fréquente sur iPhone 15 après remplacement de vitre arrière par tiers : Qi coupée.",
      })); continue;
    }
    if (mask === 0x200000 || mask === 2097152) {
      out.push(card({
        id: '15-0x200000', matchScore: 94,
        codeBadges: ['S.sensor 0x200000', 'Recharge sans fil'],
        severity: 'HARDWARE',
        title: 'Nappe recharge sans fil (Qi/MagSafe) · iPhone 15',
        subtitle: '0x200000 = bobine Qi non lisible',
        component: 'Nappe Qi/MagSafe intégrée à la vitre arrière',
        likelyCause: 'Vitre arrière remplacée, nappe Qi non reconnectée ou coupée',
        keywords: ['0x200000', 'Qi', 'MagSafe'],
        steps: [
          "Vérifier si la vitre arrière a été remplacée récemment",
          "Ouvrir le téléphone, vérifier le connecteur nappe Qi sur la carte mère",
          "Remplacer la nappe Qi OEM",
          "Si persiste → vérifier IC gestion Qi sur la carte mère",
        ],
      }));
    }
    if (mask === 0x80000 || mask === 524288) {
      out.push(card({
        id: '15-0x80000', matchScore: 95,
        codeBadges: ['S.sensor 0x80000', 'USB-C + MIC1 bas'],
        severity: 'HARDWARE',
        title: 'Assemblage USB-C + module micro du bas · iPhone 15',
        subtitle: '0x80000 = flex charge + MIC1 (MEMS) / baromètre — oxydation fréquente',
        component: 'Module micro bas (MIC1) + nappe dock USB-C',
        likelyCause: 'Module MEMS oxydé, clip MIC1 faux, liquide, flex aftermarket',
        keywords: ['0x80000', 'USB-C', 'MIC1', 'MEMS', 'oxydation'],
        steps: [
          'Inspecter module micro bas (connecteur MEMS, joint mousse)',
          'Reseat clip MIC1 sur nappe USB-C',
          'Remplacer nappe dock USB-C OEM (baromètre + MIC1 sur même assemblage)',
          'Nettoyage ultrason si oxydation visible',
        ],
        note: IPHONE15_BOTTOM_MIC_HW,
      }));
    }
    if (mask === 0x100000 || mask === 1048576) {
      out.push(card({
        id: '15-0x100000', matchScore: 91,
        codeBadges: ['S.sensor 0x100000', 'Proximité'],
        severity: 'HARDWARE',
        title: 'Capteur de proximité · Capteurs avant · iPhone 15',
        subtitle: '0x100000 = capteurs avant / proximité',
        component: 'Écouteur avant / capteur proximité / câble écran avant',
        likelyCause: "Écran aftermarket, réparation précédente, câble déconnecté",
        keywords: ['0x100000', 'proximité'],
        steps: ["Rebrancher tous les câbles écran avant", "Tester avec écran OEM", "Si persiste → board-level"],
      }));
    }
    if (mask === 0xa1 || mask === 161) {
      out.push(card({
        id: '15-0xa1', matchScore: 92,
        codeBadges: ['S.sensor 0xA1', 'Batterie'],
        severity: 'HARDWARE',
        title: 'Données batterie · Jauge batterie · iPhone 15',
        subtitle: '0xA1 = capteur ou données batterie',
        component: 'Batterie · connecteur batterie',
        likelyCause: 'Connecteur batterie mal rebranché, batterie HS',
        keywords: ['0xA1', 'batterie'],
        steps: ['Rebrancher le connecteur batterie', 'Tester avec batterie OEM connue bonne', 'Mesure diode si persiste'],
      }));
    }
  }
  if (!out.length) out.push(...genericSensorFallback(ctx, '15'));
  return out;
}

// ── iPhone 15 Pro / 15 Pro Max ───────────────────────────────────────────────
function iphone15ProCards(ctx: MatchCtx): ModelWorkshopCard[] {
  const out: ModelWorkshopCard[] = [...iphone15BottomMicCards(ctx, '15p')];
  for (const mask of ctx.sensorMasks) {
    if (mask === 0x700000 || mask === 7340032) {
      out.push(card({
        id: '15p-0x700000', matchScore: 96,
        codeBadges: ['S.sensor 0x700000', 'Dock + Qi'],
        severity: 'HARDWARE',
        title: 'Double défaut · USB-C + Recharge sans fil · 15 Pro',
        subtitle: '0x700000 = dock USB-C ET bobine Qi',
        component: 'Nappe dock USB-C + nappe Qi/MagSafe',
        likelyCause: 'Immersion, vitre arrière remplacée + dock endommagé',
        keywords: ['0x700000', 'USB-C', 'Qi'],
        steps: ["Remplacer nappe dock USB-C OEM", "Remplacer nappe Qi/MagSafe OEM", "Tester séparément"],
      })); continue;
    }
    if (mask === 0x600000 || mask === 6291456) {
      out.push(card({
        id: '15p-0x600000', matchScore: 95,
        codeBadges: ['S.sensor 0x600000', 'Qi + Proximité'],
        severity: 'HARDWARE',
        title: 'Double défaut · Recharge sans fil + Proximité · 15 Pro',
        subtitle: '0x600000 = Qi ET capteurs avant',
        component: 'Nappe Qi + capteur proximité / écouteur avant',
        likelyCause: 'Vitre arrière remplacée + écran aftermarket',
        keywords: ['0x600000', 'Qi', 'proximité'],
        steps: ["Remplacer la nappe Qi (vitre arrière) OEM", "Vérifier l'écouteur avant / capteur proximité", "Tester séparément"],
      })); continue;
    }
    if (mask === 0x400000 || mask === 4194304) {
      out.push(card({
        id: '15p-0x400000', matchScore: 93,
        codeBadges: ['S.sensor 0x400000', 'Qi arrière'],
        severity: 'HARDWARE',
        title: 'Nappe recharge sans fil (Qi) · iPhone 15 Pro',
        subtitle: '0x400000 = bobine Qi non lisible',
        component: 'Nappe MagSafe/Qi (vitre arrière)',
        likelyCause: 'Vitre arrière remplacée, chute, nappe Qi coupée',
        keywords: ['0x400000', 'Qi', 'MagSafe'],
        steps: ["Vérifier si vitre arrière remplacée récemment", "Contrôler le connecteur nappe Qi", "Remplacer nappe Qi OEM"],
      }));
    }
    if (mask === 0x300000 || mask === 3145728) {
      out.push(card({
        id: '15p-0x300000', matchScore: 95,
        codeBadges: ['S.sensor 0x300000', 'USB-C + MIC1 bas'],
        severity: 'HARDWARE',
        title: 'Assemblage USB-C + module micro du bas · 15 Pro',
        subtitle: '0x300000 = flex charge + MIC1 (MEMS) — oxydation fréquente',
        component: 'Module micro bas (MIC1) + nappe dock USB-C',
        likelyCause: 'Module MEMS oxydé, clip MIC1, liquide, flex aftermarket',
        keywords: ['0x300000', 'USB-C', 'MIC1', 'MEMS'],
        steps: [
          'Inspecter module micro bas et clip sur flex USB-C',
          'Remplacer nappe dock USB-C OEM',
          'Ultrason / IPA si oxydation',
        ],
        note: IPHONE15_BOTTOM_MIC_HW,
      }));
    }
    if (mask === 0x200000 || mask === 2097152) {
      out.push(card({
        id: '15p-0x200000', matchScore: 91,
        codeBadges: ['S.sensor 0x200000', 'Proximité'],
        severity: 'HARDWARE',
        title: 'Capteur de proximité · Dynamic Island · iPhone 15 Pro',
        subtitle: '0x200000 = proximité / capteurs avant Dynamic Island',
        component: 'Écouteur avant · capteur proximité Dynamic Island',
        likelyCause: "Écran aftermarket, câble déconnecté, réparation précédente",
        keywords: ['0x200000', 'proximité', 'Dynamic Island'],
        steps: [
          "Vérifier tous les câbles de l'écran avant",
          "Tester avec écran OEM (Dynamic Island intègre les capteurs sur 15 Pro)",
          "Si persiste → board-level",
        ],
        note: "Sur 15 Pro, les capteurs avant sont intégrés au module Dynamic Island — utiliser uniquement OEM.",
      }));
    }
    if (mask === 0xa1 || mask === 161) {
      out.push(card({
        id: '15p-0xa1', matchScore: 92,
        codeBadges: ['S.sensor 0xA1', 'Batterie'],
        severity: 'HARDWARE',
        title: 'Données batterie · Jauge batterie · iPhone 15 Pro',
        subtitle: '0xA1 = capteur ou données batterie',
        component: 'Batterie · connecteur batterie',
        likelyCause: 'Connecteur batterie mal rebranché, batterie HS',
        keywords: ['0xA1', 'batterie'],
        steps: ['Rebrancher le connecteur batterie', 'Tester avec batterie OEM', 'Mesure diode si persiste'],
      }));
    }
  }
  if (!out.length) out.push(...genericSensorFallback(ctx, '15p'));
  return out;
}

// ── iPhone 16 / 16 Pro ───────────────────────────────────────────────────────
function iphone16Cards(ctx: MatchCtx): ModelWorkshopCard[] {
  return iphone15Cards(ctx).map((c) => ({
    ...c, id: c.id.replace(/^15-/, '16-'),
    subtitle: c.subtitle.replace('iPhone 15', 'iPhone 16'),
    matchScore: Math.max((c.matchScore ?? 80) - 5, 70),
    note: (c.note ? c.note + ' ' : '') + '(KB iPhone 16 en cours — corrélations basées sur 15)',
  }));
}

function iphone16ProCards(ctx: MatchCtx): ModelWorkshopCard[] {
  return iphone15ProCards(ctx).map((c) => ({
    ...c, id: c.id.replace(/^15p-/, '16p-'),
    subtitle: c.subtitle.replace('15 Pro', '16 Pro'),
    matchScore: Math.max((c.matchScore ?? 80) - 3, 72),
  }));
}

// ── Fallback générique (masques inconnus + série 13+) ────────────────────────
function genericSensorFallback(ctx: MatchCtx, prefix: string): ModelWorkshopCard[] {
  const out: ModelWorkshopCard[] = [];
  if (ctx.missing.has('mic1') || ctx.blobLower.includes('mic1')) {
    out.push(card({
      id: `${prefix}-mic1-fallback`, matchScore: 75,
      codeBadges: ['Capteur manquant : mic1'],
      severity: 'HARDWARE',
      title: 'Nappe connecteur de charge · Micro bas',
      subtitle: 'mic1 sur nappe dock',
      component: 'Nappe port de charge',
      likelyCause: 'Nappe dock absente, aftermarket, eau',
      keywords: ['mic1'],
      steps: ['Remplacer nappe dock OEM', 'Si persiste → board-level'],
    }));
  }
  if (ctx.sensorMasks.length > 0) {
    const maskStr = ctx.sensorMasks.map((m) => `0x${m.toString(16).toUpperCase()}`).join(', ');
    out.push(card({
      id: `${prefix}-mask-unknown`, matchScore: 60,
      codeBadges: [`S.sensor ${maskStr}`],
      severity: 'HARDWARE',
      title: `Capteur absent · Masque ${maskStr}`,
      subtitle: 'Masque sensor array — vérifier nappes selon modèle',
      component: 'Nappe dock / Qi / bouton power / capteurs avant selon masque',
      likelyCause: 'Nappe endommagée, pièce aftermarket, eau',
      keywords: [maskStr],
      steps: [
        `Masque ${maskStr} détecté dans le S.sensor array`,
        'Vérifier en premier : nappe dock (charge), nappe Qi (sans fil), bouton power',
        "Tester avec pièces OEM pour isoler",
        "Si persiste avec OEM → board-level : inspecter composant associé sous microscope",
        "Coller le log complet dans PanicBase pour analyse plus fine",
      ],
      note: "Masque non encore répertorié pour ce modèle précis. Déposer le cas pour enrichissement de la base.",
    }));
  }
  return out;
}

// ── Carte de repli depuis analyse structurée Rust ────────────────────────────
function fallbackCardFromAnalysis(
  analysis: AnalysisResult,
  ctx: MatchCtx,
  searchToken: string,
): ModelWorkshopCard | null {
  const sd = analysis.structured_diagnostic;
  const primary = analysis.probable_cause?.replace(/\s*\[Repair Wiki\]\s*$/i, '').trim();
  if (!primary || /^Non classifié|Unclassified/i.test(primary)) return null;

  const steps = [...(sd.action_plan ?? []), ...(sd.isolation_sequence ?? []), ...(sd.recommended_checks ?? [])]
    .map((s) => s.trim()).filter(Boolean).slice(0, 6);

  // Badges lisibles : jamais de codes bruts 2-5 caractères seuls
  const rawBadges: string[] = [];
  if (searchToken && searchToken.length > 1 && !/^[a-z]{2,5}\d?$/i.test(searchToken)) {
    rawBadges.push(searchToken);
  }
  for (const sig of sd.normalized_signatures?.slice(0, 2) ?? []) {
    if (sig.length < 48 && !/^[a-z]{2,5}\d?$/i.test(sig.trim())) rawBadges.push(sig);
  }
  const badges = rawBadges.length
    ? rawBadges
    : [analysis.panic_type.replace(/_/g, ' ').replace(/\b\w/g, (c) => c.toUpperCase())];

  const isSw = /firmware fatal|software|dfu restore/i.test(primary);
  const likelyParts = (sd.likely_parts ?? []).filter(Boolean);
  const part = likelyParts.join(' · ') || sd.possible_causes?.[0]?.name?.split('·')[0]?.trim() || primary.split('·')[0]?.trim() || primary;

  return card({
    id: 'fallback-structured', matchScore: 42,
    codeBadges: badges,
    severity: isSw ? 'SOFTWARE' : 'HARDWARE',
    title: (primary.split('·').slice(0, 2).join(' · ').trim() || primary).slice(0, 80),
    subtitle: analysis.panic_type.replace(/_/g, ' ').replace(/\b\w/g, (c) => c.toUpperCase()).slice(0, 60),
    component: part.slice(0, 120),
    likelyCause: sd.possible_causes?.[1]?.name?.slice(0, 100) ?? sd.possible_causes?.[0]?.name?.slice(0, 100) ?? '—',
    keywords: (analysis.keywords ?? []).slice(0, 8),
    steps: steps.length ? steps : ['Importer un panic-full récent et recouper avec la fiche modèle dans la référence.'],
    note: sd.danger_flags?.[0],
  });
}


// ── ROUTEUR PAR MODÈLE (patch Claude) ─────────────────────────────────────────
export function resolveModelWorkshopCards(
  panicText: string,
  analysis: AnalysisResult,
  productType: string | null,
): ModelWorkshopCard[] {
  const ctx = ctxFrom(panicText, analysis, productType);
  let cards: ModelWorkshopCard[] = [] as ModelWorkshopCard[];

  switch (ctx.section) {
    case 'iphone-11': cards = iphone11Cards(ctx); break;
    case 'iphone-x': cards = iphoneXCards(ctx); break;
    case 'iphone-xs': cards = iphoneXsCards(ctx); break;
    case 'iphone-xr': cards = iphoneXrCards(ctx); break;
    case 'iphone-12': cards = iphone12Cards(ctx); break;
    case 'iphone-12-pro': cards = iphone12ProCards(ctx); break;
    case 'iphone-13': cards = iphone13Cards(ctx); break;
    case 'iphone-13-pro': cards = iphone13ProCards(ctx); break;
    case 'iphone-14': cards = iphone14Cards(ctx); break;
    case 'iphone-14-pro': cards = iphone14ProCards(ctx); break;
    case 'iphone-15': cards = iphone15Cards(ctx); break;
    case 'iphone-15-pro': cards = iphone15ProCards(ctx); break;
    case 'iphone-16': cards = iphone16Cards(ctx); break;
    case 'iphone-16-pro': cards = iphone16ProCards(ctx); break;
    case 'iphone-17':
    case 'iphone-17-pro':
      cards = iphone16ProCards(ctx).map((c) => ({
        ...c, id: c.id.replace(/^16p-/, '17-'),
        note: (c.note ?? '') + ' (données iPhone 17 en cours de validation)',
        matchScore: Math.max((c.matchScore ?? 80) - 8, 65),
      }));
      break;
    case 'iphone-8':
    case 'iphone-7':
    case 'iphone-6s':
    case 'iphone-se1':
    case 'iphone-se2':
    case 'iphone-se3':
      cards = iphoneXCards(ctx);
      break;
    default: break;
  }

  if (!cards.length && ctx.sensorMasks.length > 0) {
    cards = genericSensorFallback(ctx, 'generic');
  }

  if (!cards.length) {
    const token = inferSearchQueryFromPanicBlob(ctx.blobLower);
    const fb = fallbackCardFromAnalysis(analysis, ctx, token);
    if (fb) cards = [fb];
  }

  cards.sort((a, b) => b.matchScore - a.matchScore);
  const seen = new Set<string>();
  return cards.filter((c) => { if (seen.has(c.id)) return false; seen.add(c.id); return true; });
}

export function primaryModelWorkshopCard(
  panicText: string,
  analysis: AnalysisResult,
  productType: string | null,
): ModelWorkshopCard | null {
  const cards = resolveModelWorkshopCards(panicText, analysis, productType);
  return cards[0] ?? null;
}
