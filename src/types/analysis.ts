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
};
