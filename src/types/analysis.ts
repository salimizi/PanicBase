export type PossibleCauseDiag = {
  name: string;
  confidence: number;
};

export type StructuredDiagnostic = {
  device: string;
  /** Nom commercial local quand ProductType est connu (ex. iPhone 14) */
  marketing_name: string | null;
  panic_type: string;
  normalized_signatures: string[];
  possible_causes: PossibleCauseDiag[];
  confidence_global: number;
  repair_priority: string;
  recommended_checks: string[];
  /** Lignes critiques extraites (SMC arrays, OUTBOX, etc.) */
  critical_lines: string[];
  /** Indices masques / extraction (export outil) */
  wiki_hints: string[];
  /** Plan d’action priorisé pour technicien atelier */
  action_plan?: string[];
  /** Alertes pièges / risques de mauvais diagnostic */
  danger_flags?: string[];
  /** Séquence d’isolation rapide avant micro-soudure lourde */
  isolation_sequence?: string[];
  /** Pièces ou zones à préparer sur l’établi */
  likely_parts?: string[];
  /** Preuves courtes qui justifient le diagnostic */
  evidence_markers?: string[];
  /** Résumé atelier exploitable directement */
  technician_summary?: string;
  /** Pourquoi le score est fort/moyen/faible */
  confidence_rationale?: string;
  /** Test unique le plus rentable à faire maintenant */
  next_best_test?: string;
};

export type PanicReferenceFocus = {
  navSection: string;
  confidence: number;
  initialSearch: string;
};

export type AnalysisResult = {
  device_model: string;
  detected: boolean;
  panic_type: string;
  probable_cause: string;
  confidence: number;
  keywords: string[];
  explanation: string;
  signature: string;
  signature_hash: string;
  structured_diagnostic: StructuredDiagnostic;
  /** Calculé par le moteur Rust (`analyze_panic_log`). */
  reference_focus?: PanicReferenceFocus;
};
