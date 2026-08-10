use crate::{
    billing::{probe_billing_periods, BillingProbeResult},
    db,
    quota::{codex_app_server::fetch_codex_quota, models::RateWindowSnapshot},
    settings,
    usage::{
        cost::event_cost_usd,
        models::{Agent, InputTokenSemantics, ModelPrice, UsageEvent},
        sync::sync_usage_logs,
    },
};
use chrono::{Datelike, TimeZone};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{State, WebviewWindow};

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
    pub api_value_usd: f64,
    pub monthly_cost_usd: f64,
    pub cost_mode: String,
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveSubscriptionRequest {
    pub agent: String,
    pub label: String,
    pub monthly_cost_usd: String,
    pub billing_cycle_day: i64,
    pub cost_mode: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionSummary {
    pub agent: String,
    pub label: String,
    pub monthly_cost_usd: String,
    pub billing_cycle_day: i64,
    pub cost_mode: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfigSummary {
    pub pet_image_path: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveAppConfigRequest {
    pub pet_image_path: Option<String>,
}

#[tauri::command]
pub fn detect_billing_periods() -> Result<Vec<BillingProbeResult>, String> {
    Ok(probe_billing_periods())
}

#[tauri::command]
pub fn sync_now(state: State<AppState>) -> Result<crate::usage::sync::SyncReport, String> {
    let _guard = state
        .lock
        .lock()
        .map_err(|_| "sync lock poisoned".to_string())?;
    let conn = db::open_database(&state.db_path).map_err(|error| error.to_string())?;
    settings::seed_defaults(&conn, chrono::Utc::now().timestamp())
        .map_err(|error| error.to_string())?;
    let home = dirs::home_dir().ok_or_else(|| "home directory not found".to_string())?;
    Ok(sync_usage_logs(&conn, &home))
}

#[tauri::command]
pub fn refresh_codex_quota(state: State<AppState>) -> Result<QuotaSummary, String> {
    let _guard = state
        .lock
        .lock()
        .map_err(|_| "quota lock poisoned".to_string())?;
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
    let _guard = state
        .lock
        .lock()
        .map_err(|_| "summary lock poisoned".to_string())?;
    let conn = db::open_database(&state.db_path).map_err(|error| error.to_string())?;
    let now = chrono::Local::now().timestamp();

    usage_summary_for_billing_period(&conn, now)
}

fn safe_cycle_day(year: i32, month: u32, billing_cycle_day: i64) -> u32 {
    let requested = billing_cycle_day.clamp(1, 28) as u32;
    let last_day = last_day_of_month(year, month);
    requested.min(last_day)
}

fn last_day_of_month(year: i32, month: u32) -> u32 {
    let (next_year, next_month) = add_month(year, month);
    let next_start = chrono::Local
        .with_ymd_and_hms(next_year, next_month, 1, 0, 0, 0)
        .single()
        .expect("local next month start should exist");
    (next_start - chrono::Duration::days(1)).day()
}

fn add_month(year: i32, month: u32) -> (i32, u32) {
    if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    }
}

fn subtract_month(year: i32, month: u32) -> (i32, u32) {
    if month == 1 {
        (year - 1, 12)
    } else {
        (year, month - 1)
    }
}

fn month_boundary(year: i32, month: u32, billing_cycle_day: i64) -> i64 {
    let day = safe_cycle_day(year, month, billing_cycle_day);
    chrono::Local
        .with_ymd_and_hms(year, month, day, 0, 0, 0)
        .single()
        .expect("local billing boundary should exist")
        .timestamp()
}

fn billing_period_epoch_window(now_epoch: i64, billing_cycle_day: i64) -> (i64, i64) {
    let now = chrono::Local
        .timestamp_opt(now_epoch, 0)
        .single()
        .expect("valid current timestamp");
    let this_boundary = month_boundary(now.year(), now.month(), billing_cycle_day);
    if now_epoch >= this_boundary {
        let (next_year, next_month) = add_month(now.year(), now.month());
        (
            this_boundary,
            month_boundary(next_year, next_month, billing_cycle_day),
        )
    } else {
        let (prev_year, prev_month) = subtract_month(now.year(), now.month());
        (
            month_boundary(prev_year, prev_month, billing_cycle_day),
            this_boundary,
        )
    }
}

fn usage_summary_for_billing_period(
    conn: &rusqlite::Connection,
    now_epoch: i64,
) -> Result<Vec<UsageSummary>, String> {
    let mut subscriptions = HashMap::new();
    let mut subscription_stmt = conn
        .prepare("SELECT agent, CAST(monthly_cost_usd AS REAL), cost_mode, billing_cycle_day FROM subscriptions")
        .map_err(|error| error.to_string())?;
    let subscription_rows = subscription_stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, f64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
            ))
        })
        .map_err(|error| error.to_string())?;
    for row in subscription_rows {
        let (agent, cost, cost_mode, billing_cycle_day) = row.map_err(|error| error.to_string())?;
        subscriptions.insert(agent, (cost, cost_mode, billing_cycle_day));
    }

    let mut summaries = Vec::new();
    for agent in ["claude", "codex"] {
        let billing_cycle_day = subscriptions
            .get(agent)
            .map(|(_, _, day)| *day)
            .unwrap_or(1);
        let (period_start, period_end) = billing_period_epoch_window(now_epoch, billing_cycle_day);
        if let Some(summary) =
            usage_summary_for_period(conn, agent, period_start, period_end, &subscriptions)?
        {
            summaries.push(summary);
        }
    }
    summaries.sort_by(|left, right| left.agent.cmp(&right.agent));
    Ok(summaries)
}

fn price_for_model<'a>(
    prices: &'a HashMap<String, ModelPrice>,
    model: &str,
) -> Option<&'a ModelPrice> {
    let normalized = model.to_ascii_lowercase();
    prices.get(&normalized).or_else(|| {
        [
            "gpt-5.5",
            "gpt-5.4-mini",
            "gpt-5.4",
            "gpt-5",
            "claude-sonnet-4-5",
            "claude-sonnet-4",
            "claude-sonnet",
            "claude-opus",
            "claude-haiku",
        ]
        .into_iter()
        .find_map(|prefix| {
            normalized
                .starts_with(prefix)
                .then(|| prices.get(prefix))
                .flatten()
        })
    })
}

fn usage_summary_for_period(
    conn: &rusqlite::Connection,
    agent_filter: &str,
    period_start: i64,
    period_end: i64,
    subscriptions: &HashMap<String, (f64, String, i64)>,
) -> Result<Option<UsageSummary>, String> {
    let mut prices = HashMap::new();
    let mut price_stmt = conn
        .prepare(
            "
            SELECT model, input_usd_per_million, output_usd_per_million,
                   cache_read_usd_per_million, cache_write_usd_per_million,
                   source, effective_at
            FROM model_prices
            ",
        )
        .map_err(|error| error.to_string())?;
    let price_rows = price_stmt
        .query_map([], |row| {
            Ok(ModelPrice {
                model: row.get(0)?,
                input_usd_per_million: row.get(1)?,
                output_usd_per_million: row.get(2)?,
                cache_read_usd_per_million: row.get(3)?,
                cache_write_usd_per_million: row.get(4)?,
                source: row.get(5)?,
                effective_at: row.get(6)?,
            })
        })
        .map_err(|error| error.to_string())?;
    for price in price_rows {
        let price = price.map_err(|error| error.to_string())?;
        prices.insert(price.model.to_ascii_lowercase(), price);
    }

    #[derive(Default)]
    struct Aggregate {
        input: i64,
        output: i64,
        cache_read: i64,
        cache_write: i64,
        value: f64,
        unpriced: HashSet<String>,
    }

    let mut aggregates: HashMap<String, Aggregate> = HashMap::new();
    let mut events_stmt = conn
        .prepare(
            "
            SELECT id, agent, source, session_id, model, input_tokens, output_tokens,
                   cache_read_tokens, cache_write_tokens, input_token_semantics, occurred_at
            FROM usage_events
            WHERE agent = ?1 AND occurred_at >= ?2 AND occurred_at < ?3
            ",
        )
        .map_err(|error| error.to_string())?;
    let event_rows = events_stmt
        .query_map(
            rusqlite::params![agent_filter, period_start, period_end],
            |row| {
                let agent_name: String = row.get(1)?;
                let semantics: String = row.get(9)?;
                Ok(UsageEvent {
                    id: row.get(0)?,
                    agent: if agent_name == "codex" {
                        Agent::Codex
                    } else {
                        Agent::Claude
                    },
                    source: row.get(2)?,
                    session_id: row.get(3)?,
                    model: row.get(4)?,
                    input_tokens: row.get::<_, i64>(5)? as u64,
                    output_tokens: row.get::<_, i64>(6)? as u64,
                    cache_read_tokens: row.get::<_, i64>(7)? as u64,
                    cache_write_tokens: row.get::<_, i64>(8)? as u64,
                    input_token_semantics: if semantics == "total_includes_cache" {
                        InputTokenSemantics::TotalIncludesCache
                    } else {
                        InputTokenSemantics::Fresh
                    },
                    occurred_at: row.get(10)?,
                })
            },
        )
        .map_err(|error| error.to_string())?;

    for event in event_rows {
        let event = event.map_err(|error| error.to_string())?;
        let agent = event.agent.as_str().to_string();
        let aggregate = aggregates.entry(agent).or_default();
        aggregate.input += event.input_tokens as i64;
        aggregate.output += event.output_tokens as i64;
        aggregate.cache_read += event.cache_read_tokens as i64;
        aggregate.cache_write += event.cache_write_tokens as i64;

        if let Some(price) = price_for_model(&prices, &event.model) {
            aggregate.value += event_cost_usd(&event, price);
        } else {
            aggregate.unpriced.insert(event.model.clone());
        }
    }

    let Some(aggregate) = aggregates.remove(agent_filter) else {
        return Ok(None);
    };
    let mut unpriced_models = aggregate.unpriced.into_iter().collect::<Vec<_>>();
    unpriced_models.sort();
    Ok(Some(UsageSummary {
        monthly_cost_usd: subscriptions
            .get(agent_filter)
            .map(|(cost, mode, _)| if mode == "disabled" { 0.0 } else { *cost })
            .unwrap_or(0.0),
        cost_mode: subscriptions
            .get(agent_filter)
            .map(|(_, mode, _)| mode.clone())
            .unwrap_or_else(|| "subscription".to_string()),
        agent: agent_filter.to_string(),
        total_input_tokens: aggregate.input,
        total_output_tokens: aggregate.output,
        total_cache_read_tokens: aggregate.cache_read,
        total_cache_write_tokens: aggregate.cache_write,
        api_value_usd: aggregate.value,
        unpriced_models,
    }))
}

#[tauri::command]
pub fn get_subscriptions(state: State<AppState>) -> Result<Vec<SubscriptionSummary>, String> {
    let _guard = state
        .lock
        .lock()
        .map_err(|_| "settings lock poisoned".to_string())?;
    let conn = db::open_database(&state.db_path).map_err(|error| error.to_string())?;
    settings::seed_defaults(&conn, chrono::Utc::now().timestamp())
        .map_err(|error| error.to_string())?;

    let mut stmt = conn
        .prepare(
            "
            SELECT agent, label, monthly_cost_usd, billing_cycle_day, cost_mode
            FROM subscriptions
            ORDER BY agent
            ",
        )
        .map_err(|error| error.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok(SubscriptionSummary {
                agent: row.get(0)?,
                label: row.get(1)?,
                monthly_cost_usd: row.get(2)?,
                billing_cycle_day: row.get(3)?,
                cost_mode: row.get(4)?,
            })
        })
        .map_err(|error| error.to_string())?;

    let mut subscriptions = Vec::new();
    for row in rows {
        subscriptions.push(row.map_err(|error| error.to_string())?);
    }
    Ok(subscriptions)
}

#[tauri::command]
pub fn get_app_config(state: State<AppState>) -> Result<AppConfigSummary, String> {
    let _guard = state
        .lock
        .lock()
        .map_err(|_| "settings lock poisoned".to_string())?;
    let conn = db::open_database(&state.db_path).map_err(|error| error.to_string())?;
    let pet_image_path = conn
        .query_row(
            "SELECT value FROM app_config WHERE key = 'pet_image_path'",
            [],
            |row| row.get::<_, String>(0),
        )
        .ok()
        .filter(|value| !value.trim().is_empty());

    Ok(AppConfigSummary { pet_image_path })
}

#[tauri::command]
pub fn save_app_config(
    state: State<AppState>,
    request: SaveAppConfigRequest,
) -> Result<(), String> {
    let _guard = state
        .lock
        .lock()
        .map_err(|_| "settings lock poisoned".to_string())?;
    let conn = db::open_database(&state.db_path).map_err(|error| error.to_string())?;
    let value = request.pet_image_path.unwrap_or_default();
    conn.execute(
        "
        INSERT INTO app_config (key, value)
        VALUES ('pet_image_path', ?1)
        ON CONFLICT(key) DO UPDATE SET value = excluded.value
        ",
        [value.trim()],
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn save_subscription(
    state: State<AppState>,
    request: SaveSubscriptionRequest,
) -> Result<(), String> {
    let _guard = state
        .lock
        .lock()
        .map_err(|_| "settings lock poisoned".to_string())?;
    let conn = db::open_database(&state.db_path).map_err(|error| error.to_string())?;
    conn.execute(
        "
        INSERT INTO subscriptions (agent, label, monthly_cost_usd, billing_cycle_day, cost_mode)
        VALUES (?1, ?2, ?3, ?4, ?5)
        ON CONFLICT(agent) DO UPDATE SET
          label = excluded.label,
          monthly_cost_usd = excluded.monthly_cost_usd,
          billing_cycle_day = excluded.billing_cycle_day,
          cost_mode = excluded.cost_mode
        ",
        rusqlite::params![
            request.agent,
            request.label,
            request.monthly_cost_usd,
            request.billing_cycle_day,
            request.cost_mode
        ],
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn start_window_drag(window: WebviewWindow) -> Result<(), String> {
    window.start_dragging().map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        db::{insert_usage_event, upsert_model_price},
        usage::models::{Agent, InputTokenSemantics},
    };

    #[test]
    fn usage_summary_filters_events_to_current_month_window() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        db::migrate(&conn).unwrap();
        upsert_model_price(
            &conn,
            &ModelPrice {
                model: "gpt-5.4".to_string(),
                input_usd_per_million: "1".to_string(),
                output_usd_per_million: "10".to_string(),
                cache_read_usd_per_million: "0".to_string(),
                cache_write_usd_per_million: "0".to_string(),
                source: "test".to_string(),
                effective_at: 0,
            },
        )
        .unwrap();

        insert_usage_event(
            &conn,
            &UsageEvent {
                id: "last-month".to_string(),
                agent: Agent::Codex,
                source: "session_log".to_string(),
                session_id: None,
                model: "gpt-5.4".to_string(),
                input_tokens: 1_000_000,
                output_tokens: 0,
                cache_read_tokens: 0,
                cache_write_tokens: 0,
                input_token_semantics: InputTokenSemantics::Fresh,
                occurred_at: 1_786_726_800,
            },
        )
        .unwrap();
        insert_usage_event(
            &conn,
            &UsageEvent {
                id: "this-month".to_string(),
                agent: Agent::Codex,
                source: "session_log".to_string(),
                session_id: None,
                model: "gpt-5.4".to_string(),
                input_tokens: 2_000_000,
                output_tokens: 100_000,
                cache_read_tokens: 0,
                cache_write_tokens: 0,
                input_token_semantics: InputTokenSemantics::Fresh,
                occurred_at: 1_789_318_800,
            },
        )
        .unwrap();

        let mut subscriptions = HashMap::new();
        subscriptions.insert("codex".to_string(), (20.0, "subscription".to_string(), 1));
        let codex =
            usage_summary_for_period(&conn, "codex", 1_788_915_600, 1_791_594_000, &subscriptions)
                .unwrap()
                .unwrap();

        assert_eq!(codex.total_input_tokens, 2_000_000);
        assert_eq!(codex.total_output_tokens, 100_000);
        assert!((codex.api_value_usd - 3.0).abs() < 0.0001);
    }

    #[test]
    fn billing_period_window_uses_subscription_cycle_day() {
        let now = chrono::DateTime::parse_from_rfc3339("2026-08-10T12:00:00+08:00")
            .unwrap()
            .timestamp();

        let (start, end) = billing_period_epoch_window(now, 15);

        assert_eq!(
            chrono::DateTime::from_timestamp(start, 0)
                .unwrap()
                .to_rfc3339(),
            "2026-07-14T16:00:00+00:00"
        );
        assert_eq!(
            chrono::DateTime::from_timestamp(end, 0)
                .unwrap()
                .to_rfc3339(),
            "2026-08-14T16:00:00+00:00"
        );
    }
}
