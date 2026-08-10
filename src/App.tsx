import { invoke } from "@tauri-apps/api/core";
import { RefreshCw, Settings } from "lucide-react";
import { useEffect, useState } from "react";
import { FloatingPet } from "./components/FloatingPet";
import { SettingsPanel } from "./components/SettingsPanel";
import type { AppConfigSummary, UsageSummary } from "./types";

export default function App() {
  const [summaries, setSummaries] = useState<UsageSummary[]>([]);
  const [appConfig, setAppConfig] = useState<AppConfigSummary>({});
  const [syncing, setSyncing] = useState(false);
  const [showSettings, setShowSettings] = useState(false);

  async function refresh() {
    setSyncing(true);
    try {
      await invoke("sync_now");
      const next = await invoke<UsageSummary[]>("get_usage_summary");
      const config = await invoke<AppConfigSummary>("get_app_config");
      setSummaries(next);
      setAppConfig(config);
    } finally {
      setSyncing(false);
    }
  }

  useEffect(() => {
    refresh();
  }, []);

  return (
    <main className="app-shell">
      <div className="window-actions">
        <button className="icon-button" onClick={refresh} disabled={syncing} aria-label="Refresh usage">
          <RefreshCw size={16} />
        </button>
        <button className="icon-button" onClick={() => setShowSettings(true)} aria-label="Open settings">
          <Settings size={16} />
        </button>
      </div>
      <FloatingPet
        summaries={summaries}
        petImagePath={appConfig.petImagePath}
      />
      {showSettings ? (
        <SettingsPanel
          summaries={summaries}
          petImagePath={appConfig.petImagePath}
          onSaved={refresh}
          onClose={() => setShowSettings(false)}
        />
      ) : null}
    </main>
  );
}
