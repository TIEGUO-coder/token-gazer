import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { MouseEvent } from "react";
import { formatCompactUsd, formatUsd } from "../lib/money";
import { paybackCopy } from "../lib/paybackCopy";
import { bestIndividualPaybackRatio } from "../lib/paybackSelection";
import { moodFromPaybackRatio } from "../lib/petState";
import type { UsageSummary } from "../types";
import petCat from "../assets/pet-cat.png";

type Props = {
  summaries: UsageSummary[];
  petImagePath?: string;
};

function agentName(agent: string): string {
  if (agent === "codex") return "Codex";
  if (agent === "claude") return "Claude Code";
  return agent;
}

function costLabel(summary: UsageSummary): string {
  if (summary.costMode === "disabled") return "仅统计价值";
  const value = summary.monthlyCostUsd > 0 ? `${formatUsd(summary.monthlyCostUsd)}/月` : "未设置金额";
  return summary.costMode === "api" ? `API预算 ${value}` : `目标 ${value}`;
}

export function FloatingPet({ summaries, petImagePath }: Props) {
  const ratio = bestIndividualPaybackRatio(summaries);
  const mood = moodFromPaybackRatio(ratio);
  const petImageSrc = petImagePath ? convertFileSrc(petImagePath) : petCat;
  const startDrag = async (event: MouseEvent<HTMLDivElement>) => {
    if (event.button !== 0) return;
    event.preventDefault();
    try {
      await getCurrentWindow().startDragging();
    } catch (error) {
      console.warn("Window JS drag failed, falling back to command", error);
      await invoke("start_window_drag");
    }
  };

  return (
    <section className={`pet-shell mood-${mood}`} aria-label="AI ROI pet">
      <div className="pet-body" data-tauri-drag-region onMouseDown={startDrag}>
        <img className="pet-image" data-tauri-drag-region src={petImageSrc} alt="AI ROI pet" draggable={false} />
      </div>
      <div className="pet-hover-card" role="status">
        <div className="pet-agent-list">
          {summaries.length === 0 ? (
            <span className="empty-agent-line">还没有发现本地用量</span>
          ) : (
            summaries.map((summary) => {
              const agentCopy = paybackCopy(summary.apiValueUsd, summary.monthlyCostUsd);
              const cappedAgentPercent = Math.min(100, Math.max(0, agentCopy.percent));
              return (
                <article className="pet-agent-row" key={summary.agent}>
                  <div className="pet-agent-main">
                    <strong>{agentName(summary.agent)}</strong>
                    <span>{costLabel(summary)}</span>
                  </div>
                  <div className="pet-agent-value">
                    <strong>{formatCompactUsd(summary.apiValueUsd)}</strong>
                    <span>{agentCopy.title}</span>
                  </div>
                  {summary.monthlyCostUsd > 0 ? (
                    <div className="agent-payback-track" aria-hidden="true">
                      <span style={{ width: `${cappedAgentPercent}%` }} />
                    </div>
                  ) : null}
                  <span className="agent-payback-line">{agentCopy.subtitle}</span>
                </article>
              );
            })
          )}
        </div>
      </div>
    </section>
  );
}
