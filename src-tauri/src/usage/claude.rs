use super::models::{Agent, InputTokenSemantics, UsageEvent};
use chrono::DateTime;
use serde_json::Value;
use std::io::{BufRead, BufReader, Read};

fn timestamp_to_epoch(value: Option<&Value>) -> i64 {
    value
        .and_then(Value::as_str)
        .and_then(|timestamp| DateTime::parse_from_rfc3339(timestamp).ok())
        .map(|timestamp| timestamp.timestamp())
        .unwrap_or(0)
}

pub fn parse_claude_jsonl(reader: impl Read) -> Vec<UsageEvent> {
    let mut events = Vec::new();

    for line in BufReader::new(reader).lines().map_while(Result::ok) {
        let Ok(value) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        if value.get("type").and_then(Value::as_str) != Some("assistant") {
            continue;
        }

        let Some(message) = value.get("message") else {
            continue;
        };
        let Some(message_id) = message.get("id").and_then(Value::as_str) else {
            continue;
        };
        let Some(model) = message.get("model").and_then(Value::as_str) else {
            continue;
        };
        let Some(usage) = message.get("usage") else {
            continue;
        };

        events.push(UsageEvent {
            id: format!("claude:session:{message_id}"),
            agent: Agent::Claude,
            source: "session_log".to_string(),
            session_id: Some(message_id.to_string()),
            model: model.to_string(),
            input_tokens: usage
                .get("input_tokens")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            output_tokens: usage
                .get("output_tokens")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            cache_read_tokens: usage
                .get("cache_read_input_tokens")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            cache_write_tokens: usage
                .get("cache_creation_input_tokens")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            input_token_semantics: InputTokenSemantics::Fresh,
            occurred_at: timestamp_to_epoch(value.get("timestamp")),
        });
    }

    events
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imports_claude_assistant_usage() {
        let fixture = include_bytes!("../../tests/fixtures/claude-simple.jsonl");

        let events = parse_claude_jsonl(&fixture[..]);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].id, "claude:session:msg_1");
        assert_eq!(events[0].model, "claude-sonnet-5");
        assert_eq!(events[0].input_tokens, 1000);
        assert_eq!(events[0].output_tokens, 250);
        assert_eq!(events[0].cache_read_tokens, 300);
        assert_eq!(events[0].cache_write_tokens, 40);
        assert_eq!(events[0].input_token_semantics, InputTokenSemantics::Fresh);
    }
}
