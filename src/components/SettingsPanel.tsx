import { invoke } from "@tauri-apps/api/core";
import { Save, X } from "lucide-react";
import { useEffect, useState } from "react";
import type { AppConfigSummary } from "../types";

type Props = {
  petImagePath?: string;
  onPetImported: (imageDataUrl: string) => void;
  onSaved: () => Promise<void>;
  onClose: () => void;
};

export function SettingsPanel({ petImagePath, onPetImported, onSaved, onClose }: Props) {
  const [nextPetImagePath, setNextPetImagePath] = useState(petImagePath ?? "");
  const [saving, setSaving] = useState(false);
  const [status, setStatus] = useState<"idle" | "saved" | "error">("idle");

  useEffect(() => {
    let alive = true;
    invoke<AppConfigSummary>("get_app_config")
      .then((config) => {
        if (alive) setNextPetImagePath(config.petImagePath ?? "");
      })
      .catch((error) => console.warn("Failed to load app config", error));
    return () => {
      alive = false;
    };
  }, []);

  async function save() {
    setSaving(true);
    setStatus("idle");
    try {
      await invoke("save_app_config", {
        request: {
          petImagePath: nextPetImagePath.trim() || undefined,
        },
      });
      await onSaved();
      setStatus("saved");
      window.setTimeout(onClose, 900);
    } catch (error) {
      console.warn("Failed to save settings", error);
      setStatus("error");
    } finally {
      setSaving(false);
    }
  }

  function importPetPhoto(file: File | undefined) {
    if (!file) return;
    const reader = new FileReader();
    reader.addEventListener("load", () => {
      if (typeof reader.result !== "string") return;
      onPetImported(reader.result);
      setStatus("saved");
      window.setTimeout(onClose, 500);
    });
    reader.readAsDataURL(file);
  }

  return (
    <section className="settings-panel">
      <header>
        <div>
          <strong>Pet setup</strong>
          <span>Import your own pet</span>
        </div>
        <button onClick={onClose} aria-label="Close settings">
          <X size={14} />
        </button>
      </header>
      <div className="settings-fields">
        <div className="settings-agent-field">
          <label className="import-pet-button">
            <input
              type="file"
              accept="image/png,image/jpeg,image/webp"
              onChange={(event) => importPetPhoto(event.target.files?.[0])}
            />
            <span>Import pet photo</span>
          </label>
          <p className="settings-hint">Product flow: import a real pet photo, generate a desktop-pet style avatar, then use it as the scout.</p>
        </div>
        <div className="settings-agent-field advanced-path-field">
          <label className="wide-label">
            <span>Advanced: local image path</span>
            <input
              className="plain-input"
              value={nextPetImagePath}
              onChange={(event) => setNextPetImagePath(event.target.value)}
              placeholder="/path/to/pet.png"
            />
          </label>
          <p className="settings-hint">Use this only in the desktop app when the image path will stay fixed.</p>
        </div>
      </div>
      <button className="save-button" onClick={save} disabled={saving}>
        {status === "saved" ? <CheckIcon /> : <Save size={14} />}
        <span>{saving ? "Saving" : status === "saved" ? "Saved" : "Save pet"}</span>
      </button>
      {status === "error" ? <span className="settings-status error">Save failed</span> : null}
    </section>
  );
}

function CheckIcon() {
  return <span aria-hidden="true">✓</span>;
}
