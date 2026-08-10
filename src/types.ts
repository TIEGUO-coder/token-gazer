export type UsageSummary = {
  agent: string;
  totalInputTokens: number;
  totalOutputTokens: number;
  totalCacheReadTokens: number;
  totalCacheWriteTokens: number;
  apiValueUsd: number;
  monthlyCostUsd: number;
  costMode: CostMode;
  unpricedModels: string[];
};

export type CostMode = "subscription" | "api" | "disabled";

export type SubscriptionSummary = {
  agent: string;
  label: string;
  monthlyCostUsd: string;
  billingCycleDay: number;
  costMode: CostMode;
};

export type AppConfigSummary = {
  petImagePath?: string;
};

export type BillingProbeResult = {
  provider: "codex" | "claude" | string;
  billingCycleDay?: number;
  periodStart?: number;
  periodEnd?: number;
  planName?: string;
  confidence: "high" | "medium" | "low" | string;
  source: string;
  note: string;
};
