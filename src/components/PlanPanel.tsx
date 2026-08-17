import { ArrowRight, CalendarClock, X } from "lucide-react";
import type { OpportunityDemo } from "../lib/opportunityDemo";

type Props = {
  opportunity: OpportunityDemo;
  isActionable: boolean;
  onClose: () => void;
};

export function PlanPanel({ opportunity, isActionable, onClose }: Props) {
  return (
    <section className="plan-panel" aria-label="Profit Pet plan">
      <header>
        <div>
          <span>Profit Pet plan</span>
          <strong>{opportunity.title}</strong>
        </div>
        <button onClick={onClose} aria-label="Close plan">
          <X size={14} />
        </button>
      </header>

      <div className="plan-steps">
        {opportunity.plan.map((step, index) => (
          <article className="plan-step" key={step.title}>
            <span>{index + 1}</span>
            <div>
              <strong>{step.title}</strong>
              <p>{step.detail}</p>
            </div>
          </article>
        ))}
      </div>

      <div className="mah-handoff">
        <CalendarClock size={14} />
        <div>
          <strong>{isActionable ? "Ready for the next step" : "Waiting for approval"}</strong>
          <p>{isActionable ? opportunity.nextAction : "Approve the opportunity first, then send it to Grill-me for requirement breakdown."}</p>
        </div>
        <ArrowRight size={14} />
      </div>
    </section>
  );
}
