import { describe, expect, it } from "vitest";
import { bestIndividualPaybackRatio } from "./paybackSelection";
import type { UsageSummary } from "../types";

function summary(agent: string, apiValueUsd: number, monthlyCostUsd: number): UsageSummary {
  return {
    agent,
    totalInputTokens: 0,
    totalOutputTokens: 0,
    totalCacheReadTokens: 0,
    totalCacheWriteTokens: 0,
    apiValueUsd,
    monthlyCostUsd,
    costMode: monthlyCostUsd > 0 ? "subscription" : "disabled",
    unpricedModels: [],
  };
}

describe("bestIndividualPaybackRatio", () => {
  it("uses each agent separately instead of dividing total value by total cost", () => {
    const ratio = bestIndividualPaybackRatio([
      summary("claude", 0, 20),
      summary("codex", 202, 20),
    ]);

    expect(ratio).toBeCloseTo(10.1);
  });

  it("ignores agents without a payback target", () => {
    const ratio = bestIndividualPaybackRatio([
      summary("claude", 90, 0),
      summary("codex", 10, 20),
    ]);

    expect(ratio).toBeCloseTo(0.5);
  });
});
