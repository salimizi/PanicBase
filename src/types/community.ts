export type CommunityRepairBucket = {
  label: string;
  count: number;
  percent: number;
};

export type CommunityStats = {
  available: boolean;
  similar_count: number;
  message: string;
  buckets: CommunityRepairBucket[];
};
