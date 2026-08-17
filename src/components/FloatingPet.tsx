import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { CheckCircle2, ListChecks, Search, Sparkles } from "lucide-react";
import type { MouseEvent } from "react";
import type { OpportunityDemo } from "../lib/opportunityDemo";
import tieguo from "../assets/tieguo-desktop-pet.png";

type Props = {
  opportunity: OpportunityDemo;
  petImagePath?: string;
  importedPetImage?: string;
  isActionable: boolean;
  onConfirmActionable: () => void;
  onRejectOpportunity: () => void;
  onOpenPlan: () => void;
};

export function FloatingPet({
  opportunity,
  petImagePath,
  importedPetImage,
  isActionable,
  onConfirmActionable,
  onRejectOpportunity,
  onOpenPlan,
}: Props) {
  const mood = "ready";
  const petImageSrc = importedPetImage ?? (petImagePath ? convertFileSrc(petImagePath) : tieguo);
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
    <section className={`pet-shell mood-${mood}`} aria-label="Profit Pet">
      <div className={`found-signal ${isActionable ? "is-confirmed" : ""}`} role="status">
        {isActionable ? <CheckCircle2 size={14} /> : <Sparkles size={14} />}
        <span>{isActionable ? opportunity.confirmedStatus : opportunity.foundSignal}</span>
      </div>
      <div className="pet-body" data-tauri-drag-region onMouseDown={startDrag}>
        <img className="pet-image" data-tauri-drag-region src={petImageSrc} alt="Profit Pet" draggable={false} />
      </div>
      <div className="pet-hover-card" role="status">
        <article className="opportunity-card">
          <span className="opportunity-kicker">Your pet found a lead</span>
          <strong>{opportunity.title}</strong>
          <p>{opportunity.petLine}</p>
          <div className={`route-map-pill ${isActionable ? "is-confirmed" : ""}`}>
            {isActionable ? opportunity.confirmedStatus : opportunity.routeMapStatus}
          </div>
          <button className="confirm-action-button" onClick={onConfirmActionable}>
            <CheckCircle2 size={13} />
            <span>{isActionable ? "Approved to act" : "This is actionable"}</span>
          </button>
          <button className="scout-again-button" onClick={onRejectOpportunity}>
            <Search size={13} />
            <span>Not this one. Keep scouting.</span>
          </button>
          <button className="plan-open-button" onClick={onOpenPlan}>
            <ListChecks size={13} />
            <span>{isActionable ? "Send to Grill-me" : "Review the plan"}</span>
          </button>
        </article>
      </div>
    </section>
  );
}
