# AI ROI Pet Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a desktop floating pet that tracks Codex and Claude Code usage, converts local token usage into API-equivalent USD value, and shows both subscription payback and provider-reported quota pressure when available.

**Architecture:** The app is a local-first Tauri desktop application with two intentionally separate data planes. The value plane imports local session logs from `~/.claude/projects/**/*.jsonl` and `~/.codex/sessions/**/*.jsonl`, then calculates API-equivalent USD value from editable model prices. The quota plane reads provider-reported quota and credit snapshots where a supported local CLI exposes them, starting with Codex `app-server`; quota is displayed as pressure and reset context, never used to calculate payback.

**Tech Stack:** Tauri 2, Rust, SQLite via `rusqlite`, React, TypeScript, Vite, CSS modules or plain CSS, Vitest, Rust unit tests.

---

## Product Scope

The MVP answers four questions:

- Which supported agent did the user use this month?
- How many input, output, cache-read, and cache-write tokens were recorded?
- What is the API-equivalent value of that usage in USD?
- Has that value exceeded the monthly subscription cost configured by the user?
- For Codex, what official rate-limit windows, credits, or individual limits are currently reported by the installed Codex CLI?

The MVP supports:

- Claude Code log import from `~/.claude/projects`.
- Codex log import from `~/.codex/sessions` and `~/.codex/archived_sessions`.
- Codex quota import through `codex -s read-only -a untrusted app-server` JSON-RPC when the Codex CLI is installed and authenticated.
- Editable subscription prices for each agent.
- Editable model price table.
- Unknown model handling: token counts are retained and money is shown as unpriced until a price is configured.
- Source-aware display: API-equivalent value is labeled as estimated from local logs; Codex quota is labeled as provider-reported when available.
- Floating always-on-top pet window.
- Expanded detail window for settings and usage summaries.

The MVP intentionally does not support:

- Auto-detecting the user's paid subscription plan from OpenAI or Anthropic accounts.
- Scraping account billing pages.
- Reusing browser cookies or private auth tokens to call unpublished dashboard endpoints.
- Proxy-based request interception.
- Inferring Codex quota usage percentage from local token logs.
- Team billing, seat allocation, taxes, exchange rates, or reimbursements.

---

## Data Semantics

### Claude Code

Read JSONL files under:

```text
~/.claude/projects/<project>/*.jsonl
~/.claude/projects/<project>/<session>/subagents/*.jsonl
~/.claude/projects/<project>/<session>/subagents/workflows/wf_*/*.jsonl
```

Only import lines where:

```json
{
  "type": "assistant",
  "message": {
    "id": "msg_...",
    "model": "claude-...",
    "usage": {
      "input_tokens": 100,
      "output_tokens": 20,
      "cache_read_input_tokens": 50,
      "cache_creation_input_tokens": 10
    }
  }
}
```

Claude input token semantics:

- `input_tokens` is treated as fresh input.
- `cache_read_input_tokens` is priced separately.
- `cache_creation_input_tokens` is priced separately.

### Codex

Read JSONL files under:

```text
~/.codex/sessions/YYYY/MM/DD/*.jsonl
~/.codex/archived_sessions/*.jsonl
```

Only import `event_msg` lines whose payload type is `token_count`:

```json
{
  "type": "event_msg",
  "timestamp": "2026-08-06T08:00:00Z",
  "payload": {
    "type": "token_count",
    "info": {
      "model": "gpt-5.4",
      "total_token_usage": {
        "input_tokens": 1000,
        "cached_input_tokens": 600,
        "output_tokens": 50
      },
      "last_token_usage": {
        "input_tokens": 200,
        "cached_input_tokens": 120,
        "output_tokens": 10
      }
    }
  }
}
```

Codex input token semantics:

- Prefer `last_token_usage` when present because it is per-request.
- If only `total_token_usage` exists, compute delta against the previous high-water snapshot in the same file.
- `cached_input_tokens` is treated as cache read.
- Codex session logs do not expose cache-write tokens in this MVP, so `cache_write_tokens = 0`.
- For cost calculation, Codex `input_tokens` is treated as total input and cache-read tokens are subtracted before applying fresh input price.

### Codex Quota and Credits

CodexBar shows that Codex quota is a different source from local session token totals. For this app, quota display must come from a provider-reported snapshot when available.

Start a local Codex app server:

```bash
codex -s read-only -a untrusted app-server
```

Send newline-delimited JSON-RPC messages over stdin/stdout:

```json
{"id":1,"method":"initialize","params":{"clientInfo":{"name":"ai-roi-pet","version":"0.1.0"}}}
{"method":"initialized","params":{}}
{"id":2,"method":"account/rateLimits/read","params":{}}
{"id":3,"method":"account/read","params":{}}
```

Expected useful fields include:

- `rateLimits.primary.usedPercent`, `windowDurationMins`, and `resetsAt`.
- `rateLimits.secondary.usedPercent`, `windowDurationMins`, and `resetsAt`.
- `credits.balance`.
- `individualLimit.limit`, `used`, `remainingPercent`, and `resetsAt`.
- `planType`.
- `rateLimitsByLimitId` for additional named windows.

Quota semantics:

- Quota snapshots are used only for display, alerts, and pet excitement.
- API-equivalent payback is calculated only from local token usage and editable model prices.
- If the Codex CLI is missing, unauthenticated, or does not expose these methods, the UI shows `quota unavailable` while keeping token import and payback calculation functional.
- Hidden ChatGPT dashboard usage endpoints are deferred to a follow-up because they require browser session state and explicit consent.

---

## File Structure

Create the app from an empty directory:

```text
package.json
pnpm-lock.yaml
index.html
vite.config.ts
tsconfig.json
src/
  App.tsx
  main.tsx
  app.css
  types.ts
  lib/
    money.ts
    petState.ts
  components/
    FloatingPet.tsx
    UsagePanel.tsx
    SettingsPanel.tsx
src-tauri/
  Cargo.toml
  tauri.conf.json
  src/
    main.rs
    commands.rs
    db.rs
    usage/
      mod.rs
      models.rs
      cost.rs
      claude.rs
      codex.rs
      sync.rs
    quota/
      mod.rs
      models.rs
      codex_app_server.rs
    settings.rs
    window.rs
  tests/
    fixtures/
      claude-simple.jsonl
      codex-simple.jsonl
```

Responsibilities:

- `usage/models.rs`: shared Rust data structures for usage events, pricing, subscriptions, and summaries.
- `usage/claude.rs`: parse Claude Code JSONL files.
- `usage/codex.rs`: parse Codex JSONL files and compute deltas.
- `usage/cost.rs`: convert token usage to USD.
- `usage/sync.rs`: scan filesystem and write parsed usage events to SQLite.
- `quota/models.rs`: shared Rust data structures for provider-reported quota and credit snapshots.
- `quota/codex_app_server.rs`: query Codex CLI `app-server` through JSON-RPC and normalize quota fields.
- `db.rs`: schema creation, migrations, inserts, summary queries.
- `settings.rs`: load and save subscriptions, model pricing, and configured log roots.
- `commands.rs`: Tauri commands called by React.
- `window.rs`: always-on-top and click-through behavior helpers.
- React components: floating pet, summary dashboard, and editable settings.

---

## Database Schema

Use SQLite with these tables:

```sql
CREATE TABLE IF NOT EXISTS usage_events (
  id TEXT PRIMARY KEY,
  agent TEXT NOT NULL,
  source TEXT NOT NULL,
  session_id TEXT,
  model TEXT NOT NULL,
  input_tokens INTEGER NOT NULL,
  output_tokens INTEGER NOT NULL,
  cache_read_tokens INTEGER NOT NULL,
  cache_write_tokens INTEGER NOT NULL,
  input_token_semantics TEXT NOT NULL,
  occurred_at INTEGER NOT NULL,
  created_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS model_prices (
  model TEXT PRIMARY KEY,
  input_usd_per_million TEXT NOT NULL,
  output_usd_per_million TEXT NOT NULL,
  cache_read_usd_per_million TEXT NOT NULL,
  cache_write_usd_per_million TEXT NOT NULL,
  source TEXT NOT NULL,
  effective_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS subscriptions (
  agent TEXT PRIMARY KEY,
  label TEXT NOT NULL,
  monthly_cost_usd TEXT NOT NULL,
  billing_cycle_day INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS sync_cursors (
  file_path TEXT PRIMARY KEY,
  modified_nanos INTEGER NOT NULL,
  line_offset INTEGER NOT NULL,
  synced_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS quota_snapshots (
  id TEXT PRIMARY KEY,
  agent TEXT NOT NULL,
  source TEXT NOT NULL,
  primary_used_percent REAL,
  primary_window_minutes INTEGER,
  primary_resets_at INTEGER,
  secondary_used_percent REAL,
  secondary_window_minutes INTEGER,
  secondary_resets_at INTEGER,
  credit_balance TEXT,
  credit_limit TEXT,
  credit_used TEXT,
  credit_remaining_percent REAL,
  credit_resets_at INTEGER,
  plan_type TEXT,
  updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS quota_extra_windows (
  id TEXT PRIMARY KEY,
  snapshot_id TEXT NOT NULL,
  title TEXT NOT NULL,
  used_percent REAL NOT NULL,
  window_minutes INTEGER,
  resets_at INTEGER,
  FOREIGN KEY(snapshot_id) REFERENCES quota_snapshots(id)
);
```

`usage_events.id` must be deterministic:

- Claude: `claude:session:<message.id>`
- Codex: `codex:session:<file-stem>:<nonzero-token-event-index>`

This makes repeated scans idempotent.

`quota_snapshots` keeps the latest provider-reported quota snapshot per agent/source in the MVP. Historical quota trends can be added later by retaining older snapshot IDs instead of replacing the latest row.

---

## Task 1: Scaffold Tauri App

**Files:**
- Create: `package.json`
- Create: `index.html`
- Create: `vite.config.ts`
- Create: `tsconfig.json`
- Create: `src/main.tsx`
- Create: `src/App.tsx`
- Create: `src/app.css`
- Create: `src-tauri/Cargo.toml`
- Create: `src-tauri/tauri.conf.json`
- Create: `src-tauri/src/main.rs`

- [ ] **Step 1: Create the frontend package files**

Create `package.json`:

```json
{
  "scripts": {
    "dev": "vite --host 127.0.0.1",
    "build": "tsc && vite build",
    "test": "vitest run",
    "tauri": "tauri"
  },
  "dependencies": {
    "@tauri-apps/api": "^2.0.0",
    "lucide-react": "^0.475.0",
    "react": "^19.0.0",
    "react-dom": "^19.0.0"
  },
  "devDependencies": {
    "@tauri-apps/cli": "^2.0.0",
    "@types/react": "^19.0.0",
    "@types/react-dom": "^19.0.0",
    "@vitejs/plugin-react": "^4.3.4",
    "typescript": "^5.7.0",
    "vite": "^6.0.0",
    "vitest": "^2.1.8"
  }
}
```

Create `index.html`:

```html
<div id="root"></div>
<script type="module" src="/src/main.tsx"></script>
```

Create `vite.config.ts`:

```ts
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
  },
});
```

Create `tsconfig.json`:

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "useDefineForClassFields": true,
    "lib": ["DOM", "DOM.Iterable", "ES2022"],
    "allowJs": false,
    "skipLibCheck": true,
    "esModuleInterop": true,
    "allowSyntheticDefaultImports": true,
    "strict": true,
    "forceConsistentCasingInFileNames": true,
    "module": "ESNext",
    "moduleResolution": "Node",
    "resolveJsonModule": true,
    "isolatedModules": true,
    "noEmit": true,
    "jsx": "react-jsx"
  },
  "include": ["src"]
}
```

- [ ] **Step 2: Create the first React screen**

Create `src/main.tsx`:

```tsx
import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./app.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
```

Create `src/App.tsx`:

```tsx
export default function App() {
  return (
    <main className="app-shell">
      <section className="pet-shell">
        <div className="pet-face" aria-label="AI ROI pet">
          <span className="pet-eye" />
          <span className="pet-eye" />
          <span className="pet-mouth" />
        </div>
        <div className="pet-stats">
          <strong>$0.00</strong>
          <span>0% 回本</span>
        </div>
      </section>
    </main>
  );
}
```

Create `src/app.css`:

```css
html,
body,
#root {
  width: 100%;
  height: 100%;
  margin: 0;
  background: transparent;
  font-family: Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
}

.app-shell {
  width: 100%;
  height: 100%;
  display: grid;
  place-items: center;
}

.pet-shell {
  width: 220px;
  min-height: 128px;
  border: 1px solid rgba(28, 32, 38, 0.16);
  border-radius: 8px;
  background: rgba(255, 255, 255, 0.92);
  box-shadow: 0 18px 50px rgba(15, 23, 42, 0.18);
  display: grid;
  grid-template-columns: 96px 1fr;
  align-items: center;
  padding: 14px;
  gap: 12px;
}

.pet-face {
  width: 88px;
  height: 88px;
  border-radius: 50%;
  background: #2fbf71;
  position: relative;
}

.pet-eye {
  position: absolute;
  top: 30px;
  width: 10px;
  height: 10px;
  border-radius: 50%;
  background: #101820;
}

.pet-eye:first-child {
  left: 25px;
}

.pet-eye:nth-child(2) {
  right: 25px;
}

.pet-mouth {
  position: absolute;
  left: 34px;
  bottom: 24px;
  width: 20px;
  height: 10px;
  border-bottom: 3px solid #101820;
  border-radius: 0 0 20px 20px;
}

.pet-stats {
  display: grid;
  gap: 6px;
  color: #101820;
}

.pet-stats strong {
  font-size: 26px;
  line-height: 1;
}

.pet-stats span {
  font-size: 14px;
  color: #52606d;
}
```

- [ ] **Step 3: Create the Rust Tauri shell**

Create `src-tauri/Cargo.toml`:

```toml
[package]
name = "ai-roi-pet"
version = "0.1.0"
edition = "2021"

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
chrono = { version = "0.4", features = ["serde"] }
dirs = "6"
rusqlite = { version = "0.32", features = ["bundled"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
sha2 = "0.10"
tauri = { version = "2", features = [] }
thiserror = "2"
uuid = { version = "1", features = ["v4"] }
```

Create `src-tauri/tauri.conf.json`:

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "AI ROI Pet",
  "version": "0.1.0",
  "identifier": "dev.local.ai-roi-pet",
  "build": {
    "beforeDevCommand": "pnpm dev",
    "devUrl": "http://127.0.0.1:1420",
    "beforeBuildCommand": "pnpm build",
    "frontendDist": "../dist"
  },
  "app": {
    "windows": [
      {
        "title": "AI ROI Pet",
        "width": 260,
        "height": 160,
        "decorations": false,
        "transparent": true,
        "alwaysOnTop": true,
        "resizable": false,
        "skipTaskbar": true
      }
    ],
    "security": {
      "csp": null
    }
  }
}
```

Create `src-tauri/src/main.rs`:

```rust
fn main() {
    tauri::Builder::default()
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_always_on_top(true);
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to run AI ROI Pet");
}
```

- [ ] **Step 4: Verify scaffold**

Run:

```bash
pnpm install
pnpm build
pnpm tauri dev
```

Expected:

- `pnpm build` exits with code 0.
- `pnpm tauri dev` opens a transparent always-on-top pet window.

- [ ] **Step 5: Commit**

```bash
git add package.json pnpm-lock.yaml index.html vite.config.ts tsconfig.json src src-tauri
git commit -m "chore: scaffold ai roi pet"
```

---

## Task 2: Define Usage Domain and Cost Calculator

**Files:**
- Create: `src-tauri/src/usage/mod.rs`
- Create: `src-tauri/src/usage/models.rs`
- Create: `src-tauri/src/usage/cost.rs`
- Modify: `src-tauri/src/main.rs`

- [ ] **Step 1: Add usage module**

Create `src-tauri/src/usage/mod.rs`:

```rust
pub mod cost;
pub mod models;
```

Modify `src-tauri/src/main.rs`:

```rust
mod usage;

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_always_on_top(true);
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to run AI ROI Pet");
}
```

- [ ] **Step 2: Define core models**

Create `src-tauri/src/usage/models.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Agent {
    Claude,
    Codex,
}

impl Agent {
    pub fn as_str(&self) -> &'static str {
        match self {
            Agent::Claude => "claude",
            Agent::Codex => "codex",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputTokenSemantics {
    Fresh,
    TotalIncludesCache,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsageEvent {
    pub id: String,
    pub agent: Agent,
    pub source: String,
    pub session_id: Option<String>,
    pub model: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_write_tokens: u64,
    pub input_token_semantics: InputTokenSemantics,
    pub occurred_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelPrice {
    pub model: String,
    pub input_usd_per_million: String,
    pub output_usd_per_million: String,
    pub cache_read_usd_per_million: String,
    pub cache_write_usd_per_million: String,
    pub source: String,
    pub effective_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CostBreakdown {
    pub input_cost_micros: i64,
    pub output_cost_micros: i64,
    pub cache_read_cost_micros: i64,
    pub cache_write_cost_micros: i64,
    pub total_cost_micros: i64,
    pub priced: bool,
}
```

- [ ] **Step 3: Write cost calculator tests**

Create `src-tauri/src/usage/cost.rs`:

```rust
use super::models::{CostBreakdown, InputTokenSemantics, ModelPrice, UsageEvent};

fn parse_price_to_micros_per_million(value: &str) -> Option<i64> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    let parsed: f64 = trimmed.parse().ok()?;
    Some((parsed * 1_000_000.0).round() as i64)
}

fn cost_micros(tokens: u64, usd_per_million: &str) -> Option<i64> {
    let price_micros = parse_price_to_micros_per_million(usd_per_million)?;
    Some(((tokens as i128 * price_micros as i128) / 1_000_000) as i64)
}

pub fn calculate_cost(event: &UsageEvent, price: Option<&ModelPrice>) -> CostBreakdown {
    let Some(price) = price else {
        return CostBreakdown {
            input_cost_micros: 0,
            output_cost_micros: 0,
            cache_read_cost_micros: 0,
            cache_write_cost_micros: 0,
            total_cost_micros: 0,
            priced: false,
        };
    };

    let billable_input = match event.input_token_semantics {
        InputTokenSemantics::Fresh => event.input_tokens,
        InputTokenSemantics::TotalIncludesCache => event
            .input_tokens
            .saturating_sub(event.cache_read_tokens)
            .saturating_sub(event.cache_write_tokens),
    };

    let input = cost_micros(billable_input, &price.input_usd_per_million).unwrap_or(0);
    let output = cost_micros(event.output_tokens, &price.output_usd_per_million).unwrap_or(0);
    let cache_read =
        cost_micros(event.cache_read_tokens, &price.cache_read_usd_per_million).unwrap_or(0);
    let cache_write =
        cost_micros(event.cache_write_tokens, &price.cache_write_usd_per_million).unwrap_or(0);

    CostBreakdown {
        input_cost_micros: input,
        output_cost_micros: output,
        cache_read_cost_micros: cache_read,
        cache_write_cost_micros: cache_write,
        total_cost_micros: input + output + cache_read + cache_write,
        priced: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::usage::models::Agent;

    fn price() -> ModelPrice {
        ModelPrice {
            model: "model-a".to_string(),
            input_usd_per_million: "3".to_string(),
            output_usd_per_million: "15".to_string(),
            cache_read_usd_per_million: "0.3".to_string(),
            cache_write_usd_per_million: "3.75".to_string(),
            source: "test".to_string(),
            effective_at: 1,
        }
    }

    #[test]
    fn claude_fresh_input_does_not_subtract_cache_tokens() {
        let event = UsageEvent {
            id: "e1".to_string(),
            agent: Agent::Claude,
            source: "session_log".to_string(),
            session_id: None,
            model: "model-a".to_string(),
            input_tokens: 1_000,
            output_tokens: 500,
            cache_read_tokens: 200,
            cache_write_tokens: 100,
            input_token_semantics: InputTokenSemantics::Fresh,
            occurred_at: 1,
        };

        let cost = calculate_cost(&event, Some(&price()));

        assert_eq!(cost.input_cost_micros, 3_000);
        assert_eq!(cost.output_cost_micros, 7_500);
        assert_eq!(cost.cache_read_cost_micros, 60);
        assert_eq!(cost.cache_write_cost_micros, 375);
        assert_eq!(cost.total_cost_micros, 10_935);
        assert!(cost.priced);
    }

    #[test]
    fn codex_total_input_subtracts_cache_tokens() {
        let event = UsageEvent {
            id: "e1".to_string(),
            agent: Agent::Codex,
            source: "session_log".to_string(),
            session_id: None,
            model: "model-a".to_string(),
            input_tokens: 1_000,
            output_tokens: 500,
            cache_read_tokens: 200,
            cache_write_tokens: 100,
            input_token_semantics: InputTokenSemantics::TotalIncludesCache,
            occurred_at: 1,
        };

        let cost = calculate_cost(&event, Some(&price()));

        assert_eq!(cost.input_cost_micros, 2_100);
        assert_eq!(cost.output_cost_micros, 7_500);
        assert_eq!(cost.cache_read_cost_micros, 60);
        assert_eq!(cost.cache_write_cost_micros, 375);
        assert_eq!(cost.total_cost_micros, 10_035);
        assert!(cost.priced);
    }

    #[test]
    fn missing_price_keeps_tokens_unpriced() {
        let event = UsageEvent {
            id: "e1".to_string(),
            agent: Agent::Codex,
            source: "session_log".to_string(),
            session_id: None,
            model: "unknown".to_string(),
            input_tokens: 1_000,
            output_tokens: 500,
            cache_read_tokens: 0,
            cache_write_tokens: 0,
            input_token_semantics: InputTokenSemantics::TotalIncludesCache,
            occurred_at: 1,
        };

        let cost = calculate_cost(&event, None);

        assert_eq!(cost.total_cost_micros, 0);
        assert!(!cost.priced);
    }
}
```

- [ ] **Step 4: Run Rust tests**

Run:

```bash
cd src-tauri && cargo test usage::cost
```

Expected:

- 3 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/main.rs src-tauri/src/usage
git commit -m "feat: add usage cost model"
```

---

## Task 3: Parse Claude Code Logs

**Files:**
- Create: `src-tauri/src/usage/claude.rs`
- Modify: `src-tauri/src/usage/mod.rs`
- Create: `src-tauri/tests/fixtures/claude-simple.jsonl`

- [ ] **Step 1: Add fixture**

Create `src-tauri/tests/fixtures/claude-simple.jsonl`:

```jsonl
{"type":"system","sessionId":"session-a","timestamp":"2026-08-06T01:00:00Z"}
{"type":"assistant","sessionId":"session-a","timestamp":"2026-08-06T01:00:01Z","message":{"id":"msg_1","model":"claude-sonnet-5","usage":{"input_tokens":1000,"output_tokens":200,"cache_read_input_tokens":300,"cache_creation_input_tokens":40},"stop_reason":"end_turn"}}
{"type":"assistant","sessionId":"session-a","timestamp":"2026-08-06T01:00:02Z","message":{"id":"msg_1","model":"claude-sonnet-5","usage":{"input_tokens":1000,"output_tokens":250,"cache_read_input_tokens":300,"cache_creation_input_tokens":40},"stop_reason":"end_turn"}}
{"type":"user","sessionId":"session-a","timestamp":"2026-08-06T01:00:03Z","message":{"content":"ignored"}}
```

- [ ] **Step 2: Implement parser and tests**

Create `src-tauri/src/usage/claude.rs`:

```rust
use super::models::{Agent, InputTokenSemantics, UsageEvent};
use chrono::DateTime;
use serde_json::Value;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read};

#[derive(Debug)]
struct ParsedClaudeMessage {
    event: UsageEvent,
    stop_reason_present: bool,
}

fn timestamp_to_epoch(value: Option<&Value>) -> i64 {
    value
        .and_then(Value::as_str)
        .and_then(|raw| DateTime::parse_from_rfc3339(raw).ok())
        .map(|dt| dt.timestamp())
        .unwrap_or(0)
}

fn u64_field(parent: &Value, key: &str) -> u64 {
    parent.get(key).and_then(Value::as_u64).unwrap_or(0)
}

pub fn parse_claude_jsonl<R: Read>(reader: R) -> Vec<UsageEvent> {
    let reader = BufReader::new(reader);
    let mut by_message_id: HashMap<String, ParsedClaudeMessage> = HashMap::new();
    let mut current_session_id: Option<String> = None;

    for line in reader.lines().map_while(Result::ok) {
        if line.trim().is_empty() {
            continue;
        }

        let Ok(value) = serde_json::from_str::<Value>(&line) else {
            continue;
        };

        if current_session_id.is_none() {
            current_session_id = value
                .get("sessionId")
                .and_then(Value::as_str)
                .map(str::to_string);
        }

        if value.get("type").and_then(Value::as_str) != Some("assistant") {
            continue;
        }

        let Some(message) = value.get("message") else {
            continue;
        };
        let Some(message_id) = message.get("id").and_then(Value::as_str) else {
            continue;
        };
        let Some(usage) = message.get("usage") else {
            continue;
        };

        let input_tokens = u64_field(usage, "input_tokens");
        let output_tokens = u64_field(usage, "output_tokens");
        let cache_read_tokens = u64_field(usage, "cache_read_input_tokens");
        let cache_write_tokens = u64_field(usage, "cache_creation_input_tokens");

        if input_tokens == 0
            && output_tokens == 0
            && cache_read_tokens == 0
            && cache_write_tokens == 0
        {
            continue;
        }

        let stop_reason_present = message.get("stop_reason").and_then(Value::as_str).is_some();
        let event = UsageEvent {
            id: format!("claude:session:{message_id}"),
            agent: Agent::Claude,
            source: "session_log".to_string(),
            session_id: current_session_id.clone(),
            model: message
                .get("model")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string(),
            input_tokens,
            output_tokens,
            cache_read_tokens,
            cache_write_tokens,
            input_token_semantics: InputTokenSemantics::Fresh,
            occurred_at: timestamp_to_epoch(value.get("timestamp")),
        };

        let should_replace = match by_message_id.get(message_id) {
            None => true,
            Some(existing) => {
                (stop_reason_present && !existing.stop_reason_present)
                    || (stop_reason_present == existing.stop_reason_present
                        && event.output_tokens > existing.event.output_tokens)
            }
        };

        if should_replace {
            by_message_id.insert(
                message_id.to_string(),
                ParsedClaudeMessage {
                    event,
                    stop_reason_present,
                },
            );
        }
    }

    let mut events: Vec<UsageEvent> = by_message_id
        .into_values()
        .map(|parsed| parsed.event)
        .collect();
    events.sort_by(|a, b| a.id.cmp(&b.id));
    events
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imports_assistant_usage_and_deduplicates_by_message_id() {
        let fixture = include_bytes!("../../tests/fixtures/claude-simple.jsonl");

        let events = parse_claude_jsonl(&fixture[..]);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].id, "claude:session:msg_1");
        assert_eq!(events[0].agent, Agent::Claude);
        assert_eq!(events[0].model, "claude-sonnet-5");
        assert_eq!(events[0].input_tokens, 1000);
        assert_eq!(events[0].output_tokens, 250);
        assert_eq!(events[0].cache_read_tokens, 300);
        assert_eq!(events[0].cache_write_tokens, 40);
        assert_eq!(events[0].input_token_semantics, InputTokenSemantics::Fresh);
    }
}
```

Modify `src-tauri/src/usage/mod.rs`:

```rust
pub mod claude;
pub mod cost;
pub mod models;
```

- [ ] **Step 3: Run parser test**

Run:

```bash
cd src-tauri && cargo test usage::claude
```

Expected:

- Claude parser test passes.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/usage/mod.rs src-tauri/src/usage/claude.rs src-tauri/tests/fixtures/claude-simple.jsonl
git commit -m "feat: parse claude code usage logs"
```

---

## Task 4: Parse Codex Logs

**Files:**
- Create: `src-tauri/src/usage/codex.rs`
- Modify: `src-tauri/src/usage/mod.rs`
- Create: `src-tauri/tests/fixtures/codex-simple.jsonl`

- [ ] **Step 1: Add fixture**

Create `src-tauri/tests/fixtures/codex-simple.jsonl`:

```jsonl
{"type":"session_meta","timestamp":"2026-08-06T02:00:00Z","payload":{"id":"11111111-1111-1111-1111-111111111111"}}
{"type":"turn_context","timestamp":"2026-08-06T02:00:00Z","payload":{"model":"openai/gpt-5.4-2026-03-05"}}
{"type":"event_msg","timestamp":"2026-08-06T02:00:01Z","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":1000,"cached_input_tokens":600,"output_tokens":50}}}}
{"type":"event_msg","timestamp":"2026-08-06T02:00:02Z","payload":{"type":"token_count","info":{"model":"gpt-5.4","total_token_usage":{"input_tokens":1200,"cached_input_tokens":720,"output_tokens":60},"last_token_usage":{"input_tokens":200,"cached_input_tokens":120,"output_tokens":10}}}}
```

- [ ] **Step 2: Implement parser and tests**

Create `src-tauri/src/usage/codex.rs`:

```rust
use super::models::{Agent, InputTokenSemantics, UsageEvent};
use chrono::DateTime;
use serde_json::Value;
use std::io::{BufRead, BufReader, Read};

#[derive(Debug, Clone, Default)]
struct CumulativeTokens {
    input: u64,
    cached_input: u64,
    output: u64,
}

fn timestamp_to_epoch(value: Option<&Value>) -> i64 {
    value
        .and_then(Value::as_str)
        .and_then(|raw| DateTime::parse_from_rfc3339(raw).ok())
        .map(|dt| dt.timestamp())
        .unwrap_or(0)
}

fn normalize_model(raw: &str) -> String {
    let mut model = raw.to_lowercase();
    if let Some(index) = model.rfind('/') {
        model = model[index + 1..].to_string();
    }
    if model.len() > 11 {
        let suffix = &model[model.len() - 11..];
        if suffix.as_bytes().first() == Some(&b'-')
            && suffix[1..5].chars().all(|c| c.is_ascii_digit())
            && suffix.as_bytes().get(5) == Some(&b'-')
            && suffix[6..8].chars().all(|c| c.is_ascii_digit())
            && suffix.as_bytes().get(8) == Some(&b'-')
            && suffix[9..11].chars().all(|c| c.is_ascii_digit())
        {
            model.truncate(model.len() - 11);
        }
    }
    model
}

fn parse_cumulative(value: Option<&Value>) -> Option<CumulativeTokens> {
    let value = value?;
    Some(CumulativeTokens {
        input: value.get("input_tokens").and_then(Value::as_u64).unwrap_or(0),
        cached_input: value
            .get("cached_input_tokens")
            .or_else(|| value.get("cache_read_input_tokens"))
            .and_then(Value::as_u64)
            .unwrap_or(0),
        output: value.get("output_tokens").and_then(Value::as_u64).unwrap_or(0),
    })
}

fn delta(previous: Option<&CumulativeTokens>, current: &CumulativeTokens) -> CumulativeTokens {
    match previous {
        None => current.clone(),
        Some(previous) => CumulativeTokens {
            input: current.input.saturating_sub(previous.input),
            cached_input: current.cached_input.saturating_sub(previous.cached_input),
            output: current.output.saturating_sub(previous.output),
        },
    }
}

pub fn parse_codex_jsonl<R: Read>(reader: R, file_stem: &str) -> Vec<UsageEvent> {
    let reader = BufReader::new(reader);
    let mut current_model = "unknown".to_string();
    let mut previous_total: Option<CumulativeTokens> = None;
    let mut event_index = 0u64;
    let mut events = Vec::new();

    for line in reader.lines().map_while(Result::ok) {
        if line.trim().is_empty() {
            continue;
        }

        let Ok(value) = serde_json::from_str::<Value>(&line) else {
            continue;
        };

        match value.get("type").and_then(Value::as_str) {
            Some("turn_context") => {
                if let Some(model) = value
                    .get("payload")
                    .and_then(|payload| payload.get("model"))
                    .and_then(Value::as_str)
                {
                    current_model = normalize_model(model);
                }
            }
            Some("event_msg") => {
                let Some(payload) = value.get("payload") else {
                    continue;
                };
                if payload.get("type").and_then(Value::as_str) != Some("token_count") {
                    continue;
                }
                let Some(info) = payload.get("info") else {
                    continue;
                };
                if let Some(model) = info
                    .get("model")
                    .or_else(|| info.get("model_name"))
                    .and_then(Value::as_str)
                {
                    current_model = normalize_model(model);
                }

                let total = parse_cumulative(info.get("total_token_usage"));
                let last = parse_cumulative(info.get("last_token_usage"));
                let Some(tokens) = last.or_else(|| {
                    total
                        .as_ref()
                        .map(|current| delta(previous_total.as_ref(), current))
                }) else {
                    continue;
                };

                if let Some(total) = total {
                    previous_total = Some(total);
                }

                let cache_read = tokens.cached_input.min(tokens.input);
                if tokens.input == 0 && tokens.output == 0 && cache_read == 0 {
                    continue;
                }

                event_index += 1;
                events.push(UsageEvent {
                    id: format!("codex:session:{file_stem}:{event_index}"),
                    agent: Agent::Codex,
                    source: "session_log".to_string(),
                    session_id: Some(file_stem.to_string()),
                    model: current_model.clone(),
                    input_tokens: tokens.input,
                    output_tokens: tokens.output,
                    cache_read_tokens: cache_read,
                    cache_write_tokens: 0,
                    input_token_semantics: InputTokenSemantics::TotalIncludesCache,
                    occurred_at: timestamp_to_epoch(value.get("timestamp")),
                });
            }
            _ => {}
        }
    }

    events
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imports_codex_token_count_events_and_prefers_last_usage() {
        let fixture = include_bytes!("../../tests/fixtures/codex-simple.jsonl");

        let events = parse_codex_jsonl(&fixture[..], "rollout-test");

        assert_eq!(events.len(), 2);
        assert_eq!(events[0].id, "codex:session:rollout-test:1");
        assert_eq!(events[0].model, "gpt-5.4");
        assert_eq!(events[0].input_tokens, 1000);
        assert_eq!(events[0].cache_read_tokens, 600);
        assert_eq!(events[0].output_tokens, 50);

        assert_eq!(events[1].id, "codex:session:rollout-test:2");
        assert_eq!(events[1].input_tokens, 200);
        assert_eq!(events[1].cache_read_tokens, 120);
        assert_eq!(events[1].output_tokens, 10);
        assert_eq!(
            events[1].input_token_semantics,
            InputTokenSemantics::TotalIncludesCache
        );
    }
}
```

Modify `src-tauri/src/usage/mod.rs`:

```rust
pub mod claude;
pub mod codex;
pub mod cost;
pub mod models;
```

- [ ] **Step 3: Run parser tests**

Run:

```bash
cd src-tauri && cargo test usage::codex
```

Expected:

- Codex parser test passes.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/usage/mod.rs src-tauri/src/usage/codex.rs src-tauri/tests/fixtures/codex-simple.jsonl
git commit -m "feat: parse codex usage logs"
```

---

## Task 5: Add SQLite Persistence

**Files:**
- Create: `src-tauri/src/db.rs`
- Modify: `src-tauri/src/main.rs`

- [ ] **Step 1: Implement database schema and inserts**

Create `src-tauri/src/db.rs`:

```rust
use crate::usage::models::{Agent, InputTokenSemantics, ModelPrice, UsageEvent};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;

pub fn open_database(path: &Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    migrate(&conn)?;
    Ok(conn)
}

pub fn migrate(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS usage_events (
          id TEXT PRIMARY KEY,
          agent TEXT NOT NULL,
          source TEXT NOT NULL,
          session_id TEXT,
          model TEXT NOT NULL,
          input_tokens INTEGER NOT NULL,
          output_tokens INTEGER NOT NULL,
          cache_read_tokens INTEGER NOT NULL,
          cache_write_tokens INTEGER NOT NULL,
          input_token_semantics TEXT NOT NULL,
          occurred_at INTEGER NOT NULL,
          created_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS model_prices (
          model TEXT PRIMARY KEY,
          input_usd_per_million TEXT NOT NULL,
          output_usd_per_million TEXT NOT NULL,
          cache_read_usd_per_million TEXT NOT NULL,
          cache_write_usd_per_million TEXT NOT NULL,
          source TEXT NOT NULL,
          effective_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS subscriptions (
          agent TEXT PRIMARY KEY,
          label TEXT NOT NULL,
          monthly_cost_usd TEXT NOT NULL,
          billing_cycle_day INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS sync_cursors (
          file_path TEXT PRIMARY KEY,
          modified_nanos INTEGER NOT NULL,
          line_offset INTEGER NOT NULL,
          synced_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS quota_snapshots (
          id TEXT PRIMARY KEY,
          agent TEXT NOT NULL,
          source TEXT NOT NULL,
          primary_used_percent REAL,
          primary_window_minutes INTEGER,
          primary_resets_at INTEGER,
          secondary_used_percent REAL,
          secondary_window_minutes INTEGER,
          secondary_resets_at INTEGER,
          credit_balance TEXT,
          credit_limit TEXT,
          credit_used TEXT,
          credit_remaining_percent REAL,
          credit_resets_at INTEGER,
          plan_type TEXT,
          updated_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS quota_extra_windows (
          id TEXT PRIMARY KEY,
          snapshot_id TEXT NOT NULL,
          title TEXT NOT NULL,
          used_percent REAL NOT NULL,
          window_minutes INTEGER,
          resets_at INTEGER,
          FOREIGN KEY(snapshot_id) REFERENCES quota_snapshots(id)
        );
        ",
    )
}

fn semantics_str(value: &InputTokenSemantics) -> &'static str {
    match value {
        InputTokenSemantics::Fresh => "fresh",
        InputTokenSemantics::TotalIncludesCache => "total_includes_cache",
    }
}

pub fn insert_usage_event(conn: &Connection, event: &UsageEvent) -> rusqlite::Result<bool> {
    let rows = conn.execute(
        "
        INSERT OR IGNORE INTO usage_events (
          id, agent, source, session_id, model, input_tokens, output_tokens,
          cache_read_tokens, cache_write_tokens, input_token_semantics,
          occurred_at, created_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, strftime('%s','now'))
        ",
        params![
            event.id,
            event.agent.as_str(),
            event.source,
            event.session_id,
            event.model,
            event.input_tokens as i64,
            event.output_tokens as i64,
            event.cache_read_tokens as i64,
            event.cache_write_tokens as i64,
            semantics_str(&event.input_token_semantics),
            event.occurred_at,
        ],
    )?;
    Ok(rows > 0)
}

pub fn upsert_model_price(conn: &Connection, price: &ModelPrice) -> rusqlite::Result<()> {
    conn.execute(
        "
        INSERT INTO model_prices (
          model, input_usd_per_million, output_usd_per_million,
          cache_read_usd_per_million, cache_write_usd_per_million,
          source, effective_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        ON CONFLICT(model) DO UPDATE SET
          input_usd_per_million = excluded.input_usd_per_million,
          output_usd_per_million = excluded.output_usd_per_million,
          cache_read_usd_per_million = excluded.cache_read_usd_per_million,
          cache_write_usd_per_million = excluded.cache_write_usd_per_million,
          source = excluded.source,
          effective_at = excluded.effective_at
        ",
        params![
            price.model,
            price.input_usd_per_million,
            price.output_usd_per_million,
            price.cache_read_usd_per_million,
            price.cache_write_usd_per_million,
            price.source,
            price.effective_at,
        ],
    )?;
    Ok(())
}

pub fn find_model_price(conn: &Connection, model: &str) -> rusqlite::Result<Option<ModelPrice>> {
    conn.query_row(
        "
        SELECT model, input_usd_per_million, output_usd_per_million,
               cache_read_usd_per_million, cache_write_usd_per_million,
               source, effective_at
        FROM model_prices
        WHERE model = ?1
        ",
        [model],
        |row| {
            Ok(ModelPrice {
                model: row.get(0)?,
                input_usd_per_million: row.get(1)?,
                output_usd_per_million: row.get(2)?,
                cache_read_usd_per_million: row.get(3)?,
                cache_write_usd_per_million: row.get(4)?,
                source: row.get(5)?,
                effective_at: row.get(6)?,
            })
        },
    )
    .optional()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_usage_event_is_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        migrate(&conn).unwrap();
        let event = UsageEvent {
            id: "claude:session:msg_1".to_string(),
            agent: Agent::Claude,
            source: "session_log".to_string(),
            session_id: Some("s1".to_string()),
            model: "claude-sonnet-5".to_string(),
            input_tokens: 10,
            output_tokens: 2,
            cache_read_tokens: 1,
            cache_write_tokens: 0,
            input_token_semantics: InputTokenSemantics::Fresh,
            occurred_at: 1,
        };

        assert!(insert_usage_event(&conn, &event).unwrap());
        assert!(!insert_usage_event(&conn, &event).unwrap());

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM usage_events", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }
}
```

Modify `src-tauri/src/main.rs`:

```rust
mod db;
mod usage;

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_always_on_top(true);
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to run AI ROI Pet");
}
```

- [ ] **Step 2: Run database tests**

Run:

```bash
cd src-tauri && cargo test db
```

Expected:

- Database idempotency test passes.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/db.rs src-tauri/src/main.rs
git commit -m "feat: persist usage events"
```

---

## Task 6: Implement Local Log Sync

**Files:**
- Create: `src-tauri/src/usage/sync.rs`
- Modify: `src-tauri/src/usage/mod.rs`
- Modify: `src-tauri/src/main.rs`

- [ ] **Step 1: Implement scanner**

Create `src-tauri/src/usage/sync.rs`:

```rust
use super::{claude::parse_claude_jsonl, codex::parse_codex_jsonl};
use crate::db::insert_usage_event;
use rusqlite::Connection;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Default, Serialize)]
pub struct SyncReport {
    pub files_scanned: u64,
    pub events_imported: u64,
    pub events_skipped: u64,
    pub errors: Vec<String>,
}

fn collect_jsonl_recursive(root: &Path, max_depth: usize) -> Vec<PathBuf> {
    fn walk(dir: &Path, depth: usize, max_depth: usize, out: &mut Vec<PathBuf>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && depth < max_depth {
                walk(&path, depth + 1, max_depth, out);
            } else if path.extension().and_then(|ext| ext.to_str()) == Some("jsonl") {
                out.push(path);
            }
        }
    }

    let mut files = Vec::new();
    walk(root, 0, max_depth, &mut files);
    files.sort();
    files
}

pub fn sync_usage_logs(conn: &Connection, home: &Path) -> SyncReport {
    let mut report = SyncReport::default();

    let claude_root = home.join(".claude").join("projects");
    if claude_root.is_dir() {
        for file in collect_jsonl_recursive(&claude_root, 5) {
            report.files_scanned += 1;
            match fs::File::open(&file) {
                Ok(handle) => {
                    for event in parse_claude_jsonl(handle) {
                        match insert_usage_event(conn, &event) {
                            Ok(true) => report.events_imported += 1,
                            Ok(false) => report.events_skipped += 1,
                            Err(error) => report.errors.push(format!("{}: {error}", file.display())),
                        }
                    }
                }
                Err(error) => report.errors.push(format!("{}: {error}", file.display())),
            }
        }
    }

    let codex_sessions = home.join(".codex").join("sessions");
    if codex_sessions.is_dir() {
        for file in collect_jsonl_recursive(&codex_sessions, 4) {
            report.files_scanned += 1;
            match fs::File::open(&file) {
                Ok(handle) => {
                    let stem = file
                        .file_stem()
                        .and_then(|value| value.to_str())
                        .unwrap_or("unknown");
                    for event in parse_codex_jsonl(handle, stem) {
                        match insert_usage_event(conn, &event) {
                            Ok(true) => report.events_imported += 1,
                            Ok(false) => report.events_skipped += 1,
                            Err(error) => report.errors.push(format!("{}: {error}", file.display())),
                        }
                    }
                }
                Err(error) => report.errors.push(format!("{}: {error}", file.display())),
            }
        }
    }

    let codex_archived = home.join(".codex").join("archived_sessions");
    if codex_archived.is_dir() {
        for file in collect_jsonl_recursive(&codex_archived, 1) {
            report.files_scanned += 1;
            match fs::File::open(&file) {
                Ok(handle) => {
                    let stem = file
                        .file_stem()
                        .and_then(|value| value.to_str())
                        .unwrap_or("unknown");
                    for event in parse_codex_jsonl(handle, stem) {
                        match insert_usage_event(conn, &event) {
                            Ok(true) => report.events_imported += 1,
                            Ok(false) => report.events_skipped += 1,
                            Err(error) => report.errors.push(format!("{}: {error}", file.display())),
                        }
                    }
                }
                Err(error) => report.errors.push(format!("{}: {error}", file.display())),
            }
        }
    }

    report
}
```

Modify `src-tauri/src/usage/mod.rs`:

```rust
pub mod claude;
pub mod codex;
pub mod cost;
pub mod models;
pub mod sync;
```

- [ ] **Step 2: Run sync-related tests**

Run:

```bash
cd src-tauri && cargo test usage
```

Expected:

- All usage tests pass.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/usage/mod.rs src-tauri/src/usage/sync.rs
git commit -m "feat: sync local agent usage logs"
```

---

## Task 6A: Implement Codex Quota Probe

**Files:**
- Create: `src-tauri/src/quota/mod.rs`
- Create: `src-tauri/src/quota/models.rs`
- Create: `src-tauri/src/quota/codex_app_server.rs`
- Modify: `src-tauri/src/db.rs`
- Modify: `src-tauri/src/main.rs`

- [ ] **Step 1: Define quota models**

Create `src-tauri/src/quota/mod.rs`:

```rust
pub mod codex_app_server;
pub mod models;
```

Create `src-tauri/src/quota/models.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RateWindowSnapshot {
    pub used_percent: f64,
    pub window_minutes: Option<i64>,
    pub resets_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExtraQuotaWindow {
    pub title: String,
    pub used_percent: f64,
    pub window_minutes: Option<i64>,
    pub resets_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CodexQuotaSnapshot {
    pub source: String,
    pub primary: Option<RateWindowSnapshot>,
    pub secondary: Option<RateWindowSnapshot>,
    pub credit_balance: Option<String>,
    pub credit_limit: Option<String>,
    pub credit_used: Option<String>,
    pub credit_remaining_percent: Option<f64>,
    pub credit_resets_at: Option<i64>,
    pub plan_type: Option<String>,
    pub extra_windows: Vec<ExtraQuotaWindow>,
    pub updated_at: i64,
}
```

- [ ] **Step 2: Implement the Codex app-server probe**

Create `src-tauri/src/quota/codex_app_server.rs`:

```rust
use super::models::{CodexQuotaSnapshot, ExtraQuotaWindow, RateWindowSnapshot};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn as_i64(value: Option<&Value>) -> Option<i64> {
    value.and_then(|item| item.as_i64())
}

fn as_f64(value: Option<&Value>) -> Option<f64> {
    value.and_then(|item| item.as_f64())
}

fn as_string(value: Option<&Value>) -> Option<String> {
    value.and_then(|item| {
        item.as_str()
            .map(ToOwned::to_owned)
            .or_else(|| item.as_f64().map(|number| number.to_string()))
            .or_else(|| item.as_i64().map(|number| number.to_string()))
    })
}

fn parse_window(value: Option<&Value>) -> Option<RateWindowSnapshot> {
    let value = value?;
    Some(RateWindowSnapshot {
        used_percent: as_f64(value.get("usedPercent")).unwrap_or(0.0),
        window_minutes: as_i64(value.get("windowDurationMins"))
            .or_else(|| as_i64(value.get("windowMinutes"))),
        resets_at: as_i64(value.get("resetsAt")),
    })
}

fn read_response(reader: &mut BufReader<std::process::ChildStdout>, id: i64) -> Result<Value, String> {
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut line = String::new();

    while Instant::now() < deadline {
        line.clear();
        let bytes = reader.read_line(&mut line).map_err(|error| error.to_string())?;
        if bytes == 0 {
            return Err("codex app-server closed stdout".to_string());
        }

        let parsed: Value = match serde_json::from_str(line.trim()) {
            Ok(value) => value,
            Err(_) => continue,
        };

        if parsed.get("id").and_then(Value::as_i64) == Some(id) {
            if let Some(error) = parsed.get("error") {
                return Err(format!("codex app-server returned error: {error}"));
            }
            return Ok(parsed.get("result").cloned().unwrap_or(Value::Null));
        }
    }

    Err(format!("timed out waiting for JSON-RPC id {id}"))
}

fn send(writer: &mut std::process::ChildStdin, value: Value) -> Result<(), String> {
    writeln!(writer, "{value}").map_err(|error| error.to_string())?;
    writer.flush().map_err(|error| error.to_string())
}

pub fn fetch_codex_quota(now: i64) -> Result<CodexQuotaSnapshot, String> {
    let mut child = Command::new("codex")
        .args(["-s", "read-only", "-a", "untrusted", "app-server"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("failed to start codex app-server: {error}"))?;

    let mut stdin = child.stdin.take().ok_or_else(|| "missing codex stdin".to_string())?;
    let stdout = child.stdout.take().ok_or_else(|| "missing codex stdout".to_string())?;
    let mut reader = BufReader::new(stdout);

    send(
        &mut stdin,
        json!({"id":1,"method":"initialize","params":{"clientInfo":{"name":"ai-roi-pet","version":"0.1.0"}}}),
    )?;
    let _ = read_response(&mut reader, 1)?;
    send(&mut stdin, json!({"method":"initialized","params":{}}))?;
    send(&mut stdin, json!({"id":2,"method":"account/rateLimits/read","params":{}}))?;
    let rate_limits = read_response(&mut reader, 2)?;
    send(&mut stdin, json!({"id":3,"method":"account/read","params":{}}))?;
    let account = read_response(&mut reader, 3).unwrap_or(Value::Null);
    let _ = child.kill();

    let mut extra_windows = Vec::new();
    if let Some(map) = rate_limits.get("rateLimitsByLimitId").and_then(Value::as_object) {
        for (title, value) in map {
            if let Some(window) = parse_window(Some(value)) {
                extra_windows.push(ExtraQuotaWindow {
                    title: title.to_string(),
                    used_percent: window.used_percent,
                    window_minutes: window.window_minutes,
                    resets_at: window.resets_at,
                });
            }
        }
    }

    let credits = rate_limits.get("credits");
    let individual_limit = rate_limits.get("individualLimit");

    Ok(CodexQuotaSnapshot {
        source: "codex-app-server".to_string(),
        primary: parse_window(rate_limits.get("primary")),
        secondary: parse_window(rate_limits.get("secondary")),
        credit_balance: as_string(credits.and_then(|value| value.get("balance"))),
        credit_limit: as_string(individual_limit.and_then(|value| value.get("limit"))),
        credit_used: as_string(individual_limit.and_then(|value| value.get("used"))),
        credit_remaining_percent: as_f64(individual_limit.and_then(|value| value.get("remainingPercent"))),
        credit_resets_at: as_i64(individual_limit.and_then(|value| value.get("resetsAt"))),
        plan_type: as_string(rate_limits.get("planType")).or_else(|| as_string(account.get("planType"))),
        extra_windows,
        updated_at: now,
    })
}
```

- [ ] **Step 3: Persist the latest quota snapshot**

Add this function to `src-tauri/src/db.rs`:

```rust
pub fn replace_codex_quota_snapshot(
    conn: &mut rusqlite::Connection,
    snapshot: &crate::quota::models::CodexQuotaSnapshot,
) -> rusqlite::Result<()> {
    let tx = conn.transaction()?;
    let snapshot_id = "codex:codex-app-server:latest";

    tx.execute("DELETE FROM quota_extra_windows WHERE snapshot_id = ?1", [snapshot_id])?;
    tx.execute("DELETE FROM quota_snapshots WHERE id = ?1", [snapshot_id])?;
    tx.execute(
        "
        INSERT INTO quota_snapshots (
          id, agent, source,
          primary_used_percent, primary_window_minutes, primary_resets_at,
          secondary_used_percent, secondary_window_minutes, secondary_resets_at,
          credit_balance, credit_limit, credit_used, credit_remaining_percent, credit_resets_at,
          plan_type, updated_at
        )
        VALUES (?1, 'codex', ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
        ",
        rusqlite::params![
            snapshot_id,
            snapshot.source.as_str(),
            snapshot.primary.as_ref().map(|value| value.used_percent),
            snapshot.primary.as_ref().and_then(|value| value.window_minutes),
            snapshot.primary.as_ref().and_then(|value| value.resets_at),
            snapshot.secondary.as_ref().map(|value| value.used_percent),
            snapshot.secondary.as_ref().and_then(|value| value.window_minutes),
            snapshot.secondary.as_ref().and_then(|value| value.resets_at),
            snapshot.credit_balance.as_deref(),
            snapshot.credit_limit.as_deref(),
            snapshot.credit_used.as_deref(),
            snapshot.credit_remaining_percent,
            snapshot.credit_resets_at,
            snapshot.plan_type.as_deref(),
            snapshot.updated_at
        ],
    )?;

    for (index, window) in snapshot.extra_windows.iter().enumerate() {
        tx.execute(
            "
            INSERT INTO quota_extra_windows (id, snapshot_id, title, used_percent, window_minutes, resets_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ",
            rusqlite::params![
                format!("{snapshot_id}:{index}"),
                snapshot_id,
                window.title.as_str(),
                window.used_percent,
                window.window_minutes,
                window.resets_at
            ],
        )?;
    }

    tx.commit()
}
```

- [ ] **Step 4: Register the module**

Modify `src-tauri/src/main.rs`:

```rust
mod commands;
mod db;
mod quota;
mod settings;
mod usage;
```

- [ ] **Step 5: Verify quota code compiles**

Run:

```bash
cd src-tauri && cargo test quota
```

Expected:

- Quota module compiles.
- Command exits successfully and compiles the quota module references.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/quota src-tauri/src/db.rs src-tauri/src/main.rs
git commit -m "feat: read codex quota snapshots"
```

---

## Task 7: Add Settings and Default Prices

**Files:**
- Create: `src-tauri/src/settings.rs`
- Modify: `src-tauri/src/main.rs`

- [ ] **Step 1: Implement settings seeding**

Create `src-tauri/src/settings.rs`:

```rust
use crate::db::upsert_model_price;
use crate::usage::models::ModelPrice;
use rusqlite::{params, Connection};

pub fn seed_defaults(conn: &Connection, now: i64) -> rusqlite::Result<()> {
    let prices = vec![
        ModelPrice {
            model: "claude-sonnet-5".to_string(),
            input_usd_per_million: "2".to_string(),
            output_usd_per_million: "10".to_string(),
            cache_read_usd_per_million: "0.2".to_string(),
            cache_write_usd_per_million: "2.5".to_string(),
            source: "builtin-editable".to_string(),
            effective_at: now,
        },
        ModelPrice {
            model: "gpt-5.4".to_string(),
            input_usd_per_million: "1.25".to_string(),
            output_usd_per_million: "10".to_string(),
            cache_read_usd_per_million: "0.125".to_string(),
            cache_write_usd_per_million: "0".to_string(),
            source: "builtin-editable".to_string(),
            effective_at: now,
        },
    ];

    for price in prices {
        upsert_model_price(conn, &price)?;
    }

    conn.execute(
        "
        INSERT OR IGNORE INTO subscriptions (agent, label, monthly_cost_usd, billing_cycle_day)
        VALUES (?1, ?2, ?3, ?4)
        ",
        params!["claude", "Claude Code Custom", "100", 1],
    )?;

    conn.execute(
        "
        INSERT OR IGNORE INTO subscriptions (agent, label, monthly_cost_usd, billing_cycle_day)
        VALUES (?1, ?2, ?3, ?4)
        ",
        params!["codex", "Codex Custom", "20", 1],
    )?;

    Ok(())
}
```

Modify `src-tauri/src/main.rs`:

```rust
mod db;
mod settings;
mod usage;

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_always_on_top(true);
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to run AI ROI Pet");
}
```

- [ ] **Step 2: Add a manual verification note to README**

Create `README.md`:

```markdown
# AI ROI Pet

Local-first desktop pet for estimating whether Codex and Claude Code subscriptions have paid for themselves.

The app reads local usage logs and applies editable API-equivalent model prices. Built-in prices are defaults only; users should verify them against the official provider pricing pages before relying on the dollar value.

Codex quota display is separate from payback calculation. When the Codex CLI exposes account quota through `app-server`, the app shows provider-reported rate-limit and credit snapshots beside the estimated API-equivalent value.
```

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/settings.rs src-tauri/src/main.rs README.md
git commit -m "feat: seed editable pricing defaults"
```

---

## Task 8: Expose Tauri Commands

**Files:**
- Create: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/main.rs`

- [ ] **Step 1: Implement commands**

Create `src-tauri/src/commands.rs`:

```rust
use crate::{
    db,
    quota::{codex_app_server::fetch_codex_quota, models::RateWindowSnapshot},
    settings,
    usage::sync::sync_usage_logs,
};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::State;

pub struct AppState {
    pub db_path: PathBuf,
    pub lock: Mutex<()>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageSummary {
    pub agent: String,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub total_cache_read_tokens: i64,
    pub total_cache_write_tokens: i64,
    pub unpriced_models: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotaSummary {
    pub agent: String,
    pub source: String,
    pub primary: Option<RateWindowSnapshot>,
    pub secondary: Option<RateWindowSnapshot>,
    pub credit_balance: Option<String>,
    pub credit_limit: Option<String>,
    pub credit_used: Option<String>,
    pub credit_remaining_percent: Option<f64>,
    pub credit_resets_at: Option<i64>,
    pub plan_type: Option<String>,
    pub updated_at: i64,
}

#[tauri::command]
pub fn sync_now(state: State<AppState>) -> Result<crate::usage::sync::SyncReport, String> {
    let _guard = state.lock.lock().map_err(|_| "sync lock poisoned".to_string())?;
    let conn = db::open_database(&state.db_path).map_err(|error| error.to_string())?;
    settings::seed_defaults(&conn, chrono::Utc::now().timestamp())
        .map_err(|error| error.to_string())?;
    let home = dirs::home_dir().ok_or_else(|| "home directory not found".to_string())?;
    Ok(sync_usage_logs(&conn, &home))
}

#[tauri::command]
pub fn refresh_codex_quota(state: State<AppState>) -> Result<QuotaSummary, String> {
    let _guard = state.lock.lock().map_err(|_| "quota lock poisoned".to_string())?;
    let mut conn = db::open_database(&state.db_path).map_err(|error| error.to_string())?;
    let snapshot = fetch_codex_quota(chrono::Utc::now().timestamp())?;
    db::replace_codex_quota_snapshot(&mut conn, &snapshot).map_err(|error| error.to_string())?;

    Ok(QuotaSummary {
        agent: "codex".to_string(),
        source: snapshot.source,
        primary: snapshot.primary,
        secondary: snapshot.secondary,
        credit_balance: snapshot.credit_balance,
        credit_limit: snapshot.credit_limit,
        credit_used: snapshot.credit_used,
        credit_remaining_percent: snapshot.credit_remaining_percent,
        credit_resets_at: snapshot.credit_resets_at,
        plan_type: snapshot.plan_type,
        updated_at: snapshot.updated_at,
    })
}

#[tauri::command]
pub fn get_usage_summary(state: State<AppState>) -> Result<Vec<UsageSummary>, String> {
    let _guard = state.lock.lock().map_err(|_| "summary lock poisoned".to_string())?;
    let conn = db::open_database(&state.db_path).map_err(|error| error.to_string())?;
    let mut stmt = conn
        .prepare(
            "
            SELECT agent,
                   SUM(input_tokens),
                   SUM(output_tokens),
                   SUM(cache_read_tokens),
                   SUM(cache_write_tokens)
            FROM usage_events
            GROUP BY agent
            ORDER BY agent
            ",
        )
        .map_err(|error| error.to_string())?;

    let rows = stmt
        .query_map([], |row| {
            Ok(UsageSummary {
                agent: row.get(0)?,
                total_input_tokens: row.get(1)?,
                total_output_tokens: row.get(2)?,
                total_cache_read_tokens: row.get(3)?,
                total_cache_write_tokens: row.get(4)?,
                unpriced_models: Vec::new(),
            })
        })
        .map_err(|error| error.to_string())?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}
```

Quota failure behavior:

- `refresh_codex_quota` returns an error when `codex` is not installed, not authenticated, or does not expose the JSON-RPC methods.
- The frontend must catch that error and keep displaying the usage/payback view.
- The pet may show `quota unavailable`; it must not treat unavailable quota as zero usage.

Modify `src-tauri/src/main.rs`:

```rust
mod commands;
mod db;
mod quota;
mod settings;
mod usage;

use commands::AppState;
use std::{path::PathBuf, sync::Mutex};

fn app_db_path() -> PathBuf {
    let base = dirs::data_dir()
        .or_else(dirs::home_dir)
        .expect("data directory not found");
    base.join("ai-roi-pet").join("usage.sqlite3")
}

fn main() {
    tauri::Builder::default()
        .manage(AppState {
            db_path: app_db_path(),
            lock: Mutex::new(()),
        })
        .invoke_handler(tauri::generate_handler![
            commands::sync_now,
            commands::refresh_codex_quota,
            commands::get_usage_summary
        ])
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_always_on_top(true);
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to run AI ROI Pet");
}
```

- [ ] **Step 2: Fix database directory creation**

Modify `app_db_path` in `src-tauri/src/main.rs`:

```rust
fn app_db_path() -> PathBuf {
    let base = dirs::data_dir()
        .or_else(dirs::home_dir)
        .expect("data directory not found");
    let dir = base.join("ai-roi-pet");
    std::fs::create_dir_all(&dir).expect("failed to create app data directory");
    dir.join("usage.sqlite3")
}
```

- [ ] **Step 3: Verify commands compile**

Run:

```bash
cd src-tauri && cargo test
```

Expected:

- All Rust tests pass.
- The Tauri command module compiles.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/main.rs
git commit -m "feat: expose usage sync commands"
```

---

## Task 9: Build Frontend Summary and Pet State

**Files:**
- Create: `src/types.ts`
- Create: `src/lib/money.ts`
- Create: `src/lib/petState.ts`
- Create: `src/components/FloatingPet.tsx`
- Create: `src/components/UsagePanel.tsx`
- Modify: `src/App.tsx`
- Modify: `src/app.css`

- [ ] **Step 1: Define frontend types and helpers**

Create `src/types.ts`:

```ts
export type RateWindowSnapshot = {
  usedPercent: number;
  windowMinutes?: number;
  resetsAt?: number;
};

export type QuotaSummary = {
  agent: string;
  source: string;
  primary?: RateWindowSnapshot;
  secondary?: RateWindowSnapshot;
  creditBalance?: string;
  creditLimit?: string;
  creditUsed?: string;
  creditRemainingPercent?: number;
  creditResetsAt?: number;
  planType?: string;
  updatedAt: number;
};

export type UsageSummary = {
  agent: string;
  totalInputTokens: number;
  totalOutputTokens: number;
  totalCacheReadTokens: number;
  totalCacheWriteTokens: number;
  unpricedModels: string[];
};
```

Create `src/lib/money.ts`:

```ts
export function formatUsd(value: number): string {
  return new Intl.NumberFormat("en-US", {
    style: "currency",
    currency: "USD",
    maximumFractionDigits: 2,
  }).format(value);
}
```

Create `src/lib/petState.ts`:

```ts
export type PetMood = "sleepy" | "working" | "ready" | "paid" | "wild";

export function moodFromPaybackRatio(ratio: number): PetMood {
  if (ratio >= 2) return "wild";
  if (ratio >= 1) return "paid";
  if (ratio >= 0.8) return "ready";
  if (ratio >= 0.3) return "working";
  return "sleepy";
}
```

- [ ] **Step 2: Create pet and panel components**

Create `src/components/FloatingPet.tsx`:

```tsx
import { formatUsd } from "../lib/money";
import { moodFromPaybackRatio } from "../lib/petState";
import type { QuotaSummary } from "../types";

type Props = {
  apiValueUsd: number;
  monthlyCostUsd: number;
  quota?: QuotaSummary;
  quotaError?: string;
};

export function FloatingPet({ apiValueUsd, monthlyCostUsd, quota, quotaError }: Props) {
  const ratio = monthlyCostUsd > 0 ? apiValueUsd / monthlyCostUsd : 0;
  const mood = moodFromPaybackRatio(ratio);
  const percent = Math.round(ratio * 100);
  const primaryQuota = quota?.primary ? `${Math.round(quota.primary.usedPercent)}% quota` : null;

  return (
    <section className={`pet-shell mood-${mood}`}>
      <div className="pet-face" aria-label={`AI ROI pet ${mood}`}>
        <span className="pet-eye" />
        <span className="pet-eye" />
        <span className="pet-mouth" />
      </div>
      <div className="pet-stats">
        <strong>{formatUsd(apiValueUsd)}</strong>
        <span>{percent}% 回本</span>
        <span>{primaryQuota ?? (quotaError ? "quota unavailable" : "quota pending")}</span>
      </div>
    </section>
  );
}
```

Create `src/components/UsagePanel.tsx`:

```tsx
import type { UsageSummary } from "../types";

type Props = {
  summaries: UsageSummary[];
};

export function UsagePanel({ summaries }: Props) {
  return (
    <section className="usage-panel">
      {summaries.map((summary) => (
        <article className="usage-row" key={summary.agent}>
          <strong>{summary.agent}</strong>
          <span>{summary.totalInputTokens.toLocaleString()} in</span>
          <span>{summary.totalOutputTokens.toLocaleString()} out</span>
        </article>
      ))}
    </section>
  );
}
```

- [ ] **Step 3: Wire frontend to Tauri commands**

Modify `src/App.tsx`:

```tsx
import { invoke } from "@tauri-apps/api/core";
import { RefreshCw } from "lucide-react";
import { useEffect, useState } from "react";
import { FloatingPet } from "./components/FloatingPet";
import { UsagePanel } from "./components/UsagePanel";
import type { QuotaSummary, UsageSummary } from "./types";

export default function App() {
  const [summaries, setSummaries] = useState<UsageSummary[]>([]);
  const [codexQuota, setCodexQuota] = useState<QuotaSummary | undefined>();
  const [quotaError, setQuotaError] = useState<string | undefined>();
  const [syncing, setSyncing] = useState(false);

  async function refresh() {
    setSyncing(true);
    try {
      await invoke("sync_now");
      const next = await invoke<UsageSummary[]>("get_usage_summary");
      setSummaries(next);
      try {
        const quota = await invoke<QuotaSummary>("refresh_codex_quota");
        setCodexQuota(quota);
        setQuotaError(undefined);
      } catch (error) {
        setQuotaError(String(error));
      }
    } finally {
      setSyncing(false);
    }
  }

  useEffect(() => {
    refresh();
  }, []);

  const totalTokens = summaries.reduce(
    (sum, item) => sum + item.totalInputTokens + item.totalOutputTokens,
    0,
  );
  const placeholderApiValue = totalTokens / 1_000_000;

  return (
    <main className="app-shell">
      <FloatingPet
        apiValueUsd={placeholderApiValue}
        monthlyCostUsd={120}
        quota={codexQuota}
        quotaError={quotaError}
      />
      <button className="icon-button" onClick={refresh} disabled={syncing} aria-label="Refresh usage">
        <RefreshCw size={16} />
      </button>
      <UsagePanel summaries={summaries} />
    </main>
  );
}
```

- [ ] **Step 4: Update CSS for states**

Append to `src/app.css`:

```css
.mood-sleepy .pet-face {
  background: #8ea0ad;
}

.mood-working .pet-face {
  background: #2f80ed;
}

.mood-ready .pet-face {
  background: #f2c94c;
}

.mood-paid .pet-face {
  background: #2fbf71;
}

.mood-wild .pet-face {
  background: #eb5757;
  animation: pet-bounce 420ms infinite alternate ease-in-out;
}

.icon-button {
  position: fixed;
  right: 10px;
  top: 10px;
  width: 32px;
  height: 32px;
  border: 1px solid rgba(28, 32, 38, 0.16);
  border-radius: 8px;
  background: rgba(255, 255, 255, 0.92);
  display: grid;
  place-items: center;
  color: #101820;
}

.usage-panel {
  position: fixed;
  left: 10px;
  right: 10px;
  bottom: 10px;
  display: grid;
  gap: 6px;
}

.usage-row {
  min-height: 32px;
  display: grid;
  grid-template-columns: 1fr auto auto;
  gap: 10px;
  align-items: center;
  padding: 6px 8px;
  border: 1px solid rgba(28, 32, 38, 0.12);
  border-radius: 8px;
  background: rgba(255, 255, 255, 0.88);
  color: #101820;
  font-size: 12px;
}

@keyframes pet-bounce {
  from {
    transform: translateY(0);
  }
  to {
    transform: translateY(-8px);
  }
}
```

- [ ] **Step 5: Run frontend checks**

Run:

```bash
pnpm build
```

Expected:

- TypeScript build and Vite build both pass.

- [ ] **Step 6: Commit**

```bash
git add src
git commit -m "feat: render roi pet summary"
```

---

## Task 10: Replace Placeholder Value With Real Cost Summary

**Files:**
- Modify: `src-tauri/src/commands.rs`
- Modify: `src/types.ts`
- Modify: `src/App.tsx`

- [ ] **Step 1: Extend summary command with API-equivalent value**

Modify `UsageSummary` in `src-tauri/src/commands.rs`:

```rust
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageSummary {
    pub agent: String,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub total_cache_read_tokens: i64,
    pub total_cache_write_tokens: i64,
    pub api_value_usd: f64,
    pub monthly_cost_usd: f64,
    pub unpriced_models: Vec<String>,
}
```

Replace `get_usage_summary` with:

```rust
#[tauri::command]
pub fn get_usage_summary(state: State<AppState>) -> Result<Vec<UsageSummary>, String> {
    let _guard = state.lock.lock().map_err(|_| "summary lock poisoned".to_string())?;
    let conn = db::open_database(&state.db_path).map_err(|error| error.to_string())?;
    let mut stmt = conn
        .prepare(
            "
            SELECT e.agent,
                   SUM(e.input_tokens),
                   SUM(e.output_tokens),
                   SUM(e.cache_read_tokens),
                   SUM(e.cache_write_tokens),
                   SUM(
                     CASE
                       WHEN p.model IS NULL THEN 0
                       WHEN e.input_token_semantics = 'total_includes_cache' THEN
                         ((MAX(e.input_tokens - e.cache_read_tokens - e.cache_write_tokens, 0) * CAST(p.input_usd_per_million AS REAL)) / 1000000.0)
                         + ((e.output_tokens * CAST(p.output_usd_per_million AS REAL)) / 1000000.0)
                         + ((e.cache_read_tokens * CAST(p.cache_read_usd_per_million AS REAL)) / 1000000.0)
                         + ((e.cache_write_tokens * CAST(p.cache_write_usd_per_million AS REAL)) / 1000000.0)
                       ELSE
                         ((e.input_tokens * CAST(p.input_usd_per_million AS REAL)) / 1000000.0)
                         + ((e.output_tokens * CAST(p.output_usd_per_million AS REAL)) / 1000000.0)
                         + ((e.cache_read_tokens * CAST(p.cache_read_usd_per_million AS REAL)) / 1000000.0)
                         + ((e.cache_write_tokens * CAST(p.cache_write_usd_per_million AS REAL)) / 1000000.0)
                     END
                   ) AS api_value_usd,
                   COALESCE(CAST(s.monthly_cost_usd AS REAL), 0)
            FROM usage_events e
            LEFT JOIN model_prices p ON p.model = e.model
            LEFT JOIN subscriptions s ON s.agent = e.agent
            GROUP BY e.agent
            ORDER BY e.agent
            ",
        )
        .map_err(|error| error.to_string())?;

    let rows = stmt
        .query_map([], |row| {
            Ok(UsageSummary {
                agent: row.get(0)?,
                total_input_tokens: row.get(1)?,
                total_output_tokens: row.get(2)?,
                total_cache_read_tokens: row.get(3)?,
                total_cache_write_tokens: row.get(4)?,
                api_value_usd: row.get(5)?,
                monthly_cost_usd: row.get(6)?,
                unpriced_models: Vec::new(),
            })
        })
        .map_err(|error| error.to_string())?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}
```

- [ ] **Step 2: Update frontend types and aggregation**

Modify `src/types.ts`:

```ts
export type RateWindowSnapshot = {
  usedPercent: number;
  windowMinutes?: number;
  resetsAt?: number;
};

export type QuotaSummary = {
  agent: string;
  source: string;
  primary?: RateWindowSnapshot;
  secondary?: RateWindowSnapshot;
  creditBalance?: string;
  creditLimit?: string;
  creditUsed?: string;
  creditRemainingPercent?: number;
  creditResetsAt?: number;
  planType?: string;
  updatedAt: number;
};

export type UsageSummary = {
  agent: string;
  totalInputTokens: number;
  totalOutputTokens: number;
  totalCacheReadTokens: number;
  totalCacheWriteTokens: number;
  apiValueUsd: number;
  monthlyCostUsd: number;
  unpricedModels: string[];
};
```

Modify the summary calculation in `src/App.tsx`:

```tsx
  const apiValueUsd = summaries.reduce((sum, item) => sum + item.apiValueUsd, 0);
  const monthlyCostUsd = summaries.reduce((sum, item) => sum + item.monthlyCostUsd, 0);
```

Modify the `FloatingPet` call:

```tsx
      <FloatingPet
        apiValueUsd={apiValueUsd}
        monthlyCostUsd={monthlyCostUsd || 1}
        quota={codexQuota}
        quotaError={quotaError}
      />
```

Remove the previous `totalTokens` and `placeholderApiValue` variables.

- [ ] **Step 3: Run full checks**

Run:

```bash
pnpm build
cd src-tauri && cargo test
```

Expected:

- Frontend build passes.
- Rust tests pass.

- [ ] **Step 4: Commit**

```bash
git add src src-tauri/src/commands.rs
git commit -m "feat: calculate subscription payback"
```

---

## Task 11: Add Settings UI for Subscription Costs

**Files:**
- Create: `src/components/SettingsPanel.tsx`
- Modify: `src/App.tsx`
- Modify: `src/app.css`
- Modify: `src-tauri/src/commands.rs`

- [ ] **Step 1: Add settings commands**

Add to `src-tauri/src/commands.rs`:

```rust
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveSubscriptionRequest {
    pub agent: String,
    pub label: String,
    pub monthly_cost_usd: String,
    pub billing_cycle_day: i64,
}

#[tauri::command]
pub fn save_subscription(
    state: State<AppState>,
    request: SaveSubscriptionRequest,
) -> Result<(), String> {
    let _guard = state.lock.lock().map_err(|_| "settings lock poisoned".to_string())?;
    let conn = db::open_database(&state.db_path).map_err(|error| error.to_string())?;
    conn.execute(
        "
        INSERT INTO subscriptions (agent, label, monthly_cost_usd, billing_cycle_day)
        VALUES (?1, ?2, ?3, ?4)
        ON CONFLICT(agent) DO UPDATE SET
          label = excluded.label,
          monthly_cost_usd = excluded.monthly_cost_usd,
          billing_cycle_day = excluded.billing_cycle_day
        ",
        rusqlite::params![
            request.agent,
            request.label,
            request.monthly_cost_usd,
            request.billing_cycle_day
        ],
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}
```

Add `commands::save_subscription` to `generate_handler`.

- [ ] **Step 2: Create settings component**

Create `src/components/SettingsPanel.tsx`:

```tsx
import { invoke } from "@tauri-apps/api/core";
import { Save } from "lucide-react";
import { useState } from "react";

type Props = {
  onSaved: () => void;
};

export function SettingsPanel({ onSaved }: Props) {
  const [codexCost, setCodexCost] = useState("20");
  const [claudeCost, setClaudeCost] = useState("100");

  async function save() {
    await invoke("save_subscription", {
      request: {
        agent: "codex",
        label: "Codex Custom",
        monthlyCostUsd: codexCost,
        billingCycleDay: 1,
      },
    });
    await invoke("save_subscription", {
      request: {
        agent: "claude",
        label: "Claude Code Custom",
        monthlyCostUsd: claudeCost,
        billingCycleDay: 1,
      },
    });
    onSaved();
  }

  return (
    <section className="settings-panel">
      <label>
        <span>Codex</span>
        <input value={codexCost} onChange={(event) => setCodexCost(event.target.value)} />
      </label>
      <label>
        <span>Claude Code</span>
        <input value={claudeCost} onChange={(event) => setClaudeCost(event.target.value)} />
      </label>
      <button className="save-button" onClick={save}>
        <Save size={16} />
        <span>Save</span>
      </button>
    </section>
  );
}
```

- [ ] **Step 3: Render settings**

Modify `src/App.tsx` imports:

```tsx
import { SettingsPanel } from "./components/SettingsPanel";
```

Add under `UsagePanel`:

```tsx
      <SettingsPanel onSaved={refresh} />
```

- [ ] **Step 4: Add settings styles**

Append to `src/app.css`:

```css
.settings-panel {
  position: fixed;
  left: 10px;
  right: 10px;
  top: 48px;
  display: grid;
  grid-template-columns: 1fr 1fr auto;
  gap: 8px;
  padding: 8px;
  border: 1px solid rgba(28, 32, 38, 0.12);
  border-radius: 8px;
  background: rgba(255, 255, 255, 0.9);
}

.settings-panel label {
  display: grid;
  gap: 3px;
  font-size: 11px;
  color: #52606d;
}

.settings-panel input {
  min-width: 0;
  height: 28px;
  border: 1px solid rgba(28, 32, 38, 0.18);
  border-radius: 6px;
  padding: 0 8px;
  font-size: 13px;
}

.save-button {
  width: 72px;
  height: 28px;
  align-self: end;
  border: 1px solid rgba(28, 32, 38, 0.16);
  border-radius: 8px;
  background: #101820;
  color: white;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
}
```

- [ ] **Step 5: Verify**

Run:

```bash
pnpm build
cd src-tauri && cargo test
```

Expected:

- Frontend build passes.
- Rust tests pass.

- [ ] **Step 6: Commit**

```bash
git add src src-tauri/src/commands.rs src-tauri/src/main.rs
git commit -m "feat: configure subscription costs"
```

---

## Task 12: Final Verification

**Files:**
- Modify only files needed to fix failures discovered during verification.

- [ ] **Step 1: Run full validation**

Run:

```bash
pnpm build
cd src-tauri && cargo test
cd .. && pnpm tauri dev
```

Expected:

- Frontend build passes.
- Rust tests pass.
- App opens a floating always-on-top window.
- Clicking refresh imports local logs without crashing when `~/.claude` or `~/.codex` does not exist.
- When local logs exist, usage rows appear for `claude`, `codex`, or both.
- When the Codex CLI is installed and authenticated, the pet displays provider-reported quota pressure such as primary or secondary used percentage.
- When the Codex CLI is unavailable, the pet displays `quota unavailable` and still shows usage and payback values.

- [ ] **Step 2: Manual data verification**

Use a temporary home directory containing the fixtures:

```bash
mkdir -p /tmp/ai-roi-pet-home/.claude/projects/demo
mkdir -p /tmp/ai-roi-pet-home/.codex/sessions/2026/08/06
cp src-tauri/tests/fixtures/claude-simple.jsonl /tmp/ai-roi-pet-home/.claude/projects/demo/session.jsonl
cp src-tauri/tests/fixtures/codex-simple.jsonl /tmp/ai-roi-pet-home/.codex/sessions/2026/08/06/rollout-11111111-1111-1111-1111-111111111111.jsonl
```

Expected imported tokens:

- Claude: input `1000`, output `250`, cache read `300`, cache write `40`.
- Codex: input `1200`, output `60`, cache read `720`, cache write `0`.

- [ ] **Step 3: Commit verification fixes**

If any verification fixes were needed:

```bash
git add .
git commit -m "fix: stabilize roi pet verification"
```

If no fixes were needed:

```bash
git status --short
```

Expected:

- Working tree contains only intentional uncommitted local files, or is clean.

---

## Follow-Up Roadmap

After MVP:

- Add editable model price table UI.
- Add price source metadata and price version locking per usage event.
- Add a “this month” billing-cycle filter instead of lifetime totals.
- Add a small chart for daily API-equivalent value.
- Add historical quota snapshots and reset-time alerts.
- Add an explicit-consent ChatGPT dashboard fallback for Codex quota using browser session state only after the CLI `app-server` path is stable.
- Add optional Codex reset-credit endpoint support if the installed Codex account API exposes it.
- Add a setting to ignore selected projects or sessions.
- Add menu-bar/tray controls.
- Add proxy-based real-time usage capture for users who want more accuracy.
- Add automatic price import from a user-reviewed JSON file.
- Add export to CSV.

---

## Self-Review

Spec coverage:

- Desktop floating pet: covered by Task 1 and Task 9.
- Per-agent usage: covered by Task 3, Task 4, Task 6, Task 8, and Task 9.
- Money conversion: covered by Task 2, Task 7, and Task 10.
- Payback status: covered by Task 9 and Task 10.
- Codex quota display: covered by Task 6A, Task 8, Task 9, and Task 12.
- Separation of payback and quota: covered by Data Semantics, Task 6A, Task 8, and Task 9.
- Codex and Claude Code support: covered by Task 3 and Task 4.
- Token availability uncertainty: handled by local logs first, unknown models, and editable prices.
- Success form after payback: covered by `mood-paid` and `mood-wild`.
- More usage means more excited expression: covered by `moodFromPaybackRatio`.

Placeholder scan:

- The plan contains no required implementation placeholders.
- Follow-up roadmap items are explicitly out of MVP scope.

Type consistency:

- Rust `UsageEvent`, `ModelPrice`, and `UsageSummary` names are consistent across tasks.
- Frontend camelCase command results match Tauri `serde(rename_all = "camelCase")`.
- Agent identifiers are consistently `claude` and `codex` in persistence and UI.
