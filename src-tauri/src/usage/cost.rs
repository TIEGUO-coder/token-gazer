use super::models::{InputTokenSemantics, ModelPrice, UsageEvent};

fn parse_price(value: &str) -> f64 {
    value.parse::<f64>().unwrap_or(0.0)
}

pub fn event_cost_usd(event: &UsageEvent, price: &ModelPrice) -> f64 {
    let input_price = parse_price(&price.input_usd_per_million);
    let output_price = parse_price(&price.output_usd_per_million);
    let cache_read_price = parse_price(&price.cache_read_usd_per_million);
    let cache_write_price = parse_price(&price.cache_write_usd_per_million);

    let fresh_input_tokens = match event.input_token_semantics {
        InputTokenSemantics::Fresh => event.input_tokens,
        InputTokenSemantics::TotalIncludesCache => event
            .input_tokens
            .saturating_sub(event.cache_read_tokens)
            .saturating_sub(event.cache_write_tokens),
    };

    ((fresh_input_tokens as f64 * input_price)
        + (event.output_tokens as f64 * output_price)
        + (event.cache_read_tokens as f64 * cache_read_price)
        + (event.cache_write_tokens as f64 * cache_write_price))
        / 1_000_000.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::usage::models::{Agent, InputTokenSemantics, UsageEvent};

    #[test]
    fn prices_codex_total_input_without_double_counting_cache_read() {
        let event = UsageEvent {
            id: "codex:1".to_string(),
            agent: Agent::Codex,
            source: "session_log".to_string(),
            session_id: None,
            model: "gpt-5.4".to_string(),
            input_tokens: 1_000_000,
            output_tokens: 100_000,
            cache_read_tokens: 600_000,
            cache_write_tokens: 0,
            input_token_semantics: InputTokenSemantics::TotalIncludesCache,
            occurred_at: 0,
        };
        let price = ModelPrice {
            model: "gpt-5.4".to_string(),
            input_usd_per_million: "1.25".to_string(),
            output_usd_per_million: "10".to_string(),
            cache_read_usd_per_million: "0.125".to_string(),
            cache_write_usd_per_million: "0".to_string(),
            source: "test".to_string(),
            effective_at: 0,
        };

        assert!((event_cost_usd(&event, &price) - 1.575).abs() < 0.0001);
    }
}
