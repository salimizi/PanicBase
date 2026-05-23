import type { Locale } from '../i18n/translations';
import { localizeDiagnosticText } from './diagnosticLocale';
import type { AnalysisResult } from '../types/analysis';

/** Remove hex markers and wiki-style decimals for user-facing text only */
export function stripDiagnosticCodesFragments(s: string): string {
  let t = s
    .replace(/\b0x[\da-f]{2,16}\b/gi, '')
    .replace(/\(\s*[0-9]{3,14}\s*\)/g, '')
    .replace(/\s*·\s*/g, ' · ')
    .replace(/(^|\s)·(\s|$)/g, '$1')
    .replace(/\s{2,}/g, ' ')
    .trim();
  t = t.replace(/^\s*[·⋅.]+\s*/g, '').replace(/\s*[·⋅.]+\s*$/g, '').trim();
  return t;
}

/** Hex / large decimal markers (internal / export only — not shown in UI interpretation) */
export function extractDiagnosticCodes(line: string): string[] {
  const out: string[] = [];
  const seen = new Set<string>();
  const push = (s: string) => {
    const k = s.trim();
    if (!k || seen.has(k)) return;
    seen.add(k);
    out.push(k);
  };
  for (const m of line.matchAll(/\b0x[\da-f]{2,12}\b/gi)) push(m[0]);
  // "(1048576)" style from wiki lines
  for (const m of line.matchAll(/\(\s*([0-9]{3,10})\s*\)/g)) push(`${m[1]}`);
  return out.slice(0, 4);
}

type TFn = (key: string, vars?: Record<string, string | number>) => string;

const RULES: { re: RegExp; keys: string[] }[] = [
  {
    re: /nappe\s+bouton|nappe\s*power|bouton\s*power|power\s*button\s*flex|bouton\s*d['’]alimentation/i,
    keys: ['parts.powerButtonFlex'],
  },
  { re: /connecteur\s+de\s+charge|charge\s+port\s*flex|nappe\s+connecteur|usb[-–]?\s*c|lightning\s*flex|dock\s*fpc/i, keys: ['parts.chargePortFlex'] },
  /** MIC2 / écouteur : avant la règle « proximité » pour éviter de réduire le pré-ensemble avant à « nappe proximité » seule. */
  {
    re: /\bmic2\b|écouteur\s+interne|pré[-\s]?ensemble|earpiece\s+speaker|earpiece\s*\/|écouteur(?!\s+(sur\s+)?carte)/i,
    keys: ['parts.mic2EarpieceFront'],
  },
  {
    re: /proximit|capteurs?\s+avant|proximity\s+flex|front\s+sensors/i,
    keys: ['parts.proximityFrontFlex'],
  },
  {
    re: /recharge\s+sans\s+fil|wireless\s+charg|bobine\s+qi|\bqi\s*flex|\bmagsafe/i,
    keys: ['parts.wirelessCoilQi'],
  },
  { re: /batterie|battery|gas\s+gauge|\bbms\b|donn[ée]es\s*batter/i, keys: ['parts.batteryData'] },
  { re: /flash\s*flex|nappe\s*flash/i, keys: ['parts.flashFlex'] },
  { re: /mic1|microphone|prs0|dock.*mic/i, keys: ['parts.dockMicChain'] },
  { re: /pression\s+atmosph|barometric|\bbaro\b/i, keys: ['parts.barometer'] },
  { re: /taptic|\bvibrat/i, keys: ['parts.taptic'] },
  {
    re: /sandwich|interposer|séparation.*carte|stacked\s*board|\bsandwich\b/i,
    keys: ['parts.boardStack'],
  },
  { re: /carte\s+m[èe]re|logic\s*board(?!\s*flex)|motherboard\b|board\s*\(logic/i, keys: ['parts.logicBoard'] },
  {
    re: /gyro(scope)?|accel/i,
    keys: ['parts.motionSensorsBoard'],
  },
];

function uniqKeys(keys: string[]): string[] {
  const out: string[] = [];
  for (const k of keys) {
    if (!out.includes(k)) out.push(k);
  }
  return out;
}

/** Map one raw diagnostic phrase to localized part labels (prioritized). */
export function partLabelsForLine(raw: string, locale: Locale, t: TFn): string[] {
  const localized = localizeDiagnosticText(raw.trim(), locale);
  const blob = `${raw} ${localized}`;
  let keys: string[] = [];
  for (const rule of RULES) {
    if (rule.re.test(blob)) {
      keys.push(...rule.keys);
    }
  }
  if (keys.includes('parts.mic2EarpieceFront') && keys.includes('parts.proximityFrontFlex')) {
    keys = keys.filter((k) => k !== 'parts.proximityFrontFlex');
  }
  if (keys.length === 0) {
    const short = stripDiagnosticCodesFragments(localizeDiagnosticText(raw.trim(), locale)).slice(0, 220);
    if (!/[a-zA-ZÀ-ÿĀ-žА-я一-龥]/.test(short)) return [];
    return [short.trim()];
  }
  return uniqKeys(keys).map((id) => t(id));
}

export function spokenChecklistFromAnalysis(analysis: AnalysisResult, locale: Locale, t: TFn): string[] {
  const sd = analysis.structured_diagnostic;
  const lines: string[] = [];
  const rawProb = analysis.probable_cause?.trim() ?? '';
  if (rawProb && !/^Non classifié|Unclassified/i.test(rawProb)) {
    lines.push(rawProb.replace(/\s*\[Repair Wiki\]\s*$/i, '').trim());
  }
  for (const c of sd.possible_causes ?? []) {
    lines.push(c.name.replace(/\s*\[Repair Wiki\]\s*$/i, '').trim());
  }
  const seenLine = new Set<string>();
  const seenBullet = new Set<string>();
  const bullets: string[] = [];
  const orSep = ` ${t('parts.or')} `;
  for (const ln of lines) {
    const norm = ln.toLowerCase();
    if (!ln || seenLine.has(norm)) continue;
    seenLine.add(norm);
    const labels = partLabelsForLine(ln, locale, t);
    if (!labels.length) continue;
    const body = labels.join(orSep);
    const bullet = stripDiagnosticCodesFragments(localizeDiagnosticText(body, locale));
    if (!/[a-zA-ZÀ-ÿĀ-žА-я一-龥]/.test(bullet)) continue;
    const bKey = bullet.toLowerCase();
    if (seenBullet.has(bKey)) continue;
    seenBullet.add(bKey);
    bullets.push(bullet);
    if (bullets.length >= 6) break;
  }
  return bullets;
}

export function primaryPartHeadline(analysis: AnalysisResult, locale: Locale, t: TFn): string {
  const raw =
    analysis.probable_cause?.replace(/\s*\[Repair Wiki\]\s*$/i, '').trim() ||
    analysis.structured_diagnostic.possible_causes?.[0]?.name?.replace(/\s*\[Repair Wiki\]\s*$/i, '').trim() ||
    '';
  const labels = raw ? partLabelsForLine(raw, locale, t) : [];
  const orSep = ` ${t('parts.or')} `;
  let body = '';
  if (labels.length) {
    body = labels.join(orSep);
  } else if (raw) {
    body = stripDiagnosticCodesFragments(localizeDiagnosticText(raw, locale)).slice(0, 260);
  } else {
    return '';
  }
  return stripDiagnosticCodesFragments(localizeDiagnosticText(body, locale));
}
