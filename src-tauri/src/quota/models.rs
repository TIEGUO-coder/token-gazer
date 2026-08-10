use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RateWindowSnapshot {
    pub used_percent: f64,
    pub window_minutes: Option<i64>,
    pub resets_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExtraQuotaWindow {
    pub title: String,
    pub used_percent: f64,
    pub window_minutes: Option<i64>,
    pub resets_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CodexQuotaSnapshot {
    pub source: String,
    pub primary: Option<RateWindowSnapshot>,
    pub secondary: Option<RateWindowSnapshot>,
    pub credit_balance: Option<String>,
    pub credit_limit: Option<String>,
    pub credit_used: Option<String>,
    pub credit_remaining_percent: Option<f64>,
    pub credit_resets_at: Option<i64>,
    pub plan_type: Option<String>,
    pub extra_windows: Vec<ExtraQuotaWindow>,
    pub updated_at: i64,
}
