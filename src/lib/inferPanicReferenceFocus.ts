/**
 * Section référence atelier — logique canonique côté Rust (`reference_focus.rs`).
 * Ce module expose un cache invoke + repli sur `analysis.reference_focus`.
 */
import { invoke } from '@tauri-apps/api/core';
import type { AnalysisResult } from '../types/analysis';

export type PanicReferenceFocus = {
  navSection: string;
  confidence: number;
  initialSearch: string;
};

type RustFocus = {
  navSection: string;
  confidence: number;
  initialSearch: string;
};

const focusCache = new Map<string, PanicReferenceFocus>();

function cacheKey(panicText: string, analysis: AnalysisResult, productType: string | null): string {
  return [
    productType ?? '',
    analysis.signature_hash ?? '',
    panicText.length,
    panicText.slice(0, 96),
  ].join('|');
}

function fromRust(r: RustFocus): PanicReferenceFocus {
  return {
    navSection: r.navSection,
    confidence: r.confidence,
    initialSearch: r.initialSearch,
  };
}

const DEFAULT_FOCUS: PanicReferenceFocus = {
  navSection: 'iphone-x',
  confidence: 0,
  initialSearch: '',
};

/** Repli immédiat (sync) : focus déjà calculé par `analyze_panic_log` ou défaut. */
function syncFallback(analysis: AnalysisResult | null | undefined): PanicReferenceFocus {
  const rf = analysis?.reference_focus;
  if (rf?.navSection) {
    return {
      navSection: rf.navSection,
      confidence: rf.confidence ?? 0,
      initialSearch: rf.initialSearch ?? '',
    };
  }
  return DEFAULT_FOCUS;
}

/**
 * Inférence synchrone : utilise le focus embarqué dans l’analyse ou le cache invoke.
 * Lance un rafraîchissement Rust en arrière-plan si `productType` est fourni.
 */
export function inferPanicReferenceFocus({
  panicText,
  analysis,
  productType,
}: {
  panicText: string;
  analysis: AnalysisResult;
  productType: string | null;
}): PanicReferenceFocus {
  const key = cacheKey(panicText, analysis, productType);
  const hit = focusCache.get(key);
  if (hit) return hit;

  const fallback = syncFallback(analysis);

  if (productType?.trim()) {
    void invoke<RustFocus>('infer_panic_reference_focus', {
      panicText,
      analysis,
      productType: productType.trim(),
    })
      .then((r) => {
        focusCache.set(key, fromRust(r));
      })
      .catch(() => {
        /* WebView / dev : garder le repli */
      });
  }

  return fallback;
}

/** Token recherche panneau référence — délégué au blob (même heuristiques que Rust). */
export function inferSearchQueryFromPanicBlob(blobLower: string): string {
  const sensorMatch = blobLower.match(/missing sensor\(s?\)?:?\s*([a-z0-9,\s]+)/i);
  if (sensorMatch) {
    const sensors = sensorMatch[1]
      .split(/[,\s]+/)
      .map((s) => s.trim())
      .filter((s) => s.length >= 2 && s.length <= 10);
    if (sensors.length) return sensors[0].toUpperCase();
  }
  const maskMatch = blobLower.match(/s\.sensor\s+array[^\n]*?(?:is\s+)?(?:0x[0-9a-f]+|\d{4,})/i);
  if (maskMatch) {
    const hexMatch = maskMatch[0].match(/0x[0-9a-f]+/i);
    if (hexMatch) return hexMatch[0].toUpperCase();
    const decMatch = maskMatch[0].match(/\d{4,}/);
    if (decMatch) return `0x${parseInt(decMatch[0], 10).toString(16).toUpperCase()}`;
  }
  if (blobLower.includes('thermalmonitord')) return 'thermalmonitord';
  if (blobLower.includes('smc panic')) return 'SMC PANIC';
  if (blobLower.includes('ans2')) return 'ANS2';
  if (blobLower.includes('sep panic')) return 'SEP';
  if (blobLower.includes('baseband')) return 'Baseband';
  if (blobLower.includes('applesochot')) return 'AppleSocHot';
  if (blobLower.includes('aop nmi')) return 'AOP NMI';
  return '';
}
