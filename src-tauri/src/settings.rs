use crate::db::upsert_model_price;
use crate::usage::models::ModelPrice;
use rusqlite::{params, Connection};

pub fn seed_defaults(conn: &Connection, now: i64) -> rusqlite::Result<()> {
    let prices = vec![
        ModelPrice {
            model: "claude-opus".to_string(),
            input_usd_per_million: "15".to_string(),
            output_usd_per_million: "75".to_string(),
            cache_read_usd_per_million: "1.50".to_string(),
            cache_write_usd_per_million: "18.75".to_string(),
            source: "builtin-editable".to_string(),
            effective_at: now,
        },
        ModelPrice {
            model: "claude-sonnet-4".to_string(),
            input_usd_per_million: "3".to_string(),
            output_usd_per_million: "15".to_string(),
            cache_read_usd_per_million: "0.3".to_string(),
            cache_write_usd_per_million: "3.75".to_string(),
            source: "builtin-editable".to_string(),
            effective_at: now,
        },
        ModelPrice {
            model: "claude-sonnet-4-5".to_string(),
            input_usd_per_million: "3".to_string(),
            output_usd_per_million: "15".to_string(),
            cache_read_usd_per_million: "0.3".to_string(),
            cache_write_usd_per_million: "3.75".to_string(),
            source: "builtin-editable".to_string(),
            effective_at: now,
        },
        ModelPrice {
            model: "claude-sonnet".to_string(),
            input_usd_per_million: "3".to_string(),
            output_usd_per_million: "15".to_string(),
            cache_read_usd_per_million: "0.3".to_string(),
            cache_write_usd_per_million: "3.75".to_string(),
            source: "builtin-editable".to_string(),
            effective_at: now,
        },
        ModelPrice {
            model: "claude-haiku".to_string(),
            input_usd_per_million: "0.80".to_string(),
            output_usd_per_million: "4".to_string(),
            cache_read_usd_per_million: "0.08".to_string(),
            cache_write_usd_per_million: "1".to_string(),
            source: "builtin-editable".to_string(),
            effective_at: now,
        },
        ModelPrice {
            model: "claude-sonnet-5".to_string(),
            input_usd_per_million: "3".to_string(),
            output_usd_per_million: "15".to_string(),
            cache_read_usd_per_million: "0.3".to_string(),
            cache_write_usd_per_million: "3.75".to_string(),
            source: "builtin-editable".to_string(),
            effective_at: now,
        },
        ModelPrice {
            model: "gpt-5".to_string(),
            input_usd_per_million: "1.25".to_string(),
            output_usd_per_million: "10".to_string(),
            cache_read_usd_per_million: "0.125".to_string(),
            cache_write_usd_per_million: "0".to_string(),
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
        ModelPrice {
            model: "gpt-5.4-mini".to_string(),
            input_usd_per_million: "0.75".to_string(),
            output_usd_per_million: "4.50".to_string(),
            cache_read_usd_per_million: "0.075".to_string(),
            cache_write_usd_per_million: "0".to_string(),
            source: "builtin-editable".to_string(),
            effective_at: now,
        },
        ModelPrice {
            model: "gpt-5.5".to_string(),
            input_usd_per_million: "5".to_string(),
            output_usd_per_million: "30".to_string(),
            cache_read_usd_per_million: "0.50".to_string(),
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
