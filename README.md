# AI ROI Pet

Local-first desktop pet for estimating whether Codex and Claude Code subscriptions have paid for themselves.

The app reads local usage logs and applies editable API-equivalent model prices. Built-in prices are defaults only; users should verify them against the official provider pricing pages before relying on the dollar value.

Payback is calculated per subscription period:

```text
API-equivalent value in the current subscription period / configured subscription cost
```

Codex usage is read from local Codex session logs. Claude Code usage is read from local Claude project logs. Official provider quota or rate-limit windows are not used for payback.

## Configuration

- Set the monthly cost and billing cycle day for Codex and Claude Code in the settings panel.
- Use `自动识别订阅期` to try provider adapters. The app only applies high-confidence billing fields such as `current_period_start`, `current_period_end`, `renewal_date`, or `next_invoice_at`; rate-limit reset windows are never treated as a billing cycle.
- Set a custom desktop pet image path in settings. Transparent PNG images work best.
- Built-in model prices are seed defaults for common OpenAI and Anthropic coding models. The app matches exact model names first, then falls back to model families such as `gpt-5.5`, `gpt-5.4-mini`, `gpt-5`, and `claude-sonnet`.
- Unknown models are kept as usage, but cannot contribute dollar value until a price is configured or matched by a fallback.
