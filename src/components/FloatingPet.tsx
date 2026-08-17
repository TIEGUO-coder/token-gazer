import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { ListChecks } from "lucide-react";
import type { MouseEvent } from "react";
import { opportunityDemo } from "../lib/opportunityDemo";
import tieguo from "../assets/tieguo-desktop-pet.png";

type Props = {
  petImagePath?: string;
  importedPetImage?: string;
  onOpenPlan: () => void;
};

export function FloatingPet({ petImagePath, importedPetImage, onOpenPlan }: Props) {
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
      <div className="pet-body" data-tauri-drag-region onMouseDown={startDrag}>
        <img className="pet-image" data-tauri-drag-region src={petImageSrc} alt="Profit Pet" draggable={false} />
      </div>
      <div className="pet-hover-card" role="status">
        <article className="opportunity-card">
          <span className="opportunity-kicker">铁锅发现机会</span>
          <strong>{opportunityDemo.title}</strong>
          <p>{opportunityDemo.petLine}</p>
          <div className="route-map-pill">{opportunityDemo.routeMapStatus}</div>
          <button className="plan-open-button" onClick={onOpenPlan}>
            <ListChecks size={13} />
            <span>查看计划</span>
          </button>
        </article>
      </div>
    </section>
  );
}
