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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InputTokenSemantics {
    Fresh,
    TotalIncludesCache,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelPrice {
    pub model: String,
    pub input_usd_per_million: String,
    pub output_usd_per_million: String,
    pub cache_read_usd_per_million: String,
    pub cache_write_usd_per_million: String,
    pub source: String,
    pub effective_at: i64,
}
