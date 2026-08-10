use crate::quota::models::CodexQuotaSnapshot;
use crate::usage::models::{InputTokenSemantics, ModelPrice, UsageEvent};
use rusqlite::{params, Connection};
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

        CREATE TABLE IF NOT EXISTS app_config (
          key TEXT PRIMARY KEY,
          value TEXT NOT NULL
        );
        ",
    )?;
    conn.execute_batch(
        "
        ALTER TABLE subscriptions ADD COLUMN cost_mode TEXT NOT NULL DEFAULT 'subscription';
        ",
    )
    .or_else(|error| {
        if matches!(error, rusqlite::Error::SqliteFailure(_, Some(ref message)) if message.contains("duplicate column name")) {
            Ok(())
        } else {
            Err(error)
        }
    })
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
    if rows > 0 {
        return Ok(true);
    }

    if event.model != "unknown" {
        let updated = conn.execute(
            "
            UPDATE usage_events
            SET model = ?2
            WHERE id = ?1 AND model = 'unknown'
            ",
            params![event.id, event.model],
        )?;
        return Ok(updated > 0);
    }

    Ok(false)
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

pub fn replace_codex_quota_snapshot(
    conn: &mut Connection,
    snapshot: &CodexQuotaSnapshot,
) -> rusqlite::Result<()> {
    let tx = conn.transaction()?;
    let snapshot_id = "codex:codex-app-server:latest";

    tx.execute(
        "DELETE FROM quota_extra_windows WHERE snapshot_id = ?1",
        [snapshot_id],
    )?;
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
        params![
            snapshot_id,
            snapshot.source.as_str(),
            snapshot.primary.as_ref().map(|value| value.used_percent),
            snapshot
                .primary
                .as_ref()
                .and_then(|value| value.window_minutes),
            snapshot.primary.as_ref().and_then(|value| value.resets_at),
            snapshot.secondary.as_ref().map(|value| value.used_percent),
            snapshot
                .secondary
                .as_ref()
                .and_then(|value| value.window_minutes),
            snapshot
                .secondary
                .as_ref()
                .and_then(|value| value.resets_at),
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
            params![
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::usage::models::{Agent, InputTokenSemantics, UsageEvent};

    #[test]
    fn inserts_usage_events_idempotently() {
        let conn = Connection::open_in_memory().unwrap();
        migrate(&conn).unwrap();
        let event = UsageEvent {
            id: "claude:session:msg_1".to_string(),
            agent: Agent::Claude,
            source: "session_log".to_string(),
            session_id: Some("msg_1".to_string()),
            model: "claude-sonnet-5".to_string(),
            input_tokens: 100,
            output_tokens: 20,
            cache_read_tokens: 10,
            cache_write_tokens: 5,
            input_token_semantics: InputTokenSemantics::Fresh,
            occurred_at: 1,
        };

        assert!(insert_usage_event(&conn, &event).unwrap());
        assert!(!insert_usage_event(&conn, &event).unwrap());
    }
}
