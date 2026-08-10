import { invoke } from "@tauri-apps/api/core";
import { Check, Radar, Save, X } from "lucide-react";
import { useEffect, useState } from "react";
import type { AppConfigSummary, BillingProbeResult, CostMode, SubscriptionSummary, UsageSummary } from "../types";

type Props = {
  summaries: UsageSummary[];
  petImagePath?: string;
  onSaved: () => Promise<void>;
  onClose: () => void;
};

function costFromSummary(summaries: UsageSummary[], agent: string, fallback: string): string {
  const summary = summaries.find((item) => item.agent === agent);
  if (!summary) return fallback;
  return String(summary.monthlyCostUsd);
}

export function SettingsPanel({ summaries, petImagePath, onSaved, onClose }: Props) {
  const [codexCost, setCodexCost] = useState(() => costFromSummary(summaries, "codex", "20"));
  const [claudeCost, setClaudeCost] = useState(() => costFromSummary(summaries, "claude", "100"));
  const [codexBillingDay, setCodexBillingDay] = useState("1");
  const [claudeBillingDay, setClaudeBillingDay] = useState("1");
  const [nextPetImagePath, setNextPetImagePath] = useState(petImagePath ?? "");
  const [codexMode, setCodexMode] = useState<CostMode>("subscription");
  const [claudeMode, setClaudeMode] = useState<CostMode>("subscription");
  const [detecting, setDetecting] = useState(false);
  const [detectedPeriods, setDetectedPeriods] = useState<BillingProbeResult[]>([]);
  const [saving, setSaving] = useState(false);
  const [status, setStatus] = useState<"idle" | "saved" | "error">("idle");

  useEffect(() => {
    let alive = true;
    invoke<SubscriptionSummary[]>("get_subscriptions")
      .then((subscriptions) => {
        if (!alive) return;
        const codex = subscriptions.find((item) => item.agent === "codex");
        const claude = subscriptions.find((item) => item.agent === "claude");
        if (codex) setCodexCost(codex.monthlyCostUsd);
        if (claude) setClaudeCost(claude.monthlyCostUsd);
        if (codex) setCodexBillingDay(String(codex.billingCycleDay));
        if (claude) setClaudeBillingDay(String(claude.billingCycleDay));
        if (codex) setCodexMode(codex.costMode);
        if (claude) setClaudeMode(claude.costMode);
      })
      .catch((error) => console.warn("Failed to load subscriptions", error));
    return () => {
      alive = false;
    };
  }, []);

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

  const billingDay = (value: string) => {
    const parsed = Number.parseInt(value, 10);
    if (!Number.isFinite(parsed)) return 1;
    return Math.min(28, Math.max(1, parsed));
  };

  async function detectBillingPeriods() {
    setDetecting(true);
    setDetectedPeriods([]);
    try {
      const results = await invoke<BillingProbeResult[]>("detect_billing_periods");
      setDetectedPeriods(results);
      for (const item of results) {
        if (item.confidence !== "high" || !item.billingCycleDay) continue;
        if (item.provider === "codex") setCodexBillingDay(String(item.billingCycleDay));
        if (item.provider === "claude") setClaudeBillingDay(String(item.billingCycleDay));
      }
    } catch (error) {
      setDetectedPeriods([
        {
          provider: "system",
          confidence: "low",
          source: "adapter",
          note: `自动识别失败：${String(error)}`,
        },
      ]);
    } finally {
      setDetecting(false);
    }
  }

  async function save() {
    setSaving(true);
    setStatus("idle");
    try {
      await invoke("save_subscription", {
        request: {
          agent: "codex",
          label: "Codex Custom",
          monthlyCostUsd: codexCost,
          billingCycleDay: billingDay(codexBillingDay),
          costMode: codexMode,
        },
      });
      await invoke("save_subscription", {
        request: {
          agent: "claude",
          label: "Claude Code Custom",
          monthlyCostUsd: claudeCost,
          billingCycleDay: billingDay(claudeBillingDay),
          costMode: claudeMode,
        },
      });
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

  return (
    <section className="settings-panel">
      <header>
        <div>
          <strong>回本设置</strong>
          <span>成本基准</span>
        </div>
        <button onClick={onClose} aria-label="Close settings">
          <X size={14} />
        </button>
      </header>
      <div className="settings-fields">
        <div className="settings-agent-field">
          <label>
            <span>Codex</span>
            <div className="money-input">
              <span>$</span>
              <input inputMode="decimal" value={codexCost} onChange={(event) => setCodexCost(event.target.value)} />
            </div>
          </label>
          <label>
            <span>订阅日</span>
            <input
              className="plain-input"
              inputMode="numeric"
              value={codexBillingDay}
              onChange={(event) => setCodexBillingDay(event.target.value)}
            />
          </label>
          <div className="mode-tabs" aria-label="Codex 成本模式">
            <button className={codexMode === "subscription" ? "active" : ""} onClick={() => setCodexMode("subscription")}>订阅</button>
            <button className={codexMode === "api" ? "active" : ""} onClick={() => setCodexMode("api")}>API</button>
            <button className={codexMode === "disabled" ? "active" : ""} onClick={() => setCodexMode("disabled")}>不算</button>
          </div>
        </div>
        <div className="settings-agent-field">
          <label>
            <span>Claude Code</span>
            <div className="money-input">
              <span>$</span>
              <input inputMode="decimal" value={claudeCost} onChange={(event) => setClaudeCost(event.target.value)} />
            </div>
          </label>
          <label>
            <span>订阅日</span>
            <input
              className="plain-input"
              inputMode="numeric"
              value={claudeBillingDay}
              onChange={(event) => setClaudeBillingDay(event.target.value)}
            />
          </label>
          <div className="mode-tabs" aria-label="Claude Code 成本模式">
            <button className={claudeMode === "subscription" ? "active" : ""} onClick={() => setClaudeMode("subscription")}>订阅</button>
            <button className={claudeMode === "api" ? "active" : ""} onClick={() => setClaudeMode("api")}>API</button>
            <button className={claudeMode === "disabled" ? "active" : ""} onClick={() => setClaudeMode("disabled")}>不算</button>
          </div>
        </div>
        <div className="settings-agent-field">
          <label className="wide-label">
            <span>宠物图片</span>
            <input
              className="plain-input"
              value={nextPetImagePath}
              onChange={(event) => setNextPetImagePath(event.target.value)}
              placeholder="/path/to/pet.png"
            />
          </label>
        </div>
        <div className="settings-agent-field">
          <button className="detect-button" onClick={detectBillingPeriods} disabled={detecting}>
            <Radar size={14} />
            <span>{detecting ? "识别中" : "自动识别订阅期"}</span>
          </button>
          {detectedPeriods.length > 0 ? (
            <div className="detect-results">
              {detectedPeriods.map((item) => (
                <span key={`${item.provider}-${item.source}`}>
                  {item.provider === "codex" ? "Codex" : item.provider === "claude" ? "Claude" : "系统"}：
                  {item.confidence === "high" && item.billingCycleDay
                    ? `已识别 ${item.billingCycleDay} 号`
                    : item.note}
                </span>
              ))}
            </div>
          ) : null}
        </div>
      </div>
      <button className="save-button" onClick={save} disabled={saving}>
        {status === "saved" ? <Check size={16} /> : <Save size={16} />}
        <span>{saving ? "保存中" : "保存"}</span>
      </button>
      {status === "saved" ? <span className="settings-status success">已保存</span> : null}
      {status === "error" ? <span className="settings-status error">保存失败</span> : null}
    </section>
  );
}
