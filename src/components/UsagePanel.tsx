import { formatCompactUsd, formatTokenCount, formatUsd } from "../lib/money";
import type { UsageSummary } from "../types";

type Props = {
  summaries: UsageSummary[];
};

export function UsagePanel({ summaries }: Props) {
  if (summaries.length === 0) {
    return (
      <section className="usage-panel empty-usage">
        <span>还没有发现 Claude Code 或 Codex 的本地用量。</span>
      </section>
    );
  }

  return (
    <section className="usage-panel">
      {summaries.map((summary) => (
        <article className="usage-row" key={summary.agent}>
          <div className="usage-agent">
            <strong>{summary.agent === "codex" ? "Codex" : "Claude Code"}</strong>
            <span>{summary.monthlyCostUsd > 0 ? `订阅 ${formatUsd(summary.monthlyCostUsd)}/月` : "未设置订阅成本"}</span>
          </div>
          <div className="usage-value">
            <strong>{formatCompactUsd(summary.apiValueUsd)}</strong>
            <span>API 等价价值</span>
          </div>
          <div className="usage-tokens">
            <span>输入 {formatTokenCount(summary.totalInputTokens)}</span>
            <span>输出 {formatTokenCount(summary.totalOutputTokens)}</span>
            {summary.totalCacheReadTokens > 0 ? <span>缓存命中 {formatTokenCount(summary.totalCacheReadTokens)}</span> : null}
          </div>
          {summary.unpricedModels.length > 0 ? (
            <span className="usage-warning">有未计价模型</span>
          ) : null}
        </article>
      ))}
    </section>
  );
}
