import { invoke } from "@tauri-apps/api/core";
import { RefreshCw, Settings } from "lucide-react";
import { useEffect, useState } from "react";
import { FloatingPet } from "./components/FloatingPet";
import { PlanPanel } from "./components/PlanPanel";
import { SettingsPanel } from "./components/SettingsPanel";
import type { AppConfigSummary } from "./types";

const importedPetStorageKey = "profit-pet.importedPetImage";

export default function App() {
  const [appConfig, setAppConfig] = useState<AppConfigSummary>({});
  const [importedPetImage, setImportedPetImage] = useState<string>();
  const [syncing, setSyncing] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [showPlan, setShowPlan] = useState(false);

  async function refresh() {
    setSyncing(true);
    try {
      const config = await invoke<AppConfigSummary>("get_app_config");
      setAppConfig(config);
    } catch (error) {
      console.warn("Failed to load app config", error);
    } finally {
      setSyncing(false);
    }
  }

  useEffect(() => {
    refresh();
    setImportedPetImage(window.localStorage.getItem(importedPetStorageKey) ?? undefined);
  }, []);

  function saveImportedPetImage(imageDataUrl: string) {
    window.localStorage.setItem(importedPetStorageKey, imageDataUrl);
    setImportedPetImage(imageDataUrl);
  }

  return (
    <main className="app-shell">
      <div className="window-actions">
        <button className="icon-button" onClick={refresh} disabled={syncing} aria-label="Refresh pet">
          <RefreshCw size={16} />
        </button>
        <button className="icon-button" onClick={() => setShowSettings(true)} aria-label="Open settings">
          <Settings size={16} />
        </button>
      </div>
      <FloatingPet petImagePath={appConfig.petImagePath} importedPetImage={importedPetImage} onOpenPlan={() => setShowPlan(true)} />
      {showPlan ? <PlanPanel onClose={() => setShowPlan(false)} /> : null}
      {showSettings ? (
        <SettingsPanel
          petImagePath={appConfig.petImagePath}
          onPetImported={saveImportedPetImage}
          onSaved={refresh}
          onClose={() => setShowSettings(false)}
        />
      ) : null}
    </main>
  );
}
