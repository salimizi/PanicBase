/** Liens Repair Wiki — panic / redémarrages : hub + guides par famille (sources primaires par modèle). */

export const REPAIR_WIKI_PANIC_RESTARTS_HUB =
  'https://repair.wiki/w/How_to_Troubleshoot_And_Fix_iPhone_Random_Restarts_Using_Panic_Logs';

export type DeepWikiLink = { label: string; url: string };

function link(label: string, path: string): DeepWikiLink {
  return { label, url: `https://repair.wiki/w/${path}` };
}

/** Résout depuis ProductType (iPhone14,7) ou sous-chaîne marketing. Une entrée peut matcher plusieurs familles connexes. */
export function deepLinksForDevice(deviceOrProduct: string): DeepWikiLink[] {
  const raw = deviceOrProduct.trim();
  const t = raw.toLowerCase().replace(/\s+/g, '');

  const out: DeepWikiLink[] = [
    link('Hub · panic logs restarts', 'How_to_Troubleshoot_And_Fix_iPhone_Random_Restarts_Using_Panic_Logs'),
  ];

  // iPhone X (10,3 / 10,6 uniquement ; pas XS)
  if (/iphone10,[36]/.test(t) || /^iphonex\b/.test(t) || /\biphone\s*x\b(?!\s*s)/i.test(raw)) {
    out.push(link('iPhone X', 'How_To_Fix_an_iPhone_X_That_Randomly_Restarts'));
  }
  // XS / XS Max
  if (/iphone11,[246]/.test(t) || t.includes('xsmax') || /\bxs\b/i.test(raw)) {
    out.push(link('iPhone XS / XS Max', 'How_To_Fix_an_iPhone_XS_That_Randomly_Restarts'));
  }
  // iPhone 11 — éviter faux positifs type « iphone 11 → pro »
  if (/iphone12,1\b/.test(t) || /\biphone\s*11\b(?![\w\s]*(pro))/i.test(raw)) {
    out.push(link('iPhone 11', 'How_To_Fix_an_iPhone_11_That_Randomly_Restarts'));
  }
  if (/iphone12,[35]\b/.test(t) || /\biphone\s*11\s*pro\b/i.test(raw)) {
    out.push(link('iPhone 11 Pro / Max', 'How_To_Fix_an_iPhone_11_Pro_That_Randomly_Restarts'));
  }
  if (/iphone13,[1-4]\b/.test(t) || /\biphone\s*12\b/i.test(raw)) {
    out.push(link('iPhone 12 series', 'How_To_Fix_an_iPhone_12_That_Randomly_Restarts'));
  }
  if (/iphone14,[2345]\b/.test(t) || /\biphone\s*13\b/i.test(raw)) {
    out.push(link('iPhone 13 series', 'How_To_Fix_an_iPhone_13_That_Randomly_Restarts'));
  }
  if (/iphone14,[78]\b/.test(t) || (/\biphone\s*14\b/i.test(raw) && !/\bpro\b/i.test(raw))) {
    out.push(link('iPhone 14 / 14 Plus', 'How_To_Fix_an_iPhone_14_That_Randomly_Restarts'));
  }
  if (/iphone15,[23]\b/.test(t) || /\biphone\s*14\s*pro\b/i.test(raw)) {
    out.push(link('iPhone 14 Pro / Max', 'How_To_Fix_an_iPhone_14_Pro_That_Randomly_Restarts'));
  }
  if (/iphone15,[45]\b/.test(t) || (/\biphone\s*15\b/i.test(raw) && !/\bpro\b/i.test(raw))) {
    out.push(link('iPhone 15 / 15 Plus', 'How_To_Fix_an_iPhone_15_That_Randomly_Restarts'));
  }
  if (/iphone16,[12]\b/.test(t) || /\biphone\s*15\s*pro\b/i.test(raw)) {
    out.push(link('iPhone 15 Pro / Max', 'How_To_Fix_an_iPhone_15_Pro_That_Randomly_Restarts'));
  }
  if (/\biphone12,8\b/i.test(deviceOrProduct) || /\bse\s*2020\b/i.test(t)) {
    out.push(link('iPhone SE 2020', 'How_To_Fix_an_iPhone_SE_2020_That_Randomly_Restarts'));
    out.push(
      link(
        'SE 2020 · mic1 / touch / reboot 3 min',
        'How_To_Fix_an_iPhone_SE_2020_with_No_Touch_and/or_3_Min_Restart_(Mic1_Problem)',
      ),
    );
  }

  const seen = new Set<string>();
  return out.filter((x) => {
    if (seen.has(x.url)) return false;
    seen.add(x.url);
    return true;
  });
}
