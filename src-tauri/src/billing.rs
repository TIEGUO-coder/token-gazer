use chrono::{DateTime, Datelike, Local, TimeZone};
use serde::Serialize;
use serde_json::Value;
use std::{fs, process::Command};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BillingProbeResult {
    pub provider: String,
    pub billing_cycle_day: Option<i64>,
    pub period_start: Option<i64>,
    pub period_end: Option<i64>,
    pub plan_name: Option<String>,
    pub confidence: String,
    pub source: String,
    pub note: String,
}

fn result(provider: &str, confidence: &str, source: &str, note: &str) -> BillingProbeResult {
    BillingProbeResult {
        provider: provider.to_string(),
        billing_cycle_day: None,
        period_start: None,
        period_end: None,
        plan_name: None,
        confidence: confidence.to_string(),
        source: source.to_string(),
        note: note.to_string(),
    }
}

fn explicit_period_keys(key: &str) -> bool {
    matches!(
        key,
        "current_period_start"
            | "currentPeriodStart"
            | "billing_period_start"
            | "billingPeriodStart"
            | "period_start"
            | "periodStart"
            | "subscription_period_start"
            | "subscriptionPeriodStart"
            | "current_period_end"
            | "currentPeriodEnd"
            | "billing_period_end"
            | "billingPeriodEnd"
            | "period_end"
            | "periodEnd"
            | "subscription_period_end"
            | "subscriptionPeriodEnd"
            | "renewal_date"
            | "renewalDate"
            | "renews_at"
            | "renewsAt"
            | "next_invoice_at"
            | "nextInvoiceAt"
    )
}

fn start_key(key: &str) -> bool {
    key.ends_with("_start")
        || key.ends_with("Start")
        || key == "current_period_start"
        || key == "billing_period_start"
        || key == "period_start"
}

fn end_key(key: &str) -> bool {
    key.ends_with("_end")
        || key.ends_with("End")
        || key == "renewal_date"
        || key == "renewalDate"
        || key == "renews_at"
        || key == "renewsAt"
        || key == "next_invoice_at"
        || key == "nextInvoiceAt"
}

fn parse_time(value: &Value) -> Option<i64> {
    if let Some(seconds) = value.as_i64() {
        return Some(if seconds > 10_000_000_000 {
            seconds / 1000
        } else {
            seconds
        });
    }
    let raw = value.as_str()?;
    DateTime::parse_from_rfc3339(raw)
        .map(|date| date.timestamp())
        .ok()
        .or_else(|| {
            chrono::NaiveDate::parse_from_str(raw, "%Y-%m-%d")
                .ok()
                .and_then(|date| {
                    Local
                        .from_local_datetime(&date.and_hms_opt(0, 0, 0)?)
                        .single()
                })
                .map(|date| date.timestamp())
        })
}

fn walk_json(value: &Value, found: &mut Vec<(String, i64)>, plan: &mut Option<String>) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                if plan.is_none()
                    && matches!(
                        key.as_str(),
                        "plan"
                            | "planName"
                            | "plan_name"
                            | "subscriptionPlan"
                            | "subscription_plan"
                    )
                {
                    if let Some(name) = child.as_str() {
                        *plan = Some(name.to_string());
                    }
                }
                if explicit_period_keys(key) {
                    if let Some(timestamp) = parse_time(child) {
                        found.push((key.clone(), timestamp));
                    }
                }
                walk_json(child, found, plan);
            }
        }
        Value::Array(items) => {
            for child in items {
                walk_json(child, found, plan);
            }
        }
        _ => {}
    }
}

pub fn billing_result_from_json(provider: &str, source: &str, json: &Value) -> BillingProbeResult {
    let mut found = Vec::new();
    let mut plan = None;
    walk_json(json, &mut found, &mut plan);

    let period_start = found
        .iter()
        .find_map(|(key, value)| start_key(key).then_some(*value));
    let period_end = found
        .iter()
        .find_map(|(key, value)| end_key(key).then_some(*value));
    let anchor = period_start.or(period_end);
    let Some(anchor) = anchor else {
        let mut miss = result(
            provider,
            "low",
            source,
            "只检测到用量/限额窗口，未发现明确订阅周期字段",
        );
        miss.plan_name = plan;
        return miss;
    };

    let billing_cycle_day = Local
        .timestamp_opt(anchor, 0)
        .single()
        .map(|date| date.day() as i64);
    BillingProbeResult {
        provider: provider.to_string(),
        billing_cycle_day,
        period_start,
        period_end,
        plan_name: plan,
        confidence: "high".to_string(),
        source: source.to_string(),
        note: "检测到明确订阅周期字段".to_string(),
    }
}

fn curl_json(url: &str, headers: &[(&str, String)]) -> Result<Value, String> {
    let mut command = Command::new("curl");
    command.args(["--silent", "--show-error", "--fail", "--max-time", "8"]);
    for (name, value) in headers {
        command.args(["-H", &format!("{name}: {value}")]);
    }
    command.arg(url);
    let output = command
        .output()
        .map_err(|error| format!("failed to run curl: {error}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    serde_json::from_slice(&output.stdout).map_err(|error| error.to_string())
}

fn codex_auth() -> Option<(String, Option<String>)> {
    let home = dirs::home_dir()?;
    for path in [
        home.join(".codex/auth.json"),
        home.join(".config/codex/auth.json"),
    ] {
        let text = fs::read_to_string(path).ok()?;
        let json: Value = serde_json::from_str(&text).ok()?;
        let tokens = json.get("tokens")?;
        let access = tokens.get("access_token")?.as_str()?.to_string();
        let account = tokens
            .get("account_id")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);
        return Some((access, account));
    }
    None
}

fn keychain_password(service: &str) -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        let output = Command::new("/usr/bin/security")
            .args(["find-generic-password", "-s", service, "-w"])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
        (!text.is_empty()).then_some(text)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = service;
        None
    }
}

fn decode_hex_if_needed(text: &str) -> String {
    if text.starts_with('{')
        || text.len() % 2 != 0
        || !text.chars().all(|char| char.is_ascii_hexdigit())
    {
        return text.to_string();
    }
    let bytes = (0..text.len())
        .step_by(2)
        .filter_map(|index| u8::from_str_radix(&text[index..index + 2], 16).ok())
        .collect::<Vec<_>>();
    String::from_utf8(bytes).unwrap_or_else(|_| text.to_string())
}

fn claude_token() -> Option<String> {
    let raw = keychain_password("Claude Code-credentials").or_else(|| {
        let home = dirs::home_dir()?;
        fs::read_to_string(home.join(".claude/.credentials.json")).ok()
    })?;
    let decoded = decode_hex_if_needed(raw.trim());
    let json: Value = serde_json::from_str(&decoded).ok()?;
    json.get("claudeAiOauth")?
        .get("accessToken")?
        .as_str()
        .map(ToOwned::to_owned)
}

pub fn probe_codex_billing() -> BillingProbeResult {
    let Some((token, account)) = codex_auth() else {
        return result("codex", "low", "oauth", "未找到 Codex 登录凭据");
    };
    let mut headers = vec![
        ("Authorization", format!("Bearer {token}")),
        ("Accept", "application/json".to_string()),
        ("User-Agent", "AI ROI Pet".to_string()),
    ];
    if let Some(account) = account {
        headers.push(("ChatGPT-Account-Id", account));
    }
    match curl_json("https://chatgpt.com/backend-api/wham/usage", &headers) {
        Ok(json) => billing_result_from_json("codex", "oauth", &json),
        Err(error) => result("codex", "low", "oauth", &format!("Codex 查询失败：{error}")),
    }
}

pub fn probe_claude_billing() -> BillingProbeResult {
    let Some(token) = claude_token() else {
        return result("claude", "low", "oauth", "未找到 Claude Code 登录凭据");
    };
    let headers = vec![
        ("Authorization", format!("Bearer {token}")),
        ("Accept", "application/json".to_string()),
        ("Content-Type", "application/json".to_string()),
        ("anthropic-beta", "oauth-2025-04-20".to_string()),
        ("User-Agent", "claude-code/2.1.69".to_string()),
    ];
    match curl_json("https://api.anthropic.com/api/oauth/usage", &headers) {
        Ok(json) => billing_result_from_json("claude", "oauth", &json),
        Err(error) => result(
            "claude",
            "low",
            "oauth",
            &format!("Claude 查询失败：{error}"),
        ),
    }
}

pub fn probe_billing_periods() -> Vec<BillingProbeResult> {
    vec![probe_codex_billing(), probe_claude_billing()]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn extracts_high_confidence_billing_cycle_from_period_json() {
        let result = billing_result_from_json(
            "codex",
            "web",
            &json!({
                "subscription": {
                    "planName": "Plus",
                    "current_period_start": "2026-07-15T00:00:00+08:00",
                    "current_period_end": "2026-08-15T00:00:00+08:00"
                }
            }),
        );

        assert_eq!(result.confidence, "high");
        assert_eq!(result.billing_cycle_day, Some(15));
        assert_eq!(result.plan_name, Some("Plus".to_string()));
    }

    #[test]
    fn does_not_treat_usage_reset_as_billing_cycle() {
        let result = billing_result_from_json(
            "codex",
            "oauth",
            &json!({
                "rate_limit": {
                    "primary_window": {
                        "used_percent": 70,
                        "resets_in_seconds": 1200
                    }
                }
            }),
        );

        assert_eq!(result.confidence, "low");
        assert_eq!(result.billing_cycle_day, None);
    }
}
