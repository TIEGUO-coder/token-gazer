use super::models::{CodexQuotaSnapshot, ExtraQuotaWindow, RateWindowSnapshot};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn as_i64(value: Option<&Value>) -> Option<i64> {
    value.and_then(Value::as_i64)
}

fn as_f64(value: Option<&Value>) -> Option<f64> {
    value.and_then(Value::as_f64)
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

fn read_response(
    reader: &mut BufReader<std::process::ChildStdout>,
    id: i64,
) -> Result<Value, String> {
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut line = String::new();

    while Instant::now() < deadline {
        line.clear();
        let bytes = reader
            .read_line(&mut line)
            .map_err(|error| error.to_string())?;
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

    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| "missing codex stdin".to_string())?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "missing codex stdout".to_string())?;
    let mut reader = BufReader::new(stdout);

    send(
        &mut stdin,
        json!({"id":1,"method":"initialize","params":{"clientInfo":{"name":"ai-roi-pet","version":"0.1.0"}}}),
    )?;
    let _ = read_response(&mut reader, 1)?;
    send(&mut stdin, json!({"method":"initialized","params":{}}))?;
    send(
        &mut stdin,
        json!({"id":2,"method":"account/rateLimits/read","params":{}}),
    )?;
    let rate_limits = read_response(&mut reader, 2)?;
    send(
        &mut stdin,
        json!({"id":3,"method":"account/read","params":{}}),
    )?;
    let account = read_response(&mut reader, 3).unwrap_or(Value::Null);
    let _ = child.kill();

    let mut extra_windows = Vec::new();
    if let Some(map) = rate_limits
        .get("rateLimitsByLimitId")
        .and_then(Value::as_object)
    {
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
        credit_remaining_percent: as_f64(
            individual_limit.and_then(|value| value.get("remainingPercent")),
        ),
        credit_resets_at: as_i64(individual_limit.and_then(|value| value.get("resetsAt"))),
        plan_type: as_string(rate_limits.get("planType"))
            .or_else(|| as_string(account.get("planType"))),
        extra_windows,
        updated_at: now,
    })
}
