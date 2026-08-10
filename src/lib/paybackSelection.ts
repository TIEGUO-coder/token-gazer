import type { UsageSummary } from "../types";

export function bestIndividualPaybackRatio(summaries: UsageSummary[]): number {
  return summaries.reduce((best, summary) => {
    if (summary.monthlyCostUsd <= 0) return best;
    return Math.max(best, summary.apiValueUsd / summary.monthlyCostUsd);
  }, 0);
}
