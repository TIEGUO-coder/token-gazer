use super::models::{Agent, InputTokenSemantics, UsageEvent};
use chrono::DateTime;
use serde_json::Value;
use std::io::{BufRead, BufReader, Read};

#[derive(Debug, Clone, Copy)]
struct TokenSnapshot {
    input: u64,
    cached_input: u64,
    output: u64,
}

fn timestamp_to_epoch(value: Option<&Value>) -> i64 {
    value
        .and_then(Value::as_str)
        .and_then(|timestamp| DateTime::parse_from_rfc3339(timestamp).ok())
        .map(|timestamp| timestamp.timestamp())
        .unwrap_or(0)
}

fn parse_tokens(value: Option<&Value>) -> Option<TokenSnapshot> {
    let value = value?;
    Some(TokenSnapshot {
        input: value
            .get("input_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        cached_input: value
            .get("cached_input_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        output: value
            .get("output_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(0),
    })
}

fn delta(current: TokenSnapshot, previous: TokenSnapshot) -> TokenSnapshot {
    TokenSnapshot {
        input: current.input.saturating_sub(previous.input),
        cached_input: current.cached_input.saturating_sub(previous.cached_input),
        output: current.output.saturating_sub(previous.output),
    }
}

pub fn parse_codex_jsonl(reader: impl Read, file_stem: &str) -> Vec<UsageEvent> {
    let mut events = Vec::new();
    let mut previous_total = TokenSnapshot {
        input: 0,
        cached_input: 0,
        output: 0,
    };
    let mut event_index = 0_u64;
    let mut current_model = "unknown".to_string();

    for line in BufReader::new(reader).lines().map_while(Result::ok) {
        let Ok(value) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let Some(payload) = value.get("payload") else {
            continue;
        };
        if value.get("type").and_then(Value::as_str) == Some("turn_context") {
            if let Some(model) = payload.get("model").and_then(Value::as_str) {
                current_model = model.to_string();
            }
            continue;
        }
        if value.get("type").and_then(Value::as_str) != Some("event_msg") {
            continue;
        }

        if let Some(model) = payload
            .get("info")
            .and_then(|info| info.get("model"))
            .and_then(Value::as_str)
        {
            current_model = model.to_string();
        }

        if payload.get("type").and_then(Value::as_str) != Some("token_count") {
            continue;
        }

        let Some(info) = payload.get("info") else {
            continue;
        };
        let tokens = if let Some(last_usage) = parse_tokens(info.get("last_token_usage")) {
            last_usage
        } else if let Some(total_usage) = parse_tokens(info.get("total_token_usage")) {
            let next = delta(total_usage, previous_total);
            previous_total = total_usage;
            next
        } else {
            continue;
        };

        if tokens.input == 0 && tokens.cached_input == 0 && tokens.output == 0 {
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
            cache_read_tokens: tokens.cached_input,
            cache_write_tokens: 0,
            input_token_semantics: InputTokenSemantics::TotalIncludesCache,
            occurred_at: timestamp_to_epoch(value.get("timestamp")),
        });
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

    #[test]
    fn reads_model_from_turn_context_before_token_count_events() {
        let jsonl = r#"
{"timestamp":"2026-08-10T01:00:00.000Z","type":"turn_context","payload":{"model":"gpt-5.5"}}
{"timestamp":"2026-08-10T01:00:01.000Z","type":"event_msg","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":1000,"cached_input_tokens":400,"output_tokens":80}}}}
"#;

        let events = parse_codex_jsonl(jsonl.as_bytes(), "rollout-model");

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].model, "gpt-5.5");
    }
}
