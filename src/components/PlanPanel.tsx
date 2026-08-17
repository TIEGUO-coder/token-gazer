import { ArrowRight, CalendarClock, X } from "lucide-react";
import { opportunityDemo } from "../lib/opportunityDemo";

type Props = {
  onClose: () => void;
};

export function PlanPanel({ onClose }: Props) {
  return (
    <section className="plan-panel" aria-label="Profit Pet plan">
      <header>
        <div>
          <span>Profit Pet plan</span>
          <strong>{opportunityDemo.title}</strong>
        </div>
        <button onClick={onClose} aria-label="Close plan">
          <X size={14} />
        </button>
      </header>

      <div className="plan-steps">
        {opportunityDemo.plan.map((step, index) => (
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
          <strong>MAH 接入位</strong>
          <p>{opportunityDemo.nextAction}</p>
        </div>
        <ArrowRight size={14} />
      </div>
    </section>
  );
}

