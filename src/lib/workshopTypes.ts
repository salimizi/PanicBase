export type WorkshopCardSeverity = 'HARDWARE' | 'SOFTWARE' | 'BOARD-LEVEL' | 'COMBINÉ';

export type WorkshopCardDraft = Omit<ReferenceWorkshopCard, 'uiKey'>;

export type ReferenceWorkshopCard = {
  id: string;
  /** Clé unique UI (plusieurs fiches peuvent partager le même `id` logique). */
  uiKey: string;
  codeBadges: string[];
  severity: WorkshopCardSeverity;
  title: string;
  subtitle: string;
  component: string;
  likelyCause: string;
  keywords: string[];
  quickTest?: string;
  steps: string[];
  note?: string;
  matchScore: number;
};
